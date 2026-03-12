pub fn run(args: &[&str], _cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let show_human = parsed.has('h');
    let show_megs = parsed.has('m');
    let show_gigs = parsed.has('g');

    match aios_kernel::memory::get_memory_info() {
        Ok(info) => {
            let (total, used, free, available, buffers, cached, swap_total, swap_free, swap_used) = (
                info.total_kb,
                info.used_kb(),
                info.free_kb,
                info.available_kb,
                info.buffers_kb,
                info.cached_kb,
                info.swap_total_kb,
                info.swap_free_kb,
                info.swap_used_kb(),
            );

            let fmt = |kb: u64| -> String {
                if show_human {
                    if kb >= 1_048_576 {
                        format!("{:.1}Gi", kb as f64 / 1_048_576.0)
                    } else if kb >= 1024 {
                        format!("{:.1}Mi", kb as f64 / 1024.0)
                    } else {
                        format!("{}Ki", kb)
                    }
                } else if show_gigs {
                    format!("{:.1}", kb as f64 / 1_048_576.0)
                } else if show_megs {
                    format!("{}", kb / 1024)
                } else {
                    format!("{}", kb)
                }
            };

            let unit_label = if show_human {
                ""
            } else if show_gigs {
                " (GiB)"
            } else if show_megs {
                " (MiB)"
            } else {
                " (KiB)"
            };

            let mut output = String::new();
            output.push_str(&format!(
                "{:<15} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}\n",
                format!("             {}", unit_label),
                "total", "used", "free", "shared", "buff/cache", "available"
            ));
            output.push_str(&format!(
                "{:<15} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}\n",
                "Mem:",
                fmt(total),
                fmt(used),
                fmt(free),
                fmt(0),
                fmt(buffers + cached),
                fmt(available),
            ));
            output.push_str(&format!(
                "{:<15} {:>12} {:>12} {:>12}\n",
                "Swap:",
                fmt(swap_total),
                fmt(swap_used),
                fmt(swap_free),
            ));

            let structured = serde_json::json!({
                "mem": {
                    "total_kb": total,
                    "used_kb": used,
                    "free_kb": free,
                    "available_kb": available,
                    "buffers_kb": buffers,
                    "cached_kb": cached,
                },
                "swap": {
                    "total_kb": swap_total,
                    "used_kb": swap_used,
                    "free_kb": swap_free,
                }
            });

            crate::CommandOutput::success_structured(output, structured)
        }
        Err(e) => crate::CommandOutput::error(format!("free: {}", e), 1),
    }
}
