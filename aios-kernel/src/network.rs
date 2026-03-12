use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(unix)]
    #[error("nix error: {0}")]
    Nix(#[from] nix::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[cfg(unix)]
pub fn list_interfaces() -> Result<Vec<NetworkInterface>, NetworkError> {
    if let Ok(content) = std::fs::read_to_string("/proc/net/dev") {
        return list_interfaces_linux(&content);
    }

    #[cfg(target_os = "macos")]
    {
        return list_interfaces_macos();
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(vec![NetworkInterface {
            name: "lo".to_string(),
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
        }])
    }
}

#[cfg(unix)]
fn list_interfaces_linux(content: &str) -> Result<Vec<NetworkInterface>, NetworkError> {
    let mut interfaces = Vec::new();

    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }
        let name = parts[0].trim_end_matches(':').to_string();
        interfaces.push(NetworkInterface {
            name,
            rx_bytes: parts[1].parse().unwrap_or(0),
            rx_packets: parts[2].parse().unwrap_or(0),
            tx_bytes: parts[9].parse().unwrap_or(0),
            tx_packets: parts[10].parse().unwrap_or(0),
        });
    }

    if interfaces.is_empty() {
        interfaces.push(NetworkInterface {
            name: "lo".to_string(),
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
        });
    }

    Ok(interfaces)
}

#[cfg(target_os = "macos")]
fn list_interfaces_macos() -> Result<Vec<NetworkInterface>, NetworkError> {
    use std::process::Command;

    let output = Command::new("netstat")
        .args(["-ibn"])
        .output()
        .map_err(|e| NetworkError::Io(e))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut interfaces = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            continue;
        }
        let name = parts[0].to_string();
        if !seen.insert(name.clone()) {
            continue;
        }

        let (rx_bytes, rx_packets, tx_bytes, tx_packets) = if parts.len() >= 11 {
            (
                parts[6].parse().unwrap_or(0),
                parts[4].parse().unwrap_or(0),
                parts[9].parse().unwrap_or(0),
                parts[7].parse().unwrap_or(0),
            )
        } else {
            (0, 0, 0, 0)
        };

        interfaces.push(NetworkInterface {
            name,
            rx_bytes,
            tx_bytes,
            rx_packets,
            tx_packets,
        });
    }

    if interfaces.is_empty() {
        interfaces.push(NetworkInterface {
            name: "lo0".to_string(),
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
        });
    }

    Ok(interfaces)
}

#[cfg(unix)]
pub fn get_hostname() -> String {
    nix::unistd::gethostname()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(not(unix))]
pub fn get_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(not(unix))]
pub fn list_interfaces() -> Result<Vec<NetworkInterface>, NetworkError> {
    Ok(Vec::new())
}
