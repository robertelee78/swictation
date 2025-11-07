# Tauri Commands API Reference

## Overview
This document provides complete API reference for all Tauri commands available to the frontend.

## Usage Pattern

```typescript
import { invoke } from '@tauri-apps/api';

// Call a command
const result = await invoke<ReturnType>('command_name', { param1: value1, param2: value2 });
```

---

## Commands

### `get_recent_sessions`

Get list of recent recording sessions.

**Parameters:**
- `limit: number` - Maximum number of sessions to return

**Returns:** `SessionSummary[]`

```typescript
interface SessionSummary {
  id: number;
  start_time: number;        // Unix timestamp (seconds)
  end_time: number | null;   // Unix timestamp (seconds)
  duration_ms: number | null; // Duration in milliseconds
  words_dictated: number;
  segments_count: number;
  wpm: number | null;        // Words per minute
}
```

**Example:**
```typescript
const sessions = await invoke<SessionSummary[]>('get_recent_sessions', { limit: 50 });
console.log(`Loaded ${sessions.length} sessions`);
```

**Error Cases:**
- Database not found
- Database query error
- Invalid limit parameter

---

### `get_session_details`

Get all transcriptions for a specific session.

**Parameters:**
- `session_id: number` - Session ID to retrieve

**Returns:** `TranscriptionRecord[]`

```typescript
interface TranscriptionRecord {
  id: number;
  session_id: number;
  text: string;
  timestamp: number;           // Unix timestamp (seconds)
  latency_ms: number | null;   // Processing latency
  words: number;
}
```

**Example:**
```typescript
const transcriptions = await invoke<TranscriptionRecord[]>(
  'get_session_details',
  { session_id: 123 }
);

transcriptions.forEach(t => {
  console.log(`[${new Date(t.timestamp * 1000).toLocaleTimeString()}] ${t.text}`);
});
```

**Error Cases:**
- Session not found
- Database query error
- Invalid session_id parameter

---

### `search_transcriptions`

Search all transcriptions by text content (case-insensitive).

**Parameters:**
- `query: string` - Search query
- `limit: number` - Maximum number of results

**Returns:** `TranscriptionRecord[]`

```typescript
// Same as TranscriptionRecord from get_session_details
```

**Example:**
```typescript
const results = await invoke<TranscriptionRecord[]>(
  'search_transcriptions',
  { query: 'important', limit: 100 }
);

console.log(`Found ${results.length} transcriptions containing "important"`);
```

**Notes:**
- Uses SQL LIKE pattern matching
- Case-insensitive search
- Returns results ordered by timestamp (newest first)

**Error Cases:**
- Database query error
- Empty query string
- Invalid limit parameter

---

### `get_lifetime_stats`

Get aggregate statistics across all sessions.

**Parameters:** None

**Returns:** `LifetimeStats`

```typescript
interface LifetimeStats {
  total_words: number;
  total_characters: number;
  total_sessions: number;
  total_time_minutes: number;
  average_wpm: number;
  average_latency_ms: number;
  best_wpm_value: number;
  best_wpm_session: number | null;
}
```

**Example:**
```typescript
const stats = await invoke<LifetimeStats>('get_lifetime_stats');

console.log(`Total words dictated: ${stats.total_words.toLocaleString()}`);
console.log(`Average WPM: ${stats.average_wpm.toFixed(1)}`);
console.log(`Total time: ${(stats.total_time_minutes / 60).toFixed(1)} hours`);
```

**Error Cases:**
- Database query error
- No statistics available (returns zeros)

---

### `toggle_recording`

Toggle recording state (start/stop).

**Parameters:** None

**Returns:** `string` - Status message

**Example:**
```typescript
const result = await invoke<string>('toggle_recording');
console.log(result); // "Toggle command sent"
```

**Notes:**
- Currently a placeholder
- Will be enhanced to send actual command to daemon
- May trigger daemon hotkey or send IPC message

**Error Cases:**
- Socket connection error
- Daemon not responding

---

### `get_connection_status`

Check if connected to metrics broadcaster socket.

**Parameters:** None

**Returns:** `ConnectionStatus`

```typescript
interface ConnectionStatus {
  connected: boolean;
  socket_path: string;
}
```

**Example:**
```typescript
const status = await invoke<ConnectionStatus>('get_connection_status');

if (status.connected) {
  console.log(`Connected to ${status.socket_path}`);
} else {
  console.log('Not connected to metrics broadcaster');
}
```

**Error Cases:**
- None (always returns status)

---

## Events

The UI can listen to real-time events from the daemon via Tauri events.

### Usage Pattern

```typescript
import { listen } from '@tauri-apps/api/event';

// Listen to event
const unlisten = await listen<EventPayload>('event-name', (event) => {
  console.log('Received event:', event.payload);
});

// Stop listening
unlisten();
```

### Event: `session_start`

Fired when a new recording session begins.

**Payload:**
```typescript
{
  type: "session_start",
  session_id: number,
  timestamp: number  // Unix timestamp (seconds)
}
```

**Example:**
```typescript
await listen('session_start', (event) => {
  const { session_id, timestamp } = event.payload;
  console.log(`Session ${session_id} started at ${new Date(timestamp * 1000)}`);
  // Clear transcription buffer in UI
});
```

---

### Event: `session_end`

Fired when recording session ends.

**Payload:**
```typescript
{
  type: "session_end",
  session_id: number,
  timestamp: number
}
```

**Example:**
```typescript
await listen('session_end', (event) => {
  const { session_id } = event.payload;
  console.log(`Session ${session_id} ended`);
  // Keep transcriptions visible in UI
});
```

---

### Event: `transcription`

New transcription segment received.

**Payload:**
```typescript
{
  type: "transcription",
  text: string,
  timestamp: string,      // HH:MM:SS format
  wpm: number,
  latency_ms: number,
  words: number
}
```

**Example:**
```typescript
await listen('transcription', (event) => {
  const { text, wpm, latency_ms } = event.payload;
  console.log(`[${wpm.toFixed(0)} WPM, ${latency_ms.toFixed(0)}ms] ${text}`);
  // Add to transcription buffer
});
```

---

### Event: `metrics_update`

Real-time metrics update from daemon.

**Payload:**
```typescript
{
  type: "metrics_update",
  state: string,              // "idle" | "recording" | "processing" | "error"
  session_id: number | null,
  segments: number,
  words: number,
  wpm: number,
  duration_s: number,
  latency_ms: number,
  gpu_memory_mb: number,
  gpu_memory_percent: number,
  cpu_percent: number
}
```

**Example:**
```typescript
await listen('metrics_update', (event) => {
  const { wpm, latency_ms, gpu_memory_percent } = event.payload;
  // Update UI with real-time metrics
  updateMetricsDisplay(event.payload);
});
```

---

### Event: `state_change`

Daemon state changed.

**Payload:**
```typescript
{
  type: "state_change",
  state: string,      // "idle" | "recording" | "processing" | "error"
  timestamp: number
}
```

**Example:**
```typescript
await listen('state_change', (event) => {
  const { state } = event.payload;
  console.log(`Daemon state: ${state}`);
  // Update UI state indicator
});
```

---

### Event: `socket-connected`

Socket connection status changed.

**Payload:** `boolean` - true if connected, false if disconnected

**Example:**
```typescript
await listen<boolean>('socket-connected', (event) => {
  const connected = event.payload;
  console.log(`Socket ${connected ? 'connected' : 'disconnected'}`);
  // Show connection indicator in UI
});
```

---

### Event: `toggle-recording-requested`

System tray "Toggle Recording" menu item clicked.

**Payload:** `null`

**Example:**
```typescript
await listen('toggle-recording-requested', async () => {
  console.log('Toggle recording requested from system tray');
  await invoke('toggle_recording');
});
```

---

## Error Handling

All commands return `Result<T, String>`. Handle errors with try-catch:

```typescript
try {
  const sessions = await invoke<SessionSummary[]>('get_recent_sessions', { limit: 50 });
  // Success
} catch (error) {
  console.error('Failed to load sessions:', error);
  // Show error message to user
  showErrorToast(String(error));
}
```

### Common Error Messages

- `"Failed to get recent sessions: ..."` - Database query error
- `"Failed to connect to metrics socket"` - Socket connection error
- `"Metrics database not found at ..."` - Database doesn't exist yet
- `"Failed to search transcriptions: ..."` - Search query error

---

## Complete Example: Session History View

```typescript
import { invoke, listen } from '@tauri-apps/api';
import { useEffect, useState } from 'react';

function HistoryView() {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [selectedSession, setSelectedSession] = useState<number | null>(null);
  const [transcriptions, setTranscriptions] = useState<TranscriptionRecord[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    // Load recent sessions
    loadSessions();

    // Listen to connection status
    const unlistenConnected = listen<boolean>('socket-connected', (event) => {
      setConnected(event.payload);
    });

    // Listen to new sessions
    const unlistenSessionStart = listen('session_start', () => {
      loadSessions(); // Refresh list
    });

    return () => {
      unlistenConnected.then(f => f());
      unlistenSessionStart.then(f => f());
    };
  }, []);

  const loadSessions = async () => {
    try {
      const sessions = await invoke<SessionSummary[]>('get_recent_sessions', { limit: 50 });
      setSessions(sessions);
    } catch (error) {
      console.error('Failed to load sessions:', error);
    }
  };

  const loadSessionDetails = async (sessionId: number) => {
    try {
      const records = await invoke<TranscriptionRecord[]>('get_session_details', {
        session_id: sessionId
      });
      setTranscriptions(records);
      setSelectedSession(sessionId);
    } catch (error) {
      console.error('Failed to load session details:', error);
    }
  };

  return (
    <div>
      <div className="connection-status">
        {connected ? '🟢 Connected' : '🔴 Disconnected'}
      </div>

      <div className="sessions-list">
        {sessions.map(session => (
          <div
            key={session.id}
            onClick={() => loadSessionDetails(session.id)}
            className={selectedSession === session.id ? 'active' : ''}
          >
            <div>{new Date(session.start_time * 1000).toLocaleString()}</div>
            <div>{session.words_dictated} words · {session.wpm?.toFixed(0)} WPM</div>
          </div>
        ))}
      </div>

      <div className="transcriptions">
        {transcriptions.map(t => (
          <div key={t.id}>
            <span>{new Date(t.timestamp * 1000).toLocaleTimeString()}</span>
            <span>{t.text}</span>
            {t.latency_ms && <span>{t.latency_ms.toFixed(0)}ms</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## Database Paths

- **Linux/macOS**: `~/.local/share/swictation/metrics.db`
- **Socket**: `/tmp/swictation_metrics.sock`

## Type Definitions

All TypeScript interfaces can be generated from Rust types:

```bash
# Generate TypeScript bindings (future enhancement)
cargo tauri dev
```

---

## Testing

### Test Database Queries
```bash
sqlite3 ~/.local/share/swictation/metrics.db
> SELECT COUNT(*) FROM sessions;
> SELECT * FROM sessions ORDER BY start_time DESC LIMIT 5;
```

### Test Socket Connection
```bash
nc -U /tmp/swictation_metrics.sock
# Should see JSON events streaming
```

### Test Commands (DevTools)
```javascript
// In Tauri DevTools console
window.__TAURI__.invoke('get_recent_sessions', { limit: 10 })
  .then(console.log)
  .catch(console.error);
```
