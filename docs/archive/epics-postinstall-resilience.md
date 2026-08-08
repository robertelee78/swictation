# Epics & Stories: Postinstall Resilience, Progress, and Error Messaging

**PRD:** `docs/prd-postinstall-resilience.md`
**Date:** 2026-03-23
**Target file:** `npm-package/postinstall.js`

---

## Epic 1: Download Resilience

**Goal:** Replace the fragile single-shot `downloadFile()` with a robust `downloadWithRetry()` that handles retry, resume, and validation.

**Dependency:** None (foundational — Epic 2 and 3 build on this)

### Story 1.1: Extract downloadWithRetry utility

**File:** `npm-package/postinstall.js` (replace `downloadFile()` at line 634)

**Description:**
Create a new `downloadWithRetry(url, dest, options)` function that wraps the existing download logic with retry and resume capabilities. Replace all call sites of `downloadFile()` with this new function.

**Acceptance Criteria:**
- [ ] New function signature: `downloadWithRetry(url, dest, { maxRetries: 3, backoff: [2000, 8000, 32000], onProgress: null })`
- [ ] Follows up to 5 HTTP redirects (current limit is 1 at line 638-649)
- [ ] On network error or HTTP 5xx, retries up to `maxRetries` times with exponential backoff
- [ ] Between retries, checks if partial file exists and sends `Range` header to resume
- [ ] Sets 30-second socket idle timeout via `request.setTimeout()`
- [ ] On success, returns `{ bytesDownloaded, resumed, retries }` metadata
- [ ] On final failure, throws an error with the partial file path preserved (not deleted)
- [ ] All existing call sites (`downloadFile` at lines ~886, ~1000+) updated to use new function
- [ ] No external dependencies added — uses Node.js built-in `https` and `fs`

**Technical notes:**
- Check `Accept-Ranges: bytes` header on first request; if server doesn't support ranges, full retry instead of resume
- Use `If-Range` header with `ETag` or `Last-Modified` from first response to avoid stale resume
- Partial file kept at `dest.partial` during download, renamed to `dest` on completion + checksum pass

---

### Story 1.2: Pre-download disk space validation

**File:** `npm-package/postinstall.js`

**Description:**
Before starting any large download, check available disk space against `Content-Length` and abort early with a clear message if insufficient.

**Acceptance Criteria:**
- [ ] Before download starts, send a HEAD request to get `Content-Length`
- [ ] Check available disk space at the destination directory using `fs.statfsSync()` (Node 18.15+) or `df` fallback
- [ ] If available space < `Content-Length` * 1.1 (10% buffer), abort with error: disk path, space available, space needed
- [ ] Works on both Linux and macOS

---

### Story 1.3: Skip download on checksum match

**File:** `npm-package/postinstall.js`

**Description:**
Extend the existing GPU libs cache check (line 862-880) to all downloads. Before downloading, check if the destination file exists and its checksum matches the expected value. If so, skip the download entirely.

**Acceptance Criteria:**
- [ ] `downloadWithRetry` accepts an optional `expectedChecksum` parameter
- [ ] If dest file exists and checksum matches, returns immediately with `{ skipped: true }`
- [ ] Log message: `[skip] File already downloaded and verified (SHA256 match)`
- [ ] Works for GPU libs, ONNX Runtime CoreML, and any future downloads
- [ ] Re-install after successful install completes in <5 seconds (no re-download)

---

## Epic 2: Clear Progress Output

**Goal:** Users always know what's happening, what step they're on, and how long is left.

**Dependency:** Epic 1 (Story 1.1 provides the `onProgress` callback hook)

### Story 2.1: Progress bar for downloads

**File:** `npm-package/postinstall.js`

**Description:**
Implement an `onProgress` callback for `downloadWithRetry` that renders a progress bar during downloads larger than 1MB.

**Acceptance Criteria:**
- [ ] Progress display format: `  [=========>          ] 847MB / 1.5GB  56%  12.3MB/s  ETA 53s`
- [ ] Updates in-place using `\r` (carriage return), at most once per second
- [ ] If `Content-Length` is unknown: `  847MB downloaded  12.3MB/s` (no bar, no percentage)
- [ ] On retry/resume, shows: `  Retry 2/3 — resuming from 847MB...`
- [ ] No external dependencies — pure `process.stdout.write()`
- [ ] Detects non-TTY (`!process.stdout.isTTY`) and falls back to line-based output every 10%: `  Progress: 10% (150MB / 1.5GB)`

---

### Story 2.2: Phase banners and install summary

**File:** `npm-package/postinstall.js`

**Description:**
Add numbered phase banners at each major install stage and a time estimate at the start.

**Acceptance Criteria:**
- [ ] At install start, print: `Installing swictation v0.7.29 for <platform> <arch>` and `This may take 5-10 minutes (downloads ~1.5GB of libraries)`
- [ ] Each major phase prefixed with step counter: `[1/6] Checking platform compatibility...`, `[2/6] Cleaning up previous installations...`, `[3/6] Downloading GPU libraries...`, `[4/6] Configuring system services...`, `[5/6] Downloading speech models...`, `[6/6] Verifying installation...`
- [ ] Phase numbers are dynamic — if GPU is not detected, GPU download phase is skipped and numbers adjust
- [ ] At install end, print a summary box:
  ```
  ✓ swictation v0.7.29 installed successfully
    Platform: Ubuntu 24.04 (x64, NVIDIA RTX 4090)
    Daemon:   ~/.npm-global/lib/node_modules/swictation/bin/swictation-daemon
    Service:  systemd user service (running)
    Duration: 4m 32s
  ```
- [ ] If any warnings occurred during install, list them in the summary with fix suggestions

---

## Epic 3: Graceful and Useful Error Messages

**Goal:** Every error tells the user what happened, why, and what to do next. No raw stack traces.

**Dependency:** Can start in parallel with Epic 2, but should land after Epic 1 (error class used in downloadWithRetry)

### Story 3.1: InstallError class and error code registry

**File:** `npm-package/postinstall.js` (or extract to `npm-package/src/install-error.js`)

**Description:**
Create a standardized error class for all user-facing installation errors, with error codes and help URLs.

**Acceptance Criteria:**
- [ ] `InstallError` class with properties: `code` (string, e.g. "SW-E003"), `cause` (human-readable string), `fix` (actionable string), `helpUrl` (string)
- [ ] Error code registry as a constant map covering all known failure modes:
  - `SW-E001`: Unsupported platform/architecture
  - `SW-E002`: Insufficient disk space
  - `SW-E003`: Download failed (network)
  - `SW-E004`: Checksum verification failed
  - `SW-E005`: GPU detection failed
  - `SW-E006`: Service setup failed (permission)
  - `SW-E007`: Python/hf CLI not found
  - `SW-E008`: Model download failed
  - `SW-E009`: Binary not found in platform package
  - `SW-E010`: ONNX Runtime load failed
- [ ] Each code maps to a help URL: `https://github.com/robertelee78/swictation/wiki/errors#<code>`
- [ ] `InstallError.format()` method returns the standard output block:
  ```
  [FAIL] <message> (<code>)
    Cause: <cause>
    Fix:   <fix>
    Help:  <helpUrl>
  ```

---

### Story 3.2: Error message mapping for system errors

**File:** `npm-package/postinstall.js`

**Description:**
Map common Node.js and system error codes to human-readable messages with platform-specific fix suggestions.

**Acceptance Criteria:**
- [ ] Map of errno codes to human messages:
  - `ECONNRESET` → "Network connection was interrupted"
  - `ETIMEDOUT` → "Server did not respond within 30 seconds"
  - `ECONNREFUSED` → "Could not reach download server — check your internet connection"
  - `ENOSPC` → "Not enough disk space at <path> (need <X>GB, have <Y>GB)"
  - `EACCES` → "Permission denied writing to <path>"
  - `ENOENT` → "Required file not found: <path>"
- [ ] On `EACCES` for service setup: suggest specific fix per platform
  - Linux: "Run: sudo loginctl enable-linger $USER" or "Check ~/.config/systemd/user/ permissions"
  - macOS: "Check ~/Library/LaunchAgents/ permissions"
- [ ] When Python/hf not found:
  - Linux: "Install with: pip install huggingface-hub"
  - macOS: "Install with: brew install huggingface-cli or pip install huggingface-hub"

---

### Story 3.3: Install log file and catch block cleanup

**File:** `npm-package/postinstall.js`

**Description:**
Route all output (including raw errors and stack traces) to a log file. Clean up all ~140 catch blocks to use `InstallError` format.

**Acceptance Criteria:**
- [ ] All console output also written to `~/.local/share/swictation/install.log` (create dir if needed)
- [ ] Log file includes timestamps and full stack traces (for debugging)
- [ ] User-facing output shows only the formatted `InstallError` message (no stack traces)
- [ ] On any error, print: `Full log: ~/.local/share/swictation/install.log`
- [ ] Audit all catch blocks (~140) and convert to use `InstallError` with appropriate error code
- [ ] No catch block prints raw `err.message` or `err.stack` to stdout
- [ ] Non-fatal warnings collected during install and printed in summary (Story 2.2)

---

## Story Dependency Graph

```
Epic 1 (Download Resilience)
  Story 1.1: downloadWithRetry ──────────┐
  Story 1.2: Disk space check            │ (independent, parallel)
  Story 1.3: Skip on checksum match      │ (depends on 1.1)
                                         │
Epic 2 (Progress Output)                 │
  Story 2.1: Progress bar ───────────────┤ (depends on 1.1 onProgress callback)
  Story 2.2: Phase banners + summary     │ (independent)
                                         │
Epic 3 (Error Messages)                  │
  Story 3.1: InstallError class ─────────┤ (independent, start early)
  Story 3.2: Error message mapping       │ (depends on 3.1)
  Story 3.3: Log file + catch cleanup ───┘ (depends on 3.1, do last)
```

## Recommended Execution Order

1. **Story 3.1** — InstallError class (no dependencies, used everywhere)
2. **Story 1.1** — downloadWithRetry (foundational)
3. **Story 1.2** — Disk space check (parallel with 1.1)
4. **Story 1.3** — Skip on checksum match (after 1.1)
5. **Story 2.1** — Progress bar (after 1.1)
6. **Story 2.2** — Phase banners + summary (independent, any time)
7. **Story 3.2** — Error message mapping (after 3.1)
8. **Story 3.3** — Log file + catch cleanup (last — touches all catch blocks)
