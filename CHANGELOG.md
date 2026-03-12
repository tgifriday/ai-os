# Changelog

## 0.1.4 -- 2026-03-12

### CI fixes

- Switched `reqwest` from `native-tls` to `rustls-tls` (pure Rust TLS) so Linux and Windows CI builds no longer require OpenSSL system headers
- Removed `aarch64-pc-windows-msvc` target (can't cross-compile from x86_64 Windows runners)
- Added Rust build caching via `Swatinem/rust-cache` to speed up CI

## 0.1.3 -- 2026-03-12

### Release workflow and Windows compatibility

- Added `.github/workflows/release.yml` -- push a `v*` tag and GitHub Actions builds release binaries for all platforms
- Targets: macOS (Intel + Apple Silicon), Linux (x86_64 + ARM64 + musl), Windows (x86_64 + ARM64)
- Each release includes `aish`, `aios-os`, default config, licenses, and README in a tarball (Unix) or zip (Windows)
- Added `#[cfg(unix)]` / `#[cfg(not(unix))]` guards to all Unix-specific code (`libc`, `nix`, `pre_exec`, signal handling, `MetadataExt`)
- `nix` and `libc` are now Unix-only dependencies via `[target.'cfg(unix)'.dependencies]`
- Windows builds compile with stub fallbacks for signal handling, process management, filesystem metadata, disk usage, and hostname

### AI context control

- Added `sanitize` builtin to clear the AI conversation context without affecting shell history, cwd, env vars, or LLM configuration
- Added `sanitize` to shell command recognition, help output, and tab completion in both `aish` and `aios-os`

### AI pipe isolation and capture

- `cmd | @question` no longer sends prior conversation history to the LLM and does not pollute later `@query` conversations
- AI-pipe execution now forces captured stdout for analysis, so commands like `ssh host "nvidia-smi" | @summarize` work correctly
- Added a dedicated AI-pipe prompt format with explicit `<command-output>` markers so short or simple piped text like `echo "hello" | @summarize` is handled reliably

---

## 0.1.2 -- 2026-03-12

### AI pipe (`cmd | @question`)

- `ls -al | @what's the largest file` now shows the command output **first**, then the AI's analysis below a separator
- Previously, piped output was silently captured and only sent to the AI -- the user never saw it
- Improved AI prompt: asks the LLM to answer the user's question directly and concisely rather than giving generic advice
- Handles errors gracefully: command-not-found triggers investigation; non-zero exit with no output warns the user

### Knowledge base accuracy

- Rewrote `aios-overview` concept to reflect actual 6-crate workspace and dual-binary (`aish` / `aios-os`) architecture
- Rewrote `ai-shell` concept to document real features (`@query`, `| @question`, fallback mode) instead of non-existent ones (`|ai`, `Ctrl+A`, `--explain`)
- Updated `llm-config` concept with actual backend names (Ollama, OpenAI, Anthropic) and `llm use/model/reload/off` commands
- Updated `pipes-redirects` to use `| @` syntax instead of `|ai`
- Replaced `shell-scripting` concept with accurate chaining/glob/substitution documentation

### System prompt improvements

- AI no longer suggests random commands for informational questions (e.g., `man hostname` when asked about AIOS)
- Prompt now instructs: "Only suggest a command when it directly answers what the user asked"
- Added instruction to answer from knowledge context when asked about aish/AIOS itself

---

## 0.1.1 -- 2026-03-12

### Shell plumbing -- glob, chaining, command substitution

- **Glob expansion**: `*`, `?`, `[...]` patterns are now expanded by the shell. `du -sh ./*`, `ls *.txt`, `rm src/*.o` all work correctly. Quoted globs (`"*.txt"`) are passed literally as expected.
- **Command chaining**: `;`, `&&`, `||` are now supported. `mkdir foo && cd foo`, `make || echo failed`, `cmd1; cmd2; cmd3` all work.
- **Command substitution**: `$(cmd)` and `` `cmd` `` are expanded inline. `echo $(date)`, `export VER=$(git rev-parse HEAD)` work.
- **`is_known_command` now checks PATH** -- commands no longer need to be in a hardcoded list to be classified correctly. Any executable on PATH is recognized.
- **Pipe splitting respects `||`** -- `cmd1 || cmd2` is no longer misinterpreted as a pipe.

### Tab completion rewrite

- Common-prefix fill: multiple matches auto-complete to shared prefix before listing options
- Columnar display of completions, sized to terminal width
- Tilde expansion in completions (`~/Doc<tab>` works)
- Hidden files filtered unless you type the leading dot
- Only real shell builtins in the completion list; all other commands come from PATH

### Config format flexibility

- Config files can now be TOML, YAML, or JSON -- auto-detected by extension
- Search paths now try `llm.toml`, `llm.yaml`, `llm.yml`, `llm.json` in each config directory

### Signal handling

- Ctrl-C no longer kills the shell -- only the running child process is terminated
- Shell ignores SIGINT, SIGQUIT, and SIGTSTP; child processes receive them normally
- Signal-killed processes report correct exit codes (e.g., 130 for SIGINT)

### Interactive / TUI programs

- `vi`, `vim`, `nano`, `less`, `top`, `htop`, `man`, `ssh`, `python`, `node`, `psql`, `tmux`, and ~40 other interactive programs now work correctly
- These programs get direct terminal access (inherited stdio) instead of having their output piped
- Pagers in pipelines (`ps -ef | more`, `cat file | less`) now work -- stdin is piped while stdout stays on the terminal

### Install

- Added `aish` as the primary binary name (short for AI Shell)
- `make install` copies `aish` to `/usr/local/bin` and default config to `~/.config/aios/llm.yml`
- Existing config is never overwritten during install
- `cargo install --path aios-shell` also works
- `aios-shell` binary still exists as an alias

### License

- Changed from MIT-only to dual **MIT OR Apache-2.0** (Rust ecosystem standard)
- Added `LICENSE-MIT` and `LICENSE-APACHE` files
- All 6 crates now inherit license and repository from workspace
- Added `CONTRIBUTING.md` with contribution licensing terms

---

## 0.1.0 -- 2026-03-12

Initial release.

### Shell (`aish`)

- Drop-in shell replacement -- all commands pass through to the native OS
- AI catches failed commands and suggests fixes with real system investigation
- Plain English input auto-routed to AI when it's not a recognized command
- `@query` sends explicit questions to the AI with directory context
- `cmd | @action` pipes command output to AI for analysis
- Tilde expansion (`~/`) and `$VAR` expansion in all contexts
- Tab completion for commands and paths
- Persistent command history (JSON-backed)
- Pipes (`|`), redirects (`>`, `>>`, `<`), and quoting
- Raw terminal mode with cursor movement, Ctrl-C, Ctrl-D, Ctrl-L

### LLM control (live switching, no restart)

- `llm` -- show current backend and model status
- `llm use <backend> [model]` -- switch between ollama, openai, anthropic
- `llm model <name>` -- change model on active backend
- `llm reload` -- re-read config/llm.toml from disk
- `llm off` -- disable AI

### LLM backends

- **Ollama / network** -- any Ollama instance on localhost or LAN
- **OpenAI** -- GPT-4o and compatible models via API
- **Anthropic** -- Claude models via API
- **Local GGUF** -- placeholder for llama-cpp-rs integration
- Priority-based routing with automatic fallback

### AI features

- Directory-aware `@` queries -- cwd listing injected as context automatically
- Path detection in queries -- `@what is in src/` scopes context to that folder
- System investigation on missing commands (PATH search, similar commands, package manager detection)
- Platform-aware prompts (macOS vs Linux, brew vs apt, Homebrew prefix detection)
- Conversation history maintained across queries within a session
- Built-in knowledge base (TF-IDF) as offline fallback

### Project

- Rust workspace with 6 crates: aios-shell, aios-llm, aios-knowledge, aios-core, aios-kernel, aios-init
- `aish` binary for daily use; `aios-os` binary preserved for experimental OS mode
- Installable via `make install` or `cargo install --path aios-shell`
- Docker support
- MIT license
