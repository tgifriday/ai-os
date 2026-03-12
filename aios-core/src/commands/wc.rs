use serde_json::json;
use std::path::Path;

pub fn run(args: &[&str], cwd: &std::path::Path) -> crate::CommandOutput {
    let parsed = crate::flags::parse(args, &[]);
    let lines_only = parsed.has('l');
    let words_only = parsed.has('w');
    let bytes_only = parsed.has('c');
    let files = parsed.paths(cwd);

    if files.is_empty() {
        return crate::CommandOutput::error("wc: file required\n".into(), 1);
    }

    let show_all = !lines_only && !words_only && !bytes_only;

    let mut total_lines = 0u64;
    let mut total_words = 0u64;
    let mut total_bytes = 0u64;
    let mut file_results: Vec<serde_json::Value> = Vec::new();
    let mut output_lines = Vec::new();

    for path in &files {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return crate::CommandOutput::error(format!("wc: {}: {}\n", path.display(), e), 1),
        };
        let lines = content.lines().count() as u64;
        let words = content
            .split_whitespace()
            .count() as u64;
        let bytes = content.as_bytes().len() as u64;

        total_lines += lines;
        total_words += words;
        total_bytes += bytes;

        file_results.push(json!({
            "file": path.to_string_lossy(),
            "lines": lines,
            "words": words,
            "bytes": bytes
        }));

        let mut parts = Vec::new();
        if show_all || lines_only {
            parts.push(lines.to_string());
        }
        if show_all || words_only {
            parts.push(words.to_string());
        }
        if show_all || bytes_only {
            parts.push(bytes.to_string());
        }
        parts.push(path.to_string_lossy().into_owned());
        output_lines.push(parts.join(" "));
    }

    if files.len() > 1 {
        let mut parts = Vec::new();
        if show_all || lines_only {
            parts.push(total_lines.to_string());
        }
        if show_all || words_only {
            parts.push(total_words.to_string());
        }
        if show_all || bytes_only {
            parts.push(total_bytes.to_string());
        }
        parts.push("total".to_string());
        output_lines.push(parts.join(" "));
    }

    let stdout = output_lines.join("\n");
    let structured = json!({
        "files": file_results,
        "totals": { "lines": total_lines, "words": total_words, "bytes": total_bytes }
    });
    crate::CommandOutput::success_structured(stdout, structured)
}
