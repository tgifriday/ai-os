use std::path::Path;

const SHELL_BUILTINS: &[&str] = &[
    "cd", "clear", "exit", "export", "help", "history", "llm", "quit", "sanitize",
];

pub fn get_completions(line: &str, cwd: &Path) -> Vec<String> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let (before, word) = split_last_word(trimmed);

    if before.is_empty() && !trimmed.ends_with(' ') {
        return complete_command(word);
    }

    let word = if trimmed.ends_with(' ') { "" } else { word };
    complete_path(word, cwd)
}

pub fn longest_common_prefix(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let first = &items[0];
    let mut len = first.len();
    for item in &items[1..] {
        len = len.min(item.len());
        for (i, (a, b)) in first.bytes().zip(item.bytes()).enumerate() {
            if a != b {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
}

pub fn format_columns(items: &[String], terminal_width: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    let max_len = items.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len + 2;
    let cols = (terminal_width / col_width).max(1);

    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 && i % cols == 0 {
            out.push_str("\r\n");
        }
        out.push_str(item);
        if (i + 1) % cols != 0 {
            let padding = col_width.saturating_sub(item.len());
            for _ in 0..padding {
                out.push(' ');
            }
        }
    }
    out.push_str("\r\n");
    out
}

fn split_last_word(input: &str) -> (&str, &str) {
    let bytes = input.as_bytes();
    let mut i = bytes.len();
    let mut in_single = false;
    let mut in_double = false;
    let mut last_word_start = 0;

    let mut pos = 0;
    while pos < bytes.len() {
        let ch = bytes[pos];
        match ch {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b' ' | b'\t' if !in_single && !in_double => {
                last_word_start = pos + 1;
            }
            _ => {}
        }
        pos += 1;
    }

    (&input[..last_word_start], &input[last_word_start..])
}

fn complete_command(partial: &str) -> Vec<String> {
    let mut matches: Vec<String> = SHELL_BUILTINS
        .iter()
        .filter(|cmd| cmd.starts_with(partial))
        .map(|s| s.to_string())
        .collect();

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(partial) && !matches.contains(&name) {
                        matches.push(name);
                    }
                }
            }
        }
    }

    matches.sort();
    matches.dedup();
    matches
}

fn complete_path(partial: &str, cwd: &Path) -> Vec<String> {
    let expanded = if partial.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            let rest = partial[1..].trim_start_matches('/');
            let expanded_str = if partial.len() > 1 && partial.as_bytes()[1] == b'/' {
                format!("{}/{}", home.display(), rest)
            } else if partial == "~" {
                format!("{}/", home.display())
            } else {
                format!("{}{}", home.display(), rest)
            };
            Some(expanded_str)
        } else {
            None
        }
    } else {
        None
    };

    let lookup = expanded.as_deref().unwrap_or(partial);

    let (dir_part, file_part) = if let Some(pos) = lookup.rfind('/') {
        (&lookup[..=pos], &lookup[pos + 1..])
    } else {
        ("", lookup)
    };

    let search_dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else {
        let p = Path::new(dir_part);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(dir_part)
        }
    };

    let prefix_for_display = if partial.starts_with('~') {
        if let Some(pos) = partial.rfind('/') {
            &partial[..=pos]
        } else if partial == "~" {
            "~/"
        } else {
            partial
        }
    } else {
        dir_part
    };

    let mut completions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !file_part.starts_with('.') {
                continue;
            }
            if name.starts_with(file_part) {
                let display = format!("{}{}", prefix_for_display, name);
                if entry.path().is_dir() {
                    completions.push(format!("{}/", display));
                } else {
                    completions.push(display);
                }
            }
        }
    }

    completions.sort();
    completions
}
