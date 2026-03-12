use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde_json::{json, Value};
use std::pin::Pin;
use tokio_stream::StreamExt;
use tracing::debug;

use crate::backend::{
    CompletionRequest, CompletionResponse, LlmBackend, MessageRole, TokenUsage,
};
use crate::config::NetworkConfig;

pub struct NetworkBackend {
    client: Client,
    config: NetworkConfig,
}

impl NetworkBackend {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn format_messages(&self, request: &CompletionRequest) -> Vec<Value> {
        let mut messages = Vec::new();
        if let Some(ref system) = request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system,
            }));
        }
        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            messages.push(json!({
                "role": role,
                "content": &msg.content,
            }));
        }
        messages
    }
}

#[async_trait]
impl LlmBackend for NetworkBackend {
    async fn complete(&self, request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let messages = self.format_messages(&request);
        let url = format!("{}/api/chat", self.config.url);

        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(temperature) = request.temperature {
            body["options"] = json!({ "temperature": temperature });
        }

        debug!(model = %self.config.model, url = %url, "sending ollama chat request");

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let data: Value = resp.json().await?;

        let content = data["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let usage = data.get("eval_count").map(|eval| {
            let prompt_tokens = data["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
            let completion_tokens = eval.as_u64().unwrap_or(0) as u32;
            TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }
        });

        Ok(CompletionResponse {
            content,
            model: self.config.model.clone(),
            usage,
        })
    }

    async fn stream_complete(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        let messages = self.format_messages(&request);
        let url = format!("{}/api/chat", self.config.url);

        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(temperature) = request.temperature {
            body["options"] = json!({ "temperature": temperature });
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let byte_stream = resp.bytes_stream();
        let mut buffer = String::new();

        let stream = async_stream::stream! {
            let mut byte_stream = std::pin::pin!(byte_stream);
            loop {
                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<Value>(&line) {
                                Ok(v) => {
                                    if let Some(content) = v["message"]["content"].as_str() {
                                        if !content.is_empty() {
                                            yield Ok(content.to_string());
                                        }
                                    }
                                    if v["done"].as_bool() == Some(true) {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    yield Err(anyhow::anyhow!("failed to parse ollama chunk: {e}"));
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        yield Err(anyhow::anyhow!("stream error: {e}"));
                        return;
                    }
                    None => return,
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "network"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn is_available(&self) -> bool {
        self.config.enabled
    }
}
