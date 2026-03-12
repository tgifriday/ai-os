use aios_kernel::memory;
use aios_kernel::process;
use serde_json::json;

pub fn run(_args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let processes = match process::list_processes() {
        Ok(p) => p,
        Err(e) => return crate::CommandOutput::error(format!("top: {}\n", e), 1),
    };

    let mem_info = match memory::get_memory_info() {
        Ok(m) => m,
        Err(e) => return crate::CommandOutput::error(format!("top: {}\n", e), 1),
    };

    let used_mb = mem_info.used_kb() / 1024;
    let total_mb = mem_info.total_kb / 1024;
    let swap_used = mem_info.swap_used_kb() / 1024;
    let swap_total = mem_info.swap_total_kb / 1024;

    let mut lines = vec![
        format!(
            "Mem: {}M total, {}M used, {}M free  Swap: {}M total, {}M used",
            total_mb,
            used_mb,
            mem_info.free_kb / 1024,
            swap_total,
            swap_used
        ),
        String::new(),
        format!(
            "{:>8} {:>8} {:>8} {:>8} {}",
            "PID", "PPID", "STATE", "MEM", "CMD"
        ),
    ];

    let mut sorted: Vec<_> = processes.into_iter().collect();
    sorted.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));
    let top: Vec<_> = sorted.into_iter().take(20).collect();

    let mut json_procs = Vec::new();
    for p in &top {
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
    let structured = json!({
        "memory": {
            "total_kb": mem_info.total_kb,
            "used_kb": mem_info.used_kb(),
            "free_kb": mem_info.free_kb,
            "swap_total_kb": mem_info.swap_total_kb,
            "swap_used_kb": mem_info.swap_used_kb()
        },
        "processes": json_procs
    });
    crate::CommandOutput::success_structured(stdout, structured)
}
