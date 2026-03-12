mod completion;
mod executor;
mod history;
mod parser;
mod prompt;
mod router;
mod scripting;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use router::{HandleResult, ShellMode, ShellRouter};
use std::io::{self, Write};

use aios_llm::cloud::{AnthropicBackend, OpenAiBackend};
use aios_llm::config::LlmConfig;
use aios_llm::local::LocalBackend;
use aios_llm::network::NetworkBackend;
use aios_llm::LlmRouter;

fn load_llm_config() -> LlmConfig {
    llm_config_search_paths()
        .iter()
        .find_map(|p| LlmConfig::load(p).ok())
        .unwrap_or_default()
}

fn llm_config_search_paths() -> Vec<std::path::PathBuf> {
    let dirs = [
        Some(std::path::PathBuf::from("config")),
        Some(std::path::PathBuf::from("/etc/aios")),
        dirs::home_dir().map(|h| h.join(".config/aios")),
    ];
    let extensions = ["toml", "yaml", "yml", "json"];

    let mut paths = Vec::new();
    for dir in dirs.iter().flatten() {
        for ext in &extensions {
            paths.push(dir.join(format!("llm.{}", ext)));
        }
    }
    paths
}

fn build_llm_router(config: &LlmConfig) -> LlmRouter {
    let mut llm_router = LlmRouter::new();

    if config.local.enabled {
        llm_router.add_backend(Box::new(LocalBackend::new(config.local.clone())));
    }

    if config.network.enabled {
        llm_router.add_backend(Box::new(NetworkBackend::new(config.network.clone())));
    }

    if let Some(ref openai) = config.cloud.openai {
        if openai.enabled {
            llm_router.add_backend(Box::new(OpenAiBackend::new(openai.clone())));
        }
    }

    if let Some(ref anthropic) = config.cloud.anthropic {
        if anthropic.enabled {
            llm_router.add_backend(Box::new(AnthropicBackend::new(anthropic.clone())));
        }
    }

    llm_router
}

fn print_banner(has_ai: bool) {
    println!("\x1b[1;36m");
    println!("     _    ___ ___  ____  ");
    println!("    / \\  |_ _/ _ \\/ ___| ");
    println!("   / _ \\  | | | | \\___ \\ ");
    println!("  / ___ \\ | | |_| |___) |");
    println!(" /_/   \\_\\___\\___/|____/ ");
    println!("\x1b[0m");
    println!(" AI Operating System v0.1.0");
    if has_ai {
        println!(" \x1b[32mAI: online\x1b[0m | Type \x1b[1m@help\x1b[0m for AI commands");
    } else {
        println!(" \x1b[33mAI: offline\x1b[0m | Configure LLM in ~/.config/aios/llm.yml");
    }
    println!(" Type \x1b[1mhelp\x1b[0m for commands, \x1b[1mexit\x1b[0m to quit");
    println!();
}

#[tokio::main]
async fn main() {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let config = load_llm_config();
    let llm_router = build_llm_router(&config);
    let mut shell_router = ShellRouter::with_mode(Some(llm_router), config, ShellMode::Os);

    let os_system_prompt = r#"You are AIOS, an AI-powered operating system assistant running on {hostname} as user {username}.
Platform: {os_type} ({arch})
Current directory: {cwd}
System uptime: {uptime} seconds.

IMPORTANT platform facts:
- This is {os_type}. Give platform-correct commands ONLY.
- On macOS: use `brew` for packages. GNU coreutils are prefixed with `g` (e.g. `gfree`, `gstat`, `greadlink`). Use `vm_stat` for memory, `sysctl` for kernel info.
- On Linux: use `apt`, `dnf`, or `pacman` for packages. GNU commands have normal names.

You translate natural language into shell commands, explain errors, manage files, and help the user interact with their system efficiently.
When suggesting commands, prefer safe and reversible operations. Always explain potentially destructive actions before executing them.

Available commands: {commands}

Recent command history:
{history}"#;
    shell_router.context_manager =
        aios_llm::ContextManager::with_template(os_system_prompt.to_string());

    print_banner(shell_router.has_ai());

    let mut line_buffer = String::new();
    let mut cursor_pos: usize = 0;
    let mut history_index: Option<usize> = None;
    let mut saved_line = String::new();

    loop {
        let prompt_config = prompt::PromptConfig {
            ai_available: shell_router.has_ai(),
            ..Default::default()
        };
        let prompt_str = prompt::format_prompt(
            &shell_router.executor.cwd,
            shell_router.executor.last_exit_code,
            &prompt_config,
        );

        print!("{}", prompt_str);
        io::stdout().flush().unwrap();

        line_buffer.clear();
        cursor_pos = 0;
        history_index = None;

        if terminal::enable_raw_mode().is_err() {
            if read_line_fallback(&mut line_buffer).is_err() {
                break;
            }
        } else {
            let result = read_line_raw(
                &mut line_buffer,
                &mut cursor_pos,
                &mut history_index,
                &mut saved_line,
                &shell_router,
                &prompt_str,
            );
            let _ = terminal::disable_raw_mode();
            println!();

            match result {
                LineReadResult::Line => {}
                LineReadResult::Eof => break,
                LineReadResult::Interrupt => {
                    line_buffer.clear();
                    continue;
                }
            }
        }

        let input = line_buffer.trim().to_string();
        if input.is_empty() {
            continue;
        }

        if input == "history" {
            print!("{}", shell_router.history.format_display());
            continue;
        }

        match shell_router.handle_input(&input).await {
            HandleResult::Continue => {}
            HandleResult::Exit => break,
        }
    }

    println!("Goodbye.");
}

enum LineReadResult {
    Line,
    Eof,
    Interrupt,
}

fn read_line_raw(
    buffer: &mut String,
    cursor_pos: &mut usize,
    history_index: &mut Option<usize>,
    saved_line: &mut String,
    shell_router: &ShellRouter,
    prompt_str: &str,
) -> LineReadResult {
    loop {
        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event {
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => return LineReadResult::Line,

                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => return LineReadResult::Interrupt,

                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    if buffer.is_empty() {
                        return LineReadResult::Eof;
                    }
                }

                KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    print!("\x1b[2J\x1b[H{}{}", prompt_str, buffer);
                    io::stdout().flush().unwrap();
                }

                KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                    ..
                } => {
                    buffer.insert(*cursor_pos, c);
                    *cursor_pos += 1;
                    redraw_line(prompt_str, buffer, *cursor_pos);
                }

                KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                } => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                        buffer.remove(*cursor_pos);
                        redraw_line(prompt_str, buffer, *cursor_pos);
                    }
                }

                KeyEvent {
                    code: KeyCode::Delete,
                    ..
                } => {
                    if *cursor_pos < buffer.len() {
                        buffer.remove(*cursor_pos);
                        redraw_line(prompt_str, buffer, *cursor_pos);
                    }
                }

                KeyEvent {
                    code: KeyCode::Left,
                    ..
                } => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                        print!("\x1b[D");
                        io::stdout().flush().unwrap();
                    }
                }

                KeyEvent {
                    code: KeyCode::Right,
                    ..
                } => {
                    if *cursor_pos < buffer.len() {
                        *cursor_pos += 1;
                        print!("\x1b[C");
                        io::stdout().flush().unwrap();
                    }
                }

                KeyEvent {
                    code: KeyCode::Home,
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    *cursor_pos = 0;
                    redraw_line(prompt_str, buffer, *cursor_pos);
                }

                KeyEvent {
                    code: KeyCode::End,
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Char('e'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                } => {
                    *cursor_pos = buffer.len();
                    redraw_line(prompt_str, buffer, *cursor_pos);
                }

                KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    let entries = shell_router.history.entries();
                    if entries.is_empty() {
                        continue;
                    }
                    let new_index = match *history_index {
                        None => {
                            *saved_line = buffer.clone();
                            entries.len() - 1
                        }
                        Some(idx) => {
                            if idx > 0 {
                                idx - 1
                            } else {
                                continue;
                            }
                        }
                    };
                    *history_index = Some(new_index);
                    *buffer = entries[new_index].command.clone();
                    *cursor_pos = buffer.len();
                    redraw_line(prompt_str, buffer, *cursor_pos);
                }

                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    let entries = shell_router.history.entries();
                    match *history_index {
                        None => {}
                        Some(idx) => {
                            if idx + 1 < entries.len() {
                                let new_index = idx + 1;
                                *history_index = Some(new_index);
                                *buffer = entries[new_index].command.clone();
                            } else {
                                *history_index = None;
                                *buffer = saved_line.clone();
                            }
                            *cursor_pos = buffer.len();
                            redraw_line(prompt_str, buffer, *cursor_pos);
                        }
                    }
                }

                KeyEvent {
                    code: KeyCode::Tab, ..
                } => {
                    let completions =
                        completion::get_completions(buffer, &shell_router.executor.cwd);
                    if completions.is_empty() {
                        // nothing
                    } else {
                        let word_start = buffer.rfind(|c: char| c == ' ' || c == '\t')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let prefix = completion::longest_common_prefix(&completions);

                        if completions.len() == 1 {
                            let mut replacement = completions[0].clone();
                            if !replacement.ends_with('/') {
                                replacement.push(' ');
                            }
                            buffer.replace_range(word_start.., &replacement);
                            *cursor_pos = buffer.len();
                            redraw_line(prompt_str, buffer, *cursor_pos);
                        } else if prefix.len() > buffer[word_start..].len() {
                            buffer.replace_range(word_start.., &prefix);
                            *cursor_pos = buffer.len();
                            redraw_line(prompt_str, buffer, *cursor_pos);
                        } else {
                            let width = crossterm::terminal::size()
                                .map(|(w, _)| w as usize)
                                .unwrap_or(80);
                            let display = completion::format_columns(&completions, width);
                            print!("\r\n{}", display);
                            print!("{}{}", prompt_str, buffer);
                            io::stdout().flush().unwrap();
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

fn redraw_line(prompt: &str, buffer: &str, cursor_pos: usize) {
    let prompt_len = prompt::strip_ansi(prompt).len();
    print!("\r\x1b[K{}{}", prompt, buffer);
    let total_pos = prompt_len + cursor_pos;
    print!("\r\x1b[{}C", total_pos);
    io::stdout().flush().unwrap();
}

fn read_line_fallback(buffer: &mut String) -> io::Result<()> {
    io::stdin().read_line(buffer)?;
    if buffer.is_empty() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
    }
    *buffer = buffer
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string();
    Ok(())
}
