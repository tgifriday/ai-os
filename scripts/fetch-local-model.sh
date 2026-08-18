#!/usr/bin/env bash
# Download a GGUF model for the in-process `local` backend.
#
# Defaults to Qwen2.5-Coder-3B-Instruct (q4_k_m): a small, fast, code/CLI-focused
# model that runs on CPU without a GPU. Override AIOS_LOCAL_MODEL_URL to use a
# different model (e.g. the 1.5B "turbo" variant for lower latency).
set -euo pipefail

MODEL_URL="${AIOS_LOCAL_MODEL_URL:-https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf}"
DEST_DIR="${AIOS_MODELS_DIR:-$HOME/.aios/models}"
DEST="$DEST_DIR/$(basename "$MODEL_URL")"

mkdir -p "$DEST_DIR"

if [ -f "$DEST" ]; then
  echo "Model already present: $DEST"
else
  echo "Downloading $(basename "$MODEL_URL") -> $DEST"
  curl -fL --retry 3 --retry-delay 2 -o "$DEST.part" "$MODEL_URL"
  mv "$DEST.part" "$DEST"
  echo "Downloaded: $DEST"
fi

echo
echo "To use it, set in config/llm.toml (and disable other backends if desired):"
echo "  [local]"
echo "  enabled = true"
echo "  model_path = \"$DEST\""
echo
echo "Then build with local inference support: make build-local"
