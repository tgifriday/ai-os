#!/usr/bin/env bash
# Cloud Agent install script for AIOS.
#
# Provisions the default LLM service (a local Ollama server serving the model
# referenced by config/llm.toml) and builds the Rust workspace. The script is
# idempotent: on a snapshot that already has Ollama and the model baked in, the
# guarded steps are skipped and only the incremental cargo build runs.
set -euo pipefail

MODEL="${AIOS_DEFAULT_MODEL:-llama3.1:8b}"
OLLAMA_HOST="${OLLAMA_HOST:-127.0.0.1:11434}"
export OLLAMA_HOST

sudo_if_needed() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo "$@"
  fi
}

# 1. System dependency required by the Ollama installer.
if ! command -v zstd >/dev/null 2>&1; then
  sudo_if_needed apt-get update -qq
  sudo_if_needed apt-get install -y -qq zstd
fi

# 2. Install the Ollama runtime (skipped when the snapshot already has it).
if ! command -v ollama >/dev/null 2>&1; then
  curl -fsSL https://ollama.com/install.sh | sh
fi

# 3. Ensure the default model is present. Pulling requires a running server, so
#    start a temporary one if none is reachable and stop it when finished.
temp_serve_pid=""
if ! curl -fsS -m 2 "http://${OLLAMA_HOST}/api/version" >/dev/null 2>&1; then
  ollama serve >/tmp/ollama-install-serve.log 2>&1 &
  temp_serve_pid="$!"
  for _ in $(seq 1 30); do
    if curl -fsS -m 2 "http://${OLLAMA_HOST}/api/version" >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
fi

if ! ollama list | awk 'NR>1 {print $1}' | grep -qx "$MODEL"; then
  ollama pull "$MODEL"
fi

if [ -n "$temp_serve_pid" ]; then
  kill "$temp_serve_pid" 2>/dev/null || true
  wait "$temp_serve_pid" 2>/dev/null || true
fi

# 4. Build the workspace.
cargo build --workspace --release
