#!/usr/bin/env bash
# Check system dependencies

echo "Checking system dependencies..."
echo ""

echo "=== Mandatory Dependencies ==="

# Check rustc
if command -v rustc > /dev/null 2>&1; then
    echo "✓ rustc: $(rustc --version)"
else
    echo "✗ rustc: NOT FOUND (required)"
    exit 1
fi

# Check cargo
if command -v cargo > /dev/null 2>&1; then
    echo "✓ cargo: $(cargo --version)"
else
    echo "✗ cargo: NOT FOUND (required)"
    exit 1
fi

echo ""
echo "=== Optional Dependencies ==="

# Check python3
if command -v python3 > /dev/null 2>&1; then
    echo "✓ python: $(python3 --version)"
else
    echo "✗ python: NOT FOUND (optional, needed for generate-example)"
fi

echo ""
echo "All mandatory dependencies are installed!"
