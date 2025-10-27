#!/usr/bin/env bash
# Apply clippy auto-fixes

set -e

echo "Applying clippy auto-fixes..."
cargo clippy --fix --allow-dirty --allow-staged -- -D warnings

echo "✓ Clippy fixes applied"
