#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::sys::wait::{self, WaitStatus};
#[cfg(unix)]
use nix::unistd::{self, ForkResult, Pid};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("fork failed: {0}")]
    ForkFailed(#[from] nix::Error),
    #[error("exec failed: {0}")]
    ExecFailed(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("process not found: {0}")]
    NotFound(i32),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub state: String,
    pub memory_kb: u64,
    pub cpu_time: String,
    pub command: String,
}

#[cfg(unix)]
pub fn fork_and_exec(program: &str, args: &[&str]) -> Result<i32, ProcessError> {
    let c_program = CString::new(program).map_err(|_| ProcessError::InvalidPath(program.to_string()))?;

    let mut c_args: Vec<CString> = vec![c_program.clone()];
    for arg in args {
        c_args.push(CString::new(*arg).map_err(|_| ProcessError::InvalidPath(arg.to_string()))?);
    }

    match unsafe { unistd::fork() }? {
        ForkResult::Parent { child } => Ok(child.as_raw()),
        ForkResult::Child => {
            let c_refs: Vec<&std::ffi::CStr> = c_args.iter().map(|s| s.as_c_str()).collect();
            unistd::execvp(&c_program, &c_refs).ok();
            std::process::exit(127);
        }
    }
}

#[cfg(unix)]
pub fn wait_for_pid(pid: i32) -> Result<i32, ProcessError> {
    let pid = Pid::from_raw(pid);
    match wait::waitpid(pid, None)? {
        WaitStatus::Exited(_, code) => Ok(code),
        WaitStatus::Signaled(_, sig, _) => Ok(128 + sig as i32),
        _ => Ok(-1),
    }
}

#[cfg(unix)]
pub fn send_signal(pid: i32, sig: i32) -> Result<(), ProcessError> {
    let signal = Signal::try_from(sig).map_err(|e| ProcessError::ExecFailed(e.to_string()))?;
    signal::kill(Pid::from_raw(pid), signal)?;
    Ok(())
}

#[cfg(unix)]
pub fn list_processes() -> Result<Vec<ProcessInfo>, ProcessError> {
    let proc_dir = Path::new("/proc");
    if proc_dir.exists() {
        return list_processes_linux();
    }

    #[cfg(target_os = "macos")]
    {
        return list_processes_macos();
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

#[cfg(unix)]
fn list_processes_linux() -> Result<Vec<ProcessInfo>, ProcessError> {
    let mut processes = Vec::new();
    let proc_dir = Path::new("/proc");

    let entries = std::fs::read_dir(proc_dir)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<i32>() {
            if let Ok(info) = read_process_info(pid) {
                processes.push(info);
            }
        }
    }

    Ok(processes)
}

#[cfg(target_os = "macos")]
fn list_processes_macos() -> Result<Vec<ProcessInfo>, ProcessError> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-axo", "pid,ppid,stat,rss,comm"])
        .output()
        .map_err(|e| ProcessError::Io(e))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let pid: i32 = match parts[0].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let ppid: i32 = parts[1].parse().unwrap_or(0);
        let state = parts[2].to_string();
        let memory_kb: u64 = parts[3].parse().unwrap_or(0);
        let command = parts[4..].join(" ");
        let name = command
            .rsplit('/')
            .next()
            .unwrap_or(&command)
            .to_string();

        processes.push(ProcessInfo {
            pid,
            ppid,
            name,
            state,
            memory_kb,
            cpu_time: "0:00".to_string(),
            command,
        });
    }

    Ok(processes)
}

#[cfg(unix)]
fn read_process_info(pid: i32) -> Result<ProcessInfo, ProcessError> {
    let stat_path = format!("/proc/{}/stat", pid);
    let stat_content = std::fs::read_to_string(&stat_path)?;
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let cmdline = std::fs::read_to_string(&cmdline_path)
        .unwrap_or_default()
        .replace('\0', " ")
        .trim()
        .to_string();

    let parts: Vec<&str> = stat_content.split_whitespace().collect();
    if parts.len() < 24 {
        return Err(ProcessError::NotFound(pid));
    }

    let name = parts[1].trim_matches(|c| c == '(' || c == ')').to_string();
    let state = parts[2].to_string();
    let ppid: i32 = parts[3].parse().unwrap_or(0);
    let utime: u64 = parts[13].parse().unwrap_or(0);
    let stime: u64 = parts[14].parse().unwrap_or(0);

    let status_path = format!("/proc/{}/status", pid);
    let memory_kb = if let Ok(status) = std::fs::read_to_string(&status_path) {
        status
            .lines()
            .find(|l| l.starts_with("VmRSS:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
    } else {
        0
    };

    let total_time = utime + stime;
    let seconds = total_time / 100;
    let cpu_time = format!("{}:{:02}", seconds / 60, seconds % 60);

    Ok(ProcessInfo {
        pid,
        ppid,
        name,
        state,
        memory_kb,
        cpu_time,
        command: if cmdline.is_empty() {
            format!("[{}]", parts[1].trim_matches(|c| c == '(' || c == ')'))
        } else {
            cmdline
        },
    })
}

#[cfg(unix)]
pub fn get_current_pid() -> i32 {
    unistd::getpid().as_raw()
}

#[cfg(unix)]
pub fn get_parent_pid() -> i32 {
    unistd::getppid().as_raw()
}

#[cfg(not(unix))]
pub fn fork_and_exec(_program: &str, _args: &[&str]) -> Result<i32, ProcessError> {
    Err(ProcessError::ExecFailed("fork not supported on this platform".to_string()))
}

#[cfg(not(unix))]
pub fn wait_for_pid(_pid: i32) -> Result<i32, ProcessError> {
    Err(ProcessError::NotFound(_pid))
}

#[cfg(not(unix))]
pub fn send_signal(_pid: i32, _sig: i32) -> Result<(), ProcessError> {
    Err(ProcessError::ExecFailed("signals not supported on this platform".to_string()))
}

#[cfg(not(unix))]
pub fn list_processes() -> Result<Vec<ProcessInfo>, ProcessError> {
    Ok(Vec::new())
}

#[cfg(not(unix))]
pub fn get_current_pid() -> i32 {
    std::process::id() as i32
}

#[cfg(not(unix))]
pub fn get_parent_pid() -> i32 {
    0
}
