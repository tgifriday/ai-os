# AIOS Roadmap

> A terminal that understands intent and fails gracefully.
> Not a coding agent. Not a wrapper. A new kind of shell.

This document tracks where AIOS is headed. Checkmarks mean shipped.
Unchecked items are planned. Nothing here is set in stone — if you have
ideas, [open an issue](https://github.com/tgifriday/ai-os/issues).

---

## Phase 1: Foundation (v0.1 → v0.2)

**Goal:** Ship something genuinely useful for daily terminal work.

### Core Shell

- [x] Native command passthrough — `ls`, `git`, `docker` all run on your OS
- [x] Glob expansion (`*.rs`), chaining (`;`, `&&`, `||`), command substitution (`$(...)`)
- [x] Pipes and redirects (`|`, `>`, `>>`, `<`)
- [x] Tab completion (commands from PATH + filesystem paths, columnar display)
- [x] Command history with arrow keys
- [x] Signal handling — Ctrl-C kills the child, not the shell
- [x] Interactive program support (vim, ssh, top, less, etc.)
- [x] Environment variable expansion (`$HOME`, `export KEY=VALUE`)

### AI Integration

- [x] `@query` — ask the AI anything, directory context included
- [x] `cmd | @question` — pipe command output to AI for analysis
- [x] Plain English routing — type what you mean, AI handles it
- [x] Error recovery — failed commands trigger AI investigation
- [x] Missing command investigation (PATH search, similar commands, package manager hints)
- [x] Live model switching (`llm use ollama`, `llm use openai`, etc.)
- [x] Conversation context with `sanitize` to reset
- [x] Built-in knowledge base (TF-IDF) as offline fallback

### Backends

- [x] Ollama (any model, any host on your LAN)
- [x] OpenAI (GPT-4o and compatible)
- [x] Anthropic (Claude models)
- [ ] Local GGUF inference (llama.cpp / candle, no network required)
- [ ] OpenRouter / LiteLLM (any model behind a unified API)

### Config & Install

- [x] TOML / YAML / JSON config — auto-detected by extension
- [x] Config search: `./config/`, `/etc/aios/`, `~/.config/aios/`
- [x] `make install` → `/usr/local/bin/aish` + default config
- [x] `cargo install --path aios-shell`
- [x] Docker image

### Self-Contained OS Layer (`aios-os`)

- [x] Rust reimplementations of 25+ coreutils (ls, ps, df, grep, etc.)
- [x] Works without host coreutils (minimal containers, embedded)
- [x] Kernel wrappers via `nix` crate (process, fs, memory, network, device)
- [x] Init system and service manager (`aios-init`)

### CI/CD

- [x] CI on push to main (Linux + Windows)
- [x] Release workflow on `v*` tags
- [x] Multi-platform binaries: macOS (Intel + Apple Silicon), Linux (x86_64 + ARM64 + musl), Windows
- [ ] Docker multi-arch images in CI (`linux/amd64`, `linux/arm64`)
- [ ] Automated integration tests

### Documentation

- [x] README with install, usage, architecture, config reference
- [x] CHANGELOG with every release
- [x] CONTRIBUTING.md
- [ ] `--help` and `man` page
- [ ] Embedded deployment guide (air-gapped, minimal container)
- [ ] Video/GIF demos of core features

---

## Phase 2: Enterprise Readiness (v0.2 → v0.3)

**Goal:** Make AIOS adoptable by teams without vendor lock-in.

### Observability

- [ ] Audit log hooks (JSON structured logs, Slack webhook, Grafana push)
- [ ] Token usage tracking per session / per user
- [ ] Cost estimation per query (model-aware pricing)

### Deployment

- [ ] Self-hosted deployment docs (Docker Compose, Kubernetes)
- [ ] Helm chart
- [ ] `--offline` mode flag (local models only, zero network calls)
- [ ] `--no-cloud` config flag

### Access Control

- [ ] Optional SSO/auth hooks (OIDC, LDAP)
- [ ] Usage quota settings (tokens/day, queries/hour)
- [ ] Role-based model access (e.g., only admins get GPT-4)

### Privacy & Security

- [ ] `audit-log: disabled` config option
- [ ] Transparent prompt visibility (show exactly what's sent to the LLM)
- [ ] Clear data retention policy (none by default)
- [ ] No telemetry, ever

---

## Phase 3: Ecosystem (v0.3 → v0.5)

**Goal:** Build tools around AIOS.

- [ ] MCP server — expose AIOS as a tool for other AI agents
- [ ] Plugin system for custom backends and middleware
- [ ] Multi-session mode (parallel agents, different models)
- [ ] Container runtime images (`aios-alpine`, `aios-ubi`, `aios-scratch`)
- [ ] GitHub Actions integration (run AIOS in CI for AI-assisted debugging)
- [ ] Shell scripting with AI (`aish script.sh` with `@` directives)

---

## Phase 4: Platform Vision (v0.5+)

**Goal:** AIOS as an operating environment, not just a shell.

- [ ] Ships as a bootable container image
- [ ] Rust-native package manager for the OS layer
- [ ] Pre-loaded Mission Control integration (cost/analytics dashboard)
- [ ] Runs on: Raspberry Pi, embedded Linux, bare-metal, Kubernetes sidecars
- [ ] Init system manages AI backends as first-class services

---

## Design Principles

These don't change between phases:

1. **Native first.** Real commands run on your real OS. Nothing is intercepted or reimplemented unless you opt into the OS layer.
2. **AI assists, never blocks.** If the LLM is down, you still have a shell. AI catches errors — it doesn't gatekeep.
3. **No vendor lock-in.** Ollama, OpenAI, Anthropic, local GGUF, or your own endpoint. Switch live.
4. **Privacy by default.** No telemetry. No data sent anywhere unless you configure a cloud backend. Full local operation is a first-class path.
5. **Transparent.** You can always see what the AI is doing and why. No hidden prompts, no magic.

---

## How to Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md). The best way to help right now:

- **Try it.** Use `aish` as your daily shell for a week and file issues.
- **Break it.** Find edge cases in parsing, piping, or AI routing.
- **Add backends.** Implement the `LlmBackend` trait for your favorite model provider.
- **Write docs.** Especially deployment guides and real-world use cases.

---

*Last updated: March 2026*
