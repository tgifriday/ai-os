use serde::{Deserialize, Serialize};

use crate::backend::{CompletionRequest, Message, MessageRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsState {
    pub hostname: String,
    pub username: String,
    pub cwd: String,
    pub os_type: String,
    pub arch: String,
    pub shell_name: String,
    pub uptime_secs: f64,
    pub available_commands: Vec<String>,
    pub recent_history: Vec<String>,
}

pub struct ContextManager {
    system_prompt_template: String,
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are the AI built into {shell_name}, running on {hostname} as user {username}.
Platform: {os_type} ({arch})
Current directory: {cwd}

IMPORTANT platform facts:
- This is {os_type}. Give platform-correct commands ONLY.
- On macOS: use `brew` for packages. GNU coreutils are prefixed with `g` (e.g. `gfree`, `gstat`, `greadlink`). There is no /proc filesystem. Use `vm_stat` for memory, `sysctl` for kernel info, `diskutil` for disks.
- On Linux: use `apt`, `dnf`, or `pacman` for packages depending on distro. GNU commands have normal names.
- NEVER suggest Linux-only commands on macOS or vice versa without noting the difference.

You are the user's always-available assistant. Your job:
- When they type something that isn't a command, figure out what they meant and help them.
- If a command failed or wasn't found, explain why and give them the correct command to run.
- If they mistyped a command, show the corrected version.
- If they need to install something, tell them how using the RIGHT package manager for this OS.
- If they asked a question in natural language, answer it directly.
- When suggesting commands, show them clearly so they can copy/paste.
- Keep answers concise. One or two sentences of explanation, then the command.
- Prefer safe, reversible operations. Warn before anything destructive.

Built-in commands: {commands}

Recent command history:
{history}"#;

impl ContextManager {
    pub fn new() -> Self {
        Self {
            system_prompt_template: DEFAULT_SYSTEM_PROMPT.to_string(),
        }
    }

    pub fn with_template(template: String) -> Self {
        Self {
            system_prompt_template: template,
        }
    }

    pub fn build_system_prompt(&self, os_state: &OsState) -> String {
        let commands = if os_state.available_commands.is_empty() {
            "(none loaded)".to_string()
        } else {
            os_state.available_commands.join(", ")
        };

        let history = if os_state.recent_history.is_empty() {
            "(no recent history)".to_string()
        } else {
            os_state
                .recent_history
                .iter()
                .map(|h| format!("  $ {h}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        self.system_prompt_template
            .replace("{hostname}", &os_state.hostname)
            .replace("{username}", &os_state.username)
            .replace("{cwd}", &os_state.cwd)
            .replace("{os_type}", &os_state.os_type)
            .replace("{arch}", &os_state.arch)
            .replace("{shell_name}", &os_state.shell_name)
            .replace("{uptime}", &format!("{:.0}", os_state.uptime_secs))
            .replace("{commands}", &commands)
            .replace("{history}", &history)
    }

    pub fn build_request(
        &self,
        user_input: &str,
        os_state: &OsState,
        history: &[Message],
        stream: bool,
    ) -> CompletionRequest {
        let system_prompt = self.build_system_prompt(os_state);

        let mut messages: Vec<Message> = history.to_vec();
        messages.push(Message {
            role: MessageRole::User,
            content: user_input.to_string(),
        });

        CompletionRequest {
            system_prompt: Some(system_prompt),
            messages,
            max_tokens: None,
            temperature: Some(0.7),
            stream,
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
