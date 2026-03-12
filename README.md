# AIOS -- AI Shell

A drop-in shell replacement with AI built in. Every command runs natively on your OS -- nothing is reimplemented or intercepted. When something fails, the AI catches it and helps. When you type plain English, the AI handles it. When you need to switch models or backends, you do it live without restarting.

---

## Quick Start

### Install

```bash
git clone git@github.com:tgifriday/ai-os.git
cd ai-os
make install
```

This installs two things:

- `aish` binary to `/usr/local/bin/`
- Default config to `~/.config/aios/llm.yml` (won't overwrite an existing config)

Start the shell:

```bash
aish
```

It works immediately as a normal shell. All your system commands (`ls`, `ps`, `git`, `docker`, etc.) pass straight through to the OS.

To enable AI, edit `~/.config/aios/llm.yml` and enable a backend -- see [Bringing AI Online](#bringing-ai-online) below.

Other install methods

```bash
# Via cargo (config not auto-installed -- copy config/llm.yml to ~/.config/aios/ manually)
cargo install --path aios-shell

# Just build and run locally
cargo build --release
./target/release/aish
```



---

## What It Does

**No "command not found" dead ends.** If a command doesn't exist, the AI investigates your system, finds similar commands, and suggests the right install command for your package manager.

**Plain English works.** Type what you mean. If it's not a command, the AI handles it.

```
$ show me files larger than 100MB
$ what's using all my disk space
$ how do I undo the last git commit
```

**AI pipes.** Send command output to the AI for analysis:

```
$ ps aux | @which process is using the most memory
$ cat /var/log/system.log | @summarize the errors
$ df -h | @am I running low on disk space
```

**Error recovery.** When commands fail, the AI explains what went wrong and how to fix it.

**Switch AI models live.** No restart needed:

```
$ llm use ollama mistral
  AI: online via network (mistral)

$ llm use openai gpt-4o
  AI: online via openai (gpt-4o)

$ llm model llama3.1:70b
  AI: online via network (llama3.1:70b)
```

---

## Bringing AI Online

Out of the box, AIOS works as a normal shell. To activate AI, configure at least one backend in `config/llm.toml`.

### Option 1: Ollama (private, no cloud)

Run [Ollama](https://ollama.ai) on any machine on your network (or localhost):

```bash
ollama pull llama3.1:8b
ollama serve
```

```toml
# config/llm.toml
[network]
enabled = true
url = "http://localhost:11434"     # Or http://192.168.1.100:11434
model = "llama3.1:8b"
```

No API key needed. Works entirely on your LAN.

### Option 2: OpenAI

```toml
# config/llm.toml
[cloud.openai]
enabled = true
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"
```

```bash
export OPENAI_API_KEY="sk-..."
cargo run -p aios-shell
```

### Option 3: Anthropic

```toml
# config/llm.toml
[cloud.anthropic]
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
```

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
cargo run -p aios-shell
```

### Switching Backends Live

You don't have to restart the shell to change backends. Use the `llm` command:

```
$ llm                          # Show current backend and model
$ llm use ollama mistral       # Switch to Ollama with mistral
$ llm use openai gpt-4o        # Switch to OpenAI
$ llm use anthropic            # Switch to Anthropic (default model)
$ llm model llama3.1:70b       # Change model on current backend
$ llm reload                   # Re-read config/llm.toml
$ llm off                      # Disable AI
```

### Config File Locations

AIOS searches for config files in this order, stopping at the first one found:

1. `./config/llm.{toml,yaml,yml,json}`
2. `/etc/aios/llm.{toml,yaml,yml,json}`
3. `~/.config/aios/llm.{toml,yaml,yml,json}`

Use whichever format you prefer. TOML, YAML, and JSON are all supported -- the format is auto-detected from the file extension.

YAML example

```yaml
# config/llm.yaml
network:
  enabled: true
  url: "http://localhost:11434"
  model: "llama3.1:8b"
```



JSON example

```json
{
  "network": {
    "enabled": true,
    "url": "http://localhost:11434",
    "model": "llama3.1:8b"
  }
}
```



---

## Shell Reference

### Builtins

These are the only commands handled by the shell itself. Everything else runs on your OS:


| Command            | Purpose                        |
| ------------------ | ------------------------------ |
| `cd <dir>`         | Change directory               |
| `export KEY=VALUE` | Set environment variable       |
| `clear`            | Clear screen                   |
| `history`          | Show command history           |
| `help`             | Show help                      |
| `exit` / `quit`    | Exit shell                     |
| `sanitize`         | Clear AI conversation context  |
| `llm [subcommand]` | Control AI backend (see above) |


### AI Features


| Syntax            | What it does                       |
| ----------------- | ---------------------------------- |
| `@<query>`        | Force input through the AI         |
| `cmd | @<action>` | Pipe command output to AI          |
| Plain English     | Auto-routed to AI if not a command |
| Failed commands   | AI automatically suggests fixes    |

### AI Usage Examples

Use plain `@query` when you want advice or explanation without first running a command:

```bash
@what does this project do
@what files here look most important
@how do I install ollama on this machine
@what is the difference between aish and aios-os
```

Use `sanitize` when you want to clear AI conversation context before a fresh question:

```bash
sanitize
@tell me what this repo is for
```

Use `cmd | @question` when the command output is the thing you want analyzed:

```bash
ls -lah | @summarize
du -sh ./* | @what is using the most space
ps aux | @which processes look unusual
df -h | @which filesystem is closest to full
git status --short | @tell me what changed
git diff --stat | @summarize the scope of this work
cargo test 2>&1 | @summarize failures
docker ps -a | @which containers look unhealthy
```

Remote SSH commands work the same way:

```bash
ssh server01 "uptime && df -h" | @summarize host health
ssh server01 "ps aux --sort=-%cpu | head" | @which processes are hottest
ssh gpu01 "nvidia-smi" | @summarize gpu status
ssh app01 "tail -n 200 /var/log/app.log" | @summarize the errors
ssh web01 "netstat -an | grep LISTEN" | @what services are exposed
```

You can also narrow output before sending it to AI:

```bash
docker logs myapp 2>&1 | tail -n 100 | @what is failing here
rg "TODO|FIXME" . | @summarize outstanding work
ls -R src | @describe the code structure
echo "what's going on in the world" | @summarize
```

Prompt styles that tend to work well:

```bash
... | @summarize
... | @what stands out
... | @what is wrong here
... | @what should I check next
... | @which item is largest
... | @explain this output
```


### Pipes and Redirects

Standard shell plumbing works:

```
$ ls -la | grep .rs
$ cat file.txt | wc -l
$ echo "hello" > output.txt
$ echo "more" >> output.txt
$ sort < input.txt
```

---

## Architecture

### `aish` -- the AI shell (what you use)

Commands pass through to your OS. The AI handles errors, natural language, and queries.

```
┌──────────────────────────────────────────┐
│               User Input                 │
└────────────────────┬─────────────────────┘
                     │
┌────────────────────▼─────────────────────┐
│               aios-shell                 │
│                                          │
│  ┌─────────┐ ┌──────────┐ ┌───────────┐  │
│  │ Parser  │ │  Router  │ │ Completer │  │
│  │ globs,  │ │ cmd? AI? │ │ paths,    │  │
│  │ ;  &&   │ │ signals, │ │ tab, cmds │  │
│  │ || $()  │ │ errors   │ │           │  │
│  └─────────┘ └─────┬────┘ └───────────┘  │
└────────────────────┼─────────────────────┘
       ┌─────────────┼────────────┐
       ▼             ▼            ▼
┌────────────┐ ┌──────────┐ ┌───────────┐
│ Native OS  │ │ aios-llm │ │   aios-   │
│ ls, git,   │ │   LLM    │ │ knowledge │
│ vim, etc.  │ │  router  │ │ RAG store │
└────────────┘ └────┬─────┘ └───────────┘
                    │
                    ├── Ollama (network)
                    ├── OpenAI (cloud)
                    ├── Anthropic (cloud)
                    └── Local GGUF (planned)
```

### `aios-os` -- the experimental OS layer

Self-contained environment with built-in Rust reimplementations of core commands
(`ls`, `ps`, `df`, `grep`, `cat`, `wc`, etc.). Does not depend on the host OS
having these tools installed. Useful for minimal containers, embedded systems,
or bare-metal scenarios where you want a fully Rust-native command environment
with AI built in.

```
┌──────────────────────────────────────────┐
│               User Input                 │
└────────────────────┬─────────────────────┘
                     │
┌────────────────────▼─────────────────────┐
│               aios-shell                 │
│  ┌─────────┐ ┌───────────┐ ┌───────────┐ │
│  │ Parser  │ │   Router  │ │ Completer │ │
│  └─────────┘ └─────┬─────┘ └───────────┘ │
└────────────────────┼─────────────────────┘
       ┌─────────────┼────────────┐
       ▼             ▼            ▼
┌────────────┐ ┌──────────┐ ┌───────────┐
│ aios-core  │ │ aios-llm │ │   aios-   │
│ Rust cmds: │ │   LLM    │ │ knowledge │
│ ls, ps, df │ │  router  │ │ RAG store │
│ grep, etc. │ └────┬─────┘ └───────────┘
└─────┬──────┘      │
      │             ├── Ollama (network)
┌─────▼──────┐      ├── OpenAI (cloud)
│aios-kernel │      ├── Anthropic (cloud)
│ syscalls   │      └── Local GGUF (planned)
└─────┬──────┘
      │
┌─────▼──────┐
│ aios-init  │
│ services   │
└────────────┘
```

### Crates


| Crate              | Used by | Purpose                                                                        |
| ------------------ | ------- | ------------------------------------------------------------------------------ |
| **aios-shell**     | both    | REPL, parser, router, tab completion, history, `llm` command, signal handling  |
| **aios-llm**       | both    | Pluggable LLM runtime. Ollama, OpenAI, Anthropic, and local (planned) backends |
| **aios-knowledge** | both    | Embedded knowledge base. TF-IDF search, injected into LLM context              |
| **aios-core**      | OS only | Rust reimplementations of coreutils (ls, grep, ps, df, etc.)                   |
| **aios-kernel**    | OS only | Thin Rust wrappers over syscalls via `nix` crate                               |
| **aios-init**      | OS only | Init system and service manager                                                |


### Binaries


| Binary        | What it does                                                                    |
| ------------- | ------------------------------------------------------------------------------- |
| `**aish`**    | The AI shell. Install and use daily. Commands pass through to your OS.          |
| `**aios-os**` | Self-contained OS layer. Built-in Rust commands, no host dependency. See below. |


### Installing and using `aios-os`

```bash
# Install both binaries
make install-all

# Or just the OS binary
make install-os

# Or run directly from source
cargo run -p aios-shell --bin aios-os
```

`aios-os` works the same as `aish` (same prompt, same AI, same `llm` command) with one key difference: commands like `ls`, `ps`, `df`, `grep`, `cat`, `head`, `tail`, `wc`, `du`, `cp`, `rm`, and `mkdir` are handled by built-in Rust implementations instead of the host OS. This means:

- **No external dependencies** -- works even if the host has no coreutils installed
- **Cross-platform consistency** -- same output format on Linux, macOS, containers
- **Syscall-level access** -- the built-ins use `aios-kernel` (Rust wrappers over `nix`) for direct system interaction

Commands that are *not* reimplemented (e.g., `git`, `docker`, `python`) still pass through to the host OS, same as `aish`.

**When to use `aios-os` instead of `aish`:**


| Scenario                                | Use       |
| --------------------------------------- | --------- |
| Daily terminal use on macOS/Linux       | `aish`    |
| Minimal Docker containers               | `aios-os` |
| Embedded / bare-metal systems           | `aios-os` |
| Exploring the self-contained OS concept | `aios-os` |


---

## Extending AIOS

### Adding a New LLM Backend

Implement the `LlmBackend` trait in `aios-llm/src/`:

```rust
use async_trait::async_trait;
use crate::backend::*;

pub struct MyBackend { /* config fields */ }

#[async_trait]
impl LlmBackend for MyBackend {
    async fn complete(&self, request: CompletionRequest)
        -> anyhow::Result<CompletionResponse>
    {
        // Call your model/API
    }

    async fn stream_complete(&self, request: CompletionRequest)
        -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<String>> + Send>>>
    {
        // Return a stream of text chunks
    }

    fn name(&self) -> &str { "my-backend" }
    fn model_name(&self) -> &str { "my-model" }
    fn is_available(&self) -> bool { true }
}
```

Register it in `build_llm_router()` in `aios-shell/src/main.rs` and it works with the `llm` command automatically. Restart `aish` to pick it up, or use `llm reload` if you wire it into the config.

### Adding Knowledge

Edit `aios-knowledge/src/index.rs` and add documents in `populate_builtin_docs()`:

```rust
store.add_document(Document {
    id: "concept-networking".into(),
    title: "Network Configuration".into(),
    content: "Detailed explanation...".into(),
    category: "concept".into(),
    tags: vec!["network".into(), "ip".into()],
});
```

Documents are automatically injected into LLM context when relevant to a query.

---

## Configuration Reference

### `config/llm.toml` (or `.yaml` / `.json`)

```toml
[defaults]
primary = "network"               # First backend to try: "local", "network", "cloud"
fallback = ["cloud"]              # Backends to try if primary fails
max_context_tokens = 4096
stream_responses = true

[local]
enabled = false
model_path = "/var/aios/models/phi-3-mini-Q4.gguf"
threads = 4
gpu_layers = 0

[network]
enabled = true
url = "http://localhost:11434"
model = "llama3.1:8b"

[cloud.openai]
enabled = false
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"

[cloud.anthropic]
enabled = false
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
```

---

## Docker

```bash
docker build -t aish .

# Run the AI shell
docker run -it aish
docker run -it -e OPENAI_API_KEY="sk-..." aish
docker run -it --network host aish

# Run the OS layer instead (self-contained commands)
docker run -it --entrypoint aios-os aish
```

### Uninstall

```bash
make uninstall          # Removes both aish and aios-os
cargo uninstall aish    # If installed via cargo
```

---

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.