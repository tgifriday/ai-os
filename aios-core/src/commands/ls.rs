use aios_kernel::fs::{get_file_info, list_directory};
use std::path::Path;

fn format_permissions(mode: u32) -> String {
    let r = if mode & 0o400 != 0 { 'r' } else { '-' };
    let w = if mode & 0o200 != 0 { 'w' } else { '-' };
    let x = if mode & 0o100 != 0 { 'x' } else { '-' };
    let r2 = if mode & 0o40 != 0 { 'r' } else { '-' };
    let w2 = if mode & 0o20 != 0 { 'w' } else { '-' };
    let x2 = if mode & 0o10 != 0 { 'x' } else { '-' };
    let r3 = if mode & 0o4 != 0 { 'r' } else { '-' };
    let w3 = if mode & 0o2 != 0 { 'w' } else { '-' };
    let x3 = if mode & 0o1 != 0 { 'x' } else { '-' };
    format!("{}{}{}{}{}{}{}{}{}", r, w, x, r2, w2, x2, r3, w3, x3)
}

fn human_size(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.1}G", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1}M", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1}K", n as f64 / KB as f64)
    } else {
        n.to_string()
    }
}

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let long = parsed.has_any('l', "long");
    let all = parsed.has_any('a', "all");
    let human = parsed.has_any('h', "human-readable");
    let recursive = parsed.has('R');
    let mut paths = parsed.paths(cwd);

    if paths.is_empty() {
        paths.push(cwd.to_path_buf());
    }

    let mut all_entries: Vec<serde_json::Value> = Vec::new();
    let mut stdout_lines: Vec<String> = Vec::new();

    for path in &paths {
        let info = match get_file_info(path) {
            Ok(i) => i,
            Err(e) => return crate::CommandOutput::error(format!("ls: {}: {}", path.display(), e), 1),
        };

        if info.is_dir {
            let entries = match list_directory(path) {
                Ok(e) => e,
                Err(e) => return crate::CommandOutput::error(format!("ls: {}: {}", path.display(), e), 1),
            };
            let filtered: Vec<_> = if all {
                entries.into_iter().collect()
            } else {
                entries.into_iter().filter(|e| !e.name.starts_with('.')).collect()
            };

            for entry in &filtered {
                let perms = format_permissions(entry.permissions);
                let size_str = if human {
                    human_size(entry.size)
                } else {
                    entry.size.to_string()
                };
                let typ = if entry.is_dir { "d" } else { "-" };
                let json_entry = serde_json::json!({
                    "name": entry.name,
                    "path": entry.path,
                    "size": entry.size,
                    "is_dir": entry.is_dir,
                    "permissions": perms,
                    "owner_uid": entry.owner_uid,
                    "group_gid": entry.group_gid,
                });
                all_entries.push(json_entry);
                if long {
                    stdout_lines.push(format!(
                        "{}{} {} {} {} {} {}",
                        typ, perms, entry.nlinks, entry.owner_uid, entry.group_gid, size_str, entry.name
                    ));
                } else {
                    stdout_lines.push(entry.name.clone());
                }
            }

            if recursive {
                for entry in &filtered {
                    if entry.is_dir && entry.name != "." && entry.name != ".." {
                        let subpath = Path::new(&entry.path);
                        let mut recurse_args: Vec<String> = Vec::new();
                        if long {
                            recurse_args.push("-l".to_string());
                        }
                        if all {
                            recurse_args.push("-a".to_string());
                        }
                        if human {
                            recurse_args.push("-h".to_string());
                        }
                        if recursive {
                            recurse_args.push("-R".to_string());
                        }
                        recurse_args.push(subpath.to_string_lossy().into_owned());
                        let refs: Vec<&str> = recurse_args.iter().map(|s| s.as_str()).collect();
                        let sub_result = run(&refs, subpath);
                        if sub_result.exit_code != 0 {
                            return sub_result;
                        }
                        stdout_lines.push(format!("{}:", subpath.display()));
                        stdout_lines.extend(sub_result.stdout.lines().map(|s| format!("  {}", s)));
                    }
                }
            }
        } else {
            let perms = format_permissions(info.permissions);
            let size_str = if human {
                human_size(info.size)
            } else {
                info.size.to_string()
            };
            let typ = if info.is_dir { "d" } else { "-" };
            all_entries.push(serde_json::json!({
                "name": info.name,
                "path": info.path,
                "size": info.size,
                "is_dir": info.is_dir,
                "permissions": perms,
            }));
            if long {
                stdout_lines.push(format!(
                    "{}{} {} {} {} {} {}",
                    typ, perms, info.nlinks, info.owner_uid, info.group_gid, size_str, info.name
                ));
            } else {
                stdout_lines.push(info.name);
            }
        }
    }

    let stdout = stdout_lines.join("\n");
    let structured = serde_json::json!({ "files": all_entries });
    crate::CommandOutput::success_structured(stdout, structured)
}
