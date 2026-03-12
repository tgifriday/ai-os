# Contributing to AIOS

Contributions are welcome. Here's what you need to know.

## Getting Started

```bash
git clone git@github.com:tgifriday/ai-os.git
cd ai-os
cargo build
cargo run -p aios-shell
```

## Making Changes

1. Fork the repository and create a branch from `main`.
2. Make your changes. Run `cargo build` and `cargo test` (when tests exist).
3. Open a pull request against `main`.

## What's Useful

- Bug fixes and reports
- Shell compatibility improvements (things that work in bash/zsh but not in aish)
- New LLM backend integrations
- Documentation improvements
- Performance work

## License

This project is dual licensed under **MIT OR Apache-2.0**.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
