use aios_kernel::memory::get_uptime;
use serde_json::json;

pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let (uptime_secs, _) = match get_uptime() {
        Ok(u) => u,
        Err(e) => return crate::CommandOutput::error(e.to_string(), 1),
    };
    let days = (uptime_secs / 86400.0) as u64;
    let remainder = uptime_secs % 86400.0;
    let hours = (remainder / 3600.0) as u64;
    let mins = ((remainder % 3600.0) / 60.0) as u64;
    let stdout = if days > 0 {
        format!("up {} days, {:02}:{:02}", days, hours, mins)
    } else {
        format!("up {:02}:{:02}", hours, mins)
    };
    let structured = json!({ "uptime_seconds": uptime_secs });
    crate::CommandOutput::success_structured(stdout, structured)
}
