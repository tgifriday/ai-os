# AIOS - AI Operating System

AIOS is an AI-native operating system written entirely in Rust. It fuses a
Unix-like userland with deep large-language-model integration so that every
command, pipe, and shell session has first-class access to AI capabilities.

## Architecture

The system is organized into modular crates:

- **aios-kernel** — Virtual filesystem, process scheduling, memory management, and IPC.
- **aios-shell** — Interactive shell with natural-language input, `@` AI prefix, and AI pipes.
- **aios-core** — Shared types, configuration, and cross-crate utilities.
- **aios-llm** — Pluggable LLM backend supporting local, network, and cloud inference.
- **aios-knowledge** — Embedded knowledge base with keyword search for self-aware help.
- **aios-commands** — Built-in command implementations (ls, cp, grep, etc.).
- **aios-ai** — Agent orchestration, prompt construction, and tool-use framework.

## Design Goals

1. **AI-first** — Every subsystem can call the LLM to explain, suggest, or transform data.
2. **Self-aware** — The OS carries its own documentation and can answer questions about itself.
3. **Rust-native** — Memory safety, fearless concurrency, and zero-cost abstractions throughout.
4. **Modular** — Each crate is independently testable and swappable.
5. **Offline-capable** — With a local model, AIOS works without any network connection.
