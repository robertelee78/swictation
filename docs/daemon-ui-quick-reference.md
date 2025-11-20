# Swictation Daemon-UI Communication: Quick Reference Card

**Last Updated:** 2025-11-20

---

## Architecture At-A-Glance

```
Daemon (systemd) ──┬─► Metrics Socket ──► QT Tray UI
                   │   (broadcast)       Tauri UI
                   │                     Future UIs
                   │
                   └─► Command Socket ◄─► Any UI
                       (request-response)
```

**Technology:** Unix domain sockets + JSON Lines
**Why:** Lowest latency (<10ms), simplest implementation, most secure

---

## Socket Paths

```bash
# Metrics Broadcasting (Daemon → UI)
/tmp/swictation_metrics.sock

# Command Control (UI ↔ Daemon)
/tmp/swictation_ipc.sock

# Permissions
0600 (owner-only read/write)
```

---

## Event Types (Metrics Socket)

### 1. session_start
Recording begins, UI should clear buffer

```json
{"type":"session_start","session_id":123,"timestamp":1699000000.0}
```

### 2. session_end
Recording stops, UI keeps buffer visible

```json
{"type":"session_end","session_id":123,"timestamp":1699000000.0}
```

### 3. state_change
Daemon state transition

```json
{"type":"state_change","state":"recording","timestamp":1699000000.0}
```

**States:** `idle` | `recording` | `processing` | `error`

### 4. transcription
Real-time transcription text + metrics

```json
{
  "type":"transcription",
  "text":"Hello world",
  "timestamp":"14:23:15",
  "wpm":145.2,
  "latency_ms":234.5,
  "words":2
}
```

### 5. metrics_update
Periodic system metrics (every ~1 second)

```json
{
  "type":"metrics_update",
  "state":"recording",
  "session_id":123,
  "segments":5,
  "words":42,
  "wpm":145.2,
  "duration_s":30.5,
  "latency_ms":234.5,
  "gpu_memory_mb":1823.4,
  "gpu_memory_percent":45.2,
  "cpu_percent":23.1
}
```

---

## Commands (Command Socket)

### toggle
Start or stop recording

```json
// Request
{"action":"toggle"}

// Response (success)
{"status":"success","message":"Recording started"}

// Response (error)
{"status":"error","error":"Device not available"}
```

### status
Query current daemon state

```json
// Request
{"action":"status"}

// Response
{"status":"success","state":"recording"}
```

### quit
Shutdown daemon

```json
// Request
{"action":"quit"}

// Response (daemon exits immediately)
```

---

## Client Lifecycle

### 1. Connect
```rust
let stream = UnixStream::connect("/tmp/swictation_metrics.sock").await?;
```

### 2. Receive Catch-Up Data
Daemon sends:
1. Current state (`state_change` event)
2. Active session if recording (`session_start` event)
3. Buffered transcriptions (replay of current session)

### 3. Listen for Events
```rust
let reader = BufReader::new(stream);
let mut lines = reader.lines();

while let Some(line) = lines.next_line().await? {
    let event: BroadcastEvent = serde_json::from_str(&line)?;
    // Handle event
}
```

### 4. Auto-Reconnect on Disconnect
```rust
loop {
    match connect_and_listen().await {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Disconnected: {}", e);
            sleep(Duration::from_secs(2)).await;
        }
    }
}
```

---

## Common Code Patterns

### Daemon: Broadcast Event
```rust
use swictation_broadcaster::MetricsBroadcaster;

let broadcaster = MetricsBroadcaster::new("/tmp/swictation_metrics.sock").await?;
broadcaster.start().await?;

// Start session
broadcaster.start_session(123).await;

// Add transcription
broadcaster.add_transcription("Hello world", 145.2, 234.5, 2).await;

// Update metrics
broadcaster.update_metrics(&realtime).await;

// End session
broadcaster.end_session(123).await;
```

### UI: Listen for Events
```rust
use tokio::net::UnixStream;
use tokio::io::{BufReader, AsyncBufReadExt};

let stream = UnixStream::connect("/tmp/swictation_metrics.sock").await?;
let reader = BufReader::new(stream);
let mut lines = reader.lines();

while let Some(line) = lines.next_line().await? {
    let event: BroadcastEvent = serde_json::from_str(&line)?;

    match event {
        BroadcastEvent::SessionStart { session_id, .. } => {
            println!("Session started: {}", session_id);
        }
        BroadcastEvent::Transcription { text, wpm, .. } => {
            println!("Transcription: '{}' (WPM: {})", text, wpm);
        }
        BroadcastEvent::MetricsUpdate { state, .. } => {
            println!("State: {}", state);
        }
        _ => {}
    }
}
```

### UI: Send Command
```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

// Connect to command socket
let mut stream = UnixStream::connect("/tmp/swictation_ipc.sock").await?;

// Send toggle command
let command = serde_json::json!({"action": "toggle"});
stream.write_all(command.to_string().as_bytes()).await?;
stream.flush().await?;

// Read response
let mut response = String::new();
stream.read_to_string(&mut response).await?;
let result: serde_json::Value = serde_json::from_str(&response)?;

println!("Result: {:?}", result);
```

---

## Debugging Commands

### Check if daemon is running
```bash
systemctl status swictation-daemon
```

### Check if sockets exist
```bash
ls -la /tmp/swictation_*.sock
```

### Check socket permissions
```bash
stat -c "%a %n" /tmp/swictation_*.sock
# Should show: 600 /tmp/swictation_metrics.sock
```

### Check who's listening
```bash
lsof /tmp/swictation_metrics.sock
```

### Check connected clients
```bash
netstat -x | grep swictation
```

### Test metrics socket (receive events)
```bash
nc -U /tmp/swictation_metrics.sock
# Should stream JSON events
```

### Test command socket (send toggle)
```bash
echo '{"action":"toggle"}' | nc -U /tmp/swictation_ipc.sock
# Should respond with JSON
```

### View daemon logs
```bash
journalctl -u swictation-daemon -f
```

### View Tauri UI logs
```bash
tail -f ~/.local/share/com.swictation.app/logs/app.log
```

---

## Common Issues & Solutions

### "Socket does not exist"
**Cause:** Daemon not running
**Fix:**
```bash
systemctl start swictation-daemon
systemctl enable swictation-daemon  # Auto-start on boot
```

### "Permission denied"
**Cause:** Socket owned by different user
**Fix:**
```bash
sudo chown $USER /tmp/swictation_*.sock
# OR restart daemon as your user
```

### "Connection refused"
**Cause:** Daemon not listening yet (race condition)
**Fix:** Wait 2-5 seconds after daemon start, or implement retry loop

### "Events not received"
**Cause 1:** Connected to wrong socket
**Fix:** Verify socket path matches daemon (`/tmp/swictation_metrics.sock`)

**Cause 2:** Event parsing error
**Fix:** Check JSON format, ensure newline-delimited

### "Daemon crash on startup"
**Cause:** Audio device not available, GPU not detected, etc.
**Fix:** Check logs: `journalctl -u swictation-daemon`

### "UI doesn't reconnect"
**Cause:** Auto-reconnect loop not implemented
**Fix:** Implement retry loop (see "Client Lifecycle" above)

---

## Performance Benchmarks

| Metric | Value |
|--------|-------|
| **Latency (P50)** | 2ms |
| **Latency (P99)** | 8ms |
| **Throughput** | 10,000+ events/sec |
| **CPU Usage** | 2-5% |
| **Memory** | 1MB |
| **Multi-Client** | ✅ Tested (QT + Tauri) |

---

## Security Checklist

- ✅ Socket permissions: 0600 (owner-only)
- ✅ Local-only communication (no network exposure)
- ✅ Process isolation (systemd user service)
- ✅ No authentication needed (OS-level security)
- ✅ No encryption needed (data never leaves localhost)
- ✅ Auto-cleanup (socket removed on daemon exit)

---

## File Locations

### Daemon Source
```
/opt/swictation/rust-crates/swictation-broadcaster/src/broadcaster.rs
/opt/swictation/rust-crates/swictation-broadcaster/src/client.rs
/opt/swictation/rust-crates/swictation-broadcaster/src/events.rs
/opt/swictation/rust-crates/swictation-daemon/src/ipc.rs
```

### Tauri UI Source
```
/opt/swictation/tauri-ui/src-tauri/src/socket/mod.rs
/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs
```

### Documentation
```
/opt/swictation/docs/daemon-ui-communication-architecture.md
/opt/swictation/docs/daemon-ui-architecture-diagrams.md
/opt/swictation/docs/daemon-ui-implementation-summary.md
/opt/swictation/docs/daemon-ui-quick-reference.md (this file)
```

---

## Related Documentation

- **Full Architecture:** `daemon-ui-communication-architecture.md`
- **Visual Diagrams:** `daemon-ui-architecture-diagrams.md`
- **Implementation Summary:** `daemon-ui-implementation-summary.md`
- **Socket Security:** `SOCKET_SECURITY.md`
- **Main Architecture:** `architecture.md`

---

## Key Takeaways

1. **Two sockets:** Metrics (broadcast) + Command (request-response)
2. **JSON Lines protocol:** Newline-delimited JSON for streaming
3. **Auto-reconnect:** UI must implement retry loop (2-5 second delay)
4. **Catch-up protocol:** New clients get current state + buffer replay
5. **Multi-client native:** Broadcasting to all clients automatic
6. **Security:** 0600 permissions, local-only, OS-level isolation

---

**Questions? Check the full architecture docs or contact the development team.**
