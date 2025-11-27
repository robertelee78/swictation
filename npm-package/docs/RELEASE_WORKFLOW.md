# Swictation Release Workflow

Complete guide for releasing new versions of Swictation with platform-specific packages.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Release Process](#release-process)
4. [Version Synchronization](#version-synchronization)
5. [GitHub Actions Automation](#github-actions-automation)
6. [Manual Release Process](#manual-release-process)
7. [Rollback Procedure](#rollback-procedure)
8. [Troubleshooting](#troubleshooting)

## Overview

Swictation uses a **multi-package architecture** with platform-specific builds:

```
Release Flow:
1. Bump versions → versions.json updated
2. Build Linux   → @agidreams/linux-x64 (ELF binaries)
3. Build macOS   → @agidreams/darwin-arm64 (Mach-O binaries)
4. Publish       → Platform packages to npm
5. Wait          → npm registry propagation
6. Publish       → Main swictation package to npm
```

**Key Principle:** All three packages (main + two platform packages) share the same distribution version number (e.g., `0.7.9`).

## Prerequisites

### Required Accounts & Access

- [ ] **npm Account** with publish rights to:
  - `swictation`
  - `@agidreams/linux-x64`
  - `@agidreams/darwin-arm64`
- [ ] **GitHub Repository** write access
- [ ] **NPM_TOKEN** configured in GitHub Secrets (see [GitHub Actions Setup](./GITHUB_ACTIONS_SETUP.md))

### Required Tools (for manual releases)

- [ ] **Node.js** 18.0.0+
- [ ] **Rust** toolchain
  - `rustup target add x86_64-unknown-linux-gnu` (Linux)
  - `rustup target add aarch64-apple-darwin` (macOS)
- [ ] **Tauri CLI** for UI builds
- [ ] **Build environments**:
  - Linux x64 machine (for Linux builds)
  - macOS ARM64 machine (for macOS builds)

### Verify Setup

```bash
# Check npm authentication
npm whoami

# Check you can publish to all three packages
npm access ls-packages

# Verify Rust toolchains
rustup target list --installed
```

## Release Process

### Automated Release (Recommended)

**Prerequisites:** GitHub Actions configured (see [GitHub Actions Setup](./GITHUB_ACTIONS_SETUP.md))

#### Step 1: Bump Version

```bash
# From repository root
cd npm-package

# Bump to next version (e.g., 0.7.8 → 0.7.9)
npm run version:bump

# Or specify version manually
npm run version:bump -- 0.8.0
```

This updates:
- `versions.json` (source of truth)
- `package.json` (main package)
- `packages/linux-x64/package.json`
- `packages/darwin-arm64/package.json`

#### Step 2: Commit and Tag

```bash
# Commit version bump
git add .
git commit -m "chore: bump version to 0.7.9"

# Create version tag (this triggers release workflow)
git tag v0.7.9
git push origin main --tags
```

**Note:** Pushing the `v*` tag automatically triggers the GitHub Actions release workflow.

#### Step 3: Monitor Release

1. Go to **Actions** tab in GitHub
2. Find "Release" workflow run
3. Monitor progress:
   - Build Linux binaries (5-8 minutes)
   - Build macOS binaries (5-8 minutes)
   - Publish packages (2-3 minutes)
   - Wait for npm propagation (1-2 minutes)

#### Step 4: Verify Release

```bash
# Wait 2-5 minutes for npm CDN propagation
sleep 120

# Verify packages are published
npm view swictation@0.7.9
npm view @agidreams/linux-x64@0.7.9
npm view @agidreams/darwin-arm64@0.7.9

# Test installation
npm install -g swictation@0.7.9
swictation --version
```

### Manual Release (Advanced)

Use this when GitHub Actions is unavailable or for testing.

**Warning:** Manual releases are complex and error-prone. Use automated releases when possible.

#### Step 1: Bump Version

```bash
cd npm-package
npm run version:bump -- 0.7.9
```

#### Step 2: Build Linux Package

**On Linux x64 machine:**

```bash
# Build binaries
cd npm-package/packages/linux-x64
./scripts/build.sh

# Verify binaries
file bin/swictation-daemon  # Should be ELF 64-bit
file bin/swictation-ui      # Should be ELF 64-bit

# Create package tarball
npm pack

# Output: swictation-linux-x64-0.7.9.tgz
```

#### Step 3: Build macOS Package

**On macOS ARM64 machine:**

```bash
# Build binaries
cd npm-package/packages/darwin-arm64
./scripts/build.sh

# Verify binaries
file bin/swictation-daemon  # Should be Mach-O arm64
file bin/swictation-ui      # Should be Mach-O arm64

# Create package tarball
npm pack

# Output: swictation-darwin-arm64-0.7.9.tgz
```

#### Step 4: Publish All Packages

**From repository root:**

```bash
cd npm-package

# Dry run first (test without publishing)
node scripts/publish-all.js --dry-run

# Actual publish
node scripts/publish-all.js

# This script:
# 1. Verifies versions are synchronized
# 2. Publishes @agidreams/linux-x64
# 3. Publishes @agidreams/darwin-arm64
# 4. Waits for npm registry propagation
# 5. Publishes main swictation package
```

#### Step 5: Create GitHub Release

```bash
# Create release on GitHub
gh release create v0.7.9 \
  --title "Release v0.7.9" \
  --notes "Release notes go here" \
  swictation-linux-x64-0.7.9.tgz \
  swictation-darwin-arm64-0.7.9.tgz \
  swictation-0.7.9.tgz
```

## Version Synchronization

All package versions are synchronized via `npm-package/versions.json`.

### Source of Truth: versions.json

```json
{
  "distribution": "0.7.9",
  "components": {
    "daemon": {
      "version": "0.7.5",
      "description": "Rust daemon binary"
    },
    "ui": {
      "version": "0.1.0",
      "description": "Tauri UI application"
    }
  },
  "libraries": {
    "onnxruntime": {
      "linux-gpu": "1.23.2",
      "macos": "1.22.0"
    }
  }
}
```

### Version Flow

```
versions.json
    ↓
npm run version:sync
    ↓
┌───────────────┬──────────────────────────┬───────────────────────────┐
│               │                          │                           │
package.json    packages/linux-x64/       packages/darwin-arm64/
(main)          package.json              package.json

All use: distribution version (0.7.9)
```

### Version Scripts

**Bump Version:**
```bash
npm run version:bump [version]
# Updates versions.json → synchronizes all package.json files
```

**Verify Synchronization:**
```bash
npm run version:verify
# Checks all versions match
# Exit code: 0 (success) or 1 (mismatch)
```

**Manual Sync:**
```bash
npm run version:sync
# Reads versions.json → updates all package.json files
```

### Version Mismatch Detection

The `verify-versions.js` script checks:
- ✅ Main package.json version matches distribution version
- ✅ Linux package.json version matches distribution version
- ✅ macOS package.json version matches distribution version
- ✅ All `optionalDependencies` versions match
- ✅ Component versions exist in versions.json

**Example output:**
```
Verifying versions are synchronized...
✓ Main package: 0.7.9
✓ Linux x64 package: 0.7.9
✓ macOS ARM64 package: 0.7.9
✓ Optional dependencies match
✓ All versions synchronized
```

## GitHub Actions Automation

### Workflow Architecture

```
                    Push tag v0.7.9
                           ↓
                    release.yml (orchestrator)
                           ↓
          ┌────────────────┴────────────────┐
          ↓                                 ↓
    build-linux.yml                   build-macos.yml
    (ubuntu-latest)                   (macos-14)
          ↓                                 ↓
    Linux binaries                    macOS binaries
    (ELF 64-bit)                      (Mach-O arm64)
          ↓                                 ↓
    @agidreams/linux-x64.tgz         @agidreams/darwin-arm64.tgz
          └────────────────┬────────────────┘
                           ↓
                      Artifacts
                           ↓
                    release.yml (publish)
                           ↓
          ┌────────────────┴────────────────┐
          ↓                ↓                 ↓
    npm publish      npm publish       npm publish
    linux-x64        darwin-arm64      swictation
```

### Workflow Files

#### `.github/workflows/release.yml`

Orchestrates the complete release:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'  # Trigger on version tags
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release'
        required: true

jobs:
  # 1. Trigger builds in parallel
  build:
    uses: ./.github/workflows/build-all.yml

  # 2. Publish after builds complete
  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download artifacts
      - name: Publish packages
        run: node scripts/publish-all.js
```

#### `.github/workflows/build-linux.yml`

Builds Linux x64 binaries:

```yaml
name: Build Linux

on:
  workflow_call:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
      - name: Install Rust
      - name: Build daemon & UI
        run: |
          cd npm-package/packages/linux-x64
          ./scripts/build.sh
      - name: Upload artifact
```

#### `.github/workflows/build-macos.yml`

Builds macOS ARM64 binaries:

```yaml
name: Build macOS

on:
  workflow_call:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  build:
    runs-on: macos-14  # Apple Silicon
    steps:
      - name: Checkout
      - name: Install Rust
      - name: Build daemon & UI
        run: |
          cd npm-package/packages/darwin-arm64
          ./scripts/build.sh
      - name: Upload artifact
```

### Triggering Releases

**Automatic (on tag push):**
```bash
git tag v0.7.9
git push origin v0.7.9
```

**Manual (via GitHub UI):**
1. Go to Actions → Release workflow
2. Click "Run workflow"
3. Enter version number
4. Click "Run workflow"

**Manual (via CLI):**
```bash
gh workflow run release.yml -f version=0.7.9
```

### Monitoring Release Progress

**GitHub UI:**
1. Go to **Actions** tab
2. Click on running release workflow
3. Expand jobs to see live logs

**GitHub CLI:**
```bash
# Watch latest run
gh run watch

# List recent runs
gh run list --workflow=release.yml

# View logs
gh run view <run-id> --log
```

## Manual Release Process

Detailed steps for manual releases without GitHub Actions.

### Prerequisites

- Access to both Linux x64 and macOS ARM64 machines
- npm authentication configured (`npm login`)
- Rust and Node.js installed on both machines

### Step-by-Step

#### 1. Version Bump (on any machine)

```bash
cd npm-package
npm run version:bump -- 0.7.9

# Commit and push
git add .
git commit -m "chore: bump version to 0.7.9"
git push origin main
```

#### 2. Build on Linux

**On Linux x64 machine:**

```bash
# Clone/pull latest code
git clone https://github.com/robertelee78/swictation
cd swictation

# Build Linux package
cd npm-package/packages/linux-x64
./scripts/build.sh

# Verify build
ls -lh bin/
# Should see: swictation-daemon, swictation-ui

# Check binary format
file bin/swictation-daemon
# Output: ELF 64-bit LSB executable, x86-64

# Verify checksums
cat CHECKSUMS.txt
```

#### 3. Build on macOS

**On macOS ARM64 machine:**

```bash
# Clone/pull latest code
git clone https://github.com/robertelee78/swictation
cd swictation

# Build macOS package
cd npm-package/packages/darwin-arm64
./scripts/build.sh

# Verify build
ls -lh bin/
# Should see: swictation-daemon, swictation-ui

# Check binary format
file bin/swictation-daemon
# Output: Mach-O 64-bit executable arm64

# Verify checksums
cat CHECKSUMS.txt
```

#### 4. Transfer Build Artifacts (if needed)

If publishing from a single machine:

```bash
# From Linux machine
cd npm-package/packages/linux-x64
tar -czf linux-build.tar.gz bin/ lib/ package.json CHECKSUMS.txt

# Transfer to publishing machine
scp linux-build.tar.gz user@publish-machine:/path/to/swictation/npm-package/packages/linux-x64/

# Similar for macOS
```

#### 5. Publish Packages

**On publishing machine with npm access:**

```bash
cd npm-package

# Verify versions are synced
npm run version:verify

# Dry run first
node scripts/publish-all.js --dry-run

# Review dry run output, then publish
node scripts/publish-all.js

# Monitor progress
# - Publishes @agidreams/linux-x64
# - Publishes @agidreams/darwin-arm64
# - Waits for npm registry propagation
# - Publishes swictation (main package)
```

#### 6. Verify Published Packages

```bash
# Check packages are available
npm view swictation@0.7.9
npm view @agidreams/linux-x64@0.7.9
npm view @agidreams/darwin-arm64@0.7.9

# Test installation on Linux
npm install -g swictation@0.7.9
swictation --version

# Test installation on macOS
npm install -g swictation@0.7.9
swictation --version
```

#### 7. Create GitHub Release

```bash
# Tag the release
git tag v0.7.9
git push origin v0.7.9

# Create GitHub release
gh release create v0.7.9 \
  --title "Release v0.7.9" \
  --notes "Release notes..." \
  --draft  # Remove --draft when ready
```

## Rollback Procedure

If a release has critical bugs, follow this rollback procedure.

### Quick Rollback (Deprecate Bad Version)

```bash
# Deprecate the broken version on npm
npm deprecate swictation@0.7.9 "Critical bug - use 0.7.8 instead"
npm deprecate @agidreams/linux-x64@0.7.9 "Critical bug - use 0.7.8"
npm deprecate @agidreams/darwin-arm64@0.7.9 "Critical bug - use 0.7.8"

# Users can still install the previous version
npm install -g swictation@0.7.8
```

### Full Rollback (Unpublish)

**Warning:** You can only unpublish within 72 hours of publishing.

```bash
# Unpublish all three packages
npm unpublish swictation@0.7.9
npm unpublish @agidreams/linux-x64@0.7.9
npm unpublish @agidreams/darwin-arm64@0.7.9
```

### Emergency Patch Release

If you can't unpublish (>72 hours), release a patch:

```bash
# Bump to patch version
npm run version:bump -- 0.7.10

# Cherry-pick the fix
git cherry-pick <fix-commit>

# Follow normal release process
git tag v0.7.10
git push origin main --tags
```

## Troubleshooting

### Version Mismatch Error

**Error:**
```
❌ Version mismatch detected
   Main package: 0.7.9
   Linux package: 0.7.8
```

**Fix:**
```bash
npm run version:sync
git add .
git commit -m "fix: synchronize package versions"
```

### Platform Package Not Found on npm

**Error:**
```
Error: Platform package @agidreams/linux-x64@0.7.9 not found on registry
```

**Causes:**
- npm registry propagation delay (usually 1-2 minutes)
- Publishing failed silently
- Package was unpublished

**Fix:**
```bash
# Check if package exists
npm view @agidreams/linux-x64@0.7.9

# If not found, republish platform package
cd npm-package/packages/linux-x64
npm publish --access public

# Wait for propagation, then publish main package
```

### Binary Architecture Mismatch

**Error:**
```
Error: swictation-daemon: cannot execute binary file: Exec format error
```

**Cause:** Wrong platform package installed (e.g., Linux binaries on macOS)

**Fix:**
```bash
# Verify correct platform package
npm list -g @agidreams/linux-x64    # Linux
npm list -g @agidreams/darwin-arm64  # macOS

# If wrong package, force reinstall
npm uninstall -g swictation
npm install -g swictation --force
```

### macOS Build Fails: Code Signing

**Error:**
```
error: failed to bundle project: Cannot sign bundle: No identity found
```

**Fix:**
```bash
# Disable code signing for CI builds
export TAURI_SIGNING_PRIVATE_KEY=
export TAURI_SIGNING_PUBLIC_KEY=

# Or add to build script
TAURI_SIGNING_PRIVATE_KEY="" ./scripts/build.sh
```

### npm Publish 2FA Prompt in CI

**Error:**
```
npm ERR! This operation requires a one-time password
```

**Cause:** Using "Publish" token type instead of "Automation" token

**Fix:**
1. Create new "Automation" token on npmjs.com
2. Update NPM_TOKEN secret in GitHub
3. Re-run workflow

### GitHub Actions: Out of Minutes

**Error:**
```
Error: You have exceeded your free minutes for this billing period
```

**Fix:**
- Upgrade to paid plan, or
- Wait until next billing period, or
- Use manual release process

## Additional Resources

- [GitHub Actions Setup Guide](./GITHUB_ACTIONS_SETUP.md)
- [Version Management](../scripts/README.md)
- [Platform Package Architecture](../README.md#architecture)
- [npm Publishing Documentation](https://docs.npmjs.com/cli/v8/commands/npm-publish)

---

**Last Updated:** 2025-11-26
**Maintained By:** Swictation Project
