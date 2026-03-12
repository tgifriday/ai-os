use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDevice {
    pub name: String,
    pub size_bytes: u64,
    pub removable: bool,
    pub device_type: String,
}

pub fn list_block_devices() -> Result<Vec<BlockDevice>, DeviceError> {
    let sys_block = Path::new("/sys/block");
    if sys_block.exists() {
        return list_block_devices_linux(sys_block);
    }

    #[cfg(target_os = "macos")]
    {
        return list_block_devices_macos();
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

fn list_block_devices_linux(sys_block: &Path) -> Result<Vec<BlockDevice>, DeviceError> {
    let mut devices = Vec::new();

    if let Ok(entries) = std::fs::read_dir(sys_block) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let dev_path = entry.path();

            let size_str = std::fs::read_to_string(dev_path.join("size")).unwrap_or_default();
            let size_sectors: u64 = size_str.trim().parse().unwrap_or(0);
            let size_bytes = size_sectors * 512;

            let removable_str =
                std::fs::read_to_string(dev_path.join("removable")).unwrap_or_default();
            let removable = removable_str.trim() == "1";

            let device_type = if name.starts_with("sd") {
                "disk"
            } else if name.starts_with("nvme") {
                "nvme"
            } else if name.starts_with("loop") {
                "loop"
            } else {
                "other"
            }
            .to_string();

            devices.push(BlockDevice {
                name,
                size_bytes,
                removable,
                device_type,
            });
        }
    }

    Ok(devices)
}

#[cfg(target_os = "macos")]
fn list_block_devices_macos() -> Result<Vec<BlockDevice>, DeviceError> {
    use std::process::Command;

    let output = Command::new("diskutil")
        .args(["list", "-plist"])
        .output()
        .map_err(|e| DeviceError::Io(e))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    let mut current_disk: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<string>/dev/") {
            let dev = trimmed
                .trim_start_matches("<string>")
                .trim_end_matches("</string>");
            current_disk = Some(dev.trim_start_matches("/dev/").to_string());
        }
    }

    if current_disk.is_none() {
        let output = Command::new("diskutil")
            .arg("list")
            .output()
            .map_err(|e| DeviceError::Io(e))?;

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("/dev/") {
                let name = trimmed
                    .split_whitespace()
                    .next()
                    .unwrap_or(trimmed)
                    .trim_start_matches("/dev/")
                    .to_string();

                let size_bytes = get_macos_disk_size(&format!("/dev/{}", name));
                let removable = name.contains("disk") && !name.contains("s");
                let device_type = if name.starts_with("disk") {
                    "disk"
                } else {
                    "other"
                }
                .to_string();

                devices.push(BlockDevice {
                    name,
                    size_bytes,
                    removable,
                    device_type,
                });
            }
        }
    }

    Ok(devices)
}

#[cfg(target_os = "macos")]
fn get_macos_disk_size(dev: &str) -> u64 {
    use std::process::Command;
    Command::new("diskutil")
        .args(["info", dev])
        .output()
        .ok()
        .and_then(|o| {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .find(|l| l.contains("Disk Size") || l.contains("Total Size"))
                .and_then(|l| {
                    l.split('(')
                        .nth(1)
                        .and_then(|s| {
                            s.split_whitespace()
                                .next()
                                .and_then(|n| n.parse::<u64>().ok())
                        })
                })
        })
        .unwrap_or(0)
}

pub fn get_cpu_info() -> Result<Vec<CpuInfo>, DeviceError> {
    let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            use std::process::Command;
            let brand = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let cores = Command::new("sysctl")
                .args(["-n", "hw.ncpu"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<usize>().ok())
                .unwrap_or(1);

            (0..cores)
                .map(|i| {
                    format!(
                        "processor\t: {}\nmodel name\t: {}\n\n",
                        i, brand
                    )
                })
                .collect::<String>()
        } else {
            String::new()
        }
    });

    let mut cpus = Vec::new();
    let mut current = CpuInfo::default();

    for line in content.lines() {
        if line.is_empty() {
            if !current.model_name.is_empty() {
                cpus.push(current.clone());
            }
            current = CpuInfo::default();
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim();
        let value = parts[1].trim().to_string();
        match key {
            "processor" => current.id = value.parse().unwrap_or(0),
            "model name" => current.model_name = value,
            "cpu MHz" => current.mhz = value.parse().unwrap_or(0.0),
            "cache size" => current.cache_size = value,
            "cpu cores" => current.cores = value.parse().unwrap_or(1),
            _ => {}
        }
    }

    if !current.model_name.is_empty() {
        cpus.push(current);
    }

    Ok(cpus)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    pub id: u32,
    pub model_name: String,
    pub mhz: f64,
    pub cache_size: String,
    pub cores: u32,
}
