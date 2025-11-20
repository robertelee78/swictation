# React Component Chain Analysis: Metrics Event to DOM Rendering

## Complete Data Flow Analysis

### 1. Event Reception Layer

**File**: `/opt/swictation/tauri-ui/src/hooks/useMetrics.ts`

**Event Listener Setup** (Lines 36-105):
```typescript
useEffect(() => {
  console.log('[useMetrics] Setting up event listener for metrics-event');

  const unlistenMetrics = listen<BroadcastEvent>('metrics-event', (event) => {
    // Event processing logic
  });

  return () => {
    unlistenMetrics.then((fn) => fn());
  };
}, []);
```

**Key Points**:
- Uses Tauri's `listen()` API to subscribe to `'metrics-event'` events
- Event listener is set up once on component mount (empty dependency array)
- Returns cleanup function that unsubscribes on unmount
- Receives events of type `BroadcastEvent` (union type)

---

### 2. Event Processing & State Updates

**Event Type Handling** (Lines 45-99):

#### A. `metrics_update` Event (Lines 46-61)
```typescript
case 'metrics_update':
  setMetrics((prev) => ({
    ...prev,
    state: payload.state,
    sessionId: payload.session_id ?? null,
    segments: payload.segments,
    words: payload.words,
    wpm: payload.wpm,
    duration: formatDuration(payload.duration_s),
    latencyMs: payload.latency_ms,
    gpuMemoryMb: payload.gpu_memory_mb,
    gpuMemoryPercent: payload.gpu_memory_percent,
    cpuPercent: payload.cpu_percent,
    connected: true,  // ⚠️ CRITICAL: Sets connection status
  }));
```

**Payload Structure** (from types.ts):
```typescript
{
  type: 'metrics_update';
  state: DaemonState;           // 'idle' | 'recording' | 'processing' | 'error'
  session_id?: number;
  segments: number;
  words: number;
  wpm: number;
  duration_s: number;
  latency_ms: number;
  gpu_memory_mb: number;
  gpu_memory_percent: number;
  cpu_percent: number;
}
```

#### B. `state_change` Event (Lines 63-68)
```typescript
case 'state_change':
  setMetrics((prev) => ({
    ...prev,
    state: payload.state,
  }));
```

**Payload Structure**:
```typescript
{
  type: 'state_change';
  state: DaemonState;
  timestamp: number;
}
```

**⚠️ CRITICAL ISSUE**: Does NOT set `connected: true`

#### C. `transcription` Event (Lines 70-81)
```typescript
case 'transcription':
  setTranscriptions((prev) => [
    ...prev,
    {
      text: payload.text,
      timestamp: payload.timestamp,
      wpm: payload.wpm,
      latency_ms: payload.latency_ms,
      words: payload.words,
    },
  ]);
```

Updates separate `transcriptions` state array.

#### D. `session_start` Event (Lines 83-94)
```typescript
case 'session_start':
  setTranscriptions([]);  // Clear old transcriptions
  setMetrics((prev) => ({
    ...prev,
    sessionId: payload.session_id,
    segments: 0,
    words: 0,
    wpm: 0,
    duration: '00:00',
  }));
```

**⚠️ CRITICAL ISSUE**: Does NOT set `connected: true`

#### E. `session_end` Event (Lines 96-98)
```typescript
case 'session_end':
  // Keep transcriptions visible
  break;
```

Does nothing - transcriptions remain visible.

---

### 3. React State Management

**State Initialization** (Lines 19-32):
```typescript
const [metrics, setMetrics] = useState<LiveMetrics>({
  state: 'idle',
  sessionId: null,
  segments: 0,
  words: 0,
  wpm: 0,
  duration: '00:00',
  latencyMs: 0,
  gpuMemoryMb: 0,
  gpuMemoryPercent: 0,
  cpuPercent: 0,
  connected: false,  // ⚠️ DEFAULT: OFFLINE
});
```

**Default State**: All metrics are zero, `connected: false`

**Hook Export** (Line 107):
```typescript
return { metrics, transcriptions };
```

---

### 4. Component Integration

**File**: `/opt/swictation/tauri-ui/src/App.tsx`

**Hook Consumption** (Line 11):
```typescript
const { metrics, transcriptions } = useMetrics();
```

**Data Flow**:
1. App.tsx calls useMetrics()
2. Hook returns current state: `{ metrics, transcriptions }`
3. App.tsx passes data to child components

---

### 5. Rendering Chain

#### A. Connection Status Indicator (Lines 16-24)
```tsx
<div className="absolute top-3 right-3 z-50">
  <div
    className={`px-4 py-2 rounded ${
      metrics.connected ? 'bg-success' : 'bg-error'
    } text-white text-xs font-bold`}
  >
    {metrics.connected ? '● LIVE' : '● OFFLINE'}
  </div>
</div>
```

**Render Logic**:
- **IF** `metrics.connected === true`: Shows green "● LIVE"
- **ELSE**: Shows red "● OFFLINE"

**⚠️ BLOCKER**: This is always rendered, but depends on `connected` flag

#### B. LiveSession Component (Line 47)
```tsx
{activeTab === 'live' && <LiveSession metrics={metrics} />}
```

**Conditional Rendering**:
- Only renders when `activeTab === 'live'`
- No dependency on `metrics.connected`

**File**: `/opt/swictation/tauri-ui/src/components/LiveSession.tsx`

**DOM Elements Rendered** (Lines 36-88):

1. **Status Header** (Lines 38-44):
```tsx
<div className="bg-card rounded-lg p-5 flex items-center justify-center gap-4">
  <span className="text-5xl">{getStateIcon()}</span>
  <span className={`text-3xl font-bold font-mono uppercase ${getStateColor()}`}>
    {metrics.state}
  </span>
</div>
```

**Icons by State**:
- `recording`: 🔴 (red text)
- `processing`: 🟡 (yellow text)
- `idle/error`: 🎤 (green text)

2. **Metric Cards Grid** (Lines 46-54):
```tsx
<div className="grid grid-cols-3 gap-4">
  <MetricCard label="WPM" value={Math.round(metrics.wpm)} />
  <MetricCard label="Words" value={metrics.words} />
  <MetricCard label="Latency" value={`${(metrics.latencyMs / 1000).toFixed(2)}s`} />
  <MetricCard label="Duration" value={metrics.duration} />
  <MetricCard label="Segments" value={metrics.segments} />
  <MetricCard label="GPU Memory" value={`${(metrics.gpuMemoryMb / 1024).toFixed(1)} GB`} />
</div>
```

**Renders**:
- WPM (rounded)
- Words count
- Latency in seconds
- Duration (formatted as MM:SS)
- Segments count
- GPU Memory in GB

3. **System Resources** (Lines 56-87):

**GPU Memory Meter** (Lines 62-72):
```tsx
<div className="text-foreground text-sm mb-2">
  GPU Memory: {metrics.gpuMemoryMb.toFixed(1)} / 8000.0 MB ({Math.round(metrics.gpuMemoryPercent)}%)
</div>
<div className="w-full h-6 bg-border rounded overflow-hidden">
  <div
    className={`h-full ${getResourceColor(metrics.gpuMemoryPercent)} transition-all duration-300`}
    style={{ width: `${Math.min(metrics.gpuMemoryPercent, 100)}%` }}
  />
</div>
```

**CPU Usage Meter** (Lines 75-85):
```tsx
<div className="text-foreground text-sm mb-2">
  CPU Usage: {metrics.cpuPercent.toFixed(1)} / 100.0 % ({Math.round(metrics.cpuPercent)}%)
</div>
<div className="w-full h-6 bg-border rounded overflow-hidden">
  <div
    className={`h-full ${getResourceColor(metrics.cpuPercent)} transition-all duration-300`}
    style={{ width: `${Math.min(metrics.cpuPercent, 100)}%` }}
  />
</div>
```

**Color Coding** (Lines 30-34):
```typescript
const getResourceColor = (percent: number) => {
  if (percent > 80) return 'bg-error';      // Red
  if (percent > 60) return 'bg-warning';    // Yellow
  return 'bg-success';                      // Green
};
```

---

## Critical Findings

### 🔴 BLOCKER: Connection Status Flag

**Issue**: Only `metrics_update` events set `connected: true`

**Impact**:
```typescript
// Initial state
connected: false  // Shows "● OFFLINE"

// After state_change event
connected: false  // STILL OFFLINE (not updated!)

// After session_start event
connected: false  // STILL OFFLINE (not updated!)

// After metrics_update event
connected: true   // NOW LIVE (finally updated!)
```

**Problem Sequence**:
1. Daemon sends `state_change` event → UI shows "OFFLINE" (wrong)
2. Daemon sends `session_start` event → UI shows "OFFLINE" (wrong)
3. Eventually daemon sends `metrics_update` → UI shows "LIVE" (correct)

**Fix Required**: All event types should set `connected: true`

---

### ✅ No Render Blocking Conditions

**Positive Finding**: The LiveSession component has NO conditional rendering based on `connected` status.

**All metrics are always rendered**:
- WPM: Always shows (defaults to 0)
- Words: Always shows (defaults to 0)
- State icon: Always shows (defaults to 🎤)
- Resource meters: Always shows (defaults to 0%)

**This means**: Even with `connected: false`, the metrics ARE being displayed (just with zero/default values).

---

### 🟡 Potential Display Issues

1. **Zero Values Before First Update**:
   - WPM: 0
   - Words: 0
   - Duration: "00:00"
   - All resource meters at 0%

2. **State Icon Default**:
   - Shows 🎤 (green "idle") even before connection

3. **Misleading "OFFLINE" Badge**:
   - Shows red "OFFLINE" even when receiving events
   - Only turns "LIVE" after `metrics_update`

---

## Complete Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ DAEMON (Rust Backend)                                       │
│                                                              │
│ Emits: listen<BroadcastEvent>('metrics-event')             │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Event Emission
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ TAURI EVENT BRIDGE                                          │
│                                                              │
│ BroadcastEvent Union Type:                                  │
│   - metrics_update    ✅ Sets connected: true               │
│   - state_change      ⚠️  Does NOT set connected            │
│   - session_start     ⚠️  Does NOT set connected            │
│   - session_end       ⚠️  Does NOT set connected            │
│   - transcription     ⚠️  Does NOT set connected            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Event Reception
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ useMetrics() Hook                                           │
│                                                              │
│ File: /tauri-ui/src/hooks/useMetrics.ts                    │
│                                                              │
│ State:                                                       │
│   - metrics: LiveMetrics (11 fields)                        │
│   - transcriptions: TranscriptionItem[]                     │
│                                                              │
│ Event Handler: switch(payload.type)                         │
│   → Updates React state via setMetrics() / setTranscriptions()│
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Hook Return: { metrics, transcriptions }
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ App.tsx                                                     │
│                                                              │
│ File: /tauri-ui/src/App.tsx                                │
│                                                              │
│ Consumes: const { metrics, transcriptions } = useMetrics()  │
│                                                              │
│ Renders:                                                     │
│   1. Connection Badge (metrics.connected)                   │
│   2. Tab Bar (live/history/transcriptions)                  │
│   3. Content Area (conditional on activeTab)                │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Props: <LiveSession metrics={metrics} />
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ LiveSession.tsx                                             │
│                                                              │
│ File: /tauri-ui/src/components/LiveSession.tsx             │
│                                                              │
│ DOM Rendering (NO connection check):                        │
│                                                              │
│ 1. Status Header                                            │
│    ├─ Icon: getStateIcon() → 🔴/🟡/🎤                      │
│    └─ Text: metrics.state (UPPERCASE)                       │
│                                                              │
│ 2. Metric Cards Grid (3 columns)                            │
│    ├─ WPM: Math.round(metrics.wpm)                          │
│    ├─ Words: metrics.words                                  │
│    ├─ Latency: (metrics.latencyMs / 1000).toFixed(2) + "s"  │
│    ├─ Duration: metrics.duration                            │
│    ├─ Segments: metrics.segments                            │
│    └─ GPU Memory: (metrics.gpuMemoryMb / 1024).toFixed(1) + " GB" │
│                                                              │
│ 3. System Resources Panel                                   │
│    ├─ GPU Memory Bar                                        │
│    │  ├─ Text: {gpuMemoryMb} / 8000.0 MB ({percent}%)       │
│    │  └─ Progress Bar: width = min(gpuMemoryPercent, 100)%  │
│    │                   color = getResourceColor(percent)     │
│    │                                                          │
│    └─ CPU Usage Bar                                         │
│       ├─ Text: {cpuPercent} / 100.0 % ({percent}%)          │
│       └─ Progress Bar: width = min(cpuPercent, 100)%        │
│                        color = getResourceColor(percent)     │
└─────────────────────────────────────────────────────────────┘
                     │
                     │ Browser Rendering
                     ↓
                 ┌─────────┐
                 │   DOM   │
                 └─────────┘
```

---

## Event-to-DOM Timing Analysis

**Scenario 1: First Metrics Update**
```
Time 0ms:   Component mounts
            ├─ metrics.connected = false
            ├─ DOM shows: "● OFFLINE", 🎤 IDLE, all zeros
            └─ Event listener registered

Time 50ms:  Daemon sends state_change event
            ├─ Hook receives: { type: 'state_change', state: 'recording' }
            ├─ setMetrics() called: state = 'recording'
            ├─ metrics.connected = false (UNCHANGED!)
            └─ DOM shows: "● OFFLINE", 🔴 RECORDING, all zeros

Time 100ms: Daemon sends session_start event
            ├─ Hook receives: { type: 'session_start', session_id: 123 }
            ├─ setMetrics() called: sessionId = 123, counters reset
            ├─ metrics.connected = false (UNCHANGED!)
            └─ DOM shows: "● OFFLINE", 🔴 RECORDING, all zeros

Time 500ms: Daemon sends metrics_update event
            ├─ Hook receives: { type: 'metrics_update', state: 'recording', ... }
            ├─ setMetrics() called: ALL metrics updated + connected = true
            ├─ metrics.connected = true (FINALLY!)
            └─ DOM shows: "● LIVE", 🔴 RECORDING, real values

Time 1000ms: Daemon sends another metrics_update
             ├─ metrics.connected = true (maintained)
             └─ DOM updates with new values
```

**Key Insight**: There's a delay of 450ms where UI shows "OFFLINE" despite active connection.

---

## Recommendations

### 1. Fix Connection Status Logic
All event types should set `connected: true`:

```typescript
// useMetrics.ts - Lines 45-99
switch (payload.type) {
  case 'state_change':
    setMetrics((prev) => ({
      ...prev,
      state: payload.state,
      connected: true,  // ADD THIS
    }));
    break;

  case 'session_start':
    setTranscriptions([]);
    setMetrics((prev) => ({
      ...prev,
      sessionId: payload.session_id,
      segments: 0,
      words: 0,
      wpm: 0,
      duration: '00:00',
      connected: true,  // ADD THIS
    }));
    break;

  case 'session_end':
    setMetrics((prev) => ({
      ...prev,
      connected: true,  // ADD THIS (if daemon is still running)
    }));
    break;

  case 'transcription':
    setTranscriptions((prev) => [...prev, { ... }]);
    setMetrics((prev) => ({
      ...prev,
      connected: true,  // ADD THIS
    }));
    break;
}
```

### 2. Add Connection Timeout
Implement heartbeat detection:

```typescript
// Auto-disconnect after 5 seconds without events
useEffect(() => {
  let timeout: NodeJS.Timeout;

  const resetTimeout = () => {
    clearTimeout(timeout);
    timeout = setTimeout(() => {
      setMetrics(prev => ({ ...prev, connected: false }));
    }, 5000);
  };

  // Call resetTimeout() on every event reception

  return () => clearTimeout(timeout);
}, []);
```

### 3. Add Loading State
Distinguish between "not connected" and "waiting for first update":

```typescript
interface LiveMetrics {
  // ... existing fields
  connected: boolean;
  initializing: boolean;  // NEW
}

// Show different UI for initializing vs disconnected
```

---

## Summary

**Complete Data Flow**:
1. ✅ Daemon emits events via Tauri
2. ✅ useMetrics hook receives events
3. ✅ React state updates correctly
4. ✅ Props pass to LiveSession component
5. ✅ DOM renders all metrics unconditionally

**Critical Issue**:
- ⚠️ `connected` flag ONLY set by `metrics_update` events
- ⚠️ Other event types don't update connection status
- ⚠️ Causes misleading "OFFLINE" indicator

**Good News**:
- ✅ No render blocking conditions
- ✅ All metrics display regardless of connection status
- ✅ Default values (zeros) shown until first update

**Root Cause**: The `connected` flag is treated as a secondary metric rather than a connection health indicator. It should be set by ALL event types.
