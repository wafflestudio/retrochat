## Makefile to replicate CI steps locally

CARGO ?= cargo
ifdef TOOLCHAIN
CARGO_BIN := $(CARGO) +$(TOOLCHAIN)
RUSTC_BIN := rustc +$(TOOLCHAIN)
else
CARGO_BIN := $(CARGO)
RUSTC_BIN := rustc
endif

.PHONY: help test clippy fmt fmt-fix fix check build build-release ci

help:
	@echo "Available targets:"
	@echo "  make test     - Run test suite (like CI)"
	@echo "  make clippy   - Run clippy with -D warnings (like CI)"
	@echo "  make fmt      - Check formatting with rustfmt --check (like CI)"
	@echo "  make fmt-fix  - Apply formatting changes with rustfmt"
	@echo "  make fix      - Apply rustfmt and attempt clippy fixes"
	@echo "  make check    - Cargo check"
	@echo "  make build    - Cargo build"
	@echo "  make build-release - Cargo build --release"
	@echo "  make ci       - Run fmt, clippy, then tests"

test:
	$(CARGO_BIN) test --verbose

clippy:
	$(CARGO_BIN) clippy -- -D warnings

fmt:
	$(CARGO_BIN) fmt --all -- --check

fmt-fix:
	$(CARGO_BIN) fmt --all

# Apply automatic fixes where possible: rustfmt and clippy --fix
fix: fmt-fix
	@# Run clippy --fix only on nightly to avoid -Z error on stable
	@if $(RUSTC_BIN) --version 2>/dev/null | grep -q nightly; then \
		echo "Applying clippy auto-fixes (nightly)..."; \
		$(CARGO_BIN) clippy --fix -Z unstable-options --allow-dirty --allow-staged; \
	else \
		echo "Skipping clippy --fix on stable; use 'make TOOLCHAIN=nightly fix' for autofix"; \
	fi
	@echo "Verifying with clippy (-D warnings)"
	$(CARGO_BIN) clippy -- -D warnings

check:
	$(CARGO_BIN) check --verbose

build:
	$(CARGO_BIN) build

build-release:
	$(CARGO_BIN) build --release

ci: fmt clippy test
	@echo "CI checks passed locally"
