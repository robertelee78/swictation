# NPM Package Preparation Checklist
**Generated:** 2025-11-10
**Status:** CRITICAL GAPS IDENTIFIED - NOT READY FOR NPM

## Executive Summary

⚠️ **READINESS ASSESSMENT: NOT READY FOR NPM RELEASE**

The Swictation project has excellent core functionality and documentation but is **MISSING CRITICAL COMPONENTS** for npm distribution. This is an npm wrapper around Rust binaries, not a standalone JavaScript package.

### Critical Blockers (MUST FIX before npm)
1. ❌ **No compiled binaries in npm-package/** - Empty bin/ directory
2. ❌ **No build pipeline for binary distribution** - No CI/CD for releases
3. ❌ **Broken package.json metadata** - Placeholder URLs and author
4. ❌ **Missing pre-built models** - 4GB+ models not npm-distributable
5. ❌ **No platform-specific binary handling** - No postinstall strategy
6. ❌ **No npm installation testing** - Never validated `npm install` works
7. ❌ **Zero automated tests** - No test suites for npm package
8. ❌ **No versioning strategy** - No semver tags or changelog

---

## 1. PREREQUISITES CHECK

### 1.1 Core Requirements ✅ COMPLETE

| Requirement | Status | Notes |
|-------------|--------|-------|
| **Rust Build System** | ✅ WORKS | Clean release build at `rust-crates/target/release/` |
| **Binary Compilation** | ✅ WORKS | `swictation-daemon` builds successfully (8.8MB) |
| **Dependencies** | ✅ DOCUMENTED | All system deps listed in README |
| **Documentation** | ✅ EXCELLENT | Comprehensive README and architecture docs |
| **License** | ✅ PRESENT | Apache 2.0 licensed |
| **Git Repository** | ✅ ACTIVE | Clean git history, proper .gitignore |

### 1.2 NPM Package Structure ❌ INCOMPLETE

| Component | Status | Issue |
|-----------|--------|-------|
| **package.json** | ⚠️ TEMPLATE | Placeholder URLs, wrong author, missing fields |
| **Binary Location** | ❌ MISSING | `npm-package/bin/` is empty - no binaries! |
| **postinstall.js** | ⚠️ PARTIAL | Script exists but references missing binaries |
| **CLI Wrapper** | ❌ BROKEN | `bin/swictation` expects binaries that don't exist |
| **Config Templates** | ⚠️ MINIMAL | Only `config/` directory, no example configs |
| **Models** | ❌ MISSING | 4GB models at `/opt/swictation/models/` not packaged |

---

## 2. MISSING COMPONENTS (CRITICAL)

### 2.1 Binary Distribution Strategy ❌ NOT IMPLEMENTED

**PROBLEM:** npm packages typically cannot include large native binaries directly.

**Current State:**
- Rust daemon binary: 8.8MB (reasonable size)
- Models directory: 4GB+ (parakeet-tdt-1.1b-onnx, silero-vad)
- npm-package/bin/ directory: **EMPTY**

**Required Solutions (choose one or combine):**

#### Option A: Postinstall Binary Download (RECOMMENDED)
```javascript
// postinstall.js enhancement needed:
// 1. Detect platform (linux-x64 only for now)
// 2. Download pre-built binaries from GitHub Releases
// 3. Extract to bin/ directory
// 4. Set execute permissions
// 5. Verify integrity with checksums
```

#### Option B: Local Build Fallback
```javascript
// For users with Rust toolchain:
// 1. Check if binaries exist
// 2. If not, try: cd /opt/swictation/rust-crates && cargo build --release
// 3. Copy binaries to npm-package/bin/
```

#### Option C: Separate Binary Package
```javascript
// Create @swictation/binaries-linux-x64 package
// Main package depends on platform-specific binary package
// Similar to esbuild's approach
```

**Model Distribution:**
- Cannot include 4GB models in npm package
- Postinstall must download from:
  - NVIDIA NGC (parakeet-tdt models)
  - Silero VAD releases
- Requires download progress indicators
- Needs cache management (~/.cache/swictation/)

### 2.2 GitHub CI/CD Pipeline ⚠️ PARTIALLY IMPLEMENTED

**Existing:**
- `.github/workflows/ci.yml` - Basic CI
- `.github/workflows/npm-publish.yml` - NPM publishing workflow

**CRITICAL GAPS:**
1. **No binary release automation** - No workflow to build and upload binaries
2. **No cross-platform builds** - Only supports linux-x64, but no automated builds
3. **No model hosting** - No strategy to host 4GB models for postinstall
4. **No version tagging** - No git tags for releases
5. **No changelog generation** - No automated CHANGELOG.md

**Required GitHub Actions:**
```yaml
# Needed: .github/workflows/release.yml
name: Release Binaries
on:
  push:
    tags: ['v*']
jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - Build Rust binaries
      - Create release
      - Upload swictation-daemon
      - Upload checksums
      - Tag Docker images (optional)
```

### 2.3 Package Metadata ❌ INCOMPLETE

**Current package.json issues:**
```json
{
  "name": "swictation",                              // ✅ Good
  "version": "0.1.0",                                // ⚠️ Pre-release version
  "description": "...",                              // ✅ Good
  "homepage": "https://github.com/yourusername/...", // ❌ Placeholder!
  "bugs": {
    "url": "https://github.com/yourusername/..."    // ❌ Placeholder!
  },
  "repository": {
    "url": "https://github.com/yourusername/..."    // ❌ Placeholder!
  },
  "author": "Your Name",                             // ❌ Placeholder!
  "license": "MIT",                                  // ⚠️ Main repo is Apache 2.0
  "files": [
    "bin/",                                          // ❌ EMPTY DIRECTORY!
    "lib/",                                          // ❌ EMPTY DIRECTORY!
    "config/",                                       // ⚠️ Minimal files
    "postinstall.js",                                // ✅ Present
    "README.md"                                      // ✅ Present
  ]
}
```

**MUST FIX:**
1. Update all placeholder URLs with actual repository
2. Set correct author information
3. Align license (Apache 2.0 vs MIT)
4. Add `keywords` for npm search
5. Add `engines.node` constraint (currently >=14.0.0, validate this)
6. Remove empty directories from `files` array
7. Add `publishConfig` for scoped package if needed

### 2.4 Installation Testing ❌ NEVER VALIDATED

**CRITICAL:** The package has never been tested via `npm install`!

**Missing validation scenarios:**
- [ ] Fresh installation: `npm install -g swictation`
- [ ] Postinstall script execution
- [ ] Binary download verification
- [ ] Permission setting validation
- [ ] CLI command functionality (`swictation --help`)
- [ ] Daemon startup: `swictation start`
- [ ] Clean uninstallation: `npm uninstall -g swictation`
- [ ] Installation without internet (should fail gracefully)
- [ ] Installation on clean Ubuntu 22.04 LTS
- [ ] Installation on Arch Linux

---

## 3. TESTING REQUIREMENTS

### 3.1 Unit Tests ❌ NOT PRESENT

**Current State:**
- Rust code has some unit tests (7 files with `#[test]`)
- npm package has **ZERO tests**
- No test suite for npm CLI wrapper
- No integration tests

**Required Test Coverage:**

#### Rust Binary Tests (Already Present) ✅
```bash
# Existing tests:
rust-crates/swictation-daemon/src/gpu.rs
rust-crates/swictation-stt/src/recognizer.rs
rust-crates/swictation-stt/src/audio.rs
# etc.

# Run with:
cd rust-crates && cargo test
```

#### npm Package Tests (MISSING) ❌
```javascript
// tests/postinstall.test.js - NEEDED
describe('postinstall', () => {
  test('detects platform correctly');
  test('fails gracefully on Windows');
  test('downloads binaries when missing');
  test('sets execute permissions');
  test('creates config directories');
});

// tests/cli.test.js - NEEDED
describe('CLI', () => {
  test('swictation --help shows usage');
  test('swictation start fails without daemon binary');
  test('swictation status detects running daemon');
});

// tests/integration.test.js - NEEDED
describe('Installation', () => {
  test('npm install completes successfully');
  test('binaries are executable after install');
  test('systemd service file is created');
});
```

### 3.2 Installation Testing ❌ NOT IMPLEMENTED

**Manual Test Plan Required:**

```bash
# Test 1: Fresh Installation
npm pack                              # Create tarball
npm install -g ./swictation-0.1.0.tgz # Install locally
swictation --help                     # Verify CLI works

# Test 2: Daemon Functionality
swictation setup                      # Run setup wizard
swictation start                      # Start daemon
swictation status                     # Check status
swictation toggle                     # Test toggle command
swictation stop                       # Stop daemon

# Test 3: Clean Uninstall
npm uninstall -g swictation
# Verify no leftover files in:
# - ~/.config/swictation/
# - ~/.local/share/swictation/
# - ~/.cache/swictation/
```

**Automated Test Environments:**
- [ ] Docker container: Ubuntu 22.04 LTS
- [ ] Docker container: Arch Linux
- [ ] VM: Fresh Manjaro installation
- [ ] CI/CD: GitHub Actions matrix

### 3.3 Platform Testing ⚠️ LIMITED

**Current Support:**
- ✅ Linux x64 - Primary target
- ❌ Linux ARM64 - Not supported
- ❌ macOS - Not supported
- ❌ Windows - Not supported

**Validation Matrix (for Linux x64 only):**

| Distribution | Version | Status | Blocker |
|--------------|---------|--------|---------|
| Ubuntu | 22.04 LTS | ⚠️ UNKNOWN | Not tested with npm install |
| Ubuntu | 24.04 LTS | ⚠️ UNKNOWN | Not tested with npm install |
| Arch Linux | Rolling | ⚠️ UNKNOWN | Not tested with npm install |
| Manjaro | Current | ⚠️ UNKNOWN | Not tested with npm install |
| Debian | 12 (Bookworm) | ⚠️ UNKNOWN | Not tested with npm install |

**Node.js Version Testing:**
- [ ] Node 14.x (minimum per package.json)
- [ ] Node 16.x (LTS)
- [ ] Node 18.x (Active LTS)
- [ ] Node 20.x (Current LTS)
- [ ] Node 22.x (Current)

---

## 4. DOCUMENTATION GAPS

### 4.1 npm-Specific Documentation ⚠️ INCOMPLETE

**Main README.md:**
- ✅ Excellent technical documentation
- ✅ Architecture details comprehensive
- ⚠️ Installation instructions assume `/opt/swictation` clone
- ❌ No npm installation instructions
- ❌ No troubleshooting for npm installation issues

**npm-package/README.md:**
- ✅ Exists and has installation instructions
- ⚠️ References binaries that don't exist yet
- ❌ No troubleshooting section
- ❌ No uninstallation instructions

**NEEDED ADDITIONS:**

```markdown
# README.md (main) - Add npm section:

## Installation via npm (Recommended)

### Prerequisites
- Linux x64
- Node.js >=14.0.0
- NVIDIA GPU with 4GB+ VRAM (optional, CPU fallback available)
- wtype (Wayland) or xdotool (X11)

### Install
npm install -g swictation

### Quick Start
swictation setup    # Configure systemd service
swictation start    # Start daemon
# Press $mod+Shift+d in any application to toggle recording

### Troubleshooting
See [NPM_TROUBLESHOOTING.md](docs/NPM_TROUBLESHOOTING.md)
```

### 4.2 Missing Documentation Files ❌

**CRITICAL MISSING:**
1. **CHANGELOG.md** - No version history
2. **CONTRIBUTING.md** - No contribution guidelines
3. **docs/NPM_TROUBLESHOOTING.md** - No npm-specific troubleshooting
4. **docs/INSTALLING.md** - No detailed installation guide
5. **docs/BUILDING.md** - No build-from-source instructions
6. **SECURITY.md** - No security policy

**NEEDED FOR npm:**
```markdown
# docs/NPM_TROUBLESHOOTING.md (NEW FILE NEEDED)

## Common npm Installation Issues

### Issue: "Permission denied" during install
Solution: Use `npm install -g swictation` with proper permissions

### Issue: "Binary not found" after installation
Solution: Check $PATH includes npm global bin directory

### Issue: "Cannot download models"
Solution: Check internet connectivity, proxy settings

### Issue: "CUDA not found"
Solution: Install nvidia-cuda-toolkit, or use CPU mode

### Issue: systemctl not found
Solution: systemd is required - install or use manual daemon startup
```

---

## 5. BUILD & RELEASE PROCESS

### 5.1 Build Automation ❌ NOT IMPLEMENTED

**Current Build Process (Manual):**
```bash
cd /opt/swictation/rust-crates
cargo build --release
# Binary at: target/release/swictation-daemon (8.8MB)
```

**NEEDED: Automated Release Build:**
```bash
# scripts/build-release.sh (NEW SCRIPT NEEDED)
#!/bin/bash
set -e

echo "Building Swictation Release..."

# 1. Clean previous builds
cd rust-crates
cargo clean

# 2. Build optimized binary
cargo build --release --locked

# 3. Strip symbols for smaller binary
strip target/release/swictation-daemon

# 4. Verify binary works
./target/release/swictation-daemon --version

# 5. Copy to npm-package
mkdir -p ../npm-package/bin
cp target/release/swictation-daemon ../npm-package/bin/

# 6. Set permissions
chmod +x ../npm-package/bin/swictation-daemon

# 7. Generate checksums
cd ../npm-package/bin
sha256sum swictation-daemon > SHA256SUMS

echo "✓ Release build complete!"
```

### 5.2 Version Management ❌ NOT IMPLEMENTED

**Current State:**
- package.json: `"version": "0.1.0"`
- No git tags for releases
- No semantic versioning strategy
- No CHANGELOG.md

**NEEDED: Version Release Process:**
```bash
# scripts/release.sh (NEW SCRIPT NEEDED)
#!/bin/bash
VERSION=$1

if [ -z "$VERSION" ]; then
  echo "Usage: ./release.sh <version>"
  exit 1
fi

# 1. Update version in all files
npm version $VERSION --no-git-tag-version

# 2. Update CHANGELOG.md
# (manual or use conventional-changelog)

# 3. Build release binaries
./scripts/build-release.sh

# 4. Commit version bump
git add package.json CHANGELOG.md
git commit -m "chore: Release v$VERSION"

# 5. Tag release
git tag -a "v$VERSION" -m "Release v$VERSION"

# 6. Push to GitHub
git push && git push --tags

# 7. GitHub Actions will:
#    - Build binaries
#    - Create GitHub Release
#    - Publish to npm
```

### 5.3 npm Publishing Workflow ⚠️ PRESENT BUT INCOMPLETE

**Existing:** `.github/workflows/npm-publish.yml`

**CRITICAL GAPS:**
1. **No binary building** - Workflow doesn't build Rust binaries
2. **No binary upload** - Binaries not attached to GitHub Release
3. **No model hosting** - No strategy to host downloadable models
4. **No dry-run testing** - Should test with `npm pack` before publish

**ENHANCED WORKFLOW NEEDED:**
```yaml
# .github/workflows/release.yml (ENHANCEMENT NEEDED)
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build-binaries:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: actions-rs/toolchain@v1
      - name: Build release binary
        run: |
          cd rust-crates
          cargo build --release
          strip target/release/swictation-daemon
      - name: Upload binary artifact
        uses: actions/upload-artifact@v4
        with:
          name: swictation-daemon-linux-x64
          path: rust-crates/target/release/swictation-daemon
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            rust-crates/target/release/swictation-daemon
            rust-crates/target/release/SHA256SUMS

  publish-npm:
    needs: build-binaries
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Download binaries
        uses: actions/download-artifact@v4
      - name: Copy binaries to npm package
        run: |
          mkdir -p npm-package/bin
          cp swictation-daemon-linux-x64/swictation-daemon npm-package/bin/
          chmod +x npm-package/bin/swictation-daemon
      - name: Test npm package
        run: |
          cd npm-package
          npm pack
          npm install -g ./swictation-*.tgz
          swictation --help
      - name: Publish to npm
        run: |
          cd npm-package
          npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

## 6. PRE-RELEASE CHECKLIST

### Phase 1: Core Fixes (MUST COMPLETE)
- [ ] Fix package.json metadata (author, URLs, license)
- [ ] Implement binary download in postinstall.js
- [ ] Create build-release.sh script
- [ ] Test manual binary distribution
- [ ] Verify postinstall downloads models correctly
- [ ] Add SHA256 checksum verification

### Phase 2: Documentation (MUST COMPLETE)
- [ ] Add npm installation section to main README
- [ ] Create docs/NPM_TROUBLESHOOTING.md
- [ ] Create CHANGELOG.md
- [ ] Create CONTRIBUTING.md
- [ ] Update npm-package/README.md with real examples
- [ ] Add uninstallation instructions

### Phase 3: Testing (MUST COMPLETE)
- [ ] Manual test: Ubuntu 22.04 LTS fresh install
- [ ] Manual test: Arch Linux fresh install
- [ ] Test Node 18.x and Node 20.x
- [ ] Test with/without NVIDIA GPU
- [ ] Test with/without systemd
- [ ] Test uninstallation cleanup

### Phase 4: Automation (RECOMMENDED)
- [ ] Enhance .github/workflows/release.yml
- [ ] Add automated binary builds
- [ ] Implement version bump script
- [ ] Setup npm provenance (npm publish --provenance)
- [ ] Add GitHub Release automation
- [ ] Configure Dependabot for security updates

### Phase 5: Quality Assurance (RECOMMENDED)
- [ ] Write postinstall tests
- [ ] Write CLI integration tests
- [ ] Add Docker-based installation tests to CI
- [ ] Setup code coverage reporting
- [ ] Add linting for npm package (eslint)
- [ ] Validate package with `npm pack` locally

---

## 7. BLOCKING ISSUES (PRIORITY ORDER)

### P0 - Critical Blockers (CANNOT RELEASE WITHOUT)
1. **Empty bin/ directory** - No binaries to install
2. **Broken postinstall** - References missing binaries
3. **No binary distribution** - No download mechanism
4. **Placeholder metadata** - Invalid package.json URLs
5. **No model distribution** - 4GB models not downloadable
6. **Never tested npm install** - Zero validation

### P1 - Major Issues (SHOULD FIX BEFORE RELEASE)
1. **No CHANGELOG.md** - No version history
2. **No release automation** - Manual release process
3. **No installation tests** - No automated validation
4. **Incomplete documentation** - Missing troubleshooting
5. **No version tags** - No git release tags

### P2 - Quality Issues (FIX SOON AFTER RELEASE)
1. **No test suite** - Zero automated tests for npm package
2. **No CI/CD for tests** - No automated quality checks
3. **No security policy** - No SECURITY.md
4. **No contribution guide** - No CONTRIBUTING.md

---

## 8. RECOMMENDED TIMELINE

### Week 1: Core Fixes (P0 Blockers)
- Day 1-2: Fix package.json metadata, implement binary download
- Day 3-4: Create build scripts, test binary distribution
- Day 5: Manual installation testing on Ubuntu/Arch

### Week 2: Documentation & Testing (P1 Issues)
- Day 1-2: Write npm-specific docs, troubleshooting guide
- Day 3-4: Create CHANGELOG, version bump scripts
- Day 5: Manual testing across different environments

### Week 3: Automation & Polish (P2 Quality)
- Day 1-3: Enhance CI/CD workflows, add automated tests
- Day 4: Final round of testing
- Day 5: Beta release to npm with --tag beta

### Week 4: Beta Testing & Release
- Day 1-3: Community testing, bug fixes
- Day 4: Final validation
- Day 5: Official v0.1.0 release to npm

---

## 9. RISK ASSESSMENT

### High Risk Issues
1. **Binary Size** - 8.8MB daemon binary may fail on slow connections
   - Mitigation: Implement download progress, retry logic, mirror CDN
2. **Model Downloads** - 4GB models will take time, may fail
   - Mitigation: Resume downloads, chunked downloads, torrent alternative
3. **Platform Detection** - postinstall may fail on unusual systems
   - Mitigation: Extensive error handling, fallback modes
4. **GPU Detection** - CUDA availability detection may be unreliable
   - Mitigation: Always provide CPU fallback, clear error messages

### Medium Risk Issues
1. **systemd Dependency** - Not all Linux systems have systemd
   - Mitigation: Provide non-systemd startup instructions
2. **Node Version Compatibility** - May break on very old Node versions
   - Mitigation: Test minimum Node 14.x, warn on older
3. **Wayland/X11 Detection** - May not work on all display servers
   - Mitigation: Detect both wtype and xdotool, provide manual setup

---

## 10. FINAL VERDICT

### Current Status: ⚠️ **NOT READY FOR NPM**

**Confidence Level:** 20% ready

**Estimated Work:** 2-3 weeks for minimum viable npm package

**Critical Path:**
1. Implement binary download mechanism (5 days)
2. Fix package metadata and documentation (3 days)
3. Manual testing on multiple platforms (3 days)
4. Beta release and community feedback (1 week)

**Recommended Approach:**
- Do NOT publish to npm yet
- Complete all P0 blockers first
- Run extensive manual testing
- Consider beta release with --tag beta flag
- Gather community feedback before stable release

---

## APPENDICES

### A. Useful Commands for Testing

```bash
# Test npm package locally
npm pack
npm install -g ./swictation-0.1.0.tgz

# Verify installation
which swictation
swictation --help
swictation status

# Check installed files
npm ls -g swictation
npm explore -g swictation -- ls -la

# Uninstall
npm uninstall -g swictation

# Validate package.json
npm pkg fix
npm pkg validate

# Test in Docker
docker run -it --rm ubuntu:22.04 bash
# Inside container:
apt update && apt install -y curl
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt install -y nodejs
npm install -g swictation
```

### B. Reference npm Packages (Similar Structure)

Study these for binary distribution patterns:
- **esbuild** - Downloads platform-specific binaries via postinstall
- **@swc/core** - Native module with platform-specific packages
- **playwright** - Downloads browser binaries via postinstall
- **node-gyp** - Compiles native addons during install

### C. Contact for Questions

This checklist generated by Swictation Tester Agent.
For questions or updates, see: `/opt/swictation/.swarm/memory.db`

---

**Document Version:** 1.0
**Last Updated:** 2025-11-10
**Next Review:** After P0 blockers resolved
