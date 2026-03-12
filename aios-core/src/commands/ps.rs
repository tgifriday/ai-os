use aios_kernel::process;
use serde_json::json;

pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let processes = match process::list_processes() {
        Ok(p) => p,
        Err(e) => return crate::CommandOutput::error(format!("ps: {}\n", e), 1),
    };

    let mut lines = vec![format!(
        "{:>8} {:>8} {:>8} {:>8} {}",
        "PID", "PPID", "STATE", "MEM", "CMD"
    )];
    let mut json_procs = Vec::new();

    for p in &processes {
        let mem = format!("{}K", p.memory_kb);
        let cmd = if p.command.len() > 40 {
            format!("{}...", &p.command[..37])
        } else {
            p.command.clone()
        };
        lines.push(format!(
            "{:>8} {:>8} {:>8} {:>8} {}",
            p.pid, p.ppid, p.state, mem, cmd
        ));
        json_procs.push(json!({
            "pid": p.pid,
            "ppid": p.ppid,
            "state": p.state,
            "memory_kb": p.memory_kb,
            "command": p.command,
            "name": p.name
        }));
    }

    let stdout = lines.join("\n");
    let structured = json!({ "processes": json_procs });
    crate::CommandOutput::success_structured(stdout, structured)
}
