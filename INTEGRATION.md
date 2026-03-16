# v0.1.5 Integration Guide

Extract the archive into your repo root. Every file in this package is
either new or a drop-in replacement for the existing file at the same path.

## Quick apply

```bash
cd ~/path/to/ai-os
tar xzf ai-os-v0.1.5.tar.gz
cp -r ai-os-v0.1.5/* .
cp -r ai-os-v0.1.5/.github .
cp ai-os-v0.1.5/.dockerignore .
rm -rf ai-os-v0.1.5/
```

## Then

```bash
cargo check          # verify it compiles
cargo build --release  # if you want to test locally

git add -A
git commit -m "v0.1.5: Mission Control, usage tracking, roadmap, demos, Docker multi-arch"
git tag v0.1.5
git push origin main --tags   # triggers release + Docker build
```

## File inventory (23 files)

### New files (11)
```
ROADMAP.md                              # 4-phase public roadmap
demos/01-error-recovery.sh              # Demo transcript
demos/02-plain-english-and-pipes.sh     # Demo transcript
demos/03-live-model-switching.sh        # Demo transcript
aios-llm/src/usage.rs                   # Mission Control usage tracker
.github/ISSUE_TEMPLATE/bug-report.yml   # Structured bug reports
.github/ISSUE_TEMPLATE/feature-request.yml  # Structured feature requests
.github/ISSUE_TEMPLATE/config.yml       # Issue template config
```

### Modified files (12) — drop-in replacements
```
Cargo.toml                    # version 0.1.4 → 0.1.5
CHANGELOG.md                  # added v0.1.5 entry
README.md                     # Why This Matters, usage, demos, Docker GHCR, mission_control
.dockerignore                 # added demos/
.github/workflows/release.yml # added Docker multi-arch job, packages:write perm
aios-llm/Cargo.toml           # added chrono, dirs deps
aios-llm/src/lib.rs           # exports usage module + UsageTracker
aios-llm/src/config.rs        # fixed Default impl for mission_control field
aios-shell/src/main.rs        # --help flag
aios-shell/src/main_os.rs     # --help flag
aios-shell/src/router.rs      # tracked_complete(), usage command, UsageTracker wiring
aios-shell/src/executor.rs    # usage in help text
aios-shell/src/completion.rs  # usage in tab completion
config/llm.toml               # [mission_control] section, fixed network url
config/llm.yml                # mission_control section
```

## What to verify after applying

1. `cargo check` passes on your machine
2. `aish --help` prints the new usage reference
3. `aish` → `usage` prints "No AI queries this session."
4. `aish` → `help` shows the usage command
5. Tab-complete `us<tab>` completes to `usage`
6. Config with `[mission_control] enabled = true` creates the JSONL log file
