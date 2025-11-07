# Swictation Tauri UI

React + TypeScript frontend for the Swictation dictation system metrics viewer.

## Architecture

### Components

1. **App.tsx** - Main application with tab navigation and connection status
2. **LiveSession.tsx** - Real-time metrics display with state indicators and resource monitors
3. **History.tsx** - Past sessions list with lifetime statistics
4. **Transcriptions.tsx** - Real-time transcription display with clipboard copy

### Hooks

1. **useMetrics.ts** - Listens to Tauri events from the daemon broadcaster
2. **useDatabase.ts** - Queries session history and lifetime stats from SQLite

### Data Flow

```
Rust Daemon (swictation-daemon)
  ↓
Metrics Broadcaster (TCP socket)
  ↓
Tauri Backend (Rust)
  ↓ (events)
React Frontend (TypeScript)
  ↓
UI Components
```

## Event Types

The daemon broadcasts these events via Tauri:

### `metrics_update`
Real-time metrics during active recording:
```typescript
{
  type: 'metrics_update',
  state: 'idle' | 'recording' | 'processing' | 'error',
  session_id?: number,
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

### `transcription`
New transcription segment:
```typescript
{
  type: 'transcription',
  text: string,
  timestamp: string,  // HH:MM:SS format
  wpm: number,
  latency_ms: number,
  words: number
}
```

### `session_start`
Session started (clears transcription buffer):
```typescript
{
  type: 'session_start',
  session_id: number,
  timestamp: number
}
```

### `session_end`
Session ended (transcriptions stay visible):
```typescript
{
  type: 'session_end',
  session_id: number,
  timestamp: number
}
```

### `state_change`
Daemon state changed:
```typescript
{
  type: 'state_change',
  state: 'idle' | 'recording' | 'processing' | 'error',
  timestamp: number
}
```

## Tauri Commands

The backend exposes these commands for database queries:

### `get_recent_sessions`
```rust
#[tauri::command]
fn get_recent_sessions(limit: usize) -> Result<Vec<HistorySession>, String>
```

Returns the last N sessions from the metrics database.

### `get_lifetime_metrics`
```rust
#[tauri::command]
fn get_lifetime_metrics() -> Result<LifetimeMetrics, String>
```

Returns aggregate statistics across all sessions.

## Tokyo Night Dark Theme

All colors use CSS custom properties defined in `src/index.css`:

| Variable | Color | Usage |
|----------|-------|-------|
| `--color-background` | #1a1b26 | Main background |
| `--color-card` | #24283b | Card backgrounds |
| `--color-border` | #414868 | Borders |
| `--color-foreground` | #a9b1d6 | Primary text |
| `--color-foreground-bright` | #c0caf5 | Bright text |
| `--color-muted` | #565f89 | Muted/secondary text |
| `--color-primary` | #7aa2f7 | Primary accent (blue) |
| `--color-success` | #9ece6a | Success/green |
| `--color-warning` | #e0af68 | Warning/yellow |
| `--color-error` | #f7768e | Error/red |

## UI Layout Reference

The UI mirrors the QML implementation in `/opt/swictation/src/ui/MetricsUI.qml`:

### Live Session Tab
- State indicator (Idle/Recording/Processing)
- 6 metric cards (WPM, Words, Latency, Duration, Segments, GPU Memory)
- System resource meters (GPU Memory, CPU Usage) with color-coded progress bars

### History Tab
- Recent sessions table (last 10)
- Refresh button
- Lifetime statistics card

### Transcriptions Tab
- Real-time transcription list with auto-scroll
- Per-item metadata (timestamp, WPM, latency)
- Copy to clipboard buttons
- Privacy notice

## Development

```bash
# Install dependencies
npm install

# Run development server (Vite only)
npm run dev

# Build for production
npm run build

# Type checking
npm run typecheck
```

## Integration with Tauri Backend

The Tauri backend needs to:

1. **Listen to daemon metrics broadcaster** on `localhost:7861`
2. **Parse JSON events** and emit them as Tauri events with name `metrics-event`
3. **Implement Tauri commands** for database queries:
   - `get_recent_sessions(limit: usize)`
   - `get_lifetime_metrics()`
4. **Open SQLite database** at `~/.config/swictation/metrics.db`

## File Structure

```
tauri-ui/
├── src/
│   ├── components/
│   │   ├── LiveSession.tsx
│   │   ├── History.tsx
│   │   └── Transcriptions.tsx
│   ├── hooks/
│   │   ├── useMetrics.ts
│   │   └── useDatabase.ts
│   ├── App.tsx
│   ├── main.tsx
│   ├── index.css
│   └── types.ts
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── postcss.config.js
```

## Next Steps

1. Implement Tauri backend in Rust (`src-tauri/`)
2. Connect to daemon broadcaster
3. Implement database query commands
4. Test event flow from daemon → Tauri → React
5. Build and package application
