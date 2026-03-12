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
pub enum InputClassification {
    DirectCommand(Pipeline),
    NaturalLanguage(String),
    Ambiguous(String, Pipeline),
    AiExplicit(String),
    AiPipe(Pipeline, String),
    Empty,
    Exit,
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
    aios_core::commands::is_builtin(cmd)
        || matches!(
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
        )
}

pub fn parse_pipeline(input: &str) -> Option<Pipeline> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let background = trimmed.ends_with('&');
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

    for i in 0..bytes.len() {
        match bytes[i] {
            b'\'' if !in_double_quote => in_single_quote = !in_single_quote,
            b'"' if !in_single_quote => in_double_quote = !in_double_quote,
            b'|' if !in_single_quote && !in_double_quote => {
                segments.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    segments.push(&input[start..]);
    segments
}

fn parse_single_command(input: &str) -> Option<ParsedCommand> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }

    let mut program = String::new();
    let mut args = Vec::new();
    let mut stdin_redirect = None;
    let mut stdout_redirect = None;

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "<" => {
                i += 1;
                if i < tokens.len() {
                    stdin_redirect = Some(tokens[i].clone());
                }
            }
            ">>" => {
                i += 1;
                if i < tokens.len() {
                    stdout_redirect = Some(RedirectTarget {
                        path: tokens[i].clone(),
                        append: true,
                    });
                }
            }
            ">" => {
                i += 1;
                if i < tokens.len() {
                    stdout_redirect = Some(RedirectTarget {
                        path: tokens[i].clone(),
                        append: false,
                    });
                }
            }
            token => {
                if program.is_empty() {
                    program = token.to_string();
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

pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
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
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '>' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                if tokens.last().map(|t| t.as_str()) == Some(">") {
                    tokens.pop();
                    tokens.push(">>".to_string());
                } else {
                    tokens.push(">".to_string());
                }
            }
            '<' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push("<".to_string());
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
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

        if ch == '$' {
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
        } else {
            result.push(ch);
        }
        at_token_start = false;
    }

    result
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
