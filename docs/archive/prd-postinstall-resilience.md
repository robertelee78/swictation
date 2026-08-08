# PRD: Postinstall Resilience, Progress, and Error Messaging

**Author:** John (PM) + team input from Winston (Architect), Amelia (Dev), Quinn (QA)
**Date:** 2026-03-23
**Status:** Draft
**Scope:** `npm-package/postinstall.js` — the `npm install -g swictation` experience only

---

## Problem Statement

The postinstall script (`postinstall.js`, 3063 lines) orchestrates a complex installation: platform detection, GPU library download (~1.5GB), ONNX Runtime download, systemd/launchd service setup, and model testing. Today, this script has three classes of problems:

1. **Download fragility** — A single network hiccup during the ~1.5GB GPU library or ONNX Runtime download causes a full failure with no recovery. The `downloadFile()` function (line 634) has no retry logic, no resume support, and no progress indication. Users on slow or unreliable connections get a cryptic error and must re-run the entire install.

2. **Silent progress** — npm suppresses postinstall output unless `--foreground-scripts` is passed. Even when visible, the script provides only milestone checkpoints ("Downloading...", "Done") with no byte-level progress, no ETA, and no indication the process is alive during multi-minute downloads. Users think the install is hung.

3. **Unhelpful error messages** — When failures occur (missing Python/hf CLI, network timeout, permission denied, corrupted download), the error messages are either raw stack traces or generic warnings that don't tell the user what to do next. The 140 catch blocks vary wildly in quality.

---

## Target Users

- **Developers** using swictation as a dictation tool while coding (Linux x64, macOS ARM64)
- **General users** with Node.js installed who want voice-to-text (less technical, lower error-recovery tolerance)

## Constraints

- Postinstall must remain a single all-in-one flow (no separate `swictation setup` step)
- 5+ minute install time is acceptable with clear progress output
- Python/hf CLI is an accepted prerequisite for the npm channel
- Supported platforms: Ubuntu 24.04+ (glibc 2.39+), macOS 14.0+ Apple Silicon only

---

## Pillar 1: Download Resilience

### Current State

`downloadFile()` (line 634) is a bare `https.get()` piped to `fs.createWriteStream()` with single-level redirect following. No retry, no resume, no timeout, no progress callback.

```js
// Current: single-shot, no recovery
async function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => { ... });
  });
}
```

### Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| DR-1 | Retry failed downloads up to 3 times with exponential backoff (2s, 8s, 32s) | P0 |
| DR-2 | Support HTTP Range requests for resume after partial download (check `Accept-Ranges` header) | P0 |
| DR-3 | Validate `Content-Length` header before download; abort early if disk space insufficient | P1 |
| DR-4 | Set a per-request socket timeout (30s idle) to detect stalled connections, not just refused ones | P1 |
| DR-5 | On final retry failure, preserve the partial file and print a `curl` resume command the user can run manually | P1 |
| DR-6 | Skip download if file already exists and checksum matches (already partially implemented for GPU libs via `gpu-package-info.json` — extend to all downloads) | P1 |
| DR-7 | Follow up to 5 redirects (current: 1 level only, line 638-649) | P2 |

### Acceptance Criteria

- On a simulated network interruption mid-download, the script resumes from the last byte on retry without re-downloading
- After 3 failed retries, the user sees a clear message with manual recovery steps (not a stack trace)
- A fully cached/checksummed download completes in <2 seconds on re-install

---

## Pillar 2: Clear Progress Output

### Current State

Downloads show only start/end messages:
```
  Downloading modern-ampere package...
  URL: https://github.com/...
  ✓ Downloaded modern-ampere package (~1.5GB)
```

No byte count, no percentage, no speed, no ETA. During the 2-10 minute download, the terminal is silent.

### Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| PO-1 | Display a progress bar during all downloads >1MB showing: bytes downloaded / total, percentage, transfer speed, ETA | P0 |
| PO-2 | Update progress in-place (carriage return `\r`) at most once per second to avoid log spam | P0 |
| PO-3 | If `Content-Length` is unavailable, show bytes downloaded + speed (no percentage/ETA) | P1 |
| PO-4 | Display a phase banner at each major install stage with step number: `[1/6] Checking platform...`, `[2/6] Downloading GPU libraries...`, etc. | P0 |
| PO-5 | At install start, print a time estimate based on platform: "This installation downloads ~1.5GB of GPU libraries and may take 5-10 minutes" | P1 |
| PO-6 | On retry, clearly indicate: `Retry 2/3 — resuming download from 847MB...` | P1 |
| PO-7 | Detect non-TTY (piped output / CI) and fall back to periodic line-based progress (every 10%) instead of `\r` overwrite | P2 |

### Acceptance Criteria

- A user watching the terminal always knows: what step they're on, that the process is alive, and roughly how long remains
- CI logs (non-TTY) show 10-line progress summaries, not thousands of `\r`-overwritten lines
- No external dependencies added (no `cli-progress`, `ora`, etc. — use built-in Node.js)

---

## Pillar 3: Graceful and Useful Error Messages

### Current State

The 140 catch blocks produce inconsistent error output. Some examples:

- **Good** (line 947): Falls back to CPU mode, prints manual curl command
- **Bad** (line 805): `"This should not happen as platform package was verified earlier"` — unhelpful
- **Ugly**: Raw `err.message` from Node.js internals like `"ECONNRESET"` or `"ETIMEDOUT"`

No standardized error format. No error codes. No "what to do next" on most failures.

### Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| EM-1 | Define a standard error output format for all user-facing errors: `[FAIL] <what happened>` / `  Cause: <why>` / `  Fix: <what to do>` | P0 |
| EM-2 | Map common Node.js/system errors to human-readable messages: ECONNRESET ("Network connection was interrupted"), ETIMEDOUT ("Server did not respond within 30s"), ENOSPC ("Not enough disk space"), EACCES ("Permission denied — try running with sudo or fix directory permissions") | P0 |
| EM-3 | When Python/hf CLI is missing, print exact install command: `pip install huggingface-hub` or `brew install huggingface-cli` depending on platform | P0 |
| EM-4 | When a download fails after all retries, print the manual download URL + checksum so the user can download with their own tool and re-run install | P0 |
| EM-5 | Assign each error class a short error code (e.g., `SW-E001`) and print a URL `https://github.com/robertelee78/swictation/wiki/errors#SW-E001` for detailed troubleshooting | P1 |
| EM-6 | At the end of postinstall, print a summary: number of warnings/errors, and if any non-fatal issues occurred, list them with fix suggestions | P1 |
| EM-7 | Never print raw stack traces to the user. Log them to `~/.local/share/swictation/install.log` for debugging and tell the user where the log is | P1 |
| EM-8 | On permission errors for service setup (systemd/launchd), suggest the specific fix rather than generic "try sudo" | P2 |

### Error Message Template

```
[FAIL] GPU library download failed (SW-E003)
  Cause: Network connection was interrupted after downloading 847MB of 1.5GB
  Fix:   Re-run installation: npm install -g swictation
         The download will resume from where it left off.

  Manual alternative:
    curl -L -C - -o /tmp/cuda-libs-modern-ampere.tar.gz \
      https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-modern-ampere.tar.gz
    Then re-run: npm install -g swictation

  Full log: ~/.local/share/swictation/install.log
  Help: https://github.com/robertelee78/swictation/wiki/errors#SW-E003
```

### Acceptance Criteria

- Every catch block produces output matching the standard format (FAIL/Cause/Fix)
- No raw `err.message` or stack trace is shown to the user
- A user who hits any error knows exactly what to try next without Googling

---

## Implementation Notes

### File to modify

`npm-package/postinstall.js` (3063 lines)

### Suggested approach

1. **Extract a `downloadWithRetry()` utility** replacing `downloadFile()` — handles retry, resume, progress, and error mapping in one place
2. **Create an `InstallError` class** with code, cause, fix, and helpUrl properties — all catch blocks instantiate this instead of raw strings
3. **Add a `ProgressReporter` helper** that detects TTY vs pipe and renders accordingly
4. **Add phase tracking** — a simple counter that prefixes output with `[N/M]`
5. **Add install log** — tee all output to `~/.local/share/swictation/install.log`

### What NOT to change

- Package resolution architecture (optionalDependencies pattern)
- Service setup logic (systemd/launchd generation)
- Platform detection logic (glibc/macOS version checks)
- Checksum verification (already solid)
- GPU detection and variant selection

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Install success rate on supported platforms | Unknown (no telemetry) | >95% on happy path |
| User-actionable error messages | ~30% of catch blocks | 100% |
| Download recovery after network interruption | 0% (full restart) | Resume from last byte |
| User confusion during download (silent terminal) | High | Zero — progress always visible |

---

## Out of Scope

- Daemon runtime errors (post-install)
- Model download failures (hf CLI responsibility)
- macOS App Store distribution
- Ubuntu 22.04 / Intel Mac support
- Adding external npm dependencies to postinstall
