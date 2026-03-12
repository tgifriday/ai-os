use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub local: LocalConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub cloud: CloudConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default)]
    pub fallback: Vec<String>,
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: u32,
    #[serde(default = "default_stream_responses")]
    pub stream_responses: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_model_path")]
    pub model_path: String,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default)]
    pub gpu_layers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_network_url")]
    pub url: String,
    #[serde(default = "default_network_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub openai: Option<CloudProviderConfig>,
    pub anthropic: Option<CloudProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub model: String,
}

fn default_primary() -> String {
    "network".to_string()
}

fn default_max_context_tokens() -> u32 {
    4096
}

fn default_stream_responses() -> bool {
    true
}

fn default_model_path() -> String {
    "~/.aios/models/default.gguf".to_string()
}

fn default_threads() -> u32 {
    4
}

fn default_network_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_network_model() -> String {
    "llama3".to_string()
}

impl LlmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let config: Self = match ext.as_str() {
            "yaml" | "yml" => serde_yaml::from_str(&content)?,
            "json" => serde_json::from_str(&content)?,
            _ => toml::from_str(&content)?,
        };
        Ok(config)
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            defaults: DefaultsConfig::default(),
            local: LocalConfig::default(),
            network: NetworkConfig::default(),
            cloud: CloudConfig::default(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            primary: default_primary(),
            fallback: vec!["cloud".to_string()],
            max_context_tokens: default_max_context_tokens(),
            stream_responses: default_stream_responses(),
        }
    }
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: default_model_path(),
            threads: default_threads(),
            gpu_layers: 0,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_network_url(),
            model: default_network_model(),
        }
    }
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            openai: None,
            anthropic: None,
        }
    }
}
