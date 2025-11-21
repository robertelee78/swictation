# ACTUAL ROOT CAUSE - MetricsSocket Not Connecting

## TL;DR
The UI shows OFFLINE because the MetricsSocket listener in the Tauri UI is NOT connecting to the daemon's socket. The Tauri v2 capabilities file was a red herring - the real problem is socket connection failure.

## Evidence

### 1. Build Verification ✅
- All 6 event names present in bundle JavaScript
- All 6 event names present in binary
- Capabilities file exists at `src-tauri/capabilities/default.json` with `core:event:default` permission
- All 3 binaries have matching checksum: `61d3555554e87147c522b6db3c305290edfe3dc5b2a1437a4e56e20db848e668`

### 2. Runtime Behavior ❌
```bash
# With RUST_LOG=debug, we see:
[2025-11-21T01:09:08Z INFO  swictation_ui] Opening database at: "/home/robert/.local/share/swictation/metrics.db"
# ... and NOTHING ELSE
```

### 3. Expected Behavior
From main.rs:124-132, the MetricsSocket should:
```rust
tauri::async_runtime::spawn(async move {
    if let Err(e) = metrics_socket.listen(app_handle).await {
        log::error!("Metrics socket error: {}", e);
    }
});
```

The `listen()` method should log connection status, but we see NO logs at all - not even error logs.

### 4. Daemon Socket Status
The daemon socket exists and is functional:
```bash
$ ls -la /run/user/1000/swictation_metrics.sock
srw-rw-rw- 1 robert robert 0 Nov 20 16:55 /run/user/1000/swictation_metrics.sock

$ echo '{"action":"subscribe"}' | nc -U /run/user/1000/swictation_metrics.sock
# Returns metrics successfully
```

## Root Cause Diagnosis

The `MetricsSocket::listen()` method is failing silently. Possible reasons:

1. **Socket path mismatch** - The UI is trying to connect to the wrong socket path
2. **Permission issues** - The UI process can't access the socket (unlikely, given 0777 permissions)
3. **Async task panic** - The spawned task is panicking before it can log
4. **Silent failure in listen()** - The method is catching all errors and not logging them

## What Tauri v2 Capabilities Actually Do

The capabilities file controls FRONTEND permissions to:
- Listen to events emitted by backend (`listen()`)
- Emit events to backend (`emit()`)
- Access window/app APIs

But capabilities are irrelevant if the BACKEND itself never emits events because it failed to connect to the daemon socket.

## Next Steps

1. Add detailed debug logging to `MetricsSocket::listen()` method
2. Check what socket path the UI is actually trying to connect to
3. Add error handling that actually logs failures instead of silently failing
4. Consider whether the async spawn is even executing (add log before `listen()` call)

## Timeline of Failed Hypotheses

1. ❌ "Event names missing from bundle" - FALSE: All events present, verified by build script
2. ❌ "Vite caching issue" - FALSE: Fresh build with `emptyOutDir: true` still shows offline
3. ❌ "Missing Tauri v2 capabilities" - FALSE: Capabilities file exists, UI still offline
4. ✅ **ACTUAL ISSUE**: MetricsSocket not connecting to daemon socket

## Version History

- v0.4.34: Published broken (build verification false negatives)
- v0.4.35: Published broken (same issue, didn't fix root cause)
- v0.4.36 (current build): Has capabilities file, still broken (socket connection issue)
