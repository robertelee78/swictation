# swictation-broadcaster

Real-time metrics broadcaster for Swictation UI clients via Unix socket.

## Features

- **Unix Domain Socket Server** - Listens on `/tmp/swictation_metrics.sock`
- **Newline-Delimited JSON Protocol** - Simple, parseable event streaming
- **Multiple Concurrent Clients** - Thread-safe client connection management
- **Session-Based Transcription Buffer** - RAM-only storage, cleared on new session
- **New Client Catch-Up** - Automatically sends current state + buffer to new connections
- **5 Event Types** - `session_start`, `session_end`, `transcription`, `metrics_update`, `state_change`

## Event Types

### session_start
Signals session beginning, clears transcription buffer in UI.
```json
{"type": "session_start", "session_id": 123, "timestamp": 1699000000.0}
```

### session_end
Signals session completion, buffer remains visible.
```json
{"type": "session_end", "session_id": 123, "timestamp": 1699000000.0}
```

### transcription
New transcription segment added to buffer.
```json
{
  "type": "transcription",
  "text": "Hello world",
  "timestamp": "14:23:15",
  "wpm": 145.2,
  "latency_ms": 234.5,
  "words": 2
}
```

### metrics_update
Real-time metrics from daemon (sent per segment).
```json
{
  "type": "metrics_update",
  "state": "recording",
  "session_id": 123,
  "segments": 5,
  "words": 42,
  "wpm": 145.2,
  "duration_s": 30.5,
  "latency_ms": 234.5,
  "gpu_memory_mb": 1823.4,
  "gpu_memory_percent": 45.2,
  "cpu_percent": 23.1
}
```

### state_change
Daemon state transition.
```json
{
  "type": "state_change",
  "state": "recording",
  "timestamp": 1699000000.0
}
```

States: `idle`, `recording`, `processing`, `error`

## Usage Example

```rust
use swictation_broadcaster::MetricsBroadcaster;
use swictation_metrics::DaemonState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and start broadcaster
    let broadcaster = MetricsBroadcaster::new("/tmp/swictation_metrics.sock").await?;
    broadcaster.start().await?;

    // Start session (clears buffer)
    let session_id = 123;
    broadcaster.start_session(session_id).await;

    // Add transcription segments
    broadcaster.add_transcription(
        "Hello world".to_string(),
        145.2,  // wpm
        234.5,  // latency_ms
        2,      // words
    ).await;

    // Update metrics from collector
    let metrics = collector.get_realtime_metrics();
    broadcaster.update_metrics(&metrics).await;

    // Broadcast state changes
    broadcaster.broadcast_state_change(DaemonState::Processing).await;

    // End session (buffer stays visible)
    broadcaster.end_session(session_id).await;

    // Stop broadcaster
    broadcaster.stop().await?;

    Ok(())
}
```

## Testing

```bash
# Run unit tests
cargo test -p swictation-broadcaster --lib

# Run integration tests
cargo test -p swictation-broadcaster --test integration_tests

# Run all tests
cargo test -p swictation-broadcaster

# Run with logging
RUST_LOG=swictation_broadcaster=debug cargo test -p swictation-broadcaster -- --nocapture
```

## Architecture

- `broadcaster.rs` - Main `MetricsBroadcaster` orchestrator
- `client.rs` - Client connection wrapper and manager
- `events.rs` - Event type definitions and JSON serialization
- `error.rs` - Error types and results
- `lib.rs` - Public API and documentation

## Thread Safety

All operations are thread-safe using:
- `Arc<RwLock<_>>` for shared state (transcription buffer, session ID)
- `Arc<Mutex<_>>` for client list management
- Tokio async/await for non-blocking I/O

## Integration with Daemon

```rust
// Add to swictation-daemon/Cargo.toml
[dependencies]
swictation-broadcaster = { path = "../swictation-broadcaster" }

// In daemon main loop
let broadcaster = MetricsBroadcaster::new("/tmp/swictation_metrics.sock").await?;
broadcaster.start().await?;

// Per-segment callback
metrics.add_segment(segment)?;
broadcaster.add_transcription(
    segment.text.clone(),
    segment.calculate_wpm(),
    segment.total_latency_ms,
    segment.words
).await;

let realtime = metrics.get_realtime_metrics();
broadcaster.update_metrics(&realtime).await;
```

## License

Same as parent project.
