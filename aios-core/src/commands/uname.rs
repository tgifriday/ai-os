pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);

    let show_all = parsed.has('a');
    let show_sysname = parsed.has('s') || (!show_all && parsed.flags.is_empty());
    let show_nodename = parsed.has('n');
    let show_release = parsed.has('r');
    let show_machine = parsed.has('m');

    let sysname = std::env::consts::OS;
    let nodename = aios_kernel::network::get_hostname();
    let machine = std::env::consts::ARCH;

    let release = get_kernel_release();

    let mut parts = Vec::new();

    if show_all || show_sysname {
        parts.push(capitalize(sysname));
    }
    if show_all || show_nodename {
        parts.push(nodename.clone());
    }
    if show_all || show_release {
        parts.push(release.clone());
    }
    if show_all || show_machine {
        parts.push(machine.to_string());
    }

    if parts.is_empty() {
        parts.push(capitalize(sysname));
    }

    let output = format!("{}\n", parts.join(" "));

    let structured = serde_json::json!({
        "sysname": sysname,
        "nodename": nodename,
        "release": release,
        "machine": machine,
    });

    crate::CommandOutput::success_structured(output, structured)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn get_kernel_release() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/version")
            .ok()
            .and_then(|v| v.split_whitespace().nth(2).map(String::from))
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}
