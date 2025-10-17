## Makefile to replicate CI steps locally

CARGO ?= cargo
ifdef TOOLCHAIN
CARGO_BIN := $(CARGO) +$(TOOLCHAIN)
RUSTC_BIN := rustc +$(TOOLCHAIN)
else
CARGO_BIN := $(CARGO)
RUSTC_BIN := rustc
endif

# Check if we're running the cli target
ifeq (cli,$(firstword $(MAKECMDGOALS)))
  # Use the rest as arguments for the cli target
  CLI_ARGS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  # Turn them into do-nothing targets
  $(eval $(CLI_ARGS):;@:)
endif

.PHONY: help test clippy fmt fmt-fix clippy-fix fix check build build-release clean clean-db generate-example e2e-import e2e cli watch tui ci doctor init

help:
	@echo "Available targets:"
	@echo "  make doctor          - Check system dependencies (rustc, cargo, python)"
	@echo "  make init            - Initialize retrochat (run init command)"
	@echo "  make test            - Run test suite (like CI)"
	@echo "  make clippy          - Run clippy with -D warnings (like CI)"
	@echo "  make fmt             - Check formatting with rustfmt --check (like CI)"
	@echo "  make fmt-fix         - Apply formatting changes with rustfmt"
	@echo "  make clippy-fix      - Apply clippy auto-fixes (with -D warnings)"
	@echo "  make fix             - Apply rustfmt and clippy fixes, then verify"
	@echo "  make check           - Cargo check"
	@echo "  make build           - Cargo build"
	@echo "  make build-release   - Cargo build --release"
	@echo "  make clean           - Remove build artifacts"
	@echo "  make clean-db        - Remove retrochat database (~/.retrochat/retrochat.db)"
	@echo "  make generate-example - Generate example files from provider directories"
	@echo "  make e2e-import      - Generate and import example files from all providers"
	@echo "  make e2e             - Run end-to-end tests"
	@echo "  make cli <args>      - Run retrochat CLI (e.g., make cli import claude)"
	@echo "                         Use 'make -- cli <args>' for flags (e.g., make -- cli watch --verbose all)"
	@echo "  make watch           - Watch all providers with verbose output (make watch)"
	@echo "  make tui             - Launch retrochat TUI interface"
	@echo "  make ci              - Run fmt, clippy, then tests"

test:
	$(CARGO_BIN) test --verbose

clippy:
	$(CARGO_BIN) clippy -- -D warnings

fmt:
	$(CARGO_BIN) fmt --all -- --check

fmt-fix:
	$(CARGO_BIN) fmt --all

# Apply clippy auto-fixes
clippy-fix:
	@echo "Applying clippy auto-fixes..."
	@$(CARGO_BIN) clippy --fix --allow-dirty --allow-staged -- -D warnings

# Apply automatic fixes: rustfmt, clippy --fix, then verify
fix: fmt-fix clippy-fix
	@echo "Verifying with clippy (-D warnings)..."
	@$(CARGO_BIN) clippy -- -D warnings

check:
	$(CARGO_BIN) check --verbose

build:
	$(CARGO_BIN) build

build-release:
	$(CARGO_BIN) build --release

clean:
	$(CARGO_BIN) clean

clean-db:
	@echo "Removing retrochat database..."
	@rm -f ~/.retrochat/retrochat.db
	@echo "Database removed: ~/.retrochat/retrochat.db"

generate-example:
	@python3 scripts/generate-example.py

e2e-import: generate-example
	@echo "Importing example files..."
	@echo "Using test database: ~/.retrochat/retrochat_e2e.db"
	@RETROCHAT_DB=~/.retrochat/retrochat_e2e.db $(CARGO_BIN) run -- init
	@RETROCHAT_DB=~/.retrochat/retrochat_e2e.db $(CARGO_BIN) run -- import --path examples/local_claude.jsonl --overwrite || true
	@RETROCHAT_DB=~/.retrochat/retrochat_e2e.db $(CARGO_BIN) run -- import --path examples/local_codex.jsonl --overwrite || true
	@RETROCHAT_DB=~/.retrochat/retrochat_e2e.db $(CARGO_BIN) run -- import --path examples/local_cursor.db --overwrite || true
	@RETROCHAT_DB=~/.retrochat/retrochat_e2e.db $(CARGO_BIN) run -- import --path examples/local_gemini.json --overwrite || true
	@echo "Example import complete"
	@echo "Cleaning up test database..."
	@rm -f ~/.retrochat/retrochat_e2e.db
	@echo "Test database (~/.retrochat/retrochat_e2e.db) removed"

e2e: e2e-import

cli:
	$(CARGO_BIN) run -- $(CLI_ARGS)

watch:
	$(CARGO_BIN) run -- watch all --verbose

tui:
	$(CARGO_BIN) run -- tui

ci: fmt clippy test
	@echo "CI checks passed locally"

doctor:
	@echo "Checking system dependencies..."
	@echo ""
	@echo "=== Mandatory Dependencies ==="
	@which $(RUSTC_BIN) > /dev/null 2>&1 && \
		echo "✓ rustc: $$($(RUSTC_BIN) --version)" || \
		(echo "✗ rustc: NOT FOUND (required)" && exit 1)
	@which $(CARGO_BIN) > /dev/null 2>&1 && \
		echo "✓ cargo: $$($(CARGO_BIN) --version)" || \
		(echo "✗ cargo: NOT FOUND (required)" && exit 1)
	@echo ""
	@echo "=== Optional Dependencies ==="
	@which python3 > /dev/null 2>&1 && \
		echo "✓ python: $$(python3 --version)" || \
		echo "✗ python: NOT FOUND (optional, needed for generate-example)"
	@echo ""
	@echo "All mandatory dependencies are installed!"

init:
	$(CARGO_BIN) run -- init
