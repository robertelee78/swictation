# Socket Handler Quick Start Guide

## Files Location

```
src-tauri/src/socket/
├── mod.rs          # Module exports
└── metrics.rs      # Async socket implementation (USE THIS)
```

## Basic Integration (3 Steps)

### 1. Add to main.rs

```rust
mod socket;
use socket::MetricsSocket;

#[tauri::command]
async fn toggle_recording() -> Result<(), String> {
    MetricsSocket::send_toggle_command()
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            tokio::spawn(async move {
                let mut socket = MetricsSocket::connect().await?;
                socket.listen(app_handle).await?;
                Ok::<(), anyhow::Error>(())
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![toggle_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Listen for Events (Frontend)

```typescript
import { listen } from '@tauri-apps/api/event';

// Metrics updates (WPM, CPU, GPU, etc.)
await listen('metrics-update', (event) => {
  console.log('Metrics:', event.payload);
});

// Transcription text
await listen('transcription', (event) => {
  console.log('Text:', event.payload.text);
});

// Connection status
await listen('metrics-connected', (event) => {
  console.log('Connected:', event.payload);
});
```

### 3. Send Commands

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Toggle recording on/off
await invoke('toggle_recording');
```

## Event Types

| Event | Payload Fields | Description |
|-------|---------------|-------------|
| `metrics-update` | state, wpm, words, latency_ms, segments, duration_s, gpu_memory_mb, cpu_percent | Performance metrics |
| `transcription` | session_id, text, timestamp, wpm, latency_ms | New transcribed text |
| `session-start` | session_id, timestamp | Session started |
| `session-end` | session_id, timestamp | Session ended |
| `state-change` | state, timestamp | Daemon state changed |
| `metrics-connected` | boolean | Socket connection status |

## Testing

```bash
# Check daemon is running
./scripts/test-socket.sh daemon

# Monitor live events
./scripts/test-socket.sh monitor

# Run all tests
./scripts/test-socket.sh all
```

## Troubleshooting

**Socket not found?**
```bash
# Start daemon
swictation-daemon
```

**Not receiving events?**
- Check browser console for errors
- Verify event listeners are registered
- Check daemon logs: `journalctl --user -u swictation-daemon -f`

## Full Documentation

- **SOCKET_INTEGRATION.md** - Complete guide with examples
- **SOCKET_IMPLEMENTATION.md** - Implementation details
- **main.rs.example** - Full integration example

## Socket Paths

- Metrics: `/tmp/swictation_metrics.sock` (read-only)
- Commands: `/tmp/swictation.sock` (write-only)
