use serde_json::json;
use std::path::Path;

fn glob_match(pattern: &str, name: &str) -> bool {
    fn match_inner(pat: &[u8], s: &[u8], pi: usize, si: usize) -> bool {
        if pi == pat.len() {
            return si == s.len();
        }
        match pat[pi] as char {
            '*' => {
                for i in si..=s.len() {
                    if match_inner(pat, s, pi + 1, i) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                if si < s.len() {
                    match_inner(pat, s, pi + 1, si + 1)
                } else {
                    false
                }
            }
            c => {
                if si < s.len() && s[si] as char == c {
                    match_inner(pat, s, pi + 1, si + 1)
                } else {
                    false
                }
            }
        }
    }
    match_inner(pattern.as_bytes(), name.as_bytes(), 0, 0)
}

fn walk(
    root: &Path,
    name_pattern: Option<&str>,
    filter_type: Option<&str>,
    results: &mut Vec<String>,
) -> Result<(), std::io::Error> {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_dir = path.is_dir();
        let matches_name = name_pattern.map_or(true, |p| glob_match(p, name));
        let matches_type = filter_type.map_or(true, |t| match t {
            "f" => !is_dir,
            "d" => is_dir,
            _ => true,
        });
        if matches_name && matches_type {
            results.push(path.to_string_lossy().into_owned());
        }
        if is_dir && name != "." && name != ".." {
            walk(&path, name_pattern, filter_type, results)?;
        }
    }
    Ok(())
}

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let mut root: Option<std::path::PathBuf> = None;
    let mut name_pattern: Option<String> = None;
    let mut filter_type: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-name" => {
                i += 1;
                if i < args.len() {
                    name_pattern = Some(args[i].to_string());
                }
                i += 1;
            }
            "-type" => {
                i += 1;
                if i < args.len() {
                    filter_type = Some(args[i].to_string());
                }
                i += 1;
            }
            _ if !args[i].starts_with('-') => {
                if root.is_none() {
                    let p = if Path::new(args[i]).is_absolute() {
                        std::path::PathBuf::from(args[i])
                    } else {
                        cwd.join(args[i])
                    };
                    root = Some(p);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    let root = match root {
        Some(r) => r,
        None => return crate::CommandOutput::error("find: path required\n".into(), 1),
    };

    if !root.is_dir() {
        return crate::CommandOutput::error(
            format!("find: {}: Not a directory\n", root.display()),
            1,
        );
    }

    let mut results = Vec::new();
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let matches_name = name_pattern.as_ref().map_or(true, |p| glob_match(p, name));
    let matches_type = filter_type.as_ref().map_or(true, |t| match t.as_str() {
        "f" => !root.is_dir(),
        "d" => root.is_dir(),
        _ => true,
    });
    if matches_name && matches_type {
        results.push(root.to_string_lossy().into_owned());
    }
    if root.is_dir() {
        if let Err(e) = walk(
            &root,
            name_pattern.as_deref(),
            filter_type.as_deref(),
            &mut results,
        ) {
            return crate::CommandOutput::error(format!("find: {}\n", e), 1);
        }
    }

    let stdout = results.join("\n");
    let structured = json!({ "paths": results });
    crate::CommandOutput::success_structured(stdout, structured)
}
