#!/usr/bin/env bash
# Link npm package globally for local testing

set -e

# Prepare npm package
bash scripts/prepare-npm.sh

# Link package globally
echo "Linking npm package globally..."
cd npm && npm link

echo ""
echo "✓ Package linked! Try running: retrochat --help"
