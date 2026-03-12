use crate::executor::Executor;
use crate::history::History;
use crate::parser::{self, InputClassification};
use aios_core::CommandOutput;
use aios_llm::{
    CompletionRequest, ContextManager, LlmRouter, Message, MessageRole, OsState,
};
use aios_llm::config::LlmConfig;
use aios_llm::cloud::{AnthropicBackend, OpenAiBackend};
use aios_llm::local::LocalBackend;
use aios_llm::network::NetworkBackend;
use aios_knowledge::KnowledgeIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellMode {
    Shell,
    Os,
}

pub struct ShellRouter {
    pub executor: Executor,
    pub history: History,
    pub llm_router: Option<LlmRouter>,
    pub llm_config: LlmConfig,
    pub context_manager: ContextManager,
    pub knowledge: KnowledgeIndex,
    pub conversation_history: Vec<Message>,
    pub max_conversation_history: usize,
    pub mode: ShellMode,
}

impl ShellRouter {
    pub fn new(llm_router: Option<LlmRouter>, config: LlmConfig) -> Self {
        Self::with_mode(llm_router, config, ShellMode::Shell)
    }

    pub fn with_mode(llm_router: Option<LlmRouter>, config: LlmConfig, mode: ShellMode) -> Self {
        Self {
            executor: Executor::new(),
            history: History::new(10000),
            llm_router,
            llm_config: config,
            context_manager: ContextManager::new(),
            knowledge: KnowledgeIndex::new(),
            conversation_history: Vec::new(),
            max_conversation_history: 20,
            mode,
        }
    }

    pub fn has_ai(&self) -> bool {
        self.llm_router
            .as_ref()
            .map(|r| !r.available_backends().is_empty())
            .unwrap_or(false)
    }

    pub fn rebuild_router_from_config(&mut self) {
        let mut router = LlmRouter::new();
        let config = &self.llm_config;

        if config.local.enabled {
            router.add_backend(Box::new(LocalBackend::new(config.local.clone())));
        }
        if config.network.enabled {
            router.add_backend(Box::new(NetworkBackend::new(config.network.clone())));
        }
        if let Some(ref openai) = config.cloud.openai {
            if openai.enabled {
                router.add_backend(Box::new(OpenAiBackend::new(openai.clone())));
            }
        }
        if let Some(ref anthropic) = config.cloud.anthropic {
            if anthropic.enabled {
                router.add_backend(Box::new(AnthropicBackend::new(anthropic.clone())));
            }
        }

        self.llm_router = Some(router);
    }

    fn handle_llm_command(&mut self, args: &str) -> HandleResult {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let subcmd = parts.first().copied().unwrap_or("status");

        match subcmd {
            "status" | "info" => {
                if let Some(ref router) = self.llm_router {
                    let backends = router.backend_info();
                    if backends.is_empty() {
                        println!("  No backends configured. Edit config/llm.toml or use: llm use <backend>");
                    } else {
                        println!("  LLM backends:");
                        for (name, model, available) in &backends {
                            let status = if *available {
                                "\x1b[32monline\x1b[0m"
                            } else {
                                "\x1b[31moffline\x1b[0m"
                            };
                            println!("    {} ({}) [{}]", name, model, status);
                        }
                    }
                } else {
                    println!("  AI is disabled. Use 'llm reload' or 'llm use <backend>' to enable.");
                }
            }

            "reload" => {
                let config_paths = crate::llm_config_search_paths();

                match config_paths.iter().find_map(|p| {
                    LlmConfig::load(p).ok().map(|c| (c, p.clone()))
                }) {
                    Some((config, path)) => {
                        self.llm_config = config;
                        self.rebuild_router_from_config();
                        println!("  Reloaded from {}", path.display());
                        self.print_llm_status();
                    }
                    None => {
                        println!("  No config file found. Searched:");
                        for p in &config_paths {
                            println!("    {}", p.display());
                        }
                    }
                }
            }

            "use" => {
                if parts.len() < 2 {
                    println!("  Usage: llm use <backend> [model]");
                    println!("  Backends: ollama/network, openai, anthropic");
                    println!("  Example: llm use ollama mistral");
                    println!("           llm use openai gpt-4o");
                    return HandleResult::Continue;
                }

                let backend = parts[1];
                let model = parts.get(2).copied();

                self.llm_config.local.enabled = false;
                self.llm_config.network.enabled = false;
                if let Some(ref mut openai) = self.llm_config.cloud.openai {
                    openai.enabled = false;
                }
                if let Some(ref mut anthropic) = self.llm_config.cloud.anthropic {
                    anthropic.enabled = false;
                }

                match backend {
                    "ollama" | "network" => {
                        self.llm_config.network.enabled = true;
                        if let Some(m) = model {
                            self.llm_config.network.model = m.to_string();
                        }
                    }
                    "openai" => {
                        let cfg = self.llm_config.cloud.openai.get_or_insert(
                            aios_llm::config::CloudProviderConfig {
                                enabled: true,
                                api_key_env: "OPENAI_API_KEY".to_string(),
                                model: "gpt-4o".to_string(),
                            },
                        );
                        cfg.enabled = true;
                        if let Some(m) = model {
                            cfg.model = m.to_string();
                        }
                    }
                    "anthropic" => {
                        let cfg = self.llm_config.cloud.anthropic.get_or_insert(
                            aios_llm::config::CloudProviderConfig {
                                enabled: true,
                                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                                model: "claude-sonnet-4-20250514".to_string(),
                            },
                        );
                        cfg.enabled = true;
                        if let Some(m) = model {
                            cfg.model = m.to_string();
                        }
                    }
                    "local" => {
                        self.llm_config.local.enabled = true;
                        if let Some(m) = model {
                            self.llm_config.local.model_path = m.to_string();
                        }
                    }
                    _ => {
                        println!("  Unknown backend '{}'. Options: ollama, openai, anthropic, local", backend);
                        return HandleResult::Continue;
                    }
                }

                self.rebuild_router_from_config();
                self.print_llm_status();
            }

            "model" => {
                if parts.len() < 2 {
                    println!("  Usage: llm model <name>");
                    println!("  Changes the model on the active backend.");
                    return HandleResult::Continue;
                }

                let model = parts[1];

                if self.llm_config.network.enabled {
                    self.llm_config.network.model = model.to_string();
                } else if let Some(ref mut openai) = self.llm_config.cloud.openai {
                    if openai.enabled {
                        openai.model = model.to_string();
                    }
                } else if let Some(ref mut anthropic) = self.llm_config.cloud.anthropic {
                    if anthropic.enabled {
                        anthropic.model = model.to_string();
                    }
                } else {
                    println!("  No active backend. Use 'llm use <backend>' first.");
                    return HandleResult::Continue;
                }

                self.rebuild_router_from_config();
                self.print_llm_status();
            }

            "off" => {
                self.llm_router = Some(LlmRouter::new());
                println!("  AI disabled.");
            }

            _ => {
                println!("  llm commands:");
                println!("    llm                    Show current AI status");
                println!("    llm use <backend> [model]  Switch backend (ollama, openai, anthropic)");
                println!("    llm model <name>       Change model on active backend");
                println!("    llm reload             Reload config/llm.toml");
                println!("    llm off                Disable AI");
            }
        }

        HandleResult::Continue
    }

    fn print_llm_status(&self) {
        if let Some(ref router) = self.llm_router {
            let backends = router.backend_info();
            let active: Vec<_> = backends.iter().filter(|(_, _, avail)| *avail).collect();
            if active.is_empty() {
                println!("  AI: \x1b[33moffline\x1b[0m (no available backends)");
            } else {
                for (name, model, _) in &active {
                    println!("  AI: \x1b[32monline\x1b[0m via {} ({})", name, model);
                }
            }
        } else {
            println!("  AI: \x1b[33mdisabled\x1b[0m");
        }
    }

    pub async fn handle_input(&mut self, input: &str) -> HandleResult {
        let trimmed = input.trim();
        if trimmed == "llm" || trimmed.starts_with("llm ") {
            let args = trimmed.strip_prefix("llm").unwrap_or("").trim();
            return self.handle_llm_command(args);
        }

        let expanded = parser::expand_variables(input, &self.executor.env_vars);

        let chain = parser::split_chain(&expanded);
        if chain.len() > 1 {
            return self.handle_chain(input, chain).await;
        }

        let classification = parser::classify_input(&expanded);

        match classification {
            InputClassification::Empty => HandleResult::Continue,
            InputClassification::Exit => HandleResult::Exit,

            InputClassification::DirectCommand(pipeline) => {
                let output = self.executor.execute_pipeline(&pipeline);

                match self.mode {
                    ShellMode::Shell => {
                        if output.exit_code == 127 {
                            self.handle_command_not_found(input, &output).await;
                        } else if output.exit_code != 0 {
                            self.print_output(&output);
                            self.handle_command_failed(input, &output).await;
                        } else {
                            self.print_output(&output);
                        }
                    }
                    ShellMode::Os => {
                        self.print_output(&output);
                        if output.exit_code != 0 && self.has_ai() {
                            self.handle_command_failed(input, &output).await;
                        }
                    }
                }

                self.record_history(input, output.exit_code);
                HandleResult::Continue
            }

            InputClassification::NaturalLanguage(text) => {
                match self.mode {
                    ShellMode::Shell => {
                        self.handle_natural_language(input, &text).await;
                    }
                    ShellMode::Os => {
                        if self.has_ai() {
                            self.handle_ai_query(&text).await;
                        } else {
                            eprintln!(
                                "aios: unknown command: {}",
                                text.split_whitespace().next().unwrap_or(&text)
                            );
                            eprintln!("  (AI not available - configure LLM in config/llm.toml)");
                        }
                    }
                }
                self.record_history(input, self.executor.last_exit_code);
                HandleResult::Continue
            }

            InputClassification::Ambiguous(_text, pipeline) => {
                let output = self.executor.execute_pipeline(&pipeline);

                match self.mode {
                    ShellMode::Shell => {
                        if output.exit_code == 127 {
                            self.handle_command_not_found(input, &output).await;
                        } else if output.exit_code != 0 {
                            self.print_output(&output);
                            self.handle_command_failed(input, &output).await;
                        } else {
                            self.print_output(&output);
                        }
                    }
                    ShellMode::Os => {
                        if output.exit_code == 127 && self.has_ai() {
                            self.handle_ai_query(input).await;
                        } else {
                            self.print_output(&output);
                            if output.exit_code != 0 && self.has_ai() {
                                self.handle_command_failed(input, &output).await;
                            }
                        }
                    }
                }

                self.record_history(input, self.executor.last_exit_code);
                HandleResult::Continue
            }

            InputClassification::AiExplicit(query) => {
                self.handle_ai_query(&query).await;
                self.record_history(input, 0);
                HandleResult::Continue
            }

            InputClassification::AiPipe(pipeline, action) => {
                let output = self.executor.execute_pipeline(&pipeline);
                if !output.stdout.is_empty() {
                    print!("{}", output.stdout);
                }
                if output.exit_code != 0 && output.exit_code == 127 {
                    self.handle_command_not_found(input, &output).await;
                } else if output.exit_code != 0 && output.stdout.is_empty() {
                    eprintln!(
                        "\x1b[33mCommand exited with code {} and produced no output.\x1b[0m",
                        output.exit_code
                    );
                } else {
                    println!("\x1b[90m--- AI analysis ---\x1b[0m");
                    let prompt = format!(
                        "The user piped the output of a command and asked: \"{}\"\n\n\
                         Answer the question directly and concisely based on the output below.\n\n\
                         Command output:\n```\n{}\n```",
                        action, output.stdout
                    );
                    self.handle_ai_query(&prompt).await;
                }
                self.record_history(input, output.exit_code);
                HandleResult::Continue
            }
        }
    }

    async fn handle_chain(
        &mut self,
        original_input: &str,
        segments: Vec<(String, parser::ChainOp)>,
    ) -> HandleResult {
        let mut last_exit = 0i32;
        for (i, (segment, op)) in segments.iter().enumerate() {
            if i > 0 {
                let prev_op = &segments[i - 1].1;
                match prev_op {
                    parser::ChainOp::And if last_exit != 0 => continue,
                    parser::ChainOp::Or if last_exit == 0 => continue,
                    _ => {}
                }
            }

            let classification = parser::classify_input(segment);
            match &classification {
                parser::InputClassification::Exit => return HandleResult::Exit,
                parser::InputClassification::Empty => {}
                parser::InputClassification::DirectCommand(pipeline)
                | parser::InputClassification::Ambiguous(_, pipeline) => {
                    let output = self.executor.execute_pipeline(pipeline);
                    last_exit = output.exit_code;
                    if output.exit_code == 127 {
                        self.handle_command_not_found(segment, &output).await;
                    } else if output.exit_code != 0 {
                        self.print_output(&output);
                    } else {
                        self.print_output(&output);
                    }
                }
                parser::InputClassification::AiExplicit(query) => {
                    self.handle_ai_query(query).await;
                    last_exit = 0;
                }
                parser::InputClassification::NaturalLanguage(text) => {
                    self.handle_natural_language(segment, text).await;
                    last_exit = self.executor.last_exit_code;
                }
                parser::InputClassification::AiPipe(pipeline, action) => {
                    let output = self.executor.execute_pipeline(pipeline);
                    last_exit = output.exit_code;
                    if !output.stdout.is_empty() {
                        print!("{}", output.stdout);
                    }
                    if output.exit_code == 127 {
                        self.handle_command_not_found(segment, &output).await;
                    } else if output.exit_code != 0 && output.stdout.is_empty() {
                        eprintln!(
                            "\x1b[33mCommand exited with code {} and produced no output.\x1b[0m",
                            output.exit_code
                        );
                    } else {
                        println!("\x1b[90m--- AI analysis ---\x1b[0m");
                        let prompt = format!(
                            "The user piped the output of a command and asked: \"{}\"\n\n\
                             Answer the question directly and concisely based on the output below.\n\n\
                             Command output:\n```\n{}\n```",
                            action, output.stdout
                        );
                        self.handle_ai_query(&prompt).await;
                    }
                }
            }
        }
        self.record_history(original_input, last_exit);
        HandleResult::Continue
    }

    async fn handle_command_not_found(&mut self, original_input: &str, output: &CommandOutput) {
        let cmd_name = original_input.split_whitespace().next().unwrap_or(original_input);
        let system_investigation = self.investigate_missing_command(cmd_name);

        if self.has_ai() {
            let mut query = format!(
                "The user typed '{}' but it was not found.\n",
                original_input,
            );

            if !system_investigation.is_empty() {
                query.push_str("\nSystem investigation results (THESE ARE FACTS, trust them):\n");
                query.push_str(&system_investigation);
                query.push('\n');
            }

            query.push_str(
                "\nBased on the FACTS above, help the user. Rules:\n\
                 - If the investigation found the command at a specific path, tell the user that exact path.\n\
                 - If similar commands were found, suggest the closest match.\n\
                 - NEVER guess paths. Only suggest paths that appear in the investigation results.\n\
                 - If nothing was found, explain how to install it using the correct package manager for this OS.\n\
                 - Be concise: one or two sentences, then the command to run."
            );

            self.handle_ai_query(&query).await;
        } else {
            if !system_investigation.is_empty() {
                eprintln!("{}", system_investigation);
            } else {
                eprintln!(
                    "\x1b[33m'{}' is not a recognized command.\x1b[0m",
                    cmd_name
                );
                self.print_similar_commands(cmd_name);
                eprintln!("\x1b[90m  Tip: configure an LLM in config/llm.toml for AI-assisted help\x1b[0m");
            }
        }
    }

    async fn handle_command_failed(&mut self, original_input: &str, output: &CommandOutput) {
        if self.has_ai() {
            let mut query = format!(
                "The user ran '{}' and it failed with exit code {}.\n\
                 stdout: {}\n\
                 stderr: {}\n",
                original_input,
                output.exit_code,
                if output.stdout.is_empty() { "(empty)" } else { &output.stdout },
                if output.stderr.is_empty() { "(empty)" } else { &output.stderr },
            );

            if output.exit_code == 126 || output.exit_code == 127 {
                let cmd_name = original_input.split_whitespace().next().unwrap_or(original_input);
                let investigation = self.investigate_missing_command(cmd_name);
                if !investigation.is_empty() {
                    query.push_str("\nSystem investigation (FACTS):\n");
                    query.push_str(&investigation);
                    query.push('\n');
                }
            }

            query.push_str(
                "\nExplain what went wrong in one or two sentences. \
                 Then suggest how to fix it. NEVER guess at file paths -- only suggest paths you know exist. Be concise."
            );

            let request = self.build_ai_request(&query);
            if let Some(ref router) = self.llm_router {
                match router.complete(request).await {
                    Ok(response) => {
                        eprintln!();
                        eprintln!("\x1b[33m  {}\x1b[0m", response.content.replace('\n', "\n  "));
                    }
                    Err(_) => {}
                }
            }
        }
    }

    async fn handle_natural_language(&mut self, original_input: &str, text: &str) {
        if self.has_ai() {
            self.handle_ai_query(text).await;
        } else {
            let knowledge = self.knowledge.query_for_context(text);
            if !knowledge.is_empty() {
                println!("{}", knowledge);
            } else {
                let first_word = original_input.split_whitespace().next().unwrap_or(original_input);
                eprintln!(
                    "\x1b[33m'{}' is not a recognized command.\x1b[0m",
                    first_word
                );
                self.print_similar_commands(first_word);
                eprintln!("\x1b[90m  Tip: configure an LLM in config/llm.toml for natural language support\x1b[0m");
            }
        }
    }

    fn gather_directory_context(&self, path: &std::path::Path, depth: u32) -> String {
        let mut lines = Vec::new();
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(e) => return format!("(cannot read {}: {})", path.display(), e),
        };

        let mut items: Vec<_> = entries.flatten().collect();
        items.sort_by_key(|e| e.file_name());

        for entry in &items {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                lines.push(format!("  {}/", name));
                if depth > 0 {
                    let sub = self.gather_directory_context(&entry.path(), depth - 1);
                    for sub_line in sub.lines() {
                        lines.push(format!("  {}", sub_line));
                    }
                }
            } else {
                let size = meta.len();
                let size_str = if size >= 1_048_576 {
                    format!("{:.1}M", size as f64 / 1_048_576.0)
                } else if size >= 1024 {
                    format!("{:.1}K", size as f64 / 1024.0)
                } else {
                    format!("{}B", size)
                };
                lines.push(format!("  {} ({})", name, size_str));
            }
        }

        if lines.is_empty() {
            "(empty directory)".to_string()
        } else {
            lines.join("\n")
        }
    }

    fn extract_query_path(&self, query: &str) -> (std::path::PathBuf, String) {
        let cwd = self.executor.cwd.clone();

        let words: Vec<&str> = query.split_whitespace().collect();
        for word in &words {
            let cleaned = word.trim_matches(|c: char| c == ',' || c == '?' || c == '!' || c == '\'' || c == '"');
            if cleaned.is_empty() {
                continue;
            }

            let candidate = if cleaned.starts_with('~') {
                if let Some(home) = dirs::home_dir() {
                    home.join(cleaned[1..].trim_start_matches('/'))
                } else {
                    continue;
                }
            } else if cleaned.starts_with('/') || cleaned.starts_with("./") || cleaned.starts_with("../") {
                if std::path::Path::new(cleaned).is_absolute() {
                    std::path::PathBuf::from(cleaned)
                } else {
                    cwd.join(cleaned)
                }
            } else if cleaned.contains('/') && !cleaned.contains("://") {
                let candidate = cwd.join(cleaned);
                if candidate.exists() {
                    candidate
                } else {
                    continue;
                }
            } else {
                continue;
            };

            if candidate.is_dir() {
                return (candidate, cleaned.to_string());
            }
        }

        (cwd, ".".to_string())
    }

    fn build_directory_prompt(&self, query: &str) -> String {
        let (target_dir, path_str) = self.extract_query_path(query);

        let depth = if path_str == "." { 0 } else { 1 };
        let listing = self.gather_directory_context(&target_dir, depth);

        format!(
            "I already ran `ls` on {}. Here are the results:\n\n{}\n\n{}",
            target_dir.display(),
            listing,
            query
        )
    }

    async fn handle_ai_query(&mut self, query: &str) {
        let knowledge_context = self.knowledge.query_for_context(query);

        let os_state = self.build_os_state();
        let mut system_prompt = self.context_manager.build_system_prompt(&os_state);
        if !knowledge_context.is_empty() {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(&knowledge_context);
        }

        let augmented_query = self.build_directory_prompt(query);

        self.conversation_history.push(Message {
            role: MessageRole::User,
            content: augmented_query.clone(),
        });

        if self.conversation_history.len() > self.max_conversation_history {
            self.conversation_history.remove(0);
        }

        let request = CompletionRequest {
            system_prompt: Some(system_prompt),
            messages: self.conversation_history.clone(),
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: false,
        };

        if let Some(ref router) = self.llm_router {
            match router.complete(request).await {
                Ok(response) => {
                    println!("\x1b[36m{}\x1b[0m", response.content);
                    self.conversation_history.push(Message {
                        role: MessageRole::Assistant,
                        content: response.content,
                    });
                }
                Err(e) => {
                    eprintln!("\x1b[31mAI error: {}\x1b[0m", e);
                }
            }
        } else {
            let knowledge = self.knowledge.query_for_context(query);
            if !knowledge.is_empty() {
                println!("{}", knowledge);
            } else {
                eprintln!("\x1b[90mNo AI backend configured. Edit config/llm.toml to enable one.\x1b[0m");
            }
        }
    }

    fn build_ai_request(&self, query: &str) -> CompletionRequest {
        let os_state = self.build_os_state();
        let system_prompt = self.context_manager.build_system_prompt(&os_state);

        CompletionRequest {
            system_prompt: Some(system_prompt),
            messages: vec![Message {
                role: MessageRole::User,
                content: query.to_string(),
            }],
            max_tokens: Some(512),
            temperature: Some(0.3),
            stream: false,
        }
    }

    fn print_similar_commands(&self, input: &str) {
        let all_commands = [
            "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "grep", "find", "wc", "head",
            "tail", "ps", "kill", "top", "echo", "env", "pwd", "chmod", "df", "du", "date",
            "uptime", "whoami", "hostname", "cd", "export", "clear", "history", "help",
        ];

        let input_lower = input.to_lowercase();
        let mut suggestions: Vec<(&str, usize)> = all_commands
            .iter()
            .filter_map(|cmd| {
                let dist = levenshtein(&input_lower, cmd);
                if dist <= 2 || cmd.starts_with(&input_lower) || input_lower.starts_with(cmd) {
                    Some((*cmd, dist))
                } else {
                    None
                }
            })
            .collect();

        suggestions.sort_by_key(|(_, d)| *d);

        if !suggestions.is_empty() {
            let names: Vec<&str> = suggestions.iter().take(3).map(|(n, _)| *n).collect();
            eprintln!("\x1b[90m  Did you mean: {}?\x1b[0m", names.join(", "));
        }
    }

    fn investigate_missing_command(&self, cmd_name: &str) -> String {
        let mut findings = Vec::new();

        // 1. Check if it exists somewhere in common paths not in PATH
        let extra_search_paths = [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/opt/homebrew/opt/coreutils/libexec/gnubin",
            "/usr/local/bin",
            "/usr/local/sbin",
            "/usr/local/opt/coreutils/libexec/gnubin",
            "/usr/sbin",
            "/sbin",
            "/snap/bin",
        ];

        let mut found_at = Vec::new();
        for dir in &extra_search_paths {
            let candidate = format!("{}/{}", dir, cmd_name);
            if std::path::Path::new(&candidate).exists() {
                found_at.push(candidate);
            }
        }

        if !found_at.is_empty() {
            findings.push(format!(
                "- '{}' EXISTS at: {}",
                cmd_name,
                found_at.join(", ")
            ));
            findings.push(format!(
                "- It's not in the user's current PATH. They can run it directly or add the directory to PATH."
            ));
        }

        // 2. Search for similar-named commands in PATH + extra dirs
        let all_search_dirs: Vec<String> = std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(String::from)
            .chain(extra_search_paths.iter().map(|s| s.to_string()))
            .collect();

        let cmd_lower = cmd_name.to_lowercase();
        let mut similar_found = Vec::new();
        for dir in &all_search_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let name_lower = name.to_lowercase();
                    let is_similar = name_lower.contains(&cmd_lower)
                        || cmd_lower.contains(&name_lower)
                        || (name_lower.starts_with("g") && name_lower[1..] == cmd_lower)
                        || levenshtein(&cmd_lower, &name_lower) <= 1;

                    if is_similar && name != cmd_name {
                        let full_path = format!("{}/{}", dir, name);
                        if !similar_found.iter().any(|(_, n): &(String, String)| n == &name) {
                            similar_found.push((full_path, name));
                        }
                    }
                }
            }
        }

        if !similar_found.is_empty() {
            similar_found.sort_by(|(_, a), (_, b)| {
                levenshtein(&a.to_lowercase(), &cmd_lower)
                    .cmp(&levenshtein(&b.to_lowercase(), &cmd_lower))
            });
            similar_found.truncate(5);
            let similar_list: Vec<String> = similar_found
                .iter()
                .map(|(path, name)| format!("'{}' (at {})", name, path))
                .collect();
            findings.push(format!(
                "- Similar commands found on this system: {}",
                similar_list.join(", ")
            ));
        }

        // 3. Detect package manager
        let pkg_manager = if std::path::Path::new("/opt/homebrew/bin/brew").exists()
            || std::path::Path::new("/usr/local/bin/brew").exists()
        {
            Some("brew")
        } else if std::path::Path::new("/usr/bin/apt").exists() {
            Some("apt")
        } else if std::path::Path::new("/usr/bin/dnf").exists() {
            Some("dnf")
        } else if std::path::Path::new("/usr/bin/pacman").exists() {
            Some("pacman")
        } else {
            None
        };

        if let Some(pm) = pkg_manager {
            findings.push(format!("- Package manager available: {}", pm));
        }

        // 4. Detect homebrew prefix specifically (Intel vs Apple Silicon)
        if cfg!(target_os = "macos") {
            if std::path::Path::new("/opt/homebrew").exists() {
                findings.push("- Homebrew prefix: /opt/homebrew (Apple Silicon)".to_string());
            } else if std::path::Path::new("/usr/local/Cellar").exists() {
                findings.push("- Homebrew prefix: /usr/local (Intel Mac)".to_string());
            }
        }

        if found_at.is_empty() && similar_found.is_empty() {
            findings.push(format!(
                "- '{}' was NOT found anywhere on this system.",
                cmd_name
            ));
        }

        findings.join("\n")
    }

    fn build_os_state(&self) -> OsState {
        let os_type = if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else {
            std::env::consts::OS.to_string()
        };

        let shell_name = match self.mode {
            ShellMode::Shell => "AIOS Shell".to_string(),
            ShellMode::Os => "AIOS OS".to_string(),
        };

        OsState {
            hostname: aios_kernel::network::get_hostname(),
            username: std::env::var("USER").unwrap_or_else(|_| "user".to_string()),
            cwd: self.executor.cwd.to_string_lossy().to_string(),
            os_type,
            arch: std::env::consts::ARCH.to_string(),
            shell_name,
            uptime_secs: aios_kernel::memory::get_uptime()
                .map(|(u, _)| u)
                .unwrap_or(0.0),
            available_commands: vec![
                "ls", "cat", "cp", "mv", "rm", "mkdir", "rmdir", "grep", "find", "wc", "head",
                "tail", "ps", "kill", "top", "echo", "env", "pwd", "chmod", "df", "du", "date",
                "uptime", "whoami", "hostname", "cd", "export", "clear", "help",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            recent_history: self.history.recent_commands(10),
        }
    }

    fn record_history(&mut self, input: &str, exit_code: i32) {
        self.history.add(
            input.to_string(),
            exit_code,
            &self.executor.cwd.to_string_lossy(),
        );
    }

    fn print_output(&self, output: &CommandOutput) {
        if !output.stdout.is_empty() {
            print!("{}", output.stdout);
            if !output.stdout.ends_with('\n') {
                println!();
            }
        }
        if !output.stderr.is_empty() {
            eprint!("{}", output.stderr);
            if !output.stderr.ends_with('\n') {
                eprintln!();
            }
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

pub enum HandleResult {
    Continue,
    Exit,
}
