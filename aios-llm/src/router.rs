use futures::Stream;
use std::pin::Pin;
use tracing::{debug, warn};

use crate::backend::{CompletionRequest, CompletionResponse, LlmBackend};

pub struct LlmRouter {
    backends: Vec<Box<dyn LlmBackend>>,
}

impl LlmRouter {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    pub fn add_backend(&mut self, backend: Box<dyn LlmBackend>) {
        self.backends.push(backend);
    }

    pub fn available_backends(&self) -> Vec<&str> {
        self.backends
            .iter()
            .filter(|b| b.is_available())
            .map(|b| b.name())
            .collect()
    }

    pub fn backend_info(&self) -> Vec<(&str, &str, bool)> {
        self.backends
            .iter()
            .map(|b| (b.name(), b.model_name(), b.is_available()))
            .collect()
    }

    pub fn clear_backends(&mut self) {
        self.backends.clear();
    }

    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<CompletionResponse> {
        let available: Vec<_> = self.backends.iter().filter(|b| b.is_available()).collect();

        if available.is_empty() {
            anyhow::bail!("no LLM backends available");
        }

        let mut last_err = None;
        for backend in &available {
            debug!(backend = backend.name(), "attempting completion");
            match backend.complete(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!(backend = backend.name(), error = %e, "backend failed, trying next");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("all backends failed")))
    }

    pub async fn stream_complete(
        &self,
        request: CompletionRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>> {
        let available: Vec<_> = self.backends.iter().filter(|b| b.is_available()).collect();

        if available.is_empty() {
            anyhow::bail!("no LLM backends available");
        }

        let mut last_err = None;
        for backend in &available {
            debug!(backend = backend.name(), "attempting stream completion");
            match backend.stream_complete(request.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    warn!(backend = backend.name(), error = %e, "backend failed, trying next");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("all backends failed")))
    }
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new()
    }
}
