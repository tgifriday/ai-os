use std::path::Path;

pub fn complete_path(partial: &str, cwd: &Path) -> Vec<String> {
    let (dir_part, file_part) = if let Some(pos) = partial.rfind('/') {
        (&partial[..=pos], &partial[pos + 1..])
        } else {
        ("", partial)
    };

    let search_dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else {
        crate::parser::resolve_path(dir_part, cwd)
    };

    let mut completions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(file_part) {
                let full = format!("{}{}", dir_part, name);
                if entry.path().is_dir() {
                    completions.push(format!("{}/", full));
                } else {
                    completions.push(full);
                }
            }
        }
    }

    completions.sort();
    completions
}

pub fn complete_command(partial: &str) -> Vec<String> {
    let builtins = [
        "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "grep", "find", "wc", "head", "tail",
        "ps", "kill", "top", "echo", "env", "pwd", "chmod", "df", "du", "date", "uptime",
        "whoami", "hostname", "cd", "export", "clear", "history", "help", "exit",
    ];

    let mut matches: Vec<String> = builtins
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

pub fn get_completions(line: &str, cwd: &Path) -> Vec<String> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    if parts.len() <= 1 && !trimmed.ends_with(' ') {
        return complete_command(parts.first().unwrap_or(&""));
    }

    let last_word = if trimmed.ends_with(' ') {
        ""
    } else {
        parts.last().unwrap_or(&"")
    };

    complete_path(last_word, cwd)
}
