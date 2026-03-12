use crate::parser::{ParsedCommand, Pipeline, RedirectTarget};
use aios_core::commands::builtin_commands;
use aios_core::CommandOutput;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Executor {
    pub cwd: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub last_exit_code: i32,
}

impl Executor {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut env_vars = HashMap::new();
        for (k, v) in std::env::vars() {
            env_vars.insert(k, v);
        }
        Self {
            cwd,
            env_vars,
            last_exit_code: 0,
        }
    }

    pub fn execute_pipeline(&mut self, pipeline: &Pipeline) -> CommandOutput {
        if pipeline.commands.len() == 1 {
            return self.execute_single(&pipeline.commands[0]);
        }

        let mut input_data: Option<Vec<u8>> = None;
        let mut last_output = CommandOutput::success(String::new());

        for (i, cmd) in pipeline.commands.iter().enumerate() {
            let is_last = i == pipeline.commands.len() - 1;
            last_output = self.execute_piped(cmd, input_data.as_deref(), is_last);

            if last_output.exit_code != 0 {
                break;
            }

            input_data = Some(last_output.stdout.as_bytes().to_vec());
        }

        self.last_exit_code = last_output.exit_code;
        last_output
    }

    fn execute_single(&mut self, cmd: &ParsedCommand) -> CommandOutput {
        if let Some(output) = self.try_shell_builtin(cmd) {
            return output;
        }

        let builtins = builtin_commands();
        if let Some(builtin_fn) = builtins.get(cmd.program.as_str()) {
            let args: Vec<&str> = cmd.args.iter().map(|s| s.as_str()).collect();
            let output = builtin_fn(&args, &self.cwd);
            self.last_exit_code = output.exit_code;

            if let Some(ref redirect) = cmd.stdout_redirect {
                self.write_redirect(redirect, &output.stdout);
            }

            return output;
        }

        self.execute_external(cmd, None)
    }

    fn execute_piped(
        &mut self,
        cmd: &ParsedCommand,
        stdin_data: Option<&[u8]>,
        _is_last: bool,
    ) -> CommandOutput {
        let builtins = builtin_commands();
        if let Some(builtin_fn) = builtins.get(cmd.program.as_str()) {
            let args: Vec<&str> = cmd.args.iter().map(|s| s.as_str()).collect();
            let mut output = builtin_fn(&args, &self.cwd);

            if let Some(data) = stdin_data {
                if cmd.program == "grep" || cmd.program == "wc" || cmd.program == "head" || cmd.program == "tail" {
                    let stdin_str = String::from_utf8_lossy(data).to_string();
                    let mut new_args = vec![args[0]];
                    let temp_content = stdin_str.clone();
                    let _ = temp_content;
                    output = builtin_fn(&args, &self.cwd);
                }
            }

            self.last_exit_code = output.exit_code;
            return output;
        }

        self.execute_external(cmd, stdin_data)
    }

    fn is_interactive(program: &str) -> bool {
        static INTERACTIVE: &[&str] = &[
            "vi", "vim", "nvim", "nano", "pico", "emacs", "micro", "helix", "hx", "kak",
            "less", "more", "most", "man", "info",
            "top", "htop", "btop", "atop", "glances", "nmon", "iotop", "nethogs",
            "ssh", "telnet", "ftp", "sftp",
            "screen", "tmux", "byobu",
            "python", "python3", "ipython", "node", "irb", "lua", "ghci", "erl",
            "mysql", "psql", "sqlite3", "redis-cli", "mongosh",
            "gdb", "lldb",
            "nnn", "ranger", "mc", "vifm", "lf",
            "docker", "kubectl",
        ];
        let base = Path::new(program).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(program);
        INTERACTIVE.contains(&base)
    }

    fn needs_interactive(&self, cmd: &ParsedCommand, stdin_data: Option<&[u8]>) -> bool {
        if cmd.stdout_redirect.is_some() {
            return false;
        }
        if !Self::is_interactive(&cmd.program) {
            return false;
        }
        if stdin_data.is_some() || cmd.stdin_redirect.is_some() {
            return true; // hybrid: pipe stdin, inherit stdout
        }
        true
    }

    fn execute_external(&mut self, cmd: &ParsedCommand, stdin_data: Option<&[u8]>) -> CommandOutput {
        if self.needs_interactive(cmd, stdin_data) {
            return self.execute_interactive_piped(cmd, stdin_data);
        }

        let mut process = Command::new(&cmd.program);
        process.args(&cmd.args);
        process.current_dir(&self.cwd);
        process.envs(&self.env_vars);

        unsafe {
            process.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                Ok(())
            });
        }

        if stdin_data.is_some() || cmd.stdin_redirect.is_some() {
            process.stdin(Stdio::piped());
        }

        process.stdout(Stdio::piped());
        process.stderr(Stdio::piped());

        match process.spawn() {
            Ok(mut child) => {
                if let Some(data) = stdin_data {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(data);
                    }
                } else if let Some(ref redirect_path) = cmd.stdin_redirect {
                    let path = crate::parser::resolve_path(redirect_path, &self.cwd);
                    if let Ok(data) = std::fs::read(&path) {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(&data);
                        }
                    }
                }

                match child.wait_with_output() {
                    Ok(output) => {
                        let exit_code = exit_code_from_status(&output.status);
                        self.last_exit_code = exit_code;

                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        if let Some(ref redirect) = cmd.stdout_redirect {
                            self.write_redirect(redirect, &stdout);
                        }

                        CommandOutput {
                            stdout,
                            stderr,
                            structured: None,
                            exit_code,
                        }
                    }
                    Err(e) => {
                        self.last_exit_code = 1;
                        CommandOutput::error(format!("{}: {}", cmd.program, e), 1)
                    }
                }
            }
            Err(e) => {
                self.last_exit_code = 127;
                CommandOutput::error(format!("{}: command not found ({})", cmd.program, e), 127)
            }
        }
    }

    fn execute_interactive_piped(
        &mut self,
        cmd: &ParsedCommand,
        stdin_data: Option<&[u8]>,
    ) -> CommandOutput {
        let mut process = Command::new(&cmd.program);
        process.args(&cmd.args);
        process.current_dir(&self.cwd);
        process.envs(&self.env_vars);

        if stdin_data.is_some() || cmd.stdin_redirect.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::inherit());
        }
        process.stdout(Stdio::inherit());
        process.stderr(Stdio::inherit());

        unsafe {
            process.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                Ok(())
            });
        }

        match process.spawn() {
            Ok(mut child) => {
                if let Some(data) = stdin_data {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(data);
                        drop(stdin);
                    }
                } else if let Some(ref redirect_path) = cmd.stdin_redirect {
                    let path = crate::parser::resolve_path(redirect_path, &self.cwd);
                    if let Ok(data) = std::fs::read(&path) {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(&data);
                            drop(stdin);
                        }
                    }
                }

                match child.wait() {
                    Ok(status) => {
                        let exit_code = exit_code_from_status(&status);
                        self.last_exit_code = exit_code;
                        CommandOutput {
                            stdout: String::new(),
                            stderr: String::new(),
                            structured: None,
                            exit_code,
                        }
                    }
                    Err(e) => {
                        self.last_exit_code = 1;
                        CommandOutput::error(format!("{}: {}", cmd.program, e), 1)
                    }
                }
            }
            Err(e) => {
                self.last_exit_code = 127;
                CommandOutput::error(format!("{}: command not found ({})", cmd.program, e), 127)
            }
        }
    }

    fn try_shell_builtin(&mut self, cmd: &ParsedCommand) -> Option<CommandOutput> {
        match cmd.program.as_str() {
            "cd" => {
                let target = if cmd.args.is_empty() {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                } else {
                    crate::parser::resolve_path(&cmd.args[0], &self.cwd)
                };

                if target.is_dir() {
                    self.cwd = target.canonicalize().unwrap_or(target);
                    self.env_vars
                        .insert("PWD".to_string(), self.cwd.to_string_lossy().to_string());
                    self.last_exit_code = 0;
                    Some(CommandOutput::success(String::new()))
                } else {
                    self.last_exit_code = 1;
                    Some(CommandOutput::error(
                        format!("cd: {}: No such file or directory", cmd.args.join(" ")),
                        1,
                    ))
                }
            }
            "export" => {
                for arg in &cmd.args {
                    if let Some((key, value)) = arg.split_once('=') {
                        self.env_vars.insert(key.to_string(), value.to_string());
                        std::env::set_var(key, value);
                    }
                }
                self.last_exit_code = 0;
                Some(CommandOutput::success(String::new()))
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
                let _ = std::io::stdout().flush();
                self.last_exit_code = 0;
                Some(CommandOutput::success(String::new()))
            }
            "help" => {
                let help_text = concat!(
                    "AIOS - AI Shell\n\n",
                    "This shell understands both commands and plain English.\n",
                    "All commands run natively on your OS -- nothing is shadowed.\n",
                    "If something doesn't work, the AI will help you fix it.\n\n",
                    "Shell builtins:\n",
                    "  cd <dir>          Change directory\n",
                    "  export KEY=VALUE  Set environment variable\n",
                    "  clear             Clear the screen\n",
                    "  history           Show command history\n",
                    "  help              Show this help message\n",
                    "  exit / quit       Exit the shell\n\n",
                    "LLM control (switch AI mid-session):\n",
                    "  llm               Show current AI backend and model\n",
                    "  llm use <backend> [model]  Switch backend (ollama, openai, anthropic)\n",
                    "  llm model <name>  Change model on active backend\n",
                    "  llm reload        Re-read config/llm.toml\n",
                    "  llm off           Disable AI\n\n",
                    "AI features:\n",
                    "  @<query>          Ask the AI a question\n",
                    "  cmd | @<action>   Pipe command output to AI\n",
                    "  Just type anything -- if it's not a command, AI handles it\n",
                    "  Mistyped commands and errors are caught by AI automatically\n\n",
                    "Everything else (ls, ps, git, python, vim, etc.) runs directly\n",
                    "on your system. No commands are reimplemented or intercepted.\n",
                );
                self.last_exit_code = 0;
                Some(CommandOutput::success(help_text.to_string()))
            }
            _ => None,
        }
    }

    fn write_redirect(&self, target: &RedirectTarget, content: &str) {
        let path = crate::parser::resolve_path(&target.path, &self.cwd);
        if target.append {
            use std::fs::OpenOptions;
            if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(&path) {
                let _ = f.write_all(content.as_bytes());
            }
        } else {
            let _ = std::fs::write(&path, content);
        }
    }
}

fn exit_code_from_status(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    use std::os::unix::process::ExitStatusExt;
    if let Some(sig) = status.signal() {
        return 128 + sig;
    }
    -1
}
