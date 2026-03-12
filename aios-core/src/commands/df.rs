use aios_kernel::fs::{get_disk_usage, DiskUsage};
use serde_json::json;

fn human_size(n: u64) -> String {
    const K: u64 = 1024;
    const M: u64 = K * 1024;
    const G: u64 = M * 1024;
    const T: u64 = G * 1024;
    if n >= T {
        format!("{:.1}T", n as f64 / T as f64)
    } else if n >= G {
        format!("{:.1}G", n as f64 / G as f64)
    } else if n >= M {
        format!("{:.1}M", n as f64 / M as f64)
    } else if n >= K {
        format!("{:.1}K", n as f64 / K as f64)
    } else {
        format!("{n}")
    }
}

fn format_row(d: &DiskUsage, human: bool) -> String {
    let use_pct = if d.total_bytes > 0 {
        (d.used_bytes as f64 / d.total_bytes as f64 * 100.0) as u64
    } else {
        0
    };
    let (size, used, avail) = if human {
        (
            human_size(d.total_bytes),
            human_size(d.used_bytes),
            human_size(d.available_bytes),
        )
    } else {
        (
            d.total_bytes.to_string(),
            d.used_bytes.to_string(),
            d.available_bytes.to_string(),
        )
    };
    format!(
        "{:<20} {:>10} {:>10} {:>10} {:>5}% {}",
        d.filesystem,
        size,
        used,
        avail,
        use_pct,
        d.mount_point
    )
}

pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let human = parsed.has_any('h', "human-readable");
    let path_args = parsed.positional.clone();

    let all_disks = match get_disk_usage() {
        Ok(d) => d,
        Err(e) => return crate::CommandOutput::error(e.to_string(), 1),
    };

    let disks: Vec<&DiskUsage> = if path_args.is_empty() {
        all_disks.iter().collect()
    } else {
        let mut matched = Vec::new();
        for path_arg in &path_args {
            let resolved = std::fs::canonicalize(path_arg)
                .unwrap_or_else(|_| std::path::PathBuf::from(path_arg));
            let resolved_str = resolved.to_string_lossy();
            if let Some(best) = all_disks
                .iter()
                .filter(|d| resolved_str.starts_with(&d.mount_point))
                .max_by_key(|d| d.mount_point.len())
            {
                if !matched.iter().any(|m: &&DiskUsage| m.mount_point == best.mount_point) {
                    matched.push(best);
                }
            }
        }
        matched
    };

    let header = format!(
        "{:<20} {:>10} {:>10} {:>10} {:>5} {}",
        "Filesystem", "Size", "Used", "Avail", "Use%", "Mounted"
    );
    let mut lines = vec![header];
    for d in &disks {
        lines.push(format_row(d, human));
    }
    let stdout = lines.join("\n");
    let structured = json!({
        "disks": disks.iter().map(|d| {
            json!({
                "filesystem": d.filesystem,
                "total_bytes": d.total_bytes,
                "used_bytes": d.used_bytes,
                "available_bytes": d.available_bytes,
                "mount_point": d.mount_point
            })
        }).collect::<Vec<_>>()
    });
    crate::CommandOutput::success_structured(stdout, structured)
}
