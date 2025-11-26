# Version Management

## Overview

The `versions.json` file is the **single source of truth** for all version numbers in the Swictation project. This document explains the versioning strategy and how to manage versions correctly.

## Version Strategy

### Distribution Version (User-Facing)

The `distribution` version is the version users see when they install Swictation:

```bash
npm install -g swictation@0.7.9
```

**Critical Rule:** The following packages **MUST always have the same version**:
- `swictation` (main package)
- `@swictation/linux-x64` (Linux binaries)
- `@swictation/darwin-arm64` (macOS binaries)

### Component Versions (Independent)

Component versions can evolve independently from the distribution version:

- **daemon** (`swictation-daemon`): Core voice recognition daemon
- **ui** (`swictation-ui`): Tauri-based UI application
- **context-learning**: Context-aware meta-learning module

Example: Distribution 0.7.9 might contain daemon 0.7.5 and UI 0.1.0.

### Library Versions (External)

Third-party library versions track upstream releases:

- **ONNX Runtime**: Different versions for different platforms
  - Linux CPU: 1.22.0
  - Linux GPU: 1.23.2 (CUDA 12.9)
  - macOS CoreML: 1.22.0 (1.23.x has regression)
- **CUDA**: 12.9 (modern GPUs) or 11.8 (legacy GPUs)
- **cuDNN**: 9.15.1

## File Structure

```json
{
  "distribution": "0.7.9",
  "components": {
    "daemon": {
      "version": "0.7.5",
      "source": "rust-crates/swictation-daemon/Cargo.toml",
      "description": "Main daemon binary"
    }
  },
  "libraries": {
    "onnxruntime": {
      "linux-cpu": "1.22.0",
      "linux-gpu": "1.23.2",
      "macos-coreml": "1.22.0"
    }
  },
  "metadata": {
    "last_updated": "2025-11-26T18:30:00.000Z",
    "schema_version": "1.0.0"
  }
}
```

## Version Bumping Workflow

### 1. Decide What to Bump

**Bump distribution version when:**
- Any user-facing change (new features, bug fixes)
- Component versions change
- Library versions change

**Don't bump distribution version when:**
- Internal refactoring only
- Documentation changes only

### 2. Run the Bump Script

```bash
# Bump patch version (0.7.10 → 0.7.11)
npm run version:bump patch

# Bump minor version (0.7.10 → 0.8.0)
npm run version:bump minor

# Bump major version (0.7.10 → 1.0.0)
npm run version:bump major

# Dry run (show what would change without modifying files)
npm run version:bump patch -- --dry-run

# Force extract component versions from Cargo.toml files
npm run version:bump patch -- --force-extract

# Verbose mode with component version details
npm run version:bump patch -- --verbose
```

### 3. Synchronize All Packages

The bump script automatically runs `sync-versions.js` which:
- Updates `npm-package/package.json`
- Updates `npm-package/packages/linux-x64/package.json`
- Updates `npm-package/packages/darwin-arm64/package.json`
- Updates optionalDependencies in main package.json

### 4. Verify Synchronization

Before publishing, always verify that all package versions are synchronized:

```bash
# Verify all packages have matching versions
npm run version:verify

# Verbose mode with detailed checking
npm run version:verify:verbose

# Allow missing platform packages (for initial development)
npm run version:verify:allow-missing
```

The verification script checks:
- Main package.json version matches distribution version in versions.json
- optionalDependencies versions match distribution version
- Platform packages (@swictation/linux-x64, @swictation/darwin-arm64) match distribution version

Exit codes:
- **0**: All versions synchronized (safe to publish)
- **1**: Versions out of sync or validation failed

This check should be run:
- Before publishing to npm
- After running version:bump
- After manually editing versions
- As part of CI/CD pre-publish checks

## Extracting Component Versions

Component versions are automatically extracted from Cargo.toml files using the `bump-version.js` script with the `--force-extract` flag:

```bash
# Extract all component versions and update versions.json
npm run version:bump patch -- --force-extract

# This will read and update:
#   - daemon: rust-crates/swictation-daemon/Cargo.toml
#   - ui: tauri-ui/src-tauri/Cargo.toml
#   - context-learning: rust-crates/swictation-context-learning/Cargo.toml
```

Manual extraction (for verification):

```bash
# Extract daemon version
grep '^version' rust-crates/swictation-daemon/Cargo.toml

# Extract UI version
grep '^version' tauri-ui/src-tauri/Cargo.toml

# Extract context-learning version
grep '^version' rust-crates/swictation-context-learning/Cargo.toml
```

The extraction process validates semver format and gracefully handles missing or malformed version strings.

## Version Display

Users can see version information with:

```bash
$ swictation --version
swictation v0.7.9 (linux-x64)
  daemon: v0.7.5
  ui: v0.1.0
  onnx-runtime: v1.23.2
```

## Platform Package Metadata

Each platform package includes build metadata:

```json
{
  "name": "@swictation/linux-x64",
  "version": "0.7.9",
  "metadata": {
    "distribution": "0.7.9",
    "daemon": "0.7.5",
    "ui": "0.1.0",
    "onnxruntime": "1.23.2",
    "platform": "linux-x64",
    "buildDate": "2025-01-26T10:11:00Z"
  }
}
```

## Schema Validation

The `versions.json` file is validated against `versions.schema.json`:

```bash
# Validate manually
npx ajv-cli validate -s versions.schema.json -d versions.json
```

The schema enforces:
- Semantic versioning format (X.Y.Z)
- Required fields
- Valid ISO 8601 timestamps

## Best Practices

### ✅ DO

- Always bump distribution version for user-facing changes
- Run `npm run version:verify` before publishing
- Keep component versions in Cargo.toml as source of truth
- Document breaking changes in CHANGELOG.md
- Use semantic versioning correctly:
  - **MAJOR**: Breaking changes
  - **MINOR**: New features (backwards compatible)
  - **PATCH**: Bug fixes (backwards compatible)

### ❌ DON'T

- Manually edit package.json versions (use scripts)
- Publish packages with out-of-sync versions
- Skip version verification step
- Forget to update CHANGELOG.md
- Use non-semver version numbers

## Troubleshooting

### Versions Out of Sync

If `version:verify` fails:

```bash
# Re-run synchronization
npm run version:sync

# Verify again
npm run version:verify
```

### Component Version Mismatch

If component versions don't match Cargo.toml:

```bash
# Force re-extract from Cargo.toml
npm run version:bump patch --force-extract
```

### Build Date Not Updating

Build scripts automatically update metadata.last_updated. If it's stale:

```bash
# Manually update
node -e "const v = require('./versions.json'); v.metadata.last_updated = new Date().toISOString(); require('fs').writeFileSync('./versions.json', JSON.stringify(v, null, 2));"
```

## Related Files

- `npm-package/versions.json` - Single source of truth
- `npm-package/versions.schema.json` - JSON schema
- `npm-package/scripts/bump-version.js` - Version bumping
- `npm-package/scripts/sync-versions.js` - Synchronization
- `npm-package/scripts/verify-versions.js` - Validation
- `npm-package/CHANGELOG.md` - User-facing changes

## References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [npm version documentation](https://docs.npmjs.com/cli/v8/commands/npm-version)
- [JSON Schema](https://json-schema.org/)
