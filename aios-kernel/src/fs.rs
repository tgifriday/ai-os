#[cfg(unix)]
use nix::libc;
#[cfg(unix)]
use nix::sys::stat;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(unix)]
    #[error("nix error: {0}")]
    Nix(#[from] nix::Error),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub permissions: u32,
    pub owner_uid: u32,
    pub group_gid: u32,
    pub modified: i64,
    pub accessed: i64,
    pub created: Option<i64>,
    pub nlinks: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub mount_point: String,
}

pub fn list_directory(path: &Path) -> Result<Vec<FileInfo>, FsError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        if let Ok(info) = get_file_info(&entry.path()) {
            entries.push(info);
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

#[cfg(unix)]
pub fn get_file_info(path: &Path) -> Result<FileInfo, FsError> {
    let metadata = std::fs::symlink_metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    Ok(FileInfo {
        name,
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        permissions: metadata.permissions().mode(),
        owner_uid: metadata.uid(),
        group_gid: metadata.gid(),
        modified: metadata.mtime(),
        accessed: metadata.atime(),
        created: metadata.created().ok().map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        }),
        nlinks: metadata.nlink(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
pub fn get_file_info(path: &Path) -> Result<FileInfo, FsError> {
    let metadata = std::fs::symlink_metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    Ok(FileInfo {
        name,
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        permissions: 0o644,
        owner_uid: 0,
        group_gid: 0,
        modified: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        accessed: 0,
        created: metadata.created().ok().map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        }),
        nlinks: 1,
        inode: 0,
    })
}

pub fn read_file(path: &Path) -> Result<String, FsError> {
    Ok(std::fs::read_to_string(path)?)
}

pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>, FsError> {
    Ok(std::fs::read(path)?)
}

pub fn write_file(path: &Path, contents: &[u8]) -> Result<(), FsError> {
    Ok(std::fs::write(path, contents)?)
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<u64, FsError> {
    Ok(std::fs::copy(src, dst)?)
}

pub fn rename(src: &Path, dst: &Path) -> Result<(), FsError> {
    Ok(std::fs::rename(src, dst)?)
}

pub fn remove_file(path: &Path) -> Result<(), FsError> {
    Ok(std::fs::remove_file(path)?)
}

pub fn create_dir(path: &Path, recursive: bool) -> Result<(), FsError> {
    if recursive {
        std::fs::create_dir_all(path)?;
    } else {
        std::fs::create_dir(path)?;
    }
    Ok(())
}

pub fn remove_dir(path: &Path, recursive: bool) -> Result<(), FsError> {
    if recursive {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_dir(path)?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn set_permissions(path: &Path, mode: u32) -> Result<(), FsError> {
    stat::fchmodat(
        None,
        path,
        stat::Mode::from_bits_truncate(mode as u16),
        stat::FchmodatFlags::FollowSymlink,
    )?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_permissions(_path: &Path, _mode: u32) -> Result<(), FsError> {
    Ok(())
}

#[cfg(unix)]
pub fn get_disk_usage() -> Result<Vec<DiskUsage>, FsError> {
    let mounts = read_mount_points();
    let mut disks = Vec::new();

    for (filesystem, mount_point) in &mounts {
        unsafe {
            let c_path = std::ffi::CString::new(mount_point.as_str()).unwrap_or_default();
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                let block_size = stat.f_frsize as u64;
                let blocks = stat.f_blocks as u64;
                let bfree = stat.f_bfree as u64;
                let bavail = stat.f_bavail as u64;
                let total = blocks * block_size;
                if total == 0 {
                    continue;
                }
                disks.push(DiskUsage {
                    filesystem: filesystem.clone(),
                    total_bytes: total,
                    used_bytes: (blocks - bfree) * block_size,
                    available_bytes: bavail * block_size,
                    mount_point: mount_point.clone(),
                });
            }
        }
    }

    Ok(disks)
}

#[cfg(not(unix))]
pub fn get_disk_usage() -> Result<Vec<DiskUsage>, FsError> {
    Ok(Vec::new())
}

#[cfg(unix)]
fn read_mount_points() -> Vec<(String, String)> {
    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        return content
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && (parts[0].starts_with('/') || parts[0] == "tmpfs")
                {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("mount").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            return text
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(" on ").collect();
                    if parts.len() >= 2 {
                        let filesystem = parts[0].to_string();
                        let rest = parts[1];
                        let mount_point = rest
                            .split(" (")
                            .next()
                            .unwrap_or(rest)
                            .to_string();
                        if filesystem.starts_with('/') || filesystem == "devfs" {
                            Some((filesystem, mount_point))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    vec![("/".to_string(), "/".to_string())]
}

pub fn dir_size(path: &Path) -> Result<u64, FsError> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                total += dir_size(&entry.path())?;
            } else {
                total += meta.len();
            }
        }
    } else {
        total = std::fs::metadata(path)?.len();
    }
    Ok(total)
}
