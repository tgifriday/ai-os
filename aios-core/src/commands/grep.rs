use regex::Regex;
use serde_json::json;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let case_insensitive = parsed.has('i');
    let line_numbers = parsed.has('n');
    let recursive = parsed.has_any('r', "recursive");
    let count_only = parsed.has('c');
    let invert = parsed.has('v');

    let mut positional = parsed.positional.iter();
    let pattern: Option<String> = positional.next().map(|s| s.to_string());
    let files: Vec<std::path::PathBuf> = positional
        .map(|p| parsed.resolve_path(p, cwd))
        .collect();

    let pattern = match pattern {
        Some(p) => p,
        None => return crate::CommandOutput::error("grep: pattern required\n".into(), 1),
    };

    if files.is_empty() {
        return crate::CommandOutput::error("grep: at least one file required\n".into(), 1);
    }

    let re = match Regex::new(&if case_insensitive {
        format!("(?i){}", regex::escape(&pattern))
    } else {
        regex::escape(&pattern)
    }) {
        Ok(r) => r,
        Err(e) => return crate::CommandOutput::error(format!("grep: invalid regex: {}\n", e), 1),
    };

    let mut matches: Vec<serde_json::Value> = Vec::new();
    let mut total_count = 0u64;
    let mut output_lines = Vec::new();

    for file in &files {
        if let Err(e) = grep_file(
            &file,
            &re,
            invert,
            line_numbers,
            count_only,
            recursive,
            files.len() > 1,
            &mut matches,
            &mut total_count,
            &mut output_lines,
        ) {
            return crate::CommandOutput::error(format!("grep: {}: {}\n", file.display(), e), 1);
        }
    }

    let stdout = output_lines.join("\n");
    let structured = json!({
        "match_count": total_count,
        "matches": matches
    });
    crate::CommandOutput::success_structured(stdout, structured)
}

fn grep_file(
    path: &Path,
    re: &Regex,
    invert: bool,
    line_numbers: bool,
    count_only: bool,
    recursive: bool,
    multiple_files: bool,
    matches: &mut Vec<serde_json::Value>,
    total_count: &mut u64,
    output_lines: &mut Vec<String>,
) -> Result<(), std::io::Error> {
    let meta = std::fs::metadata(path)?;
    if meta.is_dir() {
        if !recursive {
            return Ok(());
        }
        for e in std::fs::read_dir(path)? {
            let e = e?;
            let p = e.path();
            grep_file(
                &p,
                re,
                invert,
                line_numbers,
                count_only,
                recursive,
                multiple_files,
                matches,
                total_count,
                output_lines,
            )?;
        }
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let mut count = 0u64;
    let prefix = if multiple_files {
        format!("{}:", path.display())
    } else {
        String::new()
    };
    for (i, line) in content.lines().enumerate() {
        let ln = i + 1;
        let matched = re.is_match(line);
        if matched != invert {
            count += 1;
            matches.push(serde_json::json!({
                "file": path.to_string_lossy(),
                "line": ln,
                "content": line
            }));
            if !count_only {
                let mut out = prefix.clone();
                if line_numbers {
                    out.push_str(&format!("{}:", ln));
                }
                out.push_str(line);
                output_lines.push(out);
            }
        }
    }
    *total_count += count;
    if count_only {
        if multiple_files {
            output_lines.push(format!("{}:{}", path.display(), count));
        } else {
            output_lines.push(count.to_string());
        }
    }
    Ok(())
}
