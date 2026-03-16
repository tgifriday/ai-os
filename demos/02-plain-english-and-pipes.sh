#!/bin/bash
# ============================================================================
# Demo 2: Plain English & AI Pipes
# ============================================================================
#
# AIOS routes plain English to the AI automatically. No special syntax needed.
# For command output analysis, pipe to @question.
#
# To run this demo yourself:
#   1. Install aish: make install
#   2. Configure an LLM backend in ~/.config/aios/llm.yml
#   3. Start: aish
#   4. Type the commands below and watch what happens
#
# ============================================================================

cat << 'TRANSCRIPT'

# ── Plain English ────────────────────────────────────────────────────────────
# Just type what you want. If it's not a command, the AI handles it.

  $ what files are in this project
  AI: This is a Rust workspace with 6 crates:
      - aios-shell/    The main shell (REPL, parser, router)
      - aios-llm/      LLM backend routing (Ollama, OpenAI, Anthropic)
      - aios-core/     Rust reimplementations of coreutils
      - aios-kernel/   Syscall wrappers via nix
      - aios-knowledge/ Built-in knowledge base (TF-IDF)
      - aios-init/     Init system and service manager

      Plus config/, scripts/, Dockerfile, and CI workflows.

  $ how do I undo the last git commit
  AI: To undo the last commit but keep your changes:
        git reset --soft HEAD~1

      To undo and discard changes:
        git reset --hard HEAD~1

      To undo a commit that's already pushed:
        git revert HEAD


# ── AI Pipes: Analyze command output ─────────────────────────────────────────
# Run a command, pipe the output to @, and ask a question about it.
# The command output prints first, then the AI analysis appears below.

  $ ps aux | @which process is using the most memory
  USER       PID %CPU %MEM    VSZ   RSS TTY   STAT START   TIME COMMAND
  root         1  0.0  0.1 169436 13200 ?     Ss   Mar12   0:04 /sbin/init
  postgres   842  0.1  2.3 402816 189440 ?    Ss   Mar12   1:22 postgres
  klott     1205  3.2  8.1 1842560 653312 ?   Sl   10:15   2:45 node server.js
  ...
  --- AI analysis ---
  AI: node server.js (PID 1205) is using the most memory at 8.1% (638 MB RSS).
      postgres is second at 2.3% (185 MB). Everything else is under 1%.


  $ df -h | @am I running low on disk space
  Filesystem      Size  Used Avail Use% Mounted on
  /dev/sda1       457G  312G  122G  72% /
  /dev/sdb1       1.8T  1.6T  112G  94% /data
  ...
  --- AI analysis ---
  AI: Your /data volume is at 94% — only 112 GB free on a 1.8 TB drive.
      That's the one to watch. Your root filesystem is fine at 72%.


  $ ssh gpu01 "nvidia-smi" | @summarize gpu status
  +-----------------------------------------------------------------------------+
  | NVIDIA-SMI 535.129.03   Driver: 535.129.03   CUDA: 12.2                     |
  | GPU  Name        Persistence-M| Bus-Id        Disp.A | Volatile Uncorr. ECC |
  | Fan  Temp  Perf  Pwr:Usage/Cap|         Memory-Usage | GPU-Util  Compute M. |
  |   0  Tesla P40          On    | 00000000:04:00.0 Off |                    0 |
  | N/A   62C    P0   145W / 250W |  22134MiB / 24576MiB |     87%      Default |
  +-----------------------------------------------------------------------------+
  --- AI analysis ---
  AI: Single Tesla P40 running hot: 87% GPU utilization, 62°C, 145W/250W power.
      VRAM is 90% full (21.6 GB / 24 GB). CUDA 12.2, driver 535.129.
      If you're doing inference, you're close to the VRAM ceiling.


# ── Explicit @queries ────────────────────────────────────────────────────────
# Use @ to force a query to the AI, even if it looks like a command name.

  $ @what is the difference between aish and aios-os
  AI: aish is the daily-driver shell. Commands pass through to your OS.
      aios-os is the experimental self-contained mode — it has built-in
      Rust reimplementations of ls, ps, grep, etc. Use aios-os when you
      need a shell in a minimal container or embedded system with no
      coreutils installed.

TRANSCRIPT

echo ""
echo "To try this yourself: install aish and configure any LLM backend."
echo "See: https://github.com/tgifriday/ai-os#bringing-ai-online"
