use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use tracing::{debug, warn};

use crate::backend::TokenUsage;
use crate::config::MissionControlSection;

/// A single usage event recorded after each LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub timestamp: DateTime<Utc>,
    pub backend: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: u64,
    pub estimated_cost_usd: Option<f64>,
}

/// Accumulated stats for display.
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_queries: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_latency_ms: u64,
    pub estimated_cost_usd: f64,
    /// Per-model breakdown: (backend, model) -> (queries, tokens, cost)
    pub by_model: Vec<ModelStats>,
}

#[derive(Debug, Clone)]
pub struct ModelStats {
    pub backend: String,
    pub model: String,
    pub queries: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
}

/// Tracks token usage across a session. Optionally writes JSONL to disk.
pub struct UsageTracker {
    events: Vec<UsageEvent>,
    config: MissionControlSection,
    log_path: Option<PathBuf>,
}

impl UsageTracker {
    pub fn new(config: MissionControlSection) -> Self {
        let log_path = if config.enabled {
            let path = PathBuf::from(&config.log_path);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            Some(path)
        } else {
            None
        };

        Self {
            events: Vec::new(),
            config,
            log_path,
        }
    }

    /// Record a usage event from a completed LLM request.
    pub fn record(
        &mut self,
        backend: &str,
        model: &str,
        usage: Option<&TokenUsage>,
        latency_ms: u64,
    ) {
        let (prompt_tokens, completion_tokens, total_tokens) = match usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens, u.total_tokens),
            None => (0, 0, 0),
        };

        let estimated_cost_usd = estimate_cost(backend, model, prompt_tokens, completion_tokens);

        let event = UsageEvent {
            timestamp: Utc::now(),
            backend: backend.to_string(),
            model: model.to_string(),
            prompt_tokens,
            completion_tokens,
            total_tokens,
            latency_ms,
            estimated_cost_usd,
        };

        debug!(
            backend = %event.backend,
            model = %event.model,
            tokens = event.total_tokens,
            latency_ms = event.latency_ms,
            "mission control: recorded usage event"
        );

        // Write to JSONL log if enabled
        if let Some(ref path) = self.log_path {
            self.append_to_log(path, &event);
        }

        // Fire webhook if configured
        if let Some(ref _webhook_url) = self.config.webhook_url {
            // TODO: async webhook push — for now just log that we would send it
            debug!("mission control: webhook configured but async push not yet implemented");
        }

        self.events.push(event);
    }

    fn append_to_log(&self, path: &PathBuf, event: &UsageEvent) {
        // Check file size limit
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() / (1024 * 1024);
            if size_mb >= self.config.max_log_mb {
                debug!("mission control: log file at {}MB limit, rotating", size_mb);
                self.rotate_log(path);
            }
        }

        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut file) => {
                if let Ok(json) = serde_json::to_string(event) {
                    let _ = writeln!(file, "{}", json);
                }
            }
            Err(e) => {
                warn!("mission control: failed to write log: {}", e);
            }
        }
    }

    fn rotate_log(&self, path: &PathBuf) {
        let rotated = path.with_extension("jsonl.1");
        let _ = std::fs::rename(path, rotated);
    }

    /// Get session stats for display.
    pub fn session_stats(&self) -> SessionStats {
        let mut stats = SessionStats::default();
        let mut model_map: std::collections::HashMap<(String, String), (u64, u64, f64)> =
            std::collections::HashMap::new();

        for event in &self.events {
            stats.total_queries += 1;
            stats.total_prompt_tokens += event.prompt_tokens as u64;
            stats.total_completion_tokens += event.completion_tokens as u64;
            stats.total_tokens += event.total_tokens as u64;
            stats.total_latency_ms += event.latency_ms;
            stats.estimated_cost_usd += event.estimated_cost_usd.unwrap_or(0.0);

            let key = (event.backend.clone(), event.model.clone());
            let entry = model_map.entry(key).or_insert((0, 0, 0.0));
            entry.0 += 1;
            entry.1 += event.total_tokens as u64;
            entry.2 += event.estimated_cost_usd.unwrap_or(0.0);
        }

        stats.by_model = model_map
            .into_iter()
            .map(|((backend, model), (queries, tokens, cost))| ModelStats {
                backend,
                model,
                queries,
                total_tokens: tokens,
                estimated_cost_usd: cost,
            })
            .collect();
        stats.by_model.sort_by(|a, b| b.queries.cmp(&a.queries));

        stats
    }

    /// Format session stats for terminal display.
    pub fn format_session_stats(&self) -> String {
        let stats = self.session_stats();

        if stats.total_queries == 0 {
            return "  No AI queries this session.".to_string();
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "  Session: {} queries, {} tokens",
            stats.total_queries, stats.total_tokens
        ));

        if stats.estimated_cost_usd > 0.0 {
            lines.push(format!(
                "  Estimated cost: ${:.4}",
                stats.estimated_cost_usd
            ));
        }

        let avg_latency = if stats.total_queries > 0 {
            stats.total_latency_ms / stats.total_queries
        } else {
            0
        };
        lines.push(format!(
            "  Avg latency: {}ms | Total: {:.1}s",
            avg_latency,
            stats.total_latency_ms as f64 / 1000.0
        ));

        if stats.by_model.len() > 1 || !stats.by_model.is_empty() {
            lines.push("  By model:".to_string());
            for m in &stats.by_model {
                let cost_str = if m.estimated_cost_usd > 0.0 {
                    format!(" (${:.4})", m.estimated_cost_usd)
                } else {
                    String::new()
                };
                lines.push(format!(
                    "    {} / {} — {} queries, {} tokens{}",
                    m.backend, m.model, m.queries, m.total_tokens, cost_str
                ));
            }
        }

        if let Some(ref path) = self.log_path {
            lines.push(format!("  Log: {}", path.display()));
        }

        lines.join("\n")
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new(MissionControlSection::default())
    }
}

/// Rough cost estimation per 1K tokens. Returns None for local/network (free).
/// Prices are approximate and will drift — this is for ballpark budgeting, not billing.
fn estimate_cost(backend: &str, model: &str, prompt_tokens: u32, completion_tokens: u32) -> Option<f64> {
    let (input_per_1k, output_per_1k) = match backend {
        "openai" => match model {
            m if m.contains("gpt-4o-mini") => (0.00015, 0.0006),
            m if m.contains("gpt-4o") => (0.0025, 0.01),
            m if m.contains("gpt-4-turbo") => (0.01, 0.03),
            m if m.contains("gpt-4") => (0.03, 0.06),
            m if m.contains("gpt-3.5") => (0.0005, 0.0015),
            m if m.contains("o1-mini") => (0.003, 0.012),
            m if m.contains("o1") => (0.015, 0.06),
            _ => (0.0025, 0.01), // default to gpt-4o pricing
        },
        "anthropic" => match model {
            m if m.contains("haiku") => (0.00025, 0.00125),
            m if m.contains("sonnet") => (0.003, 0.015),
            m if m.contains("opus") => (0.015, 0.075),
            _ => (0.003, 0.015), // default to sonnet pricing
        },
        // Local and network backends are free
        _ => return None,
    };

    let cost = (prompt_tokens as f64 / 1000.0) * input_per_1k
        + (completion_tokens as f64 / 1000.0) * output_per_1k;

    Some(cost)
}
