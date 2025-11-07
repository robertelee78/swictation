# Swictation Unix Socket Integration Guide

## Overview

The `socket.rs` module provides a robust Unix socket connection handler for real-time metrics streaming from the Swictation daemon. It handles automatic reconnection, event parsing, and seamless integration with Tauri's event system.

## Architecture

```
┌─────────────────┐         Unix Socket          ┌──────────────────┐
│ Swictation      │◄────────────────────────────►│  MetricsSocket   │
│ Daemon          │  /tmp/swictation_metrics.sock│  (socket.rs)     │
│                 │                               │                  │
│ - Session mgmt  │                               │ - Auto-reconnect │
│ - Transcription │                               │ - Event parsing  │
│ - Metrics       │                               │ - Error handling │
└─────────────────┘                               └──────────────────┘
                                                           │
                                                           │ Tauri Events
                                                           ▼
                                                  ┌─────────────────┐
                                                  │  Frontend (UI)  │
                                                  │                 │
                                                  │ - React/Svelte  │
                                                  │ - Event handlers│
                                                  │ - UI updates    │
                                                  └─────────────────┘
```

## Socket Protocol

### Connection Details
- **Metrics Socket**: `/tmp/swictation_metrics.sock` (read-only, metrics broadcast)
- **Command Socket**: `/tmp/swictation.sock` (write, daemon control)
- **Format**: Newline-delimited JSON
- **Reconnect Delay**: 5 seconds on disconnect

### Event Types

#### 1. Session Start
```json
{
  "type": "session_start",
  "session_id": "uuid-v4",
  "timestamp": 1699363200
}
```

#### 2. Session End
```json
{
  "type": "session_end",
  "session_id": "uuid-v4",
  "timestamp": 1699363260
}
```

#### 3. State Change
```json
{
  "type": "state_change",
  "daemon_state": "recording",
  "timestamp": 1699363200
}
```

**States**: `idle`, `recording`, `processing`, `paused`

#### 4. Transcription
```json
{
  "type": "transcription",
  "session_id": "uuid-v4",
  "text": "Hello world",
  "timestamp": 1699363200,
  "wpm": 120.5,
  "latency_ms": 150
}
```

#### 5. Metrics Update
```json
{
  "type": "metrics_update",
  "state": "recording",
  "wpm": 120.5,
  "words": 1024,
  "latency_ms": 150,
  "segments": 42,
  "duration_s": 300.5,
  "gpu_memory_mb": 2048.0,
  "cpu_percent": 45.2
}
```

## Rust Integration

### Basic Setup

```rust
use socket::MetricsSocket;
use tauri::{AppHandle, Manager};

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Spawn socket listener
            tokio::spawn(async move {
                let mut socket = MetricsSocket::connect().await?;
                socket.listen(app_handle).await?;
                Ok::<(), anyhow::Error>(())
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Adding Commands

```rust
#[tauri::command]
async fn toggle_recording() -> Result<(), String> {
    MetricsSocket::send_toggle_command()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_connection_status(
    state: tauri::State<'_, SocketState>
) -> Result<bool, String> {
    let socket = state.socket.lock().await;
    Ok(socket.is_connected())
}
```

### Tauri Event Emissions

The socket handler automatically emits the following Tauri events:

| Event Name | Payload Type | Description |
|------------|--------------|-------------|
| `metrics-connected` | `bool` | Socket connection status |
| `session-start` | `SessionStart` | New session started |
| `session-end` | `SessionEnd` | Session ended |
| `state-change` | `StateChange` | Daemon state changed |
| `transcription` | `Transcription` | New transcription received |
| `metrics-update` | `MetricsUpdate` | Periodic metrics update |

## Frontend Integration

### TypeScript/JavaScript

```typescript
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

// Listen for connection status
await listen<boolean>('metrics-connected', (event) => {
  const connected = event.payload;
  console.log('Socket connected:', connected);
  updateConnectionIndicator(connected);
});

// Listen for metrics updates
await listen('metrics-update', (event) => {
  const metrics = event.payload;
  updateWPM(metrics.wpm);
  updateCPU(metrics.cpu_percent);
  updateGPU(metrics.gpu_memory_mb);
  updateDuration(metrics.duration_s);
});

// Listen for transcriptions
await listen('transcription', (event) => {
  const data = event.payload;
  addTranscriptionToUI(data.text, data.wpm, data.latency_ms);
});

// Listen for session events
await listen('session-start', (event) => {
  console.log('Session started:', event.payload.session_id);
  clearSessionData();
});

await listen('session-end', (event) => {
  console.log('Session ended:', event.payload.session_id);
  showSessionSummary();
});

// Listen for state changes
await listen('state-change', (event) => {
  const state = event.payload.state;
  updateRecordingState(state);
});

// Toggle recording
async function toggleRecording() {
  try {
    await invoke('toggle_recording');
    console.log('Recording toggled');
  } catch (error) {
    console.error('Failed to toggle recording:', error);
    showError('Failed to toggle recording');
  }
}
```

### React Example

```tsx
import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

interface Metrics {
  state: string;
  wpm: number;
  words: number;
  latency_ms: number;
  segments: number;
  duration_s: number;
  gpu_memory_mb: number;
  cpu_percent: number;
}

function MetricsDisplay() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    // Listen for connection status
    const unlistenConnection = listen<boolean>('metrics-connected', (event) => {
      setConnected(event.payload);
    });

    // Listen for metrics updates
    const unlistenMetrics = listen<Metrics>('metrics-update', (event) => {
      setMetrics(event.payload);
    });

    // Cleanup
    return () => {
      unlistenConnection.then(fn => fn());
      unlistenMetrics.then(fn => fn());
    };
  }, []);

  const handleToggle = async () => {
    try {
      await invoke('toggle_recording');
    } catch (error) {
      console.error('Toggle failed:', error);
    }
  };

  if (!connected) {
    return <div>Connecting to daemon...</div>;
  }

  if (!metrics) {
    return <div>Waiting for metrics...</div>;
  }

  return (
    <div className="metrics-panel">
      <div className="metric">
        <label>State:</label>
        <span>{metrics.state}</span>
      </div>
      <div className="metric">
        <label>WPM:</label>
        <span>{metrics.wpm.toFixed(1)}</span>
      </div>
      <div className="metric">
        <label>Words:</label>
        <span>{metrics.words}</span>
      </div>
      <div className="metric">
        <label>Latency:</label>
        <span>{metrics.latency_ms}ms</span>
      </div>
      <div className="metric">
        <label>CPU:</label>
        <span>{metrics.cpu_percent.toFixed(1)}%</span>
      </div>
      <div className="metric">
        <label>GPU Memory:</label>
        <span>{metrics.gpu_memory_mb.toFixed(0)}MB</span>
      </div>
      <button onClick={handleToggle}>
        Toggle Recording
      </button>
    </div>
  );
}

export default MetricsDisplay;
```

### Svelte Example

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/tauri';

  let connected = false;
  let metrics = null;
  let unlistenFunctions = [];

  onMount(async () => {
    // Listen for connection status
    const unlisten1 = await listen('metrics-connected', (event) => {
      connected = event.payload;
    });

    // Listen for metrics updates
    const unlisten2 = await listen('metrics-update', (event) => {
      metrics = event.payload;
    });

    unlistenFunctions = [unlisten1, unlisten2];
  });

  onDestroy(() => {
    unlistenFunctions.forEach(fn => fn());
  });

  async function toggleRecording() {
    try {
      await invoke('toggle_recording');
    } catch (error) {
      console.error('Toggle failed:', error);
    }
  }
</script>

<div class="metrics-panel">
  {#if !connected}
    <p>Connecting to daemon...</p>
  {:else if metrics}
    <div class="metric">
      <label>WPM:</label>
      <span>{metrics.wpm.toFixed(1)}</span>
    </div>
    <div class="metric">
      <label>CPU:</label>
      <span>{metrics.cpu_percent.toFixed(1)}%</span>
    </div>
    <button on:click={toggleRecording}>
      Toggle Recording
    </button>
  {/if}
</div>
```

## Error Handling

### Automatic Reconnection

The socket handler automatically reconnects on:
- Socket disconnect
- Read errors
- Connection timeouts
- Daemon restarts

Reconnection delay: **5 seconds**

### Error Logging

All errors are logged using the `tracing` crate:

```rust
use tracing::{error, warn, info, debug};

error!("Critical failure: {}", e);
warn!("Recoverable issue: {}", e);
info!("Normal operation");
debug!("Detailed debugging info");
```

### Frontend Error Handling

```typescript
// Handle connection loss
await listen<boolean>('metrics-connected', (event) => {
  if (!event.payload) {
    showNotification('Connection lost. Reconnecting...', 'warning');
  } else {
    showNotification('Connected to daemon', 'success');
  }
});

// Handle command failures
try {
  await invoke('toggle_recording');
} catch (error) {
  console.error('Command failed:', error);
  showNotification('Failed to toggle recording', 'error');
}
```

## Testing

### Unit Tests

```rust
cargo test --package swictation-ui
```

### Integration Testing

```bash
# Start daemon
swictation-daemon --config ~/.config/swictation/config.toml

# Start UI (automatically connects)
cargo tauri dev

# Monitor socket traffic
socat -v UNIX-CONNECT:/tmp/swictation_metrics.sock -
```

### Event Simulation

```bash
# Send test events to socket
echo '{"type":"metrics_update","state":"recording","wpm":120.5,"words":100,"latency_ms":150,"segments":10,"duration_s":60.5,"gpu_memory_mb":2048.0,"cpu_percent":45.2}' | socat - UNIX-CONNECT:/tmp/swictation_metrics.sock
```

## Performance Considerations

### Memory Usage
- Minimal overhead: ~1MB per connection
- Event buffer: 4KB read buffer
- No accumulated state (events processed immediately)

### Latency
- Event processing: <1ms per event
- Tauri emission: <2ms to frontend
- Total UI update latency: <5ms

### Resource Cleanup
- Automatic socket cleanup on disconnect
- No memory leaks in reconnection loop
- Graceful shutdown on app exit

## Troubleshooting

### Socket not found
```
Error: Metrics socket does not exist: /tmp/swictation_metrics.sock
```
**Solution**: Ensure daemon is running:
```bash
systemctl --user status swictation-daemon
# or
swictation-daemon --config ~/.config/swictation/config.toml
```

### Permission denied
```
Error: Permission denied (os error 13)
```
**Solution**: Check socket permissions:
```bash
ls -l /tmp/swictation_metrics.sock
chmod 666 /tmp/swictation_metrics.sock  # If needed
```

### Connection timeouts
```
Error: Connection timeout after 30 seconds
```
**Solution**:
1. Check daemon logs for issues
2. Verify socket is accepting connections
3. Restart daemon if stuck

### Events not received in UI
**Debug checklist**:
1. Check Tauri event listener is registered
2. Verify socket connection status
3. Monitor browser console for errors
4. Check daemon is emitting events

## Best Practices

### 1. Handle Connection States
Always display connection status to users:
```typescript
const [connectionState, setConnectionState] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');

await listen<boolean>('metrics-connected', (event) => {
  setConnectionState(event.payload ? 'connected' : 'disconnected');
});
```

### 2. Buffer UI Updates
Avoid overwhelming the UI with rapid updates:
```typescript
import { debounce } from 'lodash';

const updateMetrics = debounce((metrics) => {
  setMetrics(metrics);
}, 100); // Update UI at most every 100ms

await listen('metrics-update', (event) => {
  updateMetrics(event.payload);
});
```

### 3. Graceful Degradation
Handle missing daemon gracefully:
```typescript
if (!connected) {
  return (
    <div className="offline-mode">
      <p>Daemon not available</p>
      <button onClick={checkConnection}>Retry Connection</button>
    </div>
  );
}
```

### 4. Log Important Events
```typescript
await listen('session-start', (event) => {
  console.log('[Session]', event.payload.session_id, 'started at', new Date(event.payload.timestamp * 1000));
  analytics.track('session_started', { session_id: event.payload.session_id });
});
```

## API Reference

### MetricsSocket

#### Methods

##### `MetricsSocket::new() -> Self`
Create a new socket instance.

##### `MetricsSocket::connect() -> Result<Self>`
Connect to the metrics socket. Returns error if socket doesn't exist.

##### `MetricsSocket::listen(&mut self, app_handle: AppHandle) -> Result<()>`
Listen for events indefinitely with automatic reconnection.
- Emits Tauri events for all received metrics
- Reconnects automatically on disconnect
- Never returns (runs until app exit)

##### `MetricsSocket::send_toggle_command() -> Result<()>`
Send toggle command to daemon control socket.

##### `MetricsSocket::is_connected(&self) -> bool`
Check current connection status.

##### `MetricsSocket::socket_path(&self) -> &str`
Get the socket file path.

### Event Payloads

See [Event Types](#event-types) section for detailed payload structures.

## License

MIT License - See project LICENSE file for details.
