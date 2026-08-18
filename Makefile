PREFIX   ?= /usr/local
CONFIGDIR = $(HOME)/.config/aios

# Directory holding libstdc++.so, needed when linking the embedded llama.cpp.
LIBSTDCXX_DIR = $(shell dirname "$$(gcc -print-file-name=libstdc++.so)" 2>/dev/null)

# The local-inference deps need Rust >= 1.85. Prefer a rustup `stable` toolchain
# when present (older default toolchains can't resolve edition-2024 crates);
# otherwise fall back to plain cargo (e.g. the repo's rust:1.94 Docker image).
CARGO_LOCAL = $(shell rustup toolchain list 2>/dev/null | grep -q '^stable' && echo 'cargo +stable' || echo 'cargo')

.PHONY: build install install-os install-all uninstall clean \
        local-deps fetch-local-model build-local

build:
	cargo build --release -p aios-shell

# --- Optional in-process local LLM (embedded llama.cpp) -----------------------
# One-time system dependencies (cmake, C/C++ compiler, clang/libclang, rustfmt).
local-deps:
	bash scripts/install-local-deps.sh

# Download the default GGUF model to ~/.aios/models.
fetch-local-model:
	bash scripts/fetch-local-model.sh

# Build aish/aios-os with in-process GGUF inference enabled.
build-local:
	CC=gcc CXX=g++ RUSTFLAGS="-L $(LIBSTDCXX_DIR)" \
		$(CARGO_LOCAL) build --release -p aios-shell --features local-inference

install: build
	install -d $(PREFIX)/bin
	install -m 755 target/release/aish $(PREFIX)/bin/aish
	@# Install default config only if none exists yet
	@install -d $(CONFIGDIR)
	@if [ ! -f $(CONFIGDIR)/llm.yml ] && \
	    [ ! -f $(CONFIGDIR)/llm.yaml ] && \
	    [ ! -f $(CONFIGDIR)/llm.toml ] && \
	    [ ! -f $(CONFIGDIR)/llm.json ]; then \
		install -m 644 config/llm.yml $(CONFIGDIR)/llm.yml; \
		echo ""; \
		echo "Installed default config to $(CONFIGDIR)/llm.yml"; \
		echo "Edit it to enable an LLM backend."; \
	else \
		echo ""; \
		echo "Config already exists in $(CONFIGDIR) -- not overwritten."; \
	fi
	@echo ""
	@echo "Installed aish to $(PREFIX)/bin/aish"
	@echo "Run 'aish' to start the AI shell."

install-os: build
	install -d $(PREFIX)/bin
	install -m 755 target/release/aios-os $(PREFIX)/bin/aios-os
	@# Install default config only if none exists yet
	@install -d $(CONFIGDIR)
	@if [ ! -f $(CONFIGDIR)/llm.yml ] && \
	    [ ! -f $(CONFIGDIR)/llm.yaml ] && \
	    [ ! -f $(CONFIGDIR)/llm.toml ] && \
	    [ ! -f $(CONFIGDIR)/llm.json ]; then \
		install -m 644 config/llm.yml $(CONFIGDIR)/llm.yml; \
		echo ""; \
		echo "Installed default config to $(CONFIGDIR)/llm.yml"; \
	else \
		echo ""; \
		echo "Config already exists in $(CONFIGDIR) -- not overwritten."; \
	fi
	@echo ""
	@echo "Installed aios-os to $(PREFIX)/bin/aios-os"
	@echo "Run 'aios-os' to start the AI OS shell."

install-all: build
	@$(MAKE) install
	@$(MAKE) install-os

uninstall:
	rm -f $(PREFIX)/bin/aish $(PREFIX)/bin/aios-os
	@echo "Removed aish and aios-os from $(PREFIX)/bin"
	@echo "Config left in place at $(CONFIGDIR)/ -- remove manually if desired."

clean:
	cargo clean
