#!/usr/bin/env bash
# Run CI checks: format, clippy, and tests

set -e

echo "Running format check..."
cargo fmt --all -- --check

echo "Running clippy..."
cargo clippy -- -D warnings

echo "Running tests..."
cargo test --verbose

echo "✓ CI checks passed locally"
