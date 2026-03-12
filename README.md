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

<details>
<summary>Other install methods</summary>

```bash
# Via cargo (config not auto-installed -- copy config/llm.yml to ~/.config/aios/ manually)
cargo install --path aios-shell

# Just build and run locally
cargo build --release
./target/release/aish
```
</details>

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

<details>
<summary>YAML example</summary>

```yaml
# config/llm.yaml
network:
  enabled: true
  url: "http://localhost:11434"
  model: "llama3.1:8b"
```
</details>

<details>
<summary>JSON example</summary>

```json
{
  "network": {
    "enabled": true,
    "url": "http://localhost:11434",
    "model": "llama3.1:8b"
  }
}
```
</details>

---

## Shell Reference

### Builtins

These are the only commands handled by the shell itself. Everything else runs on your OS:

| Command | Purpose |
|---|---|
| `cd <dir>` | Change directory |
| `export KEY=VALUE` | Set environment variable |
| `clear` | Clear screen |
| `history` | Show command history |
| `help` | Show help |
| `exit` / `quit` | Exit shell |
| `llm [subcommand]` | Control AI backend (see above) |

### AI Features

| Syntax | What it does |
|---|---|
| `@<query>` | Force input through the AI |
| `cmd \| @<action>` | Pipe command output to AI |
| Plain English | Auto-routed to AI if not a command |
| Failed commands | AI automatically suggests fixes |

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

```
┌───────────────────────────────────────────────┐
│                  User Input                    │
└──────────────────────┬────────────────────────┘
                       │
┌──────────────────────▼────────────────────────┐
│                 aios-shell                      │
│  ┌──────────┐  ┌────────────┐  ┌────────────┐  │
│  │  Parser   │  │   Router   │  │    LLM     │  │
│  │ pipes,    │  │ command vs │  │  control   │  │
│  │ redirects │  │ AI routing │  │  (llm cmd) │  │
│  └──────────┘  └──────┬─────┘  └────────────┘  │
└────────────────────────┼──────────────────────┘
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
   ┌─────────────┐ ┌─────────┐  ┌─────────────┐
   │  Your OS    │ │ aios-llm│  │  aios-      │
   │  (native    │ │ runtime │  │  knowledge  │
   │  commands)  │ │         │  │  RAG store  │
   └─────────────┘ └────┬────┘  └─────────────┘
                        │
                        ├── Ollama (network)
                        ├── OpenAI (cloud)
                        ├── Anthropic (cloud)
                        └── Local GGUF (planned)
```

### Crates

| Crate | Purpose |
|---|---|
| **aios-shell** | The shell itself. REPL, parser, smart router, tab completion, history, `llm` command, AI error recovery. |
| **aios-llm** | Pluggable LLM runtime. `LlmBackend` trait with cloud (OpenAI, Anthropic), network (Ollama), and local (placeholder) backends. Priority-based router with fallback. |
| **aios-knowledge** | Embedded knowledge base. TF-IDF search over built-in documentation. Provides context to the LLM automatically. |
| **aios-kernel** | Thin Rust wrappers over syscalls. Used by the separate `aios-os` binary (see below). |
| **aios-core** | Rust command implementations. Available for `aios-os` mode; the shell itself passes through to the OS. |
| **aios-init** | Init system / service manager (for OS mode). |

### Binaries

- **`aish`** -- The AI shell. This is what you install and use daily.
- **`aios-shell`** -- Alias for `aish` (same binary, alternate name).
- **`aios-os`** -- Experimental AI OS layer with built-in command reimplementations. Preserved for future exploration. Run with `cargo run -p aios-shell --bin aios-os`.

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
docker run -it aish                                    # Basic shell
docker run -it -e OPENAI_API_KEY="sk-..." aish         # With cloud AI
docker run -it --network host aish                     # With Ollama on host
```

### Uninstall

```bash
make uninstall          # If installed via make
cargo uninstall aish    # If installed via cargo
```

---

## License

MIT
