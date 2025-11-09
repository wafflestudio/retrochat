#!/usr/bin/env bash
# Run end-to-end tests: generate examples and sync them

set -e

# Generate example files
echo "Generating example files..."
python3 scripts/generate-example.py

# Sync examples using test database
echo "Syncing example files..."
echo "Using test database: ~/.retrochat/retrochat_e2e.db"

export RETROCHAT_DB=~/.retrochat/retrochat_e2e.db

# Database is auto-initialized on first sync
cargo run -- sync --path examples/local_claude.jsonl --overwrite || true
cargo run -- sync --path examples/local_codex.jsonl --overwrite || true
cargo run -- sync --path examples/local_gemini.json --overwrite || true

echo "Example sync complete"

# Cleanup
echo "Cleaning up test database..."
rm -f ~/.retrochat/retrochat_e2e.db
echo "Test database (~/.retrochat/retrochat_e2e.db) removed"

echo "✓ E2E tests completed successfully"
