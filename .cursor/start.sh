#!/usr/bin/env bash
# Cloud Agent per-boot start script for AIOS.
#
# Ensures the default LLM service (local Ollama server) is running so the
# 'network' backend used by aish/aios-os resolves. Idempotent: if a server is
# already reachable it returns immediately; otherwise it launches one in the
# background, waits for readiness, and returns. Kept out of the foreground so
# the boot sequence is not blocked.
set -euo pipefail

OLLAMA_HOST="${OLLAMA_HOST:-127.0.0.1:11434}"
export OLLAMA_HOST
LOG_DIR="${HOME}/.aios"
LOG_FILE="${LOG_DIR}/ollama-serve.log"

is_up() {
  curl -fsS -m 2 "http://${OLLAMA_HOST}/api/version" >/dev/null 2>&1
}

if is_up; then
  echo "ollama already running on ${OLLAMA_HOST}"
  exit 0
fi

if ! command -v ollama >/dev/null 2>&1; then
  echo "ERROR: ollama is not installed; run the install script first" >&2
  exit 1
fi

mkdir -p "$LOG_DIR"
nohup ollama serve >"$LOG_FILE" 2>&1 &

for _ in $(seq 1 30); do
  if is_up; then
    echo "ollama serve is ready on ${OLLAMA_HOST} (logs: ${LOG_FILE})"
    exit 0
  fi
  sleep 1
done

echo "ERROR: ollama serve did not become ready on ${OLLAMA_HOST}; see ${LOG_FILE}" >&2
exit 1
