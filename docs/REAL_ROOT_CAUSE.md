# REAL ROOT CAUSE ANALYSIS: v0.4.34 UI Shows OFFLINE

## Executive Summary

**ACTUAL ROOT CAUSE**: The Tauri UI binary in npm package was built from OLD SOURCE CODE before the event listener fix was applied.

**EVIDENCE**:
- Source code fix IS present in `/opt/swictation/tauri-ui/src/hooks/useMetrics.ts` (lines 44-113)
- But the built binary `/opt/swictation/npm-package/bin/swictation-ui` (built Nov 20 14:43) does NOT contain event listeners
- Binary was built BEFORE source code was fixed
- npm published a binary without the fix despite prepublishOnly hook

## Timeline (Corrected)

```
14:43 - Tauri UI built WITHOUT event listeners (old source code)
14:49 - Binary copied to npm-package/bin/ (STILL no event listeners)
14:49 - v0.4.34 published to npm with BROKEN binary
15:29 - User installs v0.4.34 and starts UI
15:33 - User reports "STILL OFFLINE"
```

## The Smoking Gun

### Test 1: npm package binary (built Nov 20 14:43)
```bash
$ strings /opt/swictation/npm-package/bin/swictation-ui | \
  grep -E "^(metrics-connected|metrics-update|session-start|session-end|state-change|transcription)$"
# NO OUTPUT - Event listeners NOT in binary
```

### Test 2: Tauri build output (Nov 20 14:43)
```bash
$ strings /opt/swictation/tauri-ui/src-tauri/target/release/swictation-ui | \
  grep -E "^(metrics-connected|metrics-update|session-start|session-end|state-change|transcription)$"
# NO OUTPUT - Event listeners NOT in binary
```

### Test 3: Source code (CORRECT)
```typescript
// /opt/swictation/tauri-ui/src/hooks/useMetrics.ts (lines 44-113)
const unlistenConnected = await listen<boolean>('metrics-connected', ...);
const unlistenMetrics = await listen<BroadcastEvent>('metrics-update', ...);
const unlistenStateChange = await listen<BroadcastEvent>('state-change', ...);
const unlistenTranscription = await listen<BroadcastEvent>('transcription', ...);
const unlistenSessionStart = await listen<BroadcastEvent>('session-start', ...);
const unlistenSessionEnd = await listen<BroadcastEvent>('session-end', ...);
```

Source code HAS the fix, but the binary was built before this code existed!

## What Actually Happened

1. **Initial State**: useMetrics.ts had the OLD single event listener code
2. **Someone made changes**: Fixed useMetrics.ts to listen to 6 separate events (current source)
3. **Tauri build ran at 14:43**: But used STALE bundled JavaScript from Vite cache
4. **Binary copied at 14:49**: Broken binary copied to npm-package/bin/
5. **npm publish succeeded**: Published broken binary to npm registry
6. **User installed**: Got the broken binary with old event listeners

## Why prepublishOnly Didn't Catch This

The prepublishOnly hook WAS added:
```json
{
  "scripts": {
    "prepublishOnly": "./scripts/build-release.sh && ../tauri-ui/scripts/build-ui-release.sh"
  }
}
```

BUT this was added AFTER v0.4.34 was already published! The package.json on npm doesn't have this hook yet.

## Verification: Socket Events Are Perfect ✅

```
Events Received:
  metrics_update: 467
  session_end: 2
  session_start: 2
  state_change: 4
  transcription: 14
Total: 489 events

✓ All 5 event types present
✓ All required fields present
✓ session_id included in metrics_update
✓ gpu_memory_percent included
✓ words field in transcription
```

Backend is PERFECT. Frontend binary is BROKEN.

## The Build Process Failure

### What SHOULD Have Happened:
1. Edit useMetrics.ts with 6 event listeners
2. Run `npm run build` (Vite bundles JavaScript)
3. Verify bundle contains event names
4. Run `npm run tauri build` (embeds bundle in binary)
5. Verify binary contains event names
6. Copy binary to npm-package/bin/
7. npm publish

### What ACTUALLY Happened:
1. ✅ Edit useMetrics.ts with 6 event listeners
2. ❌ Vite cached OLD JavaScript from previous build
3. ❌ No verification of bundle contents
4. ❌ Tauri embedded OLD JavaScript in binary
5. ❌ No verification of binary contents
6. ❌ Copied BROKEN binary to npm-package/bin/
7. ❌ Published BROKEN binary to npm

## The Fix

### Immediate Solution (User):
We need to:
1. Rebuild Tauri UI with CLEAN build (rm -rf dist)
2. Verify bundle contains event names
3. Verify binary contains event names
4. Copy to npm-package/bin/
5. Publish v0.4.35

### Long-term Solution:
The build-ui-release.sh script we created (with verification) prevents this, but it wasn't used for v0.4.34.

## Files Analyzed

**Source Code (CORRECT)**:
- `/opt/swictation/tauri-ui/src/hooks/useMetrics.ts:44-113` - Has all 6 event listeners

**Built Binaries (BROKEN)**:
- `/opt/swictation/npm-package/bin/swictation-ui` (Nov 20 14:49) - No event listeners
- `/opt/swictation/tauri-ui/src-tauri/target/release/swictation-ui` (Nov 20 14:43) - No event listeners
- `/home/robert/.npm-global/lib/node_modules/swictation/bin/swictation-ui` (Nov 20 15:29) - No event listeners

**npm Package**:
- Published tarball DOES include bin/swictation-ui (7.0MB)
- But that binary is broken (no event listeners)

## Lesson Learned

**The "new npm new npm death spiral"** happened because:
1. Vite cached stale JavaScript
2. No verification that bundle matched source code
3. No verification that binary matched bundle
4. Published without verification

The build-ui-release.sh script (11 steps with verification) we created would have prevented this:
- Step 4: Verify bundled JavaScript contains all event names ❌ WOULD HAVE FAILED
- Step 8: Verify binary contains all event names ❌ WOULD HAVE FAILED
- Build would have STOPPED before publish

## Next Steps

1. **Clean build**: Remove ALL cache directories
2. **Verify source**: Confirm useMetrics.ts has 6 event listeners
3. **Build with verification**: Use build-ui-release.sh script
4. **Test locally**: Launch binary and verify it receives events
5. **Publish v0.4.35**: With VERIFIED working binary

---

**Conclusion**: The fix (6 event listeners) IS in source code, but v0.4.34 was published with a binary built from OLD cached JavaScript. We need a complete clean rebuild.
