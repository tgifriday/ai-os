use aios_kernel::fs::dir_size;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn human_size(n: u64) -> String {
    const K: u64 = 1024;
    const M: u64 = K * 1024;
    const G: u64 = M * 1024;
    if n >= G {
        format!("{:.1}G", n as f64 / G as f64)
    } else if n >= M {
        format!("{:.1}M", n as f64 / M as f64)
    } else if n >= K {
        format!("{:.1}K", n as f64 / K as f64)
    } else {
        format!("{n}")
    }
}

fn walk_dir(path: &Path) -> Vec<(PathBuf, u64)> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && p.file_name().map(|n| n != "." && n != "..").unwrap_or(false) {
                if let Ok(sz) = dir_size(&p) {
                    result.push((p.clone(), sz));
                    result.extend(walk_dir(&p));
                }
            }
        }
    }
    result
}

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let human = parsed.has_any('h', "human-readable");
    let summary_only = parsed.has('s');
    let path = {
        let p = parsed.positional.first().copied().unwrap_or(".");
        if Path::new(p).is_absolute() {
            PathBuf::from(p)
        } else {
            cwd.join(p)
        }
    };
    let total = match dir_size(&path) {
        Ok(s) => s,
        Err(e) => return crate::CommandOutput::error(e.to_string(), 1),
    };
    let mut entries = vec![(path.clone(), total)];
    if !summary_only && path.is_dir() {
        let mut sub = walk_dir(&path);
        entries.append(&mut sub);
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    }
    let stdout = entries
        .iter()
        .map(|(p, sz)| {
            let s = if human {
                human_size(*sz)
            } else {
                sz.to_string()
            };
            format!("{}\t{}", s, p.display())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let structured = json!({
        "entries": entries.iter().map(|(p, sz)| json!({
            "path": p.to_string_lossy(),
            "size_bytes": *sz
        })).collect::<Vec<_>>()
    });
    crate::CommandOutput::success_structured(stdout, structured)
}
