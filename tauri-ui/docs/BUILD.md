# Swictation Tauri UI Build Process

This document describes the proper build process for the Swictation Tauri UI, including why clean builds are required and how to avoid common issues.

## Why Clean Builds Are Required

### Vite Caching Issue

**Problem:** Vite (the frontend bundler) sometimes caches build artifacts and doesn't pick up source code changes, even when TypeScript compilation succeeds. This results in:
- Old code being bundled into the binary
- Event listeners not matching backend emissions
- UI not working despite successful build

**Solution:** Always perform clean builds that:
1. Remove the `dist/` directory
2. Remove the `src-tauri/target/release/bundle/` directory
3. Verify bundled JavaScript contains expected code
4. Verify binary contains expected code

## Proper Build Sequence

### Automated Build (Recommended)

Use the automated build script that includes verification:

```bash
cd /opt/swictation/tauri-ui
./scripts/build-ui-release.sh
```

This script:
1. Cleans build directories
2. Runs TypeScript compilation check
3. Builds frontend with Vite
4. **Verifies bundle** contains all required event names
5. Validates bundle size
6. Builds Tauri binary (Rust + packaging)
7. Validates binary size
8. **Verifies binary** contains all required event names
9. Copies binary to npm package
10. Creates SHA256 checksums
11. Generates build manifest with git commit, checksums, sizes

### Manual Build Steps

If you need to build manually:

```bash
cd /opt/swictation/tauri-ui

# 1. Clean
rm -rf dist
rm -rf src-tauri/target/release/bundle

# 2. TypeScript check
npx tsc --noEmit

# 3. Frontend build
npm run build

# 4. Verify bundle (CRITICAL!)
grep -q '"metrics-connected"' dist/assets/index-*.js || echo "Missing event!"
grep -q '"metrics-update"' dist/assets/index-*.js || echo "Missing event!"
grep -q '"session-start"' dist/assets/index-*.js || echo "Missing event!"
grep -q '"session-end"' dist/assets/index-*.js || echo "Missing event!"
grep -q '"state-change"' dist/assets/index-*.js || echo "Missing event!"
grep -q '"transcription"' dist/assets/index-*.js || echo "Missing event!"

# 5. Tauri build
npm run tauri build

# 6. Verify binary (CRITICAL!)
strings src-tauri/target/release/swictation-ui | grep -E "metrics-connected|metrics-update|session-start|session-end|state-change|transcription"

# 7. Copy to npm package
cp src-tauri/target/release/swictation-ui ../npm-package/bin/
```

## Required Event Names

The frontend MUST listen to these 6 events that the backend emits:

1. **metrics-connected** - Connection status
2. **metrics-update** - Real-time metrics
3. **session-start** - Recording session started
4. **session-end** - Recording session ended
5. **state-change** - Daemon state changes
6. **transcription** - New transcription events

### How to Verify

**Bundle verification:**
```bash
grep -o '"[a-z-]*"' dist/assets/index-*.js | grep -E "(metrics|session|state|transcription)" | sort -u
```

**Binary verification:**
```bash
strings src-tauri/target/release/swictation-ui | grep -E "metrics-connected|metrics-update|session-start|session-end|state-change|transcription"
```

Both should show all 6 event names.

## Common Build Issues

### Issue 1: UI Shows OFFLINE

**Symptoms:**
- UI shows "OFFLINE" status
- No metrics update when toggling recording
- Binary seems to work but UI doesn't

**Cause:** Vite bundled old JavaScript without the event listeners

**Solution:**
```bash
# Clean rebuild
rm -rf dist && npm run build
# Verify bundle contains event names
grep '"metrics-connected"' dist/assets/index-*.js
# If missing, check source code in src/hooks/useMetrics.ts
```

### Issue 2: Build Succeeds But Old Code Runs

**Symptoms:**
- TypeScript compilation passes
- Build completes successfully
- But behavior doesn't match source code changes

**Cause:** Vite cache issue

**Solution:**
1. Always use the automated build script
2. If manual build, ALWAYS verify bundle content
3. Check `/tmp/tauri-ui-build-manifest.json` for checksums

### Issue 3: npm Publish Contains Wrong UI

**Symptoms:**
- Published to npm successfully
- User installs latest version
- UI still has old behavior

**Cause:** `prepublishOnly` hook didn't run or verification failed silently

**Solution:**
```bash
# The prepublishOnly hook now verifies builds automatically
# Check package.json:
cat npm-package/package.json | grep prepublishOnly
# Should show: "./scripts/build-release.sh && ../tauri-ui/scripts/build-ui-release.sh"
```

## What Gets Bundled

When running `npm publish` from `npm-package/`:

1. **Daemon binary**: `bin/swictation-daemon` (from Rust workspace)
2. **UI binary**: `bin/swictation-ui` (from Tauri build)
3. **Native libs**: `lib/native/libonnxruntime.so`, etc.
4. **Scripts**: postinstall.js, setup scripts
5. **Docs**: README.md, templates

The UI binary is a **self-contained executable** that includes:
- Rust backend (src-tauri/src/)
- Embedded frontend (dist/ → bundled into binary)
- SQLite database
- Tauri runtime

## npm prepublishOnly Hook

The `prepublishOnly` hook in `package.json` ensures quality builds:

```json
{
  "scripts": {
    "prepublishOnly": "./scripts/build-release.sh && ../tauri-ui/scripts/build-ui-release.sh"
  }
}
```

This means:
1. **BOTH** daemon and UI are built fresh
2. **ALL** verification happens before publish
3. **FAIL-FAST** if any verification fails
4. **Cannot publish** broken builds

## Build Manifest

After each build, check `/tmp/tauri-ui-build-manifest.json`:

```json
{
  "build": {
    "timestamp": "2025-11-20T22:30:00Z",
    "git_commit": "d6d1476...",
    "git_branch": "main"
  },
  "verification": {
    "typescript_check": "passed",
    "event_names_verified": true,
    "required_events": [...]
  },
  "artifacts": {
    "bundle_js": {
      "path": "dist/assets/index-abc123.js",
      "size_mb": 1,
      "checksum": "sha256:..."
    },
    "binary": {
      "path": "src-tauri/target/release/swictation-ui",
      "size_mb": 7,
      "checksum": "sha256:..."
    }
  }
}
```

Use this to:
- Debug build issues
- Verify checksums match
- Track which git commit was built

## Development vs. Release Builds

### Development Build

```bash
npm run tauri dev
```

- Hot reload enabled
- Debug symbols included
- Not optimized
- Uses development frontend server

### Release Build

```bash
./scripts/build-ui-release.sh
```

- Fully optimized
- No debug symbols
- Minified frontend
- Embedded frontend in binary
- All verification steps

**Never publish development builds to npm!**

## Troubleshooting

### Build Fails at TypeScript Check

```bash
npx tsc --noEmit
# Fix errors shown
```

### Build Fails at Bundle Verification

```bash
# Check which events are missing
grep -o '"[a-z-]*"' dist/assets/index-*.js | grep -E "(metrics|session|state|transcription)"

# Check source code
cat src/hooks/useMetrics.ts | grep -A5 "listen<"
```

### Build Manifest Not Generated

```bash
# Check build script output
./scripts/build-ui-release.sh 2>&1 | tee /tmp/build-debug.log

# Look for "Step 11" output
```

## CI/CD Integration

For automated builds:

```yaml
# .github/workflows/release.yml (example)
steps:
  - name: Build Tauri UI
    run: cd tauri-ui && ./scripts/build-ui-release.sh

  - name: Verify Build Manifest
    run: |
      test -f /tmp/tauri-ui-build-manifest.json
      jq '.verification.event_names_verified == true' /tmp/tauri-ui-build-manifest.json
```

## Summary

**Golden Rule**: Always use `./scripts/build-ui-release.sh` for release builds.

This ensures:
- ✅ Clean build directories
- ✅ TypeScript compilation passes
- ✅ Bundle contains all required code
- ✅ Binary contains all required code
- ✅ Checksums generated
- ✅ Build manifest created
- ✅ Ready for npm publish

**Never skip verification steps!** They prevent the "new npm new npm death spiral" of publishing broken builds.
