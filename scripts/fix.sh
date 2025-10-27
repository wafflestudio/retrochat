#!/usr/bin/env bash
# Apply automatic fixes: rustfmt, clippy --fix, then verify

set -e

echo "Applying rustfmt..."
cargo fmt --all

echo "Applying clippy auto-fixes..."
cargo clippy --fix --allow-dirty --allow-staged -- -D warnings

echo "Verifying with clippy (-D warnings)..."
cargo clippy -- -D warnings

echo "✓ All fixes applied and verified"
