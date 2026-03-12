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
use crate::config::CloudProviderConfig;

pub struct OpenAiBackend {
    client: Client,
    config: CloudProviderConfig,
}

impl OpenAiBackend {
    pub fn new(config: CloudProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn api_key(&self) -> Option<String> {
        std::env::var(&self.config.api_key_env).ok()
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
impl LlmBackend for OpenAiBackend {
    async fn complete(&self, request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let api_key = self
            .api_key()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set in {}", self.config.api_key_env))?;

        let messages = self.format_messages(&request);
        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }

        debug!(model = %self.config.model, "sending openai completion request");

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let data: Value = resp.json().await?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let usage = data.get("usage").map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
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
        let api_key = self
            .api_key()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set in {}", self.config.api_key_env))?;

        let messages = self.format_messages(&request);
        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
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

                            if line.is_empty() || !line.starts_with("data: ") {
                                continue;
                            }
                            let payload = &line["data: ".len()..];
                            if payload == "[DONE]" {
                                return;
                            }
                            match serde_json::from_str::<Value>(payload) {
                                Ok(v) => {
                                    if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                                        yield Ok(delta.to_string());
                                    }
                                }
                                Err(e) => {
                                    yield Err(anyhow::anyhow!("failed to parse SSE chunk: {e}"));
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
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn is_available(&self) -> bool {
        self.config.enabled && self.api_key().is_some()
    }
}

pub struct AnthropicBackend {
    client: Client,
    config: CloudProviderConfig,
}

impl AnthropicBackend {
    pub fn new(config: CloudProviderConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    fn api_key(&self) -> Option<String> {
        std::env::var(&self.config.api_key_env).ok()
    }

    fn format_messages(&self, request: &CompletionRequest) -> Vec<Value> {
        request
            .messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "user",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                json!({
                    "role": role,
                    "content": &msg.content,
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn complete(&self, request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let api_key = self
            .api_key()
            .ok_or_else(|| anyhow::anyhow!("Anthropic API key not set in {}", self.config.api_key_env))?;

        let messages = self.format_messages(&request);
        let max_tokens = request.max_tokens.unwrap_or(4096);

        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
            "max_tokens": max_tokens,
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = json!(system);
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }

        debug!(model = %self.config.model, "sending anthropic completion request");

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let data: Value = resp.json().await?;

        let content = data["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or_default()
            .to_string();

        let usage = data.get("usage").map(|u| TokenUsage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
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
        let api_key = self
            .api_key()
            .ok_or_else(|| anyhow::anyhow!("Anthropic API key not set in {}", self.config.api_key_env))?;

        let messages = self.format_messages(&request);
        let max_tokens = request.max_tokens.unwrap_or(4096);

        let mut body = json!({
            "model": &self.config.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "stream": true,
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = json!(system);
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = json!(temperature);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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

                            if line.is_empty() || !line.starts_with("data: ") {
                                continue;
                            }
                            let payload = &line["data: ".len()..];
                            match serde_json::from_str::<Value>(payload) {
                                Ok(v) => {
                                    if v["type"] == "content_block_delta" {
                                        if let Some(text) = v["delta"]["text"].as_str() {
                                            yield Ok(text.to_string());
                                        }
                                    }
                                }
                                Err(_) => {}
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
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn is_available(&self) -> bool {
        self.config.enabled && self.api_key().is_some()
    }
}
