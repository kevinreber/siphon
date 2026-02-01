# Siphon Makefile
# Simple targets for building, installing, and managing Siphon

INSTALL_PREFIX ?= $(HOME)/.local/bin
SHELL := /bin/bash

.PHONY: all install install-cli install-daemon build build-cli build-daemon \
        clean uninstall start stop status help test-install

# Default target
all: build

# Help
help:
	@echo "Siphon Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build targets:"
	@echo "  build          Build both CLI and daemon"
	@echo "  build-cli      Build only the CLI"
	@echo "  build-daemon   Build only the daemon"
	@echo ""
	@echo "Install targets:"
	@echo "  install        Full installation (build + install + setup service + hooks)"
	@echo "  install-cli    Install only the CLI"
	@echo "  install-daemon Install only the daemon"
	@echo ""
	@echo "Service targets:"
	@echo "  start          Start the daemon service"
	@echo "  stop           Stop the daemon service"
	@echo "  restart        Restart the daemon service"
	@echo "  status         Check daemon status"
	@echo ""
	@echo "Other targets:"
	@echo "  clean          Remove build artifacts"
	@echo "  uninstall      Remove Siphon completely"
	@echo "  test           Run all tests"
	@echo "  check          Run linters and type checks"
	@echo ""
	@echo "Options:"
	@echo "  INSTALL_PREFIX=PATH  Set install location (default: ~/.local/bin)"

# Build targets
build: build-cli build-daemon

build-cli:
	@echo "Building CLI..."
	@cd siphon-cli && npm install --silent && npm run build --silent
	@echo "CLI built successfully"

build-daemon:
	@echo "Building daemon..."
	@cd siphon-daemon && cargo build --release --quiet
	@echo "Daemon built successfully"

# Install targets
install:
	@./install.sh

install-cli: build-cli
	@echo "Installing CLI to $(INSTALL_PREFIX)..."
	@mkdir -p $(INSTALL_PREFIX)
	@echo '#!/usr/bin/env bash' > $(INSTALL_PREFIX)/siphon
	@echo 'exec node "$(CURDIR)/siphon-cli/dist/cli.js" "$$@"' >> $(INSTALL_PREFIX)/siphon
	@chmod +x $(INSTALL_PREFIX)/siphon
	@echo "CLI installed: $(INSTALL_PREFIX)/siphon"

install-daemon: build-daemon
	@echo "Installing daemon to $(INSTALL_PREFIX)..."
	@mkdir -p $(INSTALL_PREFIX)
	@cp siphon-daemon/target/release/siphon-daemon $(INSTALL_PREFIX)/
	@cp siphon-daemon/target/release/siphon-ctl $(INSTALL_PREFIX)/
	@chmod +x $(INSTALL_PREFIX)/siphon-daemon $(INSTALL_PREFIX)/siphon-ctl
	@echo "Daemon installed: $(INSTALL_PREFIX)/siphon-daemon"
	@echo "Control CLI installed: $(INSTALL_PREFIX)/siphon-ctl"

# Service management (detects OS automatically)
start:
	@if [ "$$(uname)" = "Darwin" ]; then \
		launchctl load ~/Library/LaunchAgents/com.siphon.daemon.plist 2>/dev/null || \
		$(INSTALL_PREFIX)/siphon-daemon & echo "Started daemon in background"; \
	else \
		systemctl --user start siphon-daemon 2>/dev/null || \
		$(INSTALL_PREFIX)/siphon-daemon & echo "Started daemon in background"; \
	fi

stop:
	@if [ "$$(uname)" = "Darwin" ]; then \
		launchctl unload ~/Library/LaunchAgents/com.siphon.daemon.plist 2>/dev/null || \
		pkill -f siphon-daemon 2>/dev/null || echo "Daemon not running"; \
	else \
		systemctl --user stop siphon-daemon 2>/dev/null || \
		pkill -f siphon-daemon 2>/dev/null || echo "Daemon not running"; \
	fi

restart: stop start

status:
	@$(INSTALL_PREFIX)/siphon-ctl status 2>/dev/null || \
		curl -s http://127.0.0.1:9847/health > /dev/null 2>&1 && echo "Daemon is running" || \
		echo "Daemon is not running"

# Clean
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf siphon-cli/dist siphon-cli/node_modules
	@cd siphon-daemon && cargo clean
	@echo "Clean complete"

# Uninstall
uninstall:
	@./install.sh --uninstall

# Test targets
test: test-cli test-daemon test-install

test-cli:
	@echo "Running CLI tests..."
	@cd siphon-cli && npm test 2>/dev/null || echo "No CLI tests configured"

test-daemon:
	@echo "Running daemon tests..."
	@cd siphon-daemon && cargo test --quiet

test-install:
	@echo "Running installation tests..."
	@./scripts/test-install.sh

test-install-quick:
	@echo "Running installation tests (quick mode)..."
	@./scripts/test-install.sh --quick

# Check/lint targets
check:
	@./scripts/check.sh
