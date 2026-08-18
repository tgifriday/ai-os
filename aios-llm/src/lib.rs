pub mod backend;
pub mod cloud;
pub mod config;
pub mod context;
pub mod local;
#[cfg(feature = "local-inference")]
pub mod local_llama;
pub mod network;
pub mod router;
pub mod usage;

pub use backend::{CompletionRequest, CompletionResponse, LlmBackend, Message, MessageRole};
pub use config::LlmConfig;
pub use context::{ContextManager, OsState};
pub use router::LlmRouter;
pub use usage::UsageTracker;
