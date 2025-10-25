#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building retrochat binary...${NC}"

# Detect platform and architecture
OS=$(uname -s)
ARCH=$(uname -m)

# Map to our platform naming
case "$OS" in
  Darwin)
    PLATFORM="darwin"
    ;;
  Linux)
    PLATFORM="linux"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    PLATFORM="win32"
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64)
    ARCH_NAME="x64"
    ;;
  arm64|aarch64)
    ARCH_NAME="arm64"
    ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

PLATFORM_KEY="${PLATFORM}-${ARCH_NAME}"
echo -e "${BLUE}Detected platform: ${PLATFORM_KEY}${NC}"

# Build the binary
cargo build --release

# Determine binary name and path
if [ "$PLATFORM" = "win32" ]; then
  BINARY_NAME="retrochat.exe"
else
  BINARY_NAME="retrochat"
fi

SOURCE_BINARY="target/release/${BINARY_NAME}"
DEST_DIR="npm/vendor/${PLATFORM_KEY}/retrochat"
DEST_BINARY="${DEST_DIR}/${BINARY_NAME}"

# Create destination directory
echo -e "${BLUE}Creating vendor directory: ${DEST_DIR}${NC}"
mkdir -p "${DEST_DIR}"

# Copy binary
echo -e "${BLUE}Copying binary to ${DEST_BINARY}${NC}"
cp "${SOURCE_BINARY}" "${DEST_BINARY}"

# Make executable (Unix-like systems)
if [ "$PLATFORM" != "win32" ]; then
  chmod +x "${DEST_BINARY}"
fi

echo -e "${GREEN}✓ Binary prepared successfully!${NC}"
echo ""
echo "Next steps:"
echo "  1. Test locally with npm link:"
echo "     cd npm && npm link"
echo "     retrochat --help"
echo ""
echo "  2. Or create a package to test:"
echo "     cd npm && npm pack"
echo "     npm install -g sanggggg-retrochat-*.tgz"
echo ""
