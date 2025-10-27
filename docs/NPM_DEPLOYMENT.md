# npm Deployment Guide

This document explains how to deploy `retrochat` to npm as `@sanggggg/retrochat`.

## Architecture Overview

We use the **vendor approach** (similar to OpenAI's codex CLI) where:
- Single npm package contains all platform binaries in `vendor/` directory
- Node.js wrapper script (`bin/retrochat.js`) detects platform and runs the appropriate binary
- Simpler than optionalDependencies approach (no multiple packages to manage)

## Project Structure

```
npm/
├── package.json              # Package metadata
├── bin/
│   └── retrochat.js         # Platform-aware wrapper script
├── vendor/
│   ├── darwin-x64/retrochat/retrochat
│   ├── darwin-arm64/retrochat/retrochat
│   ├── linux-x64/retrochat/retrochat
│   ├── linux-arm64/retrochat/retrochat
│   └── win32-x64/retrochat/retrochat.exe
└── README.md
```

## Local Testing

### 1. Build and Link Locally

```bash
# Build binary and prepare npm package
./scripts/prepare-npm.sh

# Link globally for testing
./scripts/npm-link.sh

# Test the command
retrochat --help
retrochat tui
```

### 2. Test with Package Tarball

```bash
# Create a tarball
./scripts/npm-pack.sh

# Install the tarball globally
npm install -g npm/sanggggg-retrochat-*.tgz

# Test
retrochat --help
```

### 3. Unlink When Done

```bash
# Unlink the global package
cd npm && npm unlink -g
```

## Publishing to npm

### Prerequisites

1. **npm Account**: Create an account at https://npmjs.com
2. **npm Token**: Generate an access token with "Automation" type
3. **GitHub Secret**: Add the token as `NPM_TOKEN` in repository secrets

### Setup npm Token

1. Go to https://www.npmjs.com/settings/YOUR_USERNAME/tokens
2. Click "Generate New Token" → "Classic Token"
3. Select "Automation" type
4. Copy the token

### Add to GitHub Secrets

1. Go to GitHub repository → Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `NPM_TOKEN`
4. Value: Paste your npm token
5. Click "Add secret"

### Release Process

The release is fully automated via GitHub Actions:

```bash
# 1. Update version in Cargo.toml if needed
# (The npm package version will be set from the git tag)

# 2. Commit any changes
git add .
git commit -m "chore: prepare for release v0.1.0"
git push

# 3. Create and push a version tag
git tag v0.1.0
git push origin v0.1.0
```

This will trigger the release workflow which:
1. Builds binaries for all 5 platforms (macOS x64/arm64, Linux x64/arm64, Windows x64)
2. Copies binaries to `npm/vendor/` directory
3. Updates package version from git tag
4. Publishes to npm as `@sanggggg/retrochat`
5. Creates a GitHub Release

### Manual Publishing (Not Recommended)

If you need to publish manually:

```bash
# 1. Build for your current platform
./scripts/prepare-npm.sh

# 2. Login to npm
npm login

# 3. Publish (from npm directory)
cd npm
npm publish --access public
```

⚠️ **Note**: Manual publishing only includes your current platform's binary. Use GitHub Actions for proper multi-platform releases.

## Verification

After publishing, verify the package:

```bash
# Install from npm
npm install -g @sanggggg/retrochat

# Test
retrochat --help
retrochat tui

# Check package info
npm info @sanggggg/retrochat
```

## Troubleshooting

### Binary Not Found After Install

If users get "Binary not found" error:
1. Check that all platform binaries are in the package: `npm pack` and inspect the tarball
2. Verify file permissions: binaries should be executable (chmod +x)
3. Check the platform detection in `bin/retrochat.js`

### Package Size Issues

The package includes binaries for 5 platforms (~10-15MB total):
- This is normal for CLI tools with native binaries
- npm compresses during transmission (actual download is smaller)
- Similar tools (esbuild, swc) use the same approach

### Platform Not Supported

Supported platforms:
- macOS: x64, ARM64
- Linux: x64, ARM64
- Windows: x64

For other platforms, users must build from source:
```bash
git clone https://github.com/wafflestudio/retrochat
cd retrochat
cargo build --release
```

## Version Management

- **Git tags** control the version (e.g., `v0.1.0`)
- The release workflow extracts version from tag and updates `package.json`
- Keep `Cargo.toml` version in sync manually (optional, for clarity)

## CI/CD Workflows

### `.github/workflows/ci.yml`
- Runs on every push and PR
- Runs tests, clippy, and formatting checks
- Does NOT build for multiple platforms

### `.github/workflows/release.yml`
- Runs only on version tags (e.g., `v0.1.0`)
- Builds all 5 platform binaries (macOS x64/arm64, Linux x64/arm64, Windows x64)
- Publishes to npm
- Creates GitHub Release

## Best Practices

1. **Test locally first**: Use `./scripts/npm-link.sh` before creating a release
2. **Use semantic versioning**: Follow semver (major.minor.patch)
3. **Update CHANGELOG**: Document changes for each release
4. **Test installation**: After publishing, test install on a clean machine
5. **Check npm page**: Verify package appears correctly on npmjs.com

## Unpublishing

If you need to unpublish a version (within 72 hours):

```bash
npm unpublish @sanggggg/retrochat@0.1.0
```

⚠️ **Warning**: Unpublishing is permanent and can break user installations. Only use for critical issues.
