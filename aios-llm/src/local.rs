// Will use llama-cpp-rs bindings for local GGUF model inference.

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::backend::{CompletionRequest, CompletionResponse, LlmBackend};
use crate::config::LocalConfig;

pub struct LocalBackend {
    config: LocalConfig,
}

impl LocalBackend {
    pub fn new(config: LocalConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl LlmBackend for LocalBackend {
    async fn complete(&self, _request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        anyhow::bail!(
            "Local model not loaded - configure model_path in llm.toml (current: {})",
            self.config.model_path
        )
    }

    async fn stream_complete(
        &self,
        _request: CompletionRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        anyhow::bail!(
            "Local model not loaded - configure model_path in llm.toml (current: {})",
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
        false
    }
}
