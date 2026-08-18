#!/usr/bin/env bash
# Install the toolchain and system packages needed to build the optional
# `local-inference` feature (embedded llama.cpp for in-process GGUF inference).
#
# The default AIOS build needs none of this. Only run it if you want to build
# with `make build-local`.
set -euo pipefail

# llama-cpp-2 (via bindgen + a C/C++ build of llama.cpp) needs cmake, a C/C++
# compiler, and clang/libclang.
if command -v apt-get >/dev/null 2>&1; then
  SUDO=""
  [ "$(id -u)" -ne 0 ] && SUDO="sudo"
  $SUDO apt-get update -qq
  $SUDO apt-get install -y -qq cmake g++ clang libclang-dev pkg-config
elif command -v brew >/dev/null 2>&1; then
  brew install cmake llvm
else
  echo "WARNING: unknown package manager; ensure cmake, a C/C++ compiler, and clang/libclang are installed." >&2
fi

# Some transitive crates require Rust edition 2024 (Rust >= 1.85). Ensure a
# recent stable toolchain is available when rustup is present.
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install stable --profile minimal >/dev/null 2>&1 || true
  rustup component add rustfmt --toolchain stable >/dev/null 2>&1 || true
fi

echo "Local build dependencies installed. Now run: make build-local"
