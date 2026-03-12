pub mod commands;
pub mod flags;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub structured: Option<serde_json::Value>,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            structured: None,
            exit_code: 0,
        }
    }

    pub fn success_structured(stdout: String, structured: serde_json::Value) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            structured: Some(structured),
            exit_code: 0,
        }
    }

    pub fn error(stderr: String, exit_code: i32) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            structured: None,
            exit_code,
        }
    }
}
