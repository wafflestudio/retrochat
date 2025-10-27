#!/usr/bin/env bash
# Create npm package tarball for testing

set -e

# Prepare npm package
bash scripts/prepare-npm.sh

# Create tarball
echo "Creating npm package tarball..."
cd npm && npm pack

echo ""
echo "✓ Package created! Install with:"
echo "  npm install -g @sanggggg/retrochat"
echo ""
echo "Or test the local tarball:"
echo "  npm install -g npm/sanggggg-retrochat-*.tgz"
