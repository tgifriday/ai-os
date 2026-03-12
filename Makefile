PREFIX   ?= /usr/local
CONFIGDIR = $(HOME)/.config/aios

.PHONY: build install uninstall clean

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

uninstall:
	rm -f $(PREFIX)/bin/aish
	@echo "Removed aish from $(PREFIX)/bin"
	@echo "Config left in place at $(CONFIGDIR)/ -- remove manually if desired."

clean:
	cargo clean
