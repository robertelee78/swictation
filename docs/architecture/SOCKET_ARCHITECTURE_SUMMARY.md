# Socket Architecture: Executive Summary

**Project:** Swictation Tauri UI
**Component:** Unix Socket Communication Layer
**Status:** Architecture Review Complete
**Date:** 2025-11-26

---

## Quick Answer to Your Questions

### 1. How does the Tauri UI communicate with the daemon?

**TWO independent sockets for separation of concerns:**

#### Socket 1: Metrics Broadcast (Read-Only)
- **Path:** `$XDG_RUNTIME_DIR/swictation_metrics.sock`
- **Protocol:** Newline-delimited JSON events
- **Direction:** Daemon → Tauri UI (one-way broadcast)
- **Data Flow:**
  ```
  Daemon transcription → MetricsBroadcaster.broadcast() →
  swictation_metrics.sock → MetricsSocket.listen() →
  app_handle.emit("transcription") → React Component
  ```

#### Socket 2: IPC Commands (Write-Only)
- **Path:** `$XDG_RUNTIME_DIR/swictation.sock`
- **Protocol:** Line-delimited JSON commands (`{"action":"toggle"}`)
- **Direction:** Tauri UI → Daemon (request/response)
- **Command Flow:**
  ```
  Tray Menu Click → send_toggle_command() →
  write_all(b'{"action":"toggle"}\n') →
  IpcServer.accept() → daemon.toggle()
  ```

**Architecture Diagram:**
```
Daemon Process                    Tauri UI Process
┌──────────────┐                 ┌──────────────┐
│ IPC Server   │◄────commands────│ Tray Menu    │
│ (port 8001)  │                 │ Handler      │
└──────────────┘                 └──────────────┘
        │
        ▼
┌──────────────┐
│ Daemon Core  │
└──────┬───────┘
       │
       ▼
┌──────────────┐                 ┌──────────────┐
│ Metrics      │────events──────►│ Metrics      │
│ Broadcaster  │                 │ Socket       │
└──────────────┘                 └──────┬───────┘
                                        │
                                        ▼
                                 ┌──────────────┐
                                 │ React        │
                                 │ Components   │
                                 └──────────────┘
```

---

### 2. What was the architectural change? Why?

**Before (Legacy):** `SocketConnection` in `socket/mod.rs`
- Synchronous I/O (`std::os::unix::net::UnixStream`)
- Manual thread management
- `Arc<Mutex<Option<UnixStream>>>` wrapper
- **Critical bug:** Mutex deadlock in error handling (line 91-94)

**After (Modern):** `MetricsSocket` in `socket/metrics.rs`
- Async I/O (`tokio::net::UnixStream`)
- Tokio runtime integration
- Direct ownership (no mutex wrapper)
- **Fixed:** Proper async error propagation

**Why the change?**

1. **Concurrency Safety:**
   ```rust
   // LEGACY (BUGGY):
   let stream_lock = self.stream.lock().await;
   if let Some(stream) = stream_lock.as_ref() {
       match self.read_events(stream) {  // ❌ Ownership conflict
           Err(e) => {
               drop(stream_lock);
               *self.stream.lock().await = None;  // ❌ Potential deadlock
           }
       }
   }

   // MODERN (CORRECT):
   let stream = UnixStream::connect(&self.socket_path).await?;
   let reader = BufReader::new(stream);
   let mut lines = reader.lines();
   while let Some(line) = lines.next_line().await? {  // ✅ Clean async
       // Process line
   }
   ```

2. **Type Safety:**
   - **Legacy:** `serde_json::Value` (runtime validation only)
   - **Modern:** `MetricsEvent` enum (compile-time validation)

3. **Architecture Alignment:**
   - **Legacy:** Sync code in async runtime (antipattern)
   - **Modern:** Pure async with Tokio (idiomatic Rust)

**Evidence of abandonment:**
```rust
// socket/mod.rs:10
// IMPORTANT: Use MetricsSocket for all metrics streaming.
// The legacy SocketConnection implementation has critical bugs and should not be used.
```

---

### 3. What methods MUST be public for Tauri FFI?

**Answer: NONE of the socket methods are exposed to Tauri FFI.**

The socket layer is **completely encapsulated** within the Rust backend. Frontend communication happens exclusively via Tauri events.

#### Current Public API (Over-Exposed)

```rust
// In socket/metrics.rs
impl MetricsSocket {
    pub fn is_connected(&self) -> bool          // ❌ UNUSED
    pub fn socket_path(&self) -> &str           // ❌ UNUSED
    pub async fn send_toggle_command()         // ✅ USED (tray menu)
}
```

#### Tauri FFI Commands (Deprecated)

```rust
// In commands/mod.rs
#[tauri::command]
pub async fn toggle_recording() -> Result<String, String> {
    Ok("Toggle recording via hotkey (Ctrl+Shift+D) or tray menu".to_string())
}  // ❌ Placeholder only, does nothing

#[tauri::command]
pub async fn get_connection_status() -> Result<ConnectionStatus, String> {
    Ok(ConnectionStatus {
        connected: true,  // ❌ Hardcoded stub
        socket_path: "/run/user/1000/swictation_metrics.sock".to_string(),
    })
}  // ❌ Returns fake data
```

#### Frontend Communication (Event-Based)

```typescript
// Frontend code (TypeScript)
import { listen } from '@tauri-apps/api/event';

// No direct socket access - only events
listen('transcription', (event) => {
    console.log(event.payload);  // Strongly-typed MetricsEvent
});

listen('metrics-connected', (event) => {
    setConnected(event.payload);  // boolean
});
```

**Key Insight:** Tauri's event system (`AppHandle::emit`) is the **only** communication channel. Socket methods never cross FFI boundary.

---

### 4. Is SOCKET_TIMEOUT_SECS intended for future use?

**Status:** Defined but **unused** (dead code)

```rust
// socket/metrics.rs:17
const SOCKET_TIMEOUT_SECS: u64 = 30;  // ❌ Never referenced
```

**Intent:** Likely planned for detecting stuck connections in `connect_and_process()` loop

**Current Implementation:**
```rust
// No timeout - waits indefinitely
let stream = UnixStream::connect(&self.socket_path).await?;
```

**Recommendation:** Either implement or delete

**Option A - Implement:**
```rust
let stream = tokio::time::timeout(
    Duration::from_secs(SOCKET_TIMEOUT_SECS),
    UnixStream::connect(&self.socket_path)
)
.await
.context("Socket connection timeout")??;
```

**Option B - Delete:**
```rust
// Remove constant if not needed
```

---

### 5. Should we keep send_toggle_command()?

**YES - Active production use in non-hotkey environments**

#### Use Cases

1. **Tray Menu (Primary):**
   ```rust
   // main.rs:82-84
   TrayIconEvent::Click { button: MouseButton::Left, .. } => {
       let app = tray.app_handle();
       let _ = app.emit("toggle-recording-requested", ());
       // Event handler calls send_toggle_command()
   }
   ```

2. **Wayland/Sway Compatibility:**
   - Global hotkeys may not work in all Wayland compositors
   - Tray menu provides fallback control mechanism

3. **CLI Integration:**
   ```bash
   # Users can also toggle via command line
   echo '{"action":"toggle"}' | nc -U $XDG_RUNTIME_DIR/swictation.sock
   ```

#### Benefits of IPC Socket Design

✅ **Platform Independence:** Works when hotkeys fail
✅ **Multi-Client:** CLI tools and UI can coexist
✅ **Decoupling:** UI doesn't depend on hotkey implementation
✅ **Debugging:** Easy to test with netcat/socat

**Verdict:** **PRESERVE** - Essential for production reliability

---

## Final Recommendations

### 🗑️ DELETE (Dead Code)

1. **Legacy Socket Implementation**
   - File: `socket/mod.rs` lines 19-142
   - Reason: Deprecated, buggy, unused

2. **Unused Public Methods**
   - `is_connected()` → Delete (status via events)
   - `socket_path()` → Delete (internal detail)

3. **Deprecated Tauri Commands**
   - `toggle_recording()` → Delete (placeholder only)
   - `get_connection_status()` → Delete (returns fake data)

### ✅ PRESERVE (Production Code)

1. **Modern Socket Implementation**
   - File: `socket/metrics.rs` (entire file)
   - Reason: Active production code

2. **Socket Path Utilities**
   - File: `socket/socket_utils.rs` (entire file)
   - Reason: Core infrastructure, shared with daemon

3. **Toggle Command Method**
   - Method: `send_toggle_command()`
   - Reason: Used by tray menu, essential for non-hotkey environments

### 🔧 IMPLEMENT OR DELETE (Unused Constants)

1. **Socket Timeout**
   - Constant: `SOCKET_TIMEOUT_SECS`
   - Options: Implement timeout logic OR delete constant

---

## Impact Analysis

### Code Deletion Impact: ZERO Breaking Changes

| Component | Before | After | Impact |
|-----------|--------|-------|--------|
| Frontend | Events only | Events only | ✅ None |
| Tray Menu | Uses `send_toggle_command()` | Preserved | ✅ None |
| Main Loop | Uses `MetricsSocket` | Unchanged | ✅ None |
| Tests | Validate legacy code | Updated | ✅ Cleanup |

**Why zero impact?**
- Legacy code already unused in production
- No component references `SocketConnection`
- Deprecated commands return placeholder values
- All communication via events (not direct calls)

---

## Documentation Structure

This analysis consists of four documents:

1. **`tauri-socket-architecture-analysis.md`** (Detailed Technical Analysis)
   - Full code walkthrough
   - Bug analysis
   - API surface review
   - Design intent investigation

2. **`adr/ADR-003-remove-legacy-socket-implementation.md`** (Architecture Decision Record)
   - Formal decision documentation
   - Options analysis
   - Implementation plan
   - Risk assessment

3. **`diagrams/tauri-socket-flow.md`** (Visual Diagrams)
   - Component diagrams
   - Sequence diagrams
   - Data flow diagrams
   - State machines

4. **`SOCKET_ARCHITECTURE_SUMMARY.md`** (This Document)
   - Executive summary
   - Quick answers
   - Recommendations

---

## Next Steps

### Phase 1: Review and Approval
- [ ] Technical lead reviews architecture analysis
- [ ] Confirm no hidden dependencies on legacy code
- [ ] Approve ADR-003

### Phase 2: Implementation
- [ ] Delete legacy `SocketConnection` (socket/mod.rs:19-142)
- [ ] Remove unused methods (`is_connected`, `socket_path`)
- [ ] Delete deprecated commands (`toggle_recording`, `get_connection_status`)
- [ ] Update tests

### Phase 3: Validation
- [ ] Compile: `cargo build --manifest-path tauri-ui/src-tauri/Cargo.toml`
- [ ] Test: `cargo test --manifest-path tauri-ui/src-tauri/Cargo.toml`
- [ ] Lint: `cargo clippy --manifest-path tauri-ui/src-tauri/Cargo.toml`

### Phase 4: Documentation
- [ ] Update module documentation
- [ ] Archive legacy code in git history
- [ ] Update frontend integration docs

---

## References

- **XDG Base Directory Spec:** https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
- **Tokio Documentation:** https://docs.rs/tokio
- **Tauri Events Guide:** https://tauri.app/v1/guides/features/events

---

## Contact

For questions about this architecture analysis:
- Review all documents in `/opt/swictation/docs/architecture/`
- Check git history for legacy code context
- Consult daemon implementation in `rust-crates/swictation-daemon/src/`

**Architecture Status:** ✅ Production-ready, legacy code identified for removal
