# 🚨 CRITICAL: Tauri App Integration Fix Required

## The Problem (One-Line Summary)
**Tauri app looks for socket at `/tmp/swictation_metrics.sock` but daemon listens at `/run/user/1000/swictation_metrics.sock`**

---

## Why The App Shows No Data
1. Tauri tries to connect to `/tmp/swictation_metrics.sock` ❌
2. Connection fails (daemon not there)
3. `connected = false` in UI
4. No metrics updates received
5. All tabs show empty states

---

## The Fix (Change 1 Function)

### File to Edit
**Absolute Path:** `/opt/swictation/tauri-ui/src-tauri/src/utils/mod.rs`

### Current Code (WRONG)
```rust
/// Get the default socket path
pub fn get_default_socket_path() -> String {
    "/tmp/swictation_metrics.sock".to_string()  // ❌ HARDCODED!
}
```

### Fixed Code (Option 1 - RECOMMENDED)
```rust
use crate::socket::socket_utils;

/// Get the default socket path
pub fn get_default_socket_path() -> String {
    socket_utils::get_metrics_socket_path()
        .to_str()
        .expect("Invalid socket path")
        .to_string()
}
```

### Fixed Code (Option 2 - Inline XDG Logic)
```rust
/// Get the default socket path
pub fn get_default_socket_path() -> String {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        format!("{}/swictation_metrics.sock", runtime_dir)
    } else {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".local/share/swictation/swictation_metrics.sock")
            .to_str()
            .expect("Invalid path")
            .to_string()
    }
}
```

---

## Verification Steps

### 1. Before Fix
```bash
# Check where daemon is listening
$ lsof -U -a -c swictation | grep metrics.sock
swictatio ... /run/user/1000/swictation_metrics.sock type=STREAM (LISTEN)

# Check what Tauri is trying to connect to
$ grep -n "get_default_socket_path" /opt/swictation/tauri-ui/src-tauri/src/utils/mod.rs
14:pub fn get_default_socket_path() -> String {
15:    "/tmp/swictation_metrics.sock".to_string()  # ❌ WRONG!

# Try connecting to current Tauri path
$ nc -U /tmp/swictation_metrics.sock
# Result: Connection refused ❌

# Try connecting to daemon's actual path
$ nc -U /run/user/1000/swictation_metrics.sock
# Result: Live metrics streaming! ✅
```

### 2. After Fix
```bash
# Rebuild Tauri app
$ cd /opt/swictation/tauri-ui/src-tauri
$ cargo build --release

# Install updated app
$ cd /opt/swictation
$ npm run build

# Launch app
$ swictation-ui

# Expected: "● LIVE" indicator in top-right
# Expected: All tabs populate with data
```

### 3. Quick Test Without Rebuild
```bash
# Test socket path logic in Rust
$ cd /opt/swictation/tauri-ui/src-tauri
$ cargo test test_socket_paths -- --nocapture
```

---

## Critical Files Reference

### ✅ Working Correctly
- **Daemon Socket Utils:** `/opt/swictation/rust-crates/swictation-daemon/src/socket_utils.rs`
  - Creates socket at: `/run/user/1000/swictation_metrics.sock` ✅

- **Tauri Socket Utils:** `/opt/swictation/tauri-ui/src-tauri/src/socket/socket_utils.rs`
  - Has correct XDG logic but NOT USED ⚠️

- **Database:** `~/.local/share/swictation/metrics.db`
  - Contains 192KB of data ✅

### ❌ Needs Fixing
- **Tauri Utils:** `/opt/swictation/tauri-ui/src-tauri/src/utils/mod.rs`
  - Line 14-16: Hardcoded `/tmp/` path ❌

### 📋 Related Files
- **Main Entry:** `/opt/swictation/tauri-ui/src-tauri/src/main.rs`
  - Line 115: Calls `utils::get_default_socket_path()`

- **Socket Connection:** `/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs`
  - Handles async socket connection and event parsing

- **Frontend Hooks:** `/opt/swictation/tauri-ui/src/hooks/useMetrics.ts`
  - Listens for 'metrics-event' from backend

- **Commands:** `/opt/swictation/tauri-ui/src-tauri/src/commands/mod.rs`
  - Line 87: Returns hardcoded path in get_connection_status

---

## Why This Happened

The codebase has TWO socket path utilities:
1. **Correct one in socket/ directory** - uses XDG_RUNTIME_DIR ✅
2. **Wrong one in utils/ directory** - hardcoded to /tmp/ ❌

`main.rs` calls the wrong one (#2), probably from copy-paste or incomplete refactoring.

---

## Evidence of Working Daemon

```bash
# Daemon is RUNNING and WORKING
$ ps aux | grep swictation-daemon
robert 1842855 ... /home/robert/.npm-global/lib/node_modules/swictation/lib/native/swictation-daemon.bin

# Socket is LISTENING
$ lsof -U -a -c swictation
swictatio ... /run/user/1000/swictation_metrics.sock type=STREAM (LISTEN)

# Transcription is WORKING
$ nc -U /run/user/1000/swictation_metrics.sock
{"type":"transcription","text":"...","wpm":150.0,"latency_ms":129.01,"words":12}
{"type":"metrics_update","state":"recording","session_id":391,...}
```

---

## Additional Cleanup (Optional)

Remove stale socket files from old daemon version:
```bash
$ rm /tmp/swictation.sock /tmp/swictation_metrics.sock
```

---

## Impact After Fix

- ✅ Live Session tab shows real-time metrics
- ✅ History tab populates with sessions
- ✅ Transcriptions tab displays live transcriptions
- ✅ Connection indicator shows "● LIVE"
- ✅ Toggle recording button works
- ✅ All database queries succeed

---

## Questions?

**Q: Why does the app launch successfully?**
A: Everything else works! Database, UI, IPC - all fine. Only socket connection fails.

**Q: Why don't we see error logs?**
A: The socket connection gracefully fails and retries silently. `connected = false` but no crash.

**Q: Can we just create a symlink?**
A: Bad idea. The real issue is the hardcoded path. Fix the code, not the filesystem.

**Q: Will this break on other systems?**
A: No! XDG_RUNTIME_DIR is the CORRECT standard. `/tmp/` was the bug.

---

**Priority:** 🔥 CRITICAL
**Difficulty:** ✅ TRIVIAL (1 line change)
**Testing:** ⏱️ 5 minutes
**Impact:** 💯 Total (fixes all UI features)

---

**Test Report:** `/opt/swictation/tests/integration-test-report.md`
**Swarm Coordination:** Memory key `swarm/tester/socket-path-mismatch`
