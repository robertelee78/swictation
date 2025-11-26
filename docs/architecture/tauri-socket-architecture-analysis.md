# Tauri UI Socket Architecture Analysis

**Date:** 2025-11-26
**Component:** `/opt/swictation/tauri-ui/src-tauri/src/socket/`
**Purpose:** Determine if legacy code should be preserved or deleted

---

## Executive Summary

**Recommendation: DELETE legacy code with surgical precision**

The codebase contains a clear evolutionary split between legacy (buggy) and modern (production) socket implementations. The modern `MetricsSocket` is production-ready and actively used. Legacy code in `SocketConnection` should be removed to prevent confusion and maintenance burden.

---

## 1. Current Architecture: Dual-Socket Design

### Socket Architecture Overview

The daemon implements a **two-socket architecture** for separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                      DAEMON                             │
│                                                         │
│  ┌────────────────┐         ┌─────────────────┐       │
│  │ IPC Socket     │         │ Metrics Socket  │       │
│  │ (Commands)     │         │ (Broadcast)     │       │
│  └────────┬───────┘         └────────┬────────┘       │
│           │                          │                 │
└───────────┼──────────────────────────┼─────────────────┘
            │                          │
            │                          │
    swictation.sock         swictation_metrics.sock
    (Line protocol)         (JSON events)
            │                          │
            │                          │
┌───────────┼──────────────────────────┼─────────────────┐
│           │                          │                 │
│  ┌────────▼───────┐         ┌───────▼────────┐        │
│  │ send_toggle_   │         │ MetricsSocket  │        │
│  │ command()      │         │ listen()       │        │
│  └────────────────┘         └────────────────┘        │
│                                                        │
│                    TAURI UI                            │
└────────────────────────────────────────────────────────┘
```

### Socket 1: IPC Socket (Command Path)

**Path:** `$XDG_RUNTIME_DIR/swictation.sock` or `~/.local/share/swictation/swictation.sock`

**Purpose:** Bidirectional command interface for CLI and UI control

**Protocol:** Line-delimited JSON commands
```json
{"action": "toggle"}
{"action": "status"}
{"action": "quit"}
```

**Implementation:**
- **Daemon:** `ipc::IpcServer` (in `main.rs:393`)
- **Tauri:** `MetricsSocket::send_toggle_command()` (in `metrics.rs:287-318`)

**Data Flow (Toggle Command):**
```
Tray Menu Click → emit("toggle-recording-requested") →
Frontend Handler → send_toggle_command() →
UnixStream::connect("swictation.sock") →
write_all(b"toggle\n") →
Daemon IpcServer → daemon.toggle()
```

### Socket 2: Metrics Socket (Data Stream)

**Path:** `$XDG_RUNTIME_DIR/swictation_metrics.sock` or `~/.local/share/swictation/swictation_metrics.sock`

**Purpose:** Unidirectional metrics broadcast to UI clients

**Protocol:** Newline-delimited JSON events
```json
{"type": "session_start", "session_id": "...", "timestamp": 1234567890}
{"type": "transcription", "text": "...", "wpm": 120.5, "latency_ms": 150}
{"type": "metrics_update", "state": "recording", "wpm": 120.5, ...}
{"type": "state_change", "state": "recording", "timestamp": 1234567890}
{"type": "session_end", "session_id": "...", "timestamp": 1234567890}
```

**Implementation:**
- **Daemon:** `MetricsBroadcaster::new()` (in `main.rs:85`)
- **Tauri:** `MetricsSocket::listen()` (in `metrics.rs:155-171`)

**Data Flow (Metrics Stream):**
```
Daemon Event → MetricsBroadcaster::broadcast() →
swictation_metrics.sock →
MetricsSocket::listen() →
lines.next_line() →
parse MetricsEvent →
app_handle.emit("transcription"|"metrics-update"|...)  →
Frontend React Component
```

---

## 2. Legacy vs Modern Code Evolution

### The Architectural Shift

| Aspect | Legacy (`SocketConnection`) | Modern (`MetricsSocket`) |
|--------|----------------------------|-------------------------|
| **File** | `socket/mod.rs:29-142` | `socket/metrics.rs` |
| **Runtime** | `std::os::unix::net::UnixStream` (sync) | `tokio::net::UnixStream` (async) |
| **Threading** | Manual thread spawning | Tokio async/await |
| **Reconnection** | Broken mutex deadlock | Automatic retry loop |
| **Error Handling** | Lock acquisition bugs | Proper async error propagation |
| **Type Safety** | Generic `serde_json::Value` | Strongly-typed `MetricsEvent` enum |
| **Event Parsing** | Weak validation | Serde deserialization with custom deserializers |
| **Status** | **DEPRECATED** (line 10) | **PRODUCTION** (actively used) |

### Critical Bug in Legacy Code

**File:** `socket/mod.rs:85-97`

```rust
// Read events from socket
let stream_lock = self.stream.lock().await;
if let Some(stream) = stream_lock.as_ref() {
    match self.read_events(stream) {  // ❌ BUG: Passes &UnixStream reference
        Ok(_) => {}
        Err(e) => {
            log::error!("Socket read error: {}. Reconnecting...", e);
            drop(stream_lock); // Drop lock before acquiring again
            *self.stream.lock().await = None;  // ❌ Deadlock risk
            // ...
        }
    }
}
```

**Problem:** The `read_events()` method at line 103 creates a `BufReader::new(stream)`, which takes ownership semantics incompatible with the borrowed reference from the mutex. This causes either compilation errors or runtime bugs depending on Rust version.

**Modern Fix:** `MetricsSocket` directly uses `UnixStream` without wrapping in `Arc<Mutex<Option<T>>>`, eliminating the issue entirely.

### Why the Rewrite?

The comment at line 10 is explicit:

```rust
// IMPORTANT: Use MetricsSocket for all metrics streaming.
// The legacy SocketConnection implementation has critical bugs and should not be used.
```

This indicates:
1. **Concurrency bugs** - Mutex handling was fundamentally flawed
2. **Type safety** - Generic JSON parsing was error-prone
3. **Architecture mismatch** - Sync I/O in async runtime caused issues

---

## 3. Public API Surface Analysis

### Required Public Methods (Tauri FFI Requirements)

Tauri commands and frontend integration **DO NOT** directly access socket internals. The socket system is **encapsulated** within the Rust backend.

**Frontend Interface:**
```typescript
// Frontend NEVER calls socket methods directly
// All communication via Tauri events:
listen("metrics-update", (event) => { ... })
listen("session-start", (event) => { ... })
listen("transcription", (event) => { ... })
```

**Tauri Commands Exposed:**
```rust
// commands/mod.rs:163-182
tauri::generate_handler![
    commands::get_recent_sessions,       // ✓ Database query
    commands::get_session_details,       // ✓ Database query
    commands::toggle_recording,          // ❌ DEPRECATED (line 89)
    commands::get_connection_status,     // ❌ DEPRECATED (line 99)
    // ... corrections, config ...
]
```

### Currently Public Methods (Unnecessary)

**In `MetricsSocket`:**

```rust
pub fn is_connected(&self) -> bool          // ❌ NOT USED (line 321)
pub fn socket_path(&self) -> &str           // ❌ NOT USED (line 326)
pub async fn send_toggle_command()         // ✅ USED by tray menu
```

**Analysis:**
- `is_connected()` - **Never called.** Connection status is sent via `app_handle.emit("metrics-connected", bool)` events (line 185, 220)
- `socket_path()` - **Never called.** Path is internal implementation detail
- `send_toggle_command()` - **Active use.** Called when tray menu "Toggle Recording" is clicked

**Recommendation:**
1. Keep `send_toggle_command()` public (active feature)
2. Downgrade `is_connected()` and `socket_path()` to private or delete
3. Connection status communicated exclusively via events

### Method Visibility Refactoring

```rust
// Current (over-exposed)
impl MetricsSocket {
    pub fn is_connected(&self) -> bool { ... }      // ❌ Remove or make private
    pub fn socket_path(&self) -> &str { ... }       // ❌ Remove or make private
    pub async fn send_toggle_command() { ... }      // ✅ Keep public
}

// Recommended
impl MetricsSocket {
    async fn listen(&mut self, app_handle: AppHandle) { ... }  // ✅ Already private
    pub async fn send_toggle_command() { ... }                 // ✅ Active use
    // is_connected and socket_path removed entirely
}
```

---

## 4. Design Intent & Future Architecture

### SOCKET_TIMEOUT_SECS (Line 17)

```rust
const SOCKET_TIMEOUT_SECS: u64 = 30;
```

**Status:** Defined but **never used** in current implementation

**Intent:** Likely planned for detecting stuck connections in `connect_and_process()` loop

**Recommendation:** Either implement timeout logic or remove constant as dead code

### get_ipc_socket_path() (Line 42 of socket_utils.rs)

```rust
/// Get path for main IPC socket (toggle commands)
pub fn get_ipc_socket_path() -> PathBuf {
    get_socket_dir().join("swictation.sock")
}
```

**Current Use:** Called by `send_toggle_command()` in `metrics.rs:288`

**Design:** Part of the dual-socket architecture (commands vs metrics separation)

**Recommendation:** **PRESERVE** - This is core infrastructure for the IPC command channel

### send_toggle_command() Design Philosophy

**Current Implementation:** UI sends toggle commands directly to daemon via IPC socket

**Alternative Approaches Considered:**

1. **Hotkey-only (no socket)** - UI cannot trigger recording
2. **Event bus** - More complex architecture
3. **Shared memory** - Platform-specific complications

**Chosen Design Benefits:**
- ✅ Works in environments without hotkey support (Wayland, Sway)
- ✅ Enables tray menu control (line 82-84 of main.rs)
- ✅ Allows CLI scripts to control daemon (`echo '{"action":"toggle"}' | nc -U swictation.sock`)
- ✅ Decouples UI from hotkey implementation

**Recommendation:** **PRESERVE** - Essential for non-hotkey environments and tray menu functionality

---

## 5. Code Organization Issues

### Module Structure

```
socket/
├── mod.rs           ← Contains BOTH legacy SocketConnection AND exports
├── metrics.rs       ← Modern MetricsSocket (production code)
└── socket_utils.rs  ← Socket path utilities (shared infrastructure)
```

**Problems:**
1. `mod.rs` conflates:
   - Legacy implementation (lines 29-142)
   - Module exports (lines 15-17)
   - Module documentation (lines 1-10)
2. No clear deprecation path for `SocketConnection`
3. Tests validate deprecated code (lines 144-158)

### Recommended Structure

```
socket/
├── mod.rs           ← Clean module exports + documentation only
├── metrics.rs       ← MetricsSocket (unchanged)
└── socket_utils.rs  ← Socket paths (unchanged)
```

**New `mod.rs`:**
```rust
//! Socket connection module for real-time metrics and daemon control
//!
//! This module provides:
//! - Async Unix socket connection for metrics streaming (MetricsSocket)
//! - Automatic reconnection on disconnect
//! - Event parsing and Tauri integration
//! - IPC command socket for daemon control

mod metrics;
mod socket_utils;

// Public exports
pub use metrics::MetricsSocket;
pub use socket_utils::{get_metrics_socket_path, get_ipc_socket_path};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_validation() {
        let socket_path = get_metrics_socket_path();
        let socket_str = socket_path.to_string_lossy();
        assert!(!socket_str.is_empty());
        assert!(!socket_str.starts_with("/tmp/"));
        assert!(socket_str.ends_with("swictation_metrics.sock"));
    }
}
```

---

## 6. Architectural Recommendations

### DELETE: Legacy SocketConnection

**Lines to Remove:** `/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs:19-142`

**Justification:**
1. **Explicitly deprecated** in module documentation (line 10)
2. **Never used** - `main.rs` only instantiates `MetricsSocket` (line 145)
3. **Contains critical bugs** - Mutex deadlock patterns
4. **Confuses maintainers** - Violates single-source-of-truth principle

### DELETE: Unused Public Methods

**Methods to Remove/Privatize:**
```rust
// In metrics.rs
pub fn is_connected(&self) -> bool      // Line 321 - Delete
pub fn socket_path(&self) -> &str       // Line 326 - Delete
```

**Replacement:** Connection status already emitted via events (lines 185, 220)

### DELETE: Deprecated Tauri Commands

**Commands to Remove:**
```rust
// In commands/mod.rs
#[tauri::command]
pub async fn toggle_recording() -> Result<String, String>  // Lines 88-96 - Delete

#[tauri::command]
pub async fn get_connection_status() -> Result<ConnectionStatus, String>  // Lines 98-108 - Delete
```

**Justification:**
- `toggle_recording()` - Placeholder that does nothing (line 95)
- `get_connection_status()` - Returns hardcoded stub data (line 106)

**Migration Path:**
```rust
// OLD (deprecated)
invoke("toggle_recording")

// NEW (production)
listen("toggle-recording-requested", () => {
    // Handled by tray menu (main.rs:69, 84)
})
```

### PRESERVE: Core Infrastructure

**Keep in `metrics.rs`:**
```rust
pub async fn send_toggle_command() -> Result<()>  // Line 287 - ACTIVE USE
```

**Keep in `socket_utils.rs`:**
```rust
pub fn get_ipc_socket_path() -> PathBuf          // Line 42 - CORE INFRASTRUCTURE
pub fn get_metrics_socket_path() -> PathBuf      // Line 47 - CORE INFRASTRUCTURE
```

### IMPLEMENT OR DELETE: Unused Constants

**Option A - Implement Timeout:**
```rust
// In metrics.rs:174
async fn connect_and_process(&mut self, app_handle: &AppHandle) -> Result<()> {
    let stream = tokio::time::timeout(
        Duration::from_secs(SOCKET_TIMEOUT_SECS),
        UnixStream::connect(&self.socket_path)
    )
    .await
    .context("Socket connection timeout")??;
    // ...
}
```

**Option B - Delete Constant:**
```rust
// Remove line 17 if timeout not needed
const SOCKET_TIMEOUT_SECS: u64 = 30;  // ❌ Delete
```

**Recommendation:** Implement timeout to prevent hung connections

---

## 7. Migration Impact Analysis

### Breaking Changes: NONE

Removing legacy code has **zero impact** on production system:

| Component | Current State | After Deletion | Impact |
|-----------|---------------|----------------|--------|
| Frontend | Uses events only | Unchanged | ✅ None |
| `main.rs` | Uses `MetricsSocket` | Unchanged | ✅ None |
| Commands | Deprecated stubs | Removed | ✅ Cleanup |
| Tray Menu | Uses `send_toggle_command()` | Preserved | ✅ None |
| Tests | Validate legacy code | Updated | ✅ Cleanup |

### Compilation Verification

**Files to verify after deletion:**
```bash
# Should compile without errors
cargo build --manifest-path tauri-ui/src-tauri/Cargo.toml

# Should pass all tests
cargo test --manifest-path tauri-ui/src-tauri/Cargo.toml
```

---

## 8. Final Verdict: Surgical Deletion Plan

### Phase 1: Remove Legacy Implementation

**File:** `/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs`

**Delete lines 19-142:**
```rust
// DELETE: All imports used only by SocketConnection
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

// DELETE: Entire SocketConnection implementation
pub struct SocketConnection { ... }
impl SocketConnection { ... }
```

**Preserve lines 1-18** (module docs and exports)

### Phase 2: Remove Unused Public Methods

**File:** `/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs`

**Delete lines 320-328:**
```rust
// DELETE: Unused public getters
pub fn is_connected(&self) -> bool { ... }
pub fn socket_path(&self) -> &str { ... }
```

**Keep line 287-318** (`send_toggle_command()` - active use)

### Phase 3: Remove Deprecated Commands

**File:** `/opt/swictation/tauri-ui/src-tauri/src/commands/mod.rs`

**Delete lines 88-108:**
```rust
// DELETE: Deprecated command stubs
#[tauri::command]
pub async fn toggle_recording() -> Result<String, String> { ... }

#[tauri::command]
pub async fn get_connection_status() -> Result<ConnectionStatus, String> { ... }
```

**Update `main.rs:169-170`:** Remove from `generate_handler!` macro

### Phase 4: Implement Timeout (Optional Enhancement)

**File:** `/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs`

**Modify line 176-178:**
```rust
// Add timeout to prevent hung connections
let stream = tokio::time::timeout(
    Duration::from_secs(SOCKET_TIMEOUT_SECS),
    UnixStream::connect(&self.socket_path)
)
.await
.context("Socket connection timeout")??;
```

**If timeout not needed:** Delete `const SOCKET_TIMEOUT_SECS` (line 17)

### Phase 5: Update Tests

**File:** `/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs`

**Delete lines 144-158** (legacy tests)

**Keep socket path validation test** (refactor if needed)

---

## Conclusion

The Tauri UI socket architecture is **well-designed** with clear separation between command and metrics channels. The presence of legacy code is purely technical debt from an incomplete refactoring.

**Key Architectural Principles:**
1. ✅ **Dual-socket design** - Commands and metrics properly separated
2. ✅ **Event-driven UI** - Frontend uses Tauri events, not direct socket access
3. ✅ **Modern async implementation** - MetricsSocket uses Tokio correctly
4. ✅ **Security-conscious** - Socket paths follow XDG standards with proper permissions

**Deletion Safety:**
- Legacy code is already deprecated and unused
- No production code references `SocketConnection`
- All tests validate deprecated implementation
- Zero risk to system stability

**Final Recommendation:** **DELETE** all legacy code identified in Phase 1-3, implement timeout in Phase 4, and update tests in Phase 5. This will eliminate ~200 lines of dead code and prevent future confusion.

---

## Appendix: Code References

### Active Production Code
- `tauri-ui/src-tauri/src/socket/metrics.rs` - ✅ MetricsSocket (lines 119-336)
- `tauri-ui/src-tauri/src/socket/socket_utils.rs` - ✅ Socket paths (lines 15-78)
- `tauri-ui/src-tauri/src/main.rs:145-152` - ✅ Socket initialization

### Deprecated/Dead Code
- `tauri-ui/src-tauri/src/socket/mod.rs:29-142` - ❌ SocketConnection (legacy)
- `tauri-ui/src-tauri/src/commands/mod.rs:88-108` - ❌ Deprecated commands
- `tauri-ui/src-tauri/src/socket/metrics.rs:320-328` - ❌ Unused getters

### Documentation References
- `rust-crates/swictation-daemon/src/main.rs:4` - IPC socket design intent
- `rust-crates/swictation-daemon/src/socket_utils.rs:70-78` - Daemon socket paths
- `rust-crates/swictation-daemon/src/ipc.rs:1-100` - IPC command protocol
