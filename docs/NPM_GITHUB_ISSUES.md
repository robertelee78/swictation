# GitHub Issues for NPM Preparation
**Generated:** 2025-11-10
**Purpose:** Actionable issues to track npm readiness work

---

## Issue Template Format

Copy/paste these directly into GitHub Issues. Each issue is self-contained with clear acceptance criteria.

---

## Priority 0 (Critical Blockers)

### Issue #1: [P0] Fix package.json metadata - remove all placeholders

**Labels:** `P0`, `npm`, `good-first-issue`
**Estimated Effort:** 30 minutes

**Description:**
The npm package has placeholder URLs and author information that must be updated before publishing to npm registry.

**Current Issues:**
```json
"homepage": "https://github.com/yourusername/swictation",  // ❌ Placeholder
"bugs": "https://github.com/yourusername/...",              // ❌ Placeholder
"repository": "https://github.com/yourusername/...",        // ❌ Placeholder
"author": "Your Name",                                      // ❌ Placeholder
"license": "MIT",                                           // ⚠️ Conflicts with Apache 2.0
```

**Tasks:**
- [ ] Update `homepage` to actual repository URL
- [ ] Update `bugs.url` to actual GitHub issues URL
- [ ] Update `repository.url` to actual repository URL
- [ ] Set correct `author` name and email
- [ ] Align `license` field (use Apache-2.0 to match LICENSE file)
- [ ] Add proper `keywords` array for npm search
- [ ] Validate with `npm pkg validate`

**Acceptance Criteria:**
- [ ] No placeholder text remains in package.json
- [ ] All URLs point to actual repository
- [ ] License matches main LICENSE file (Apache 2.0)
- [ ] `npm pkg validate` passes without warnings

**Files to Update:**
- `npm-package/package.json`

---

### Issue #2: [P0] Implement binary download mechanism in postinstall

**Labels:** `P0`, `npm`, `enhancement`
**Estimated Effort:** 2 days

**Description:**
The npm package currently has no mechanism to download or provide the compiled Rust daemon binary. Users cannot use the software without this.

**Current State:**
- `npm-package/bin/` directory is empty
- postinstall.js references missing binaries
- No download implementation exists

**Proposed Solution:**
Implement binary download from GitHub Releases in postinstall.js:

1. Detect platform (Linux x64 only for now)
2. Download pre-built binary from GitHub Releases
3. Extract to bin/ directory
4. Set execute permissions (chmod +x)
5. Verify integrity with SHA256 checksums
6. Show progress bar during download
7. Handle errors gracefully with clear messages

**Tasks:**
- [ ] Implement binary download function in postinstall.js
- [ ] Add progress bar using existing deps (avoid new deps)
- [ ] Add SHA256 checksum verification
- [ ] Add retry logic for failed downloads
- [ ] Handle offline/no-internet scenarios gracefully
- [ ] Test download from GitHub Releases
- [ ] Update documentation with download details

**Dependencies:**
- Requires Issue #4 (GitHub Release workflow) to be complete
- Requires binaries to be uploaded to GitHub Releases

**Acceptance Criteria:**
- [ ] Running `npm install -g swictation` downloads daemon binary
- [ ] Binary is executable after download (chmod +x applied)
- [ ] SHA256 checksum is verified
- [ ] Progress bar shows during download
- [ ] Clear error messages if download fails
- [ ] Works on Ubuntu 22.04 and Arch Linux

**Files to Create/Update:**
- `npm-package/postinstall.js` (enhancement)
- `npm-package/lib/downloader.js` (new - optional)

---

### Issue #3: [P0] Implement model download mechanism

**Labels:** `P0`, `npm`, `enhancement`
**Estimated Effort:** 3 days

**Description:**
The daemon requires 4GB+ of AI models (Parakeet-TDT, Silero VAD) to function. These cannot be bundled in npm package and must be downloaded.

**Required Models:**
1. Parakeet-TDT-1.1B (3.2GB) - from NVIDIA NGC
2. Parakeet-TDT-0.6B (800MB) - from NVIDIA NGC
3. Silero VAD v6 (2.3MB) - from Silero releases

**Proposed Solution:**
1. Create download script: `npm-package/lib/model-downloader.js`
2. Download models to: `~/.cache/swictation/models/`
3. Show progress with ETA for large downloads
4. Support resume on interrupted downloads
5. Verify checksums after download
6. Allow offline mode (skip download if models exist)

**Tasks:**
- [ ] Create model downloader module
- [ ] Implement chunked download with resume support
- [ ] Add progress bars for each model (3 separate)
- [ ] Implement SHA256 verification for each model
- [ ] Handle partial downloads gracefully
- [ ] Create model manifest (URLs, checksums, sizes)
- [ ] Add `swictation download-models` CLI command
- [ ] Update postinstall to optionally download models
- [ ] Add `--skip-models` flag for postinstall

**Acceptance Criteria:**
- [ ] Models download to `~/.cache/swictation/models/`
- [ ] Progress bars show for each model with ETA
- [ ] Downloads can be interrupted and resumed
- [ ] Checksums are verified after download
- [ ] Works offline if models already exist
- [ ] Clear error messages if downloads fail

**Files to Create/Update:**
- `npm-package/lib/model-downloader.js` (new)
- `npm-package/config/model-manifest.json` (new)
- `npm-package/bin/swictation` (add download-models command)
- `npm-package/postinstall.js` (call model downloader)

---

### Issue #4: [P0] Create GitHub Release workflow for binary builds

**Labels:** `P0`, `ci-cd`, `github-actions`
**Estimated Effort:** 1 day

**Description:**
No automation exists to build and publish binaries to GitHub Releases. This blocks Issue #2 (binary download).

**Required Workflow:**
On git tag push (`v*`):
1. Build Rust daemon in release mode
2. Strip symbols for smaller binary
3. Generate SHA256 checksums
4. Create GitHub Release
5. Upload binary and checksums to release
6. Trigger npm publish workflow

**Tasks:**
- [ ] Create `.github/workflows/release.yml`
- [ ] Add Rust build job (ubuntu-latest)
- [ ] Install CUDA toolkit in CI (for GPU builds)
- [ ] Build release binary: `cargo build --release`
- [ ] Strip binary: `strip target/release/swictation-daemon`
- [ ] Generate checksums: `sha256sum > SHA256SUMS`
- [ ] Create GitHub Release with `softprops/action-gh-release`
- [ ] Upload binary and checksums as release assets
- [ ] Add release notes template
- [ ] Test workflow with dummy tag

**Acceptance Criteria:**
- [ ] Pushing tag `v0.1.0-beta.1` triggers workflow
- [ ] Binary builds successfully in CI
- [ ] GitHub Release is created automatically
- [ ] Binary and SHA256SUMS are attached to release
- [ ] Release has proper title and notes

**Files to Create:**
- `.github/workflows/release.yml` (new)

---

### Issue #5: [P0] Test npm installation on Ubuntu 22.04 LTS

**Labels:** `P0`, `testing`, `documentation`
**Estimated Effort:** 1 day

**Description:**
The npm package has never been tested via `npm install`. We need validation that it actually works.

**Test Environment:**
- Ubuntu 22.04 LTS (Docker or VM)
- Node.js 20.x LTS
- Clean system (no prior swictation installation)

**Test Plan:**
```bash
# 1. Fresh installation
npm install -g swictation

# 2. Verify CLI works
swictation --help
swictation --version

# 3. Run setup
swictation setup

# 4. Start daemon
swictation start

# 5. Check status
swictation status

# 6. Test toggle
swictation toggle

# 7. Stop daemon
swictation stop

# 8. Clean uninstall
npm uninstall -g swictation

# 9. Verify cleanup
ls ~/.config/swictation/    # Should be gone
ls ~/.cache/swictation/     # Should be gone
```

**Tasks:**
- [ ] Create Docker test environment (Dockerfile)
- [ ] Document test procedure in `docs/TESTING.md`
- [ ] Run full test manually
- [ ] Document all issues found
- [ ] Create GitHub issues for any failures
- [ ] Take screenshots of successful run
- [ ] Update README with test results

**Acceptance Criteria:**
- [ ] All 9 test steps pass without errors
- [ ] Documentation created in `docs/TESTING.md`
- [ ] Dockerfile added for reproducible tests
- [ ] Screenshots or logs captured

**Files to Create:**
- `docs/TESTING.md` (new)
- `docker/test-ubuntu-22.04/Dockerfile` (new)

---

### Issue #6: [P0] Test npm installation on Arch Linux

**Labels:** `P0`, `testing`, `documentation`
**Estimated Effort:** 4 hours

**Description:**
Same as Issue #5 but for Arch Linux (rolling release, different package manager).

**Test Environment:**
- Arch Linux (Docker or VM)
- Node.js latest from official repos
- Clean system

**Tasks:**
- [ ] Create Docker test environment (Dockerfile)
- [ ] Run full test manually (same procedure as Issue #5)
- [ ] Document Arch-specific issues
- [ ] Update `docs/TESTING.md` with Arch results

**Acceptance Criteria:**
- [ ] All test steps pass on Arch Linux
- [ ] Dockerfile added for Arch tests

**Files to Create:**
- `docker/test-arch/Dockerfile` (new)

---

## Priority 1 (Should Fix Before Release)

### Issue #7: [P1] Create CHANGELOG.md with version history

**Labels:** `P1`, `documentation`, `good-first-issue`
**Estimated Effort:** 2 hours

**Description:**
No CHANGELOG.md exists. npm packages should have clear version history.

**Format:**
Use [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog
All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.1.0] - 2025-11-XX
### Added
- Initial npm package release
- Rust daemon with VAD-triggered transcription
- Parakeet-TDT STT models (0.6B and 1.1B)
- Silero VAD v6 integration
- systemd service integration
- CLI wrapper for daemon management

### Fixed
- (none yet - first release)

### Changed
- (none yet - first release)
```

**Tasks:**
- [ ] Create CHANGELOG.md in root directory
- [ ] Populate with v0.1.0 initial release notes
- [ ] Add links to compare versions
- [ ] Update release workflow to auto-update changelog

**Acceptance Criteria:**
- [ ] CHANGELOG.md exists at root
- [ ] Follows Keep a Changelog format
- [ ] Documents v0.1.0 release

**Files to Create:**
- `CHANGELOG.md` (new)

---

### Issue #8: [P1] Create npm-specific troubleshooting guide

**Labels:** `P1`, `documentation`
**Estimated Effort:** 4 hours

**Description:**
Users will encounter npm-specific issues. Create dedicated troubleshooting document.

**Topics to Cover:**
1. Permission errors during install
2. Binary download failures
3. Model download failures
4. systemd not found
5. wtype/xdotool not found
6. CUDA not detected
7. Daemon won't start
8. Socket connection errors

**Tasks:**
- [ ] Create `docs/NPM_TROUBLESHOOTING.md`
- [ ] Document common npm installation errors
- [ ] Add solutions for each error
- [ ] Include diagnostic commands
- [ ] Link from main README
- [ ] Add to npm-package/README.md

**Acceptance Criteria:**
- [ ] Document covers 8+ common issues
- [ ] Each issue has clear solution steps
- [ ] Linked from README files

**Files to Create:**
- `docs/NPM_TROUBLESHOOTING.md` (new)

---

### Issue #9: [P1] Add npm installation section to README

**Labels:** `P1`, `documentation`, `good-first-issue`
**Estimated Effort:** 1 hour

**Description:**
Main README assumes git clone installation. Add npm installation as primary method.

**Required Sections:**
```markdown
## Installation

### Via npm (Recommended)

#### Prerequisites
- Linux x64
- Node.js >=14.0.0
- NVIDIA GPU with 4GB+ VRAM (optional, CPU fallback available)
- wtype (Wayland) or xdotool (X11)
- systemd (optional, can run manually)

#### Install
npm install -g swictation

#### Quick Start
swictation setup    # Configure systemd service
swictation start    # Start daemon
# Press $mod+Shift+d in any application to toggle recording

#### Troubleshooting
See [NPM Troubleshooting Guide](docs/NPM_TROUBLESHOOTING.md)

### From Source (Advanced)
(existing installation instructions)
```

**Tasks:**
- [ ] Add npm installation section to main README
- [ ] Move git clone to "Advanced" section
- [ ] Update Quick Start section
- [ ] Add link to troubleshooting
- [ ] Update badges if needed

**Acceptance Criteria:**
- [ ] npm installation is first method shown
- [ ] Prerequisites clearly listed
- [ ] Quick start works for npm users

**Files to Update:**
- `README.md`

---

### Issue #10: [P1] Create version bump and release script

**Labels:** `P1`, `automation`, `tooling`
**Estimated Effort:** 4 hours

**Description:**
No automation for version bumps and releases. Create helper script.

**Script: `scripts/release.sh`**
```bash
#!/bin/bash
# Usage: ./scripts/release.sh 0.1.0

VERSION=$1
if [ -z "$VERSION" ]; then
  echo "Usage: ./scripts/release.sh <version>"
  exit 1
fi

# 1. Update version in package.json
cd npm-package
npm version $VERSION --no-git-tag-version

# 2. Update CHANGELOG.md
# (prompt user to edit CHANGELOG.md)

# 3. Commit version bump
git add package.json CHANGELOG.md
git commit -m "chore: Release v$VERSION"

# 4. Tag release
git tag -a "v$VERSION" -m "Release v$VERSION"

# 5. Push (triggers GitHub Actions)
git push && git push --tags

echo "✓ Release v$VERSION started!"
echo "Monitor: https://github.com/yourusername/swictation/actions"
```

**Tasks:**
- [ ] Create `scripts/release.sh`
- [ ] Make executable: `chmod +x scripts/release.sh`
- [ ] Test with dry-run flag
- [ ] Document in CONTRIBUTING.md
- [ ] Add validation checks (clean git, tests pass)

**Acceptance Criteria:**
- [ ] Script automates version bump
- [ ] Creates git tag correctly
- [ ] Triggers CI/CD on push
- [ ] Documented for maintainers

**Files to Create:**
- `scripts/release.sh` (new)

---

## Priority 2 (Fix Soon After Release)

### Issue #11: [P2] Add automated tests for npm package

**Labels:** `P2`, `testing`, `enhancement`
**Estimated Effort:** 2 days

**Description:**
Zero automated tests exist for npm package. Add test suite.

**Test Framework:**
Use Jest or Mocha (already in ecosystem)

**Required Tests:**
1. `tests/postinstall.test.js` - postinstall script
2. `tests/cli.test.js` - CLI commands
3. `tests/downloader.test.js` - binary/model downloading
4. `tests/integration.test.js` - full install/uninstall

**Tasks:**
- [ ] Add test framework to npm-package
- [ ] Write postinstall tests (platform detection, downloads)
- [ ] Write CLI tests (help, version, commands)
- [ ] Write downloader tests (mock network)
- [ ] Write integration tests (full flow)
- [ ] Add `npm test` script to package.json
- [ ] Add to CI/CD pipeline

**Acceptance Criteria:**
- [ ] Test coverage >60% for npm package code
- [ ] All tests pass locally
- [ ] Tests run in CI/CD

**Files to Create:**
- `npm-package/tests/` (new directory)
- `npm-package/jest.config.js` or `mocha.opts`

---

### Issue #12: [P2] Create CONTRIBUTING.md with development guide

**Labels:** `P2`, `documentation`, `good-first-issue`
**Estimated Effort:** 2 hours

**Description:**
No contribution guidelines exist. Create guide for contributors.

**Required Sections:**
1. Development setup
2. Building from source
3. Running tests
4. Submitting PRs
5. Code style
6. Release process

**Tasks:**
- [ ] Create CONTRIBUTING.md
- [ ] Document dev environment setup
- [ ] Link to code of conduct
- [ ] Add PR template
- [ ] Document commit message format

**Acceptance Criteria:**
- [ ] CONTRIBUTING.md covers all sections
- [ ] Linked from README

**Files to Create:**
- `CONTRIBUTING.md` (new)
- `.github/PULL_REQUEST_TEMPLATE.md` (new)

---

### Issue #13: [P2] Add Docker-based installation tests to CI/CD

**Labels:** `P2`, `ci-cd`, `testing`
**Estimated Effort:** 1 day

**Description:**
Automate installation testing in CI/CD using Docker containers.

**Test Matrix:**
- Ubuntu 22.04 + Node 18
- Ubuntu 22.04 + Node 20
- Arch Linux + Node latest

**Tasks:**
- [ ] Create `.github/workflows/test-install.yml`
- [ ] Add Docker build jobs for each platform
- [ ] Run full installation test in each
- [ ] Fail CI if any test fails
- [ ] Cache Docker layers for speed

**Acceptance Criteria:**
- [ ] CI tests on 3 environments
- [ ] Runs on every PR
- [ ] Clear failure messages

**Files to Create:**
- `.github/workflows/test-install.yml` (new)

---

## Summary

**Total Issues:** 13
- **P0 (Critical):** 6 issues (~8 days of work)
- **P1 (Important):** 5 issues (~2 days of work)
- **P2 (Nice to have):** 2 issues (~3 days of work)

**Estimated Total Effort:** 13 days (2-3 weeks with testing/iteration)

**Critical Path:**
1. Issue #1 (metadata) - Quick win, do first
2. Issue #4 (release workflow) - Needed for Issue #2
3. Issue #2 (binary download) - Blocks testing
4. Issue #3 (model download) - Can parallel with #2
5. Issue #5 + #6 (testing) - Final validation

**Recommended Order:**
1. #1 (30 min) → #7 (2 hr) → #9 (1 hr) = Day 1 quick wins
2. #4 (1 day) = Day 2 enable automation
3. #2 (2 days) = Days 3-4 binary distribution
4. #3 (3 days) = Days 5-7 model distribution (can overlap with #2)
5. #5 + #6 (1.5 days) = Days 8-9 testing
6. #8 + #10 (8 hr) = Day 10 documentation/tools
7. #11 + #12 + #13 (4 days) = Post-beta for P2 items

---

**Next Steps:**
1. Copy issues to GitHub
2. Assign P0 issues
3. Start with Issue #1 (metadata fix)
4. Track progress in project board
