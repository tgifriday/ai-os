use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiosConfig {
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub services: ServicesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    #[serde(default = "default_hostname")]
    pub hostname: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    #[serde(default = "default_true")]
    pub llm_daemon: bool,
    #[serde(default = "default_true")]
    pub knowledge_daemon: bool,
    #[serde(default = "default_true")]
    pub shell_sessions: bool,
    #[serde(default)]
    pub llm_daemon_config: Option<ServiceOverride>,
    #[serde(default)]
    pub knowledge_daemon_config: Option<ServiceOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceOverride {
    #[serde(default = "default_true")]
    pub auto_start: bool,
    #[serde(default = "default_true")]
    pub restart_on_failure: bool,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

fn default_hostname() -> String {
    "aios".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_data_dir() -> String {
    "/var/aios".to_string()
}
fn default_true() -> bool {
    true
}
fn default_max_restarts() -> u32 {
    3
}

impl AiosConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for AiosConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig::default(),
            services: ServicesConfig::default(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            hostname: default_hostname(),
            log_level: default_log_level(),
            data_dir: default_data_dir(),
        }
    }
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            llm_daemon: true,
            knowledge_daemon: true,
            shell_sessions: true,
            llm_daemon_config: None,
            knowledge_daemon_config: None,
        }
    }
}
