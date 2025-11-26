# ADR-003: Remove Legacy SocketConnection Implementation

**Status:** Proposed
**Date:** 2025-11-26
**Deciders:** System Architecture Designer
**Technical Story:** Clean up deprecated socket code in Tauri UI

---

## Context and Problem Statement

The Tauri UI socket module (`tauri-ui/src-tauri/src/socket/mod.rs`) contains two implementations for metrics streaming:

1. **Legacy `SocketConnection`** - Synchronous UnixStream with critical mutex bugs (lines 29-142)
2. **Modern `MetricsSocket`** - Async Tokio implementation, actively used in production (metrics.rs)

The module documentation explicitly warns against using `SocketConnection` (line 10), but the code remains in the codebase. This creates confusion for maintainers and risks accidental usage.

**Question:** Should we delete the legacy `SocketConnection` implementation and associated dead code?

---

## Decision Drivers

* **Code maintainability** - Dead code increases cognitive load
* **Type safety** - Legacy code uses weak `serde_json::Value` parsing
* **Concurrency safety** - Legacy code has documented mutex deadlock bugs
* **API clarity** - Dual implementations violate single-source-of-truth
* **Production risk** - Deprecated code should not exist in production systems

---

## Considered Options

### Option 1: Delete Legacy Code (Recommended)

**DELETE:**
- `socket/mod.rs:29-142` - Entire `SocketConnection` implementation
- `socket/metrics.rs:320-328` - Unused public getters (`is_connected()`, `socket_path()`)
- `commands/mod.rs:88-108` - Deprecated command stubs (`toggle_recording()`, `get_connection_status()`)

**PRESERVE:**
- `socket/metrics.rs` - Modern `MetricsSocket` (production code)
- `socket/metrics.rs:287-318` - `send_toggle_command()` (active use in tray menu)
- `socket/socket_utils.rs` - Socket path utilities (core infrastructure)

**Pros:**
- ✅ Eliminates ~200 lines of dead code
- ✅ Removes documented bug patterns
- ✅ Clarifies module API surface
- ✅ Prevents accidental use of deprecated code
- ✅ Zero breaking changes (code already unused)

**Cons:**
- ⚠️ Requires test updates
- ⚠️ Requires `generate_handler!` macro update

### Option 2: Mark as Deprecated with #[deprecated]

**Keep code but add deprecation attributes:**
```rust
#[deprecated(since = "0.7.0", note = "Use MetricsSocket instead")]
pub struct SocketConnection { ... }
```

**Pros:**
- ✅ Provides compiler warnings
- ✅ Preserves code for reference

**Cons:**
- ❌ Dead code still exists in codebase
- ❌ Increases maintenance burden
- ❌ Doesn't prevent accidental use
- ❌ Code has critical bugs (should not be preserved)

### Option 3: Do Nothing (Keep Status Quo)

**Leave deprecated code as-is**

**Pros:**
- ✅ Zero implementation effort

**Cons:**
- ❌ Violates clean code principles
- ❌ Confuses new developers
- ❌ Maintains buggy code in production
- ❌ Increases cognitive load

---

## Decision Outcome

**Chosen option:** **Option 1 - Delete Legacy Code**

**Rationale:**

1. **Zero Production Impact:** The legacy code is already unused. No component references `SocketConnection`:
   - `main.rs:145` instantiates `MetricsSocket`, not `SocketConnection`
   - Frontend uses Tauri events exclusively, never direct socket methods
   - All deprecated commands return placeholder values

2. **Eliminates Known Bugs:** The mutex deadlock pattern in `SocketConnection` is a critical concurrency bug:
   ```rust
   // Lines 85-97 in socket/mod.rs
   let stream_lock = self.stream.lock().await;
   if let Some(stream) = stream_lock.as_ref() {
       match self.read_events(stream) {  // ❌ BUG: Ownership conflict
           Err(e) => {
               drop(stream_lock);
               *self.stream.lock().await = None;  // ❌ Deadlock risk
           }
       }
   }
   ```

3. **Architectural Clarity:** The dual-socket design (IPC + metrics) is well-implemented in the modern code. Legacy implementation obscures this architecture.

4. **Follows Rust Best Practices:**
   - Dead code should be deleted, not disabled
   - Deprecation is for library APIs, not internal implementations
   - Module should export single source of truth

---

## Validation

### Before Deletion - Current State
```rust
// socket/mod.rs exports BOTH implementations
pub use metrics::MetricsSocket;  // ✅ Modern
pub struct SocketConnection { ... }  // ❌ Legacy (unused)

// commands/mod.rs has placeholder stubs
pub async fn toggle_recording() -> Result<String, String> {
    Ok("Toggle recording via hotkey (Ctrl+Shift+D) or tray menu".to_string())
}  // ❌ Does nothing
```

### After Deletion - Clean State
```rust
// socket/mod.rs exports ONLY production code
pub use metrics::MetricsSocket;  // ✅ Single source of truth

// commands/mod.rs removed deprecated stubs
// (tray menu uses send_toggle_command() directly)
```

### Compilation Test
```bash
# Must pass after deletion
cargo build --manifest-path tauri-ui/src-tauri/Cargo.toml
cargo test --manifest-path tauri-ui/src-tauri/Cargo.toml
```

---

## Consequences

### Positive Consequences

* **Reduced Cognitive Load:** Developers see only production code
* **Clearer API:** Single `MetricsSocket` implementation to understand
* **Safer Codebase:** Eliminates buggy concurrency patterns
* **Easier Testing:** Test only production code paths
* **Better Documentation:** Module docs reflect actual implementation

### Negative Consequences

* **Lost Historical Context:** Cannot reference original implementation
  - **Mitigation:** Preserve in git history and architecture documentation

* **Requires Test Updates:** Legacy tests must be removed/refactored
  - **Mitigation:** Socket path validation tests can be preserved

### Neutral Consequences

* **Code Churn:** ~200 lines deleted in single commit
  - This is expected for cleanup work

---

## Implementation Plan

### Phase 1: Backup and Document
```bash
# Archive legacy code in documentation
git show HEAD:tauri-ui/src-tauri/src/socket/mod.rs > docs/archive/legacy-socket-connection.rs
```

### Phase 2: Delete Legacy Implementation
1. Remove `socket/mod.rs:19-142` (SocketConnection + imports)
2. Update module exports to only include MetricsSocket
3. Remove deprecated tests in `socket/mod.rs:144-158`

### Phase 3: Remove Unused Public Methods
1. Delete `metrics.rs:320-328` (`is_connected()`, `socket_path()`)
2. Connection status already emitted via events (lines 185, 220)

### Phase 4: Remove Deprecated Commands
1. Delete `commands/mod.rs:88-108` (command stubs)
2. Update `main.rs:169-170` to remove from `generate_handler!`
3. Update frontend docs to use events instead

### Phase 5: Verify Compilation
```bash
cargo clean
cargo build --manifest-path tauri-ui/src-tauri/Cargo.toml
cargo test --manifest-path tauri-ui/src-tauri/Cargo.toml
cargo clippy --manifest-path tauri-ui/src-tauri/Cargo.toml
```

---

## Confirmation

This ADR validates the recommendation from `tauri-socket-architecture-analysis.md`.

**Files to modify:**
- `/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs` (cleanup)
- `/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs` (remove unused methods)
- `/opt/swictation/tauri-ui/src-tauri/src/commands/mod.rs` (remove stubs)
- `/opt/swictation/tauri-ui/src-tauri/src/main.rs` (update handler macro)

**Files to preserve:**
- `/opt/swictation/tauri-ui/src-tauri/src/socket/socket_utils.rs` (core infrastructure)
- `/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs` (production code)

---

## References

* Architecture Analysis: `docs/architecture/tauri-socket-architecture-analysis.md`
* Daemon IPC Implementation: `rust-crates/swictation-daemon/src/ipc.rs`
* Socket Path Standards: `rust-crates/swictation-daemon/src/socket_utils.rs`
* XDG Base Directory Specification: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

---

## Review Notes

**Risk Assessment:** LOW
- No production code uses legacy implementation
- All changes are deletions, not modifications
- Test suite will validate correctness

**Rollback Plan:** Revert commit if compilation fails
- Legacy code preserved in git history
- Can be restored with `git revert <commit-hash>`

**Approval Required:** Technical Lead
- Confirm no hidden dependencies on deprecated code
- Review test coverage after deletion
