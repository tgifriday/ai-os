PREFIX   ?= /usr/local
CONFIGDIR = $(HOME)/.config/aios

.PHONY: build install install-os uninstall clean

build:
	cargo build --release -p aios-shell

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
