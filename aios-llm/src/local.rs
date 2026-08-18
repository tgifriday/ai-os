//! Local, in-process LLM backend.
//!
//! When built with the `local-inference` feature this runs a quantized GGUF
//! model directly inside the shell via an embedded llama.cpp (see
//! [`crate::local_llama`]). Without the feature it is an inert stub so the
//! default build stays lightweight and dependency-free.

use std::path::PathBuf;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::backend::{CompletionRequest, CompletionResponse, LlmBackend};
use crate::config::LocalConfig;

/// Expand a leading `~` to the user's home directory.
pub(crate) fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub struct LocalBackend {
    config: LocalConfig,
}

impl LocalBackend {
    pub fn new(config: LocalConfig) -> Self {
        Self { config }
    }

    /// Render a chat request into a Qwen2.5 ChatML prompt string.
    #[cfg(feature = "local-inference")]
    fn build_prompt(request: &CompletionRequest) -> String {
        let mut prompt = String::new();
        if let Some(system) = &request.system_prompt {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(system);
            prompt.push_str("<|im_end|>\n");
        }
        for message in &request.messages {
            let role = match message.role {
                crate::backend::MessageRole::System => "system",
                crate::backend::MessageRole::User => "user",
                crate::backend::MessageRole::Assistant => "assistant",
            };
            prompt.push_str("<|im_start|>");
            prompt.push_str(role);
            prompt.push('\n');
            prompt.push_str(&message.content);
            prompt.push_str("<|im_end|>\n");
        }
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    fn display_model(&self) -> String {
        expand_tilde(&self.config.model_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| self.config.model_path.clone())
    }
}

#[async_trait]
impl LlmBackend for LocalBackend {
    #[cfg(feature = "local-inference")]
    async fn complete(&self, request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let engine = crate::local_llama::get_or_init_engine(&self.config)?;
        let prompt = Self::build_prompt(&request);
        let max_tokens = request.max_tokens.unwrap_or(512).max(1) as usize;
        let content = engine.generate(prompt, max_tokens).await?;
        Ok(CompletionResponse {
            content,
            model: self.display_model(),
            usage: None,
        })
    }

    #[cfg(not(feature = "local-inference"))]
    async fn complete(&self, _request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        anyhow::bail!(
            "Local inference is not compiled in. Rebuild with `--features local-inference` \
             (see `make build-local`) to run GGUF models in-process. Model path: {}",
            self.config.model_path
        )
    }

    #[cfg(feature = "local-inference")]
    async fn stream_complete(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        // The engine produces a full completion; surface it as a single chunk.
        let response = self.complete(request).await?;
        let stream = async_stream::stream! {
            yield Ok(response.content);
        };
        Ok(Box::pin(stream))
    }

    #[cfg(not(feature = "local-inference"))]
    async fn stream_complete(
        &self,
        _request: CompletionRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        anyhow::bail!(
            "Local inference is not compiled in. Rebuild with `--features local-inference` \
             (see `make build-local`). Model path: {}",
            self.config.model_path
        )
    }

    fn name(&self) -> &str {
        "local"
    }

    fn model_name(&self) -> &str {
        &self.config.model_path
    }

    fn is_available(&self) -> bool {
        #[cfg(feature = "local-inference")]
        {
            self.config.enabled && expand_tilde(&self.config.model_path).exists()
        }
        #[cfg(not(feature = "local-inference"))]
        {
            false
        }
    }
}
