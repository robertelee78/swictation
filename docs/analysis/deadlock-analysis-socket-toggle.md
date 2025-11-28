# Deadlock Analysis: macOS Socket Toggle Hang

## Executive Summary

**DEADLOCK IDENTIFIED**: The socket toggle command hangs due to a lock acquisition deadlock in the broadcaster system. The issue occurs when `toggle()` calls `broadcast_state_change()` while holding internal RwLocks.

## Root Cause

The deadlock occurs in this specific scenario:

1. **Socket toggle command** is received (via `swictation toggle`)
2. `Daemon::toggle()` is called (async function)
3. During the `Recording → Idle` transition, locks are acquired in this order:
   - `pipeline.write()` → holds pipeline write lock
   - `pipeline.stop_recording()` → holds `stt.lock()` (std::sync::Mutex)
   - Lock released after STT inference
   - `state.write()` + `session_id.write()` acquired
   - All locks released
4. **THEN** `broadcaster.broadcast_state_change()` is called
5. Inside `broadcast_state_change()`:
   - `self.last_state.write().await` - acquires RwLock<String>
   - `self.client_manager.broadcast()` - tries to acquire `clients.lock().await`

## The Deadlock Chain

### Code Path Analysis

**File: `rust-crates/swictation-daemon/src/ipc.rs` (lines 100-109)**
```rust
Ok(CommandType::Toggle) => match daemon.toggle().await {
    Ok(msg) => serde_json::json!({
        "status": "success",
        "message": msg
    }),
    Err(e) => serde_json::json!({
        "status": "error",
        "error": format!("{}", e)
    }),
},
```

**File: `rust-crates/swictation-daemon/src/main.rs` (lines 117-200)**

The `toggle()` function has these phases:

**Phase 1-3: Lock management (lines 157-183)**
```rust
DaemonState::Recording => {
    // Phase 2: Stop recording (STT inference - 50-500ms)
    {
        let mut pipeline = self.pipeline.write().await;
        pipeline.stop_recording().await?;  // <-- Can block for hundreds of ms
        pipeline.clear_session_id();
    }
    // Pipeline lock released

    // Phase 3: Update state
    let (session_metrics, sid) = {
        let mut state = self.state.write().await;
        let pipeline = self.pipeline.read().await;
        let mut session_id = self.session_id.write().await;

        *state = DaemonState::Idle;
        // ... metrics collection ...

        (session_metrics, sid)
    };
    // All locks released
```

**Phase 4: Broadcasting (lines 186-192) - THE DEADLOCK**
```rust
    // Phase 4: Broadcast (no locks held - but BLOCKS on socket I/O)
    if let Some(sid) = sid {
        self.broadcaster.end_session(sid).await;  // <-- Can block
    }
    self.broadcaster
        .broadcast_state_change(swictation_metrics::DaemonState::Idle)
        .await;  // <-- DEADLOCK HERE
```

**File: `rust-crates/swictation-broadcaster/src/broadcaster.rs` (lines 239-253)**
```rust
pub async fn broadcast_state_change(&self, state: DaemonState) {
    let state_str = Self::daemon_state_to_string(&state);

    // Update last state
    *self.last_state.write().await = state_str.clone();  // <-- RwLock acquired

    let event = BroadcastEvent::StateChange {
        state: state_str,
        timestamp: Self::current_timestamp(),
    };

    if let Err(e) = self.client_manager.broadcast(&event).await {  // <-- Deadlock here
        tracing::error!("Failed to broadcast state_change: {}", e);
    }
}
```

**File: `rust-crates/swictation-broadcaster/src/client.rs` (lines 90-108)**
```rust
pub async fn broadcast(&self, event: &BroadcastEvent) -> Result<()> {
    let mut clients = self.clients.lock().await;  // <-- Waits for client lock
    let mut dead_indices = Vec::new();

    for (idx, client) in clients.iter_mut().enumerate() {
        if let Err(e) = client.send_event(event).await {  // <-- Socket I/O can BLOCK
            tracing::warn!("Failed to send to client {}: {}", idx, e);
            dead_indices.push(idx);
        }
    }
    // ... cleanup ...
}
```

## Why It Deadlocks

### The Deadlock Conditions

1. **Socket I/O Blocking**: When `client.send_event()` writes to a Unix socket, it can block if:
   - Client has disconnected but socket hasn't been closed
   - Socket buffer is full
   - Client is slow to read
   - Network/filesystem latency

2. **Lock Held During I/O**: The `clients.lock()` is held while doing socket I/O:
   ```rust
   let mut clients = self.clients.lock().await;  // Lock acquired
   // ...
   client.send_event(event).await  // Socket I/O with lock held!
   ```

3. **IPC Handler Waiting**: The IPC handler (socket command) is waiting for `toggle()` to complete:
   ```rust
   Ok(CommandType::Toggle) => match daemon.toggle().await {  // <-- Waiting here
   ```

4. **Response Never Sent**: Because `toggle()` never completes, the IPC handler never sends the response:
   ```rust
   let response_str = serde_json::to_string(&response)?;
   stream.write_all(response_str.as_bytes()).await?;  // <-- Never reaches here
   ```

5. **Client Waits Forever**: The `swictation toggle` CLI client waits forever for the response.

## Differences: Socket vs Hotkey Toggle

### Socket Toggle Path (DEADLOCKS)
```
CLI client → Unix socket → IpcServer::accept()
→ handle_connection() → daemon.toggle().await
→ blocking wait for response → HANGS
```

### Hotkey Toggle Path (WORKS)
```
Hotkey event → HotkeyEvent::Toggle
→ daemon.toggle().await (no response needed)
→ continues immediately → NO HANG
```

**KEY DIFFERENCE**: The hotkey path doesn't wait for a response. It fires and forgets:

```rust
// File: main.rs, lines 616-620
HotkeyEvent::Toggle => {
    if let Err(e) = daemon_clone.toggle().await {
        error!("Toggle error: {}", e);
    }
    // No response to send - just logs error
}
```

## Lock Acquisition Timeline

### Normal Flow (Hotkey - No Deadlock)
1. Hotkey pressed
2. `toggle()` called
3. Locks acquired/released properly
4. `broadcast_state_change()` called
5. If broadcasting blocks, hotkey handler doesn't care
6. Returns immediately

### Deadlock Flow (Socket Command)
1. Socket command received
2. IPC handler calls `daemon.toggle().await`
3. IPC handler **WAITS** for toggle to complete
4. `toggle()` reaches `broadcast_state_change()`
5. Broadcasting blocks on socket I/O
6. `toggle()` never completes
7. IPC handler never sends response
8. CLI client hangs forever

## All Locks Involved

### In toggle() function:
1. `self.state.write()` - RwLock (tokio)
2. `self.pipeline.write()` - RwLock (tokio)
3. `self.session_id.write()` - RwLock (tokio)
4. `pipeline.get_metrics().lock()` - Mutex (std::sync)
5. `pipeline.stt.lock()` - Mutex (std::sync)

### In broadcaster:
6. `self.last_state.write()` - RwLock (tokio)
7. `self.client_manager.clients.lock()` - Mutex (tokio)

**CRITICAL**: The broadcaster's `clients.lock()` is held during socket I/O, which can block indefinitely.

## Why macOS-Specific?

This is **NOT** macOS-specific. The issue can occur on any platform, but:

1. **macOS testing caught it first** - Good catch!
2. **Different socket behavior** - macOS may have different socket buffer sizes
3. **Timing differences** - STT inference may be faster/slower on M-series chips
4. **Network stack differences** - Unix domain sockets behave slightly differently

The bug exists on **all platforms** but manifests more easily on macOS.

## The Fix Strategy

### Option 1: Non-Blocking Broadcast (RECOMMENDED)
Don't wait for broadcast to complete:

```rust
// In main.rs toggle() function
// BEFORE:
self.broadcaster
    .broadcast_state_change(swictation_metrics::DaemonState::Idle)
    .await;

// AFTER:
let broadcaster = self.broadcaster.clone();
tokio::spawn(async move {
    broadcaster
        .broadcast_state_change(swictation_metrics::DaemonState::Idle)
        .await;
});
```

### Option 2: Timeout on Socket I/O
Add timeouts to client broadcasts:

```rust
// In client.rs broadcast() function
use tokio::time::{timeout, Duration};

for (idx, client) in clients.iter_mut().enumerate() {
    match timeout(Duration::from_millis(100), client.send_event(event)).await {
        Ok(Ok(())) => {},
        Ok(Err(e)) | Err(_) => {
            tracing::warn!("Failed to send to client {}", idx);
            dead_indices.push(idx);
        }
    }
}
```

### Option 3: Fire-and-Forget Broadcasting
Release lock before I/O:

```rust
// In client.rs broadcast() function
let event_json = event.to_json_line()?;

// Clone client streams while holding lock
let client_streams = {
    let clients = self.clients.lock().await;
    clients.iter().map(|c| c.stream.clone()).collect::<Vec<_>>()
};
// Lock released

// Send to each client without holding lock
for stream in client_streams {
    let _ = stream.write_all(event_json.as_bytes()).await;
}
```

## Verification Steps

1. **Reproduce the deadlock**:
   ```bash
   # Terminal 1
   swictation-daemon-macos

   # Terminal 2
   swictation toggle  # Should hang
   ```

2. **Verify hotkey works**:
   - Press configured hotkey (e.g., Cmd+Shift+D)
   - Should toggle immediately

3. **Check with timeout**:
   ```bash
   timeout 5 swictation toggle || echo "DEADLOCK CONFIRMED"
   ```

4. **Monitor with tracing**:
   ```bash
   RUST_LOG=debug swictation-daemon-macos 2>&1 | grep -E "(toggle|broadcast)"
   ```

## Recommended Fix Priority

**Priority: CRITICAL** - This completely breaks the CLI interface.

**Fix Order**:
1. Implement Option 1 (spawn broadcast) - 5 minutes
2. Add Option 2 (timeouts) as safety net - 10 minutes
3. Test thoroughly on macOS - 15 minutes
4. Test on Linux to verify fix doesn't break anything - 10 minutes

**Total time: ~40 minutes**

## Files to Modify

1. `rust-crates/swictation-daemon/src/main.rs`
   - Lines 149-153 (start_recording broadcast)
   - Lines 186-192 (stop_recording broadcast)

2. `rust-crates/swictation-broadcaster/src/client.rs`
   - Lines 90-108 (add timeout to broadcast)

3. Optional: `rust-crates/swictation-broadcaster/src/broadcaster.rs`
   - Lines 239-253 (refactor state_change broadcasting)
