use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemoryInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }

    pub fn swap_used_kb(&self) -> u64 {
        self.swap_total_kb.saturating_sub(self.swap_free_kb)
    }

    pub fn usage_percent(&self) -> f64 {
        if self.total_kb == 0 {
            return 0.0;
        }
        (self.used_kb() as f64 / self.total_kb as f64) * 100.0
    }
}

pub fn get_memory_info() -> Result<MemoryInfo, MemoryError> {
    let content = std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_else(|_| fake_meminfo());

    let mut info = MemoryInfo {
        total_kb: 0,
        free_kb: 0,
        available_kb: 0,
        buffers_kb: 0,
        cached_kb: 0,
        swap_total_kb: 0,
        swap_free_kb: 0,
    };

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let value: u64 = parts[1].parse().unwrap_or(0);
        match parts[0] {
            "MemTotal:" => info.total_kb = value,
            "MemFree:" => info.free_kb = value,
            "MemAvailable:" => info.available_kb = value,
            "Buffers:" => info.buffers_kb = value,
            "Cached:" => info.cached_kb = value,
            "SwapTotal:" => info.swap_total_kb = value,
            "SwapFree:" => info.swap_free_kb = value,
            _ => {}
        }
    }

    Ok(info)
}

fn fake_meminfo() -> String {
    use std::process::Command;
    if cfg!(target_os = "macos") {
        let page_size = 4096u64;
        let output = Command::new("vm_stat").output().ok();
        if let Some(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut free_pages = 0u64;
            let mut active_pages = 0u64;
            let mut inactive_pages = 0u64;
            let mut speculative_pages = 0u64;
            let mut wired_pages = 0u64;

            for line in text.lines() {
                let val = line
                    .split(':')
                    .nth(1)
                    .map(|v| v.trim().trim_end_matches('.'))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                if line.contains("Pages free") {
                    free_pages = val;
                } else if line.contains("Pages active") {
                    active_pages = val;
                } else if line.contains("Pages inactive") {
                    inactive_pages = val;
                } else if line.contains("Pages speculative") {
                    speculative_pages = val;
                } else if line.contains("Pages wired") {
                    wired_pages = val;
                }
            }

            let total_pages = free_pages + active_pages + inactive_pages + speculative_pages + wired_pages;
            let total_kb = total_pages * page_size / 1024;
            let free_kb = free_pages * page_size / 1024;
            let available_kb = (free_pages + inactive_pages) * page_size / 1024;

            return format!(
                "MemTotal:       {} kB\nMemFree:        {} kB\nMemAvailable:   {} kB\nBuffers:        0 kB\nCached:         {} kB\nSwapTotal:      0 kB\nSwapFree:       0 kB\n",
                total_kb, free_kb, available_kb, inactive_pages * page_size / 1024
            );
        }
    }
    "MemTotal: 0 kB\nMemFree: 0 kB\nMemAvailable: 0 kB\n".to_string()
}

pub fn get_uptime() -> Result<(f64, f64), MemoryError> {
    let content = std::fs::read_to_string("/proc/uptime").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            use std::process::Command;
            let output = Command::new("sysctl")
                .args(["-n", "kern.boottime"])
                .output()
                .ok();
            if let Some(out) = output {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(sec_str) = text.split("sec = ").nth(1) {
                    if let Some(sec_val) = sec_str.split(',').next() {
                        if let Ok(boot_sec) = sec_val.trim().parse::<u64>() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let uptime = now.saturating_sub(boot_sec) as f64;
                            return format!("{} 0.0", uptime);
                        }
                    }
                }
            }
            "0.0 0.0".to_string()
        } else {
            "0.0 0.0".to_string()
        }
    });

    let parts: Vec<&str> = content.split_whitespace().collect();
    let uptime_secs: f64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let idle_secs: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Ok((uptime_secs, idle_secs))
}
