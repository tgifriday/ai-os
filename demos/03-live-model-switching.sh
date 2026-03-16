#!/bin/bash
# ============================================================================
# Demo 3: Live Model Switching
# ============================================================================
#
# Switch between AI backends mid-session. No restart, no config editing.
# Your conversation context resets on switch, but your shell state
# (cwd, env vars, history) stays intact.
#
# To run this demo yourself:
#   1. Install aish: make install
#   2. Configure backends in ~/.config/aios/llm.yml
#   3. Start: aish
#   4. Type the commands below
#
# ============================================================================

cat << 'TRANSCRIPT'

# ── Check current status ─────────────────────────────────────────────────────

  $ llm
  LLM backends:
    network (llama3.1:8b) [online]


# ── Switch to a different Ollama model ───────────────────────────────────────

  $ llm model mistral
  AI: online via network (mistral)

  $ @explain kubernetes pods in one sentence
  AI: A pod is the smallest deployable unit in Kubernetes — one or more
      containers that share storage, network, and a lifecycle.


# ── Switch to OpenAI ─────────────────────────────────────────────────────────
# Requires OPENAI_API_KEY in your environment.

  $ llm use openai gpt-4o
  AI: online via openai (gpt-4o)

  $ cat main.rs | @review this code
  --- AI analysis ---
  AI: The code looks solid. A few suggestions:
      1. The signal handling block uses unsafe — consider wrapping in a
         safe abstraction or documenting the safety invariant.
      2. read_line_raw could benefit from a timeout to prevent hanging
         on broken terminals.
      3. The history_index logic is correct but dense — a small state
         machine would make it clearer.


# ── Switch to Anthropic ──────────────────────────────────────────────────────
# Requires ANTHROPIC_API_KEY in your environment.

  $ llm use anthropic claude-sonnet-4-20250514
  AI: online via anthropic (claude-sonnet-4-20250514)

  $ @what would you name a developer tool that's an AI-native terminal
  AI: A few directions...
      - "ashell" (AI + shell)
      - "terminus" (endpoint, finality — the last shell you need)
      - "aios" works well — it signals that the AI is at the OS level,
        not bolted on top.


# ── Go back to local ─────────────────────────────────────────────────────────

  $ llm use ollama llama3.1:8b
  AI: online via network (llama3.1:8b)


# ── Disable AI entirely ─────────────────────────────────────────────────────
# Shell keeps working. You just lose the AI features.

  $ llm off
  AI disabled.

  $ this is a plain english query
  aios: command not found: this

  $ llm use ollama
  AI: online via network (llama3.1:8b)

  $ this is a plain english query
  AI: I see you're testing plain English input! When AI is active,
      anything that isn't a recognized command gets routed to me.


# ── Reload config from disk ─────────────────────────────────────────────────
# Edit config/llm.toml (or .yaml/.json) and reload without restarting.

  $ llm reload
  Reloaded from /home/user/.config/aios/llm.yml
  AI: online via network (qwen2.5:14b)

TRANSCRIPT

echo ""
echo "To try this yourself: install aish and configure any LLM backend."
echo "See: https://github.com/tgifriday/ai-os#bringing-ai-online"
