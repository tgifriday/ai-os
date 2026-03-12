use aios_kernel::process;

pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let mut signal = 15i32;
    let mut pid: Option<i32> = None;

    for arg in args {
        if arg.starts_with('-') {
            let s = arg.trim_start_matches('-');
            if let Ok(sig) = s.parse::<i32>() {
                signal = sig;
            }
        } else if let Ok(p) = arg.parse::<i32>() {
            pid = Some(p);
            break;
        }
    }

    let pid = match pid {
        Some(p) => p,
        None => return crate::CommandOutput::error("kill: PID required\n".into(), 1),
    };

    match process::send_signal(pid, signal) {
        Ok(()) => crate::CommandOutput::success(String::new()),
        Err(e) => crate::CommandOutput::error(format!("kill: {}\n", e), 1),
    }
}
