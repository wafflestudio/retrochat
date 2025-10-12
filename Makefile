## Makefile to replicate CI steps locally

CARGO ?= cargo
ifdef TOOLCHAIN
CARGO_BIN := $(CARGO) +$(TOOLCHAIN)
RUSTC_BIN := rustc +$(TOOLCHAIN)
else
CARGO_BIN := $(CARGO)
RUSTC_BIN := rustc
endif

.PHONY: help test clippy fmt fmt-fix clippy-fix fix check build build-release clean generate-example test-import ci

help:
	@echo "Available targets:"
	@echo "  make test            - Run test suite (like CI)"
	@echo "  make clippy          - Run clippy with -D warnings (like CI)"
	@echo "  make fmt             - Check formatting with rustfmt --check (like CI)"
	@echo "  make fmt-fix         - Apply formatting changes with rustfmt"
	@echo "  make clippy-fix      - Apply clippy auto-fixes"
	@echo "  make fix             - Apply rustfmt and clippy fixes, then verify"
	@echo "  make check           - Cargo check"
	@echo "  make build           - Cargo build"
	@echo "  make build-release   - Cargo build --release"
	@echo "  make clean           - Remove build artifacts"
	@echo "  make generate-example - Generate example files from provider directories"
	@echo "  make test-import    - Generate and import example files from all providers"
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
	@$(CARGO_BIN) clippy --fix --allow-dirty --allow-staged

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

generate-example:
	@python3 scripts/generate-example.py

test-import: generate-example
	@echo "Importing example files..."
	@$(CARGO_BIN) run -- import --path examples/local_claude.jsonl --overwrite || true
	@$(CARGO_BIN) run -- import --path examples/local_codex.jsonl --overwrite || true
	@$(CARGO_BIN) run -- import --path examples/local_cursor.db --overwrite || true
	@$(CARGO_BIN) run -- import --path examples/local_gemini.json --overwrite || true
	@echo "Example import complete"

ci: fmt clippy test
	@echo "CI checks passed locally"
