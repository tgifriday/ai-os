use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub stdin_redirect: Option<String>,
    pub stdout_redirect: Option<RedirectTarget>,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct RedirectTarget {
    pub path: String,
    pub append: bool,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<ParsedCommand>,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub enum ChainOp {
    Always,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct ChainedCommand {
    pub classification: InputClassification,
    pub next_op: Option<ChainOp>,
}

#[derive(Debug, Clone)]
pub enum InputClassification {
    DirectCommand(Pipeline),
    NaturalLanguage(String),
    Ambiguous(String, Pipeline),
    AiExplicit(String),
    AiPipe(Pipeline, String),
    Empty,
    Exit,
}

pub fn split_chain(input: &str) -> Vec<(String, ChainOp)> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if escape {
            current.push(bytes[i] as char);
            escape = false;
            i += 1;
            continue;
        }

        match bytes[i] {
            b'\\' if !in_single => {
                escape = true;
                current.push('\\');
                i += 1;
            }
            b'\'' if !in_double => {
                in_single = !in_single;
                current.push('\'');
                i += 1;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                current.push('"');
                i += 1;
            }
            b'&' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'&' => {
                let seg = current.trim().to_string();
                if !seg.is_empty() {
                    segments.push((seg, ChainOp::And));
                }
                current.clear();
                i += 2;
            }
            b'|' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                let seg = current.trim().to_string();
                if !seg.is_empty() {
                    segments.push((seg, ChainOp::Or));
                }
                current.clear();
                i += 2;
            }
            b';' if !in_single && !in_double => {
                let seg = current.trim().to_string();
                if !seg.is_empty() {
                    segments.push((seg, ChainOp::Always));
                }
                current.clear();
                i += 1;
            }
            _ => {
                current.push(bytes[i] as char);
                i += 1;
            }
        }
    }

    let final_seg = current.trim().to_string();
    if !final_seg.is_empty() {
        segments.push((final_seg, ChainOp::Always));
    }

    segments
}

pub fn classify_input(input: &str) -> InputClassification {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return InputClassification::Empty;
    }

    if trimmed == "exit" || trimmed == "quit" || trimmed == "logout" {
        return InputClassification::Exit;
    }

    if let Some(query) = trimmed.strip_prefix('@') {
        return InputClassification::AiExplicit(query.trim().to_string());
    }

    if trimmed.contains("| @") {
        let parts: Vec<&str> = trimmed.splitn(2, "| @").collect();
        if parts.len() == 2 {
            if let Some(pipeline) = parse_pipeline(parts[0].trim()) {
                return InputClassification::AiPipe(pipeline, parts[1].trim().to_string());
            }
        }
    }

    if let Some(pipeline) = parse_pipeline(trimmed) {
        let first_cmd = &pipeline.commands[0].program;
        if is_known_command(first_cmd) || first_cmd.contains('/') || first_cmd.starts_with('.') {
            return InputClassification::DirectCommand(pipeline);
        }
        return InputClassification::Ambiguous(trimmed.to_string(), pipeline);
    }

    InputClassification::NaturalLanguage(trimmed.to_string())
}

fn is_known_command(cmd: &str) -> bool {
    if aios_core::commands::is_builtin(cmd) {
        return true;
    }

    if matches!(
        cmd,
        "clear"
            | "history"
            | "help"
            | "man"
            | "sudo"
            | "su"
            | "ssh"
            | "scp"
            | "curl"
            | "wget"
            | "git"
            | "make"
            | "cargo"
            | "python"
            | "python3"
            | "node"
            | "npm"
            | "pip"
            | "apt"
            | "yum"
            | "dnf"
            | "pacman"
            | "brew"
            | "tar"
            | "gzip"
            | "gunzip"
            | "zip"
            | "unzip"
            | "touch"
            | "ln"
            | "sort"
            | "uniq"
            | "cut"
            | "awk"
            | "sed"
            | "xargs"
            | "tee"
            | "less"
            | "more"
            | "vim"
            | "nano"
            | "vi"
            | "emacs"
    ) {
        return true;
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = std::path::PathBuf::from(dir).join(cmd);
            if candidate.exists() {
                return true;
            }
        }
    }

    false
}

pub fn parse_pipeline(input: &str) -> Option<Pipeline> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let background = trimmed.ends_with('&') && !trimmed.ends_with("&&");
    let input = if background {
        trimmed[..trimmed.len() - 1].trim()
    } else {
        trimmed
    };

    let segments = split_pipe_segments(input);
    let mut commands = Vec::new();

    for segment in segments {
        if let Some(cmd) = parse_single_command(segment.trim()) {
            commands.push(cmd);
        } else {
            return None;
        }
    }

    if commands.is_empty() {
        return None;
    }

    Some(Pipeline {
        commands,
        background,
    })
}

fn split_pipe_segments(input: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let bytes = input.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !in_double_quote => in_single_quote = !in_single_quote,
            b'"' if !in_single_quote => in_double_quote = !in_double_quote,
            b'|' if !in_single_quote && !in_double_quote => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                    i += 2;
                    continue;
                }
                segments.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    segments.push(&input[start..]);
    segments
}

#[derive(Debug, Clone)]
struct Token {
    value: String,
    quoted: bool,
}

fn parse_single_command(input: &str) -> Option<ParsedCommand> {
    let tokens = tokenize_rich(input);
    if tokens.is_empty() {
        return None;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let mut program = String::new();
    let mut args = Vec::new();
    let mut stdin_redirect = None;
    let mut stdout_redirect = None;

    let mut i = 0;
    while i < tokens.len() {
        let val = tokens[i].value.as_str();
        match val {
            "<" => {
                i += 1;
                if i < tokens.len() {
                    stdin_redirect = Some(tokens[i].value.clone());
                }
            }
            ">>" => {
                i += 1;
                if i < tokens.len() {
                    stdout_redirect = Some(RedirectTarget {
                        path: tokens[i].value.clone(),
                        append: true,
                    });
                }
            }
            ">" => {
                i += 1;
                if i < tokens.len() {
                    stdout_redirect = Some(RedirectTarget {
                        path: tokens[i].value.clone(),
                        append: false,
                    });
                }
            }
            token => {
                if program.is_empty() {
                    program = token.to_string();
                } else if !tokens[i].quoted && contains_glob(token) {
                    let expanded = expand_glob(token, &cwd);
                    args.extend(expanded);
                } else {
                    args.push(token.to_string());
                }
            }
        }
        i += 1;
    }

    if program.is_empty() {
        return None;
    }

    Some(ParsedCommand {
        program,
        args,
        stdin_redirect,
        stdout_redirect,
        background: false,
    })
}

fn contains_glob(s: &str) -> bool {
    let mut escape = false;
    for ch in s.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '*' || ch == '?' || ch == '[' {
            return true;
        }
    }
    false
}

fn expand_glob(pattern: &str, cwd: &Path) -> Vec<String> {
    let full_pattern = if Path::new(pattern).is_absolute() || pattern.starts_with('~') {
        pattern.to_string()
    } else {
        format!("{}/{}", cwd.display(), pattern)
    };

    let options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };

    match glob::glob_with(&full_pattern, options) {
        Ok(paths) => {
            let mut matches: Vec<String> = paths
                .flatten()
                .map(|p| {
                    if pattern.starts_with("./") {
                        format!("./{}", p.strip_prefix(cwd).unwrap_or(&p).display())
                    } else if pattern.starts_with('/') || pattern.starts_with('~') {
                        p.display().to_string()
                    } else {
                        p.strip_prefix(cwd)
                            .unwrap_or(&p)
                            .display()
                            .to_string()
                    }
                })
                .collect();
            matches.sort();
            if matches.is_empty() {
                vec![pattern.to_string()]
            } else {
                matches
            }
        }
        Err(_) => vec![pattern.to_string()],
    }
}

fn tokenize_rich(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
    let mut token_is_quoted = false;
    let chars: Vec<char> = input.chars().collect();

    for &ch in &chars {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                token_is_quoted = true;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                token_is_quoted = true;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(Token {
                        value: current.clone(),
                        quoted: token_is_quoted,
                    });
                    current.clear();
                    token_is_quoted = false;
                }
            }
            '>' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(Token {
                        value: current.clone(),
                        quoted: token_is_quoted,
                    });
                    current.clear();
                    token_is_quoted = false;
                }
                if tokens.last().map(|t| t.value.as_str()) == Some(">") {
                    tokens.pop();
                    tokens.push(Token {
                        value: ">>".to_string(),
                        quoted: false,
                    });
                } else {
                    tokens.push(Token {
                        value: ">".to_string(),
                        quoted: false,
                    });
                }
            }
            '<' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(Token {
                        value: current.clone(),
                        quoted: token_is_quoted,
                    });
                    current.clear();
                    token_is_quoted = false;
                }
                tokens.push(Token {
                    value: "<".to_string(),
                    quoted: false,
                });
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(Token {
            value: current,
            quoted: token_is_quoted,
        });
    }

    tokens
}

pub fn tokenize(input: &str) -> Vec<String> {
    tokenize_rich(input).into_iter().map(|t| t.value).collect()
}

pub fn expand_variables(input: &str, env: &std::collections::HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut at_token_start = true;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                result.push(ch);
                at_token_start = false;
                continue;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                result.push(ch);
                at_token_start = false;
                continue;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                result.push(ch);
                at_token_start = true;
                continue;
            }
            _ => {}
        }

        if ch == '~' && at_token_start && !in_single_quote && !in_double_quote {
            let next = chars.peek();
            if next.is_none() || *next.unwrap() == '/' || *next.unwrap() == ' ' || *next.unwrap() == '\t' {
                if let Some(home) = dirs::home_dir() {
                    result.push_str(&home.to_string_lossy());
                    at_token_start = false;
                    continue;
                }
            }
        }

        if ch == '$' && !in_single_quote {
            if chars.peek() == Some(&'(') {
                chars.next();
                let mut depth = 1;
                let mut subcmd = String::new();
                while let Some(c) = chars.next() {
                    if c == '(' {
                        depth += 1;
                        subcmd.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        subcmd.push(c);
                    } else {
                        subcmd.push(c);
                    }
                }
                let output = run_subcommand(&subcmd);
                result.push_str(&output);
            } else {
                let mut var_name = String::new();
                if chars.peek() == Some(&'{') {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        if c == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(c);
                        chars.next();
                    }
                } else {
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var_name.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }

                if var_name == "?" {
                    result.push_str(&env.get("?").unwrap_or(&"0".to_string()));
                } else if !var_name.is_empty() {
                    if let Some(val) = env.get(&var_name) {
                        result.push_str(val);
                    } else if let Ok(val) = std::env::var(&var_name) {
                        result.push_str(&val);
                    }
                } else {
                    result.push('$');
                }
            }
        } else if ch == '`' && !in_single_quote {
            let mut subcmd = String::new();
            while let Some(c) = chars.next() {
                if c == '`' {
                    break;
                }
                subcmd.push(c);
            }
            let output = run_subcommand(&subcmd);
            result.push_str(&output);
        } else {
            result.push(ch);
        }
        at_token_start = false;
    }

    result
}

fn run_subcommand(cmd: &str) -> String {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(trimmed)
        .output()
    {
        Ok(output) => {
            let mut s = String::from_utf8_lossy(&output.stdout).to_string();
            if s.ends_with('\n') {
                s.pop();
            }
            if s.ends_with('\r') {
                s.pop();
            }
            s
        }
        Err(_) => String::new(),
    }
}

pub fn resolve_path(path_str: &str, cwd: &Path) -> std::path::PathBuf {
    let path = Path::new(path_str);
    if path.is_absolute() {
        path.to_path_buf()
    } else if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            home.join(&path_str[1..].trim_start_matches('/'))
        } else {
            cwd.join(path_str)
        }
    } else {
        cwd.join(path_str)
    }
}
