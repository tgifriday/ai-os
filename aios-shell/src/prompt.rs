use std::path::Path;

pub struct PromptConfig {
    pub show_user: bool,
    pub show_host: bool,
    pub show_cwd: bool,
    pub show_ai_status: bool,
    pub ai_available: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            show_user: true,
            show_host: true,
            show_cwd: true,
            show_ai_status: true,
            ai_available: false,
        }
    }
}

pub fn format_prompt(cwd: &Path, last_exit_code: i32, config: &PromptConfig) -> String {
    let mut prompt = String::new();

    let status_color = if last_exit_code == 0 {
        "\x1b[32m"
    } else {
        "\x1b[31m"
    };

    if config.show_ai_status {
        if config.ai_available {
            prompt.push_str("\x1b[36m[AI]\x1b[0m ");
        } else {
            prompt.push_str("\x1b[90m[--]\x1b[0m ");
        }
    }

    if config.show_user || config.show_host {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        let host = aios_kernel::network::get_hostname();

        if config.show_user && config.show_host {
            prompt.push_str(&format!("\x1b[1;32m{}@{}\x1b[0m:", user, host));
        } else if config.show_user {
            prompt.push_str(&format!("\x1b[1;32m{}\x1b[0m:", user));
        } else {
            prompt.push_str(&format!("\x1b[1;32m{}\x1b[0m:", host));
        }
    }

    if config.show_cwd {
        let home = dirs::home_dir();
        let cwd_display = if let Some(ref home) = home {
            if let Ok(stripped) = cwd.strip_prefix(home) {
                format!("~/{}", stripped.display())
            } else {
                cwd.display().to_string()
            }
        } else {
            cwd.display().to_string()
        };
        let cwd_display = cwd_display.trim_end_matches('/');
        let cwd_display = if cwd_display.is_empty() {
            "~"
        } else {
            cwd_display
        };
        prompt.push_str(&format!("\x1b[1;34m{}\x1b[0m", cwd_display));
    }

    prompt.push_str(&format!(" {}${}\x1b[0m ", status_color, ""));
    prompt
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        result.push(ch);
    }
    result
}
