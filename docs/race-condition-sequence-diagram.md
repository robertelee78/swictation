# Race Condition Sequence Diagram

## Current (Broken) Initialization Flow

```
Timeline (Milliseconds)

T+0ms     ┌─────────────────────────────────────────────────────────────┐
          │ TAURI MAIN THREAD STARTS                                    │
          │ main.rs: fn main() executes                                 │
          └─────────────────────────────────────────────────────────────┘
          │
          ├─ Tray icon creation (optional)
          │
          ├─ Database initialization
          │
          └──> tauri::Builder::default().setup() at line 27
                └─ Lines 29-121: Setup logic
                │
T+50ms    └──> At line 128: tauri::async_runtime::spawn(async move { ... })

             ⚠️  CRITICAL: spawn() returns IMMEDIATELY
                Does NOT wait for socket to connect

T+55ms    ┌──────────────────────────┐
          │ SPAWNED TASK (background) │
          │ MetricsSocket::listen()   │
          │ (starting now)            │
          └──────────────────────────┘
               │
               └─> Connecting to Unix socket (socket/metrics.rs:122-124)

T+60ms    └──> setup() returns Ok(())

          │ BROWSER/REACT THREAD STARTS
          │
T+65ms    ├──> main.tsx: ReactDOM.createRoot()
          │
T+70ms    ├──> React.StrictMode wrapper mounts
          │
T+75ms    ├──> <ErrorBoundary> component mounts
          │
T+80ms    ├──> <App> component mounts
          │     ├─ useState hooks initialize
          │     ├─ const { metrics, transcriptions } = useMetrics() called
          │     └─ Return JSX (render commit)
          │
T+85ms    ├──> useEffect(() => { ... }, []) RUNS (BACKGROUND TASK)
          │     └─ Async IIFE starts
          │        └─ First await listen(...) call begins
          │
          │ BACK TO SPAWNED METRICS SOCKET TASK
          │
T+100ms   │ Socket connection established ✓
          │ (socket/metrics.rs:122-124 completes)
          │
T+105ms   │ "metrics-connected" event emitted (line 131)
          │ ⚠️  NO LISTENER REGISTERED YET!
          │ EVENT LOST! 🔴
          │
T+110ms   │ Metrics update event arrives from daemon
          │ ⚠️  STILL NO LISTENER!
          │ EVENT LOST! 🔴
          │
T+115ms   │ Transcription event arrives
          │ ⚠️  STILL NO LISTENER!
          │ EVENT LOST! 🔴
          │
          │ BACK TO REACT useEffect
          │
T+120ms   │ First listen() Promise resolves (metrics-connected listener registered)
          │ But we already lost the metrics-connected event!
          │
T+125ms   │ Second listen() call starts (for metrics-update)
          │
T+135ms   │ Second listen() resolves (metrics-update listener registered)
          │ But we already lost the metrics-update event!
          │
T+145ms   │ Third listen() call starts (for state-change)
          │
T+155ms   │ ... pattern continues ...
          │
T+200ms   │ All 6 listeners finally registered
          │ ✓ NOW we're ready
          │
          │ 🔴 RESULT: First 100ms of events LOST!
```

## Event Loss Window

```
VULNERABLE WINDOW:
┌────────────────────────────────────────────────────────────────┐
│ Socket Connected        React Listeners Ready                 │
│ T+100ms                 T+200ms                               │
│ ▼                       ▼                                     │
│ ║════════════════════════════════════════════════════════════ │
│ ║                  100ms UNPROTECTED WINDOW                  │
│ ║  ALL EVENTS EMITTED HERE ARE LOST! 🔴                     │
│ ║════════════════════════════════════════════════════════════ │
└────────────────────────────────────────────────────────────────┘

Actual daemon behavior during this window:
- T+100ms: "metrics-connected" → LOST 🔴
- T+105ms: "session-start" (if session in progress) → LOST 🔴
- T+110ms: "metrics-update" → LOST 🔴
- T+115ms: "metrics-update" → LOST 🔴
- T+120ms: "metrics-update" → LOST 🔴
- T+130ms: "transcription" → LOST 🔴
- T+150ms: "state-change" → LOST 🔴
- T+160ms: "metrics-update" → LOST 🔴
- T+180ms: "transcription" → LOST 🔴
- T+200ms: "metrics-update" ✓ RECEIVED (finally!)
```

## Code Flow Diagram

```
main.rs (Tauri Backend)
│
├─ setup() hook at line 27
│  │
│  ├─ Database init (lines 103-121)
│  │
│  └─ Line 128-132:
│     tauri::async_runtime::spawn(async move {
│         metrics_socket.listen(app_handle).await
│     });
│     ↓
│     Returns IMMEDIATELY ⚠️
│     (doesn't wait for socket connection)
│
└─ rest of setup continues...
   Emits signal to proceed to React


main.tsx (React Frontend)
│
├─ DOM ready
│
├─ React mounts ErrorBoundary
│  └─ Mounts App component
│     ├─ Call useMetrics() hook
│     ├─ Return JSX
│     └─ Component ready for first render
│
├─ Commit phase (browser paints)
│
└─ useEffect hook runs (schedules on next tick)
   └─ const { metrics, transcriptions } = useMetrics();
      useEffect(() => {              ← LINE 36
        (async () => {               ← LINE 42
          const unlistenConnected = await listen('metrics-connected', ...)
                                                          ↑ LINE 44
          // Next listeners sequential
          const unlistenMetrics = await listen('metrics-update', ...)
                                                   ↑ LINE 53
          const unlistenState = await listen('state-change', ...)
                                            ↑ LINE 73
          const unlistenTranscription = await listen('transcription', ...)
                                                     ↑ LINE 82
          const unlistenSessionStart = await listen('session-start', ...)
                                                   ↑ LINE 98
          const unlistenSessionEnd = await listen('session-end', ...)
                                                ↑ LINE 113
        })();
      }, []);  ← LINE 123


socket/metrics.rs
│
├─ pub async fn listen(app_handle) at line 101
│  │
│  └─ loop {
│     ├─ connect_and_process(app_handle) at line 103
│     │  │
│     │  └─ Connect to Unix socket (line 122-124)
│     │     │
│     │     ├─ Emit "metrics-connected" (line 131)  ← Might fire HERE
│     │     │
│     │     └─ while let Some(line) = lines.next_line() (line 139)
│     │        │
│     │        └─ handle_event(app_handle, event) (line 153)
│     │           ├─ Emit "session-start" (line 180)
│     │           ├─ Emit "session-end" (line 187)
│     │           ├─ Emit "state-change" (line 194)
│     │           ├─ Emit "transcription" (line 203)
│     │           └─ Emit "metrics-update" (line 222)
│     │
│     └─ On disconnect, sleep 5 seconds (line 115)
│
└─ All events above emitted BEFORE React listeners register
```

## Sequential Listener Registration Problem

```
Current Implementation (SLOW):
┌──────────────┐
│ listen()     │ Waits for first
│ metrics-     │ listener to register
│ connected    │ T+0ms to T+20ms
├──────────────┤
│              │ THEN starts second
│ listen()     │ T+20ms to T+40ms
│ metrics-     │
│ update       │
├──────────────┤
│              │ THEN starts third
│ listen()     │ T+40ms to T+60ms
│ state-       │
│ change       │
├──────────────┤
│ ...5 more... │ Each waits for previous
│              │ T+60ms to T+200ms
└──────────────┘

Total registration time: ~200ms


Proposed Optimization (FAST):
┌──────────────────────────────────────┐
│ Promise.all([                        │
│   listen('metrics-connected'),       │
│   listen('metrics-update'),          │
│   listen('state-change'),            │
│   listen('transcription'),           │
│   listen('session-start'),           │
│   listen('session-end')              │
│ ])                                   │
│ ↓ All register in PARALLEL           │
│ ↓ T+0ms to T+30ms (simultaneous)    │
└──────────────────────────────────────┘

Total registration time: ~30ms (6x faster)
```

## Missing Synchronization Handshake

```
WHAT SHOULD HAPPEN (Correct Design):

Tauri Backend                 React Frontend
     │                              │
     │  (1) Start setup()           │
     │  └──────────────────────────▶│ (2) Start React mount
     │                              │
     │  (3) Spawn socket listener   │
     │  └──────────────────────────▶│ (4) Mounts App → useMetrics
     │                              │
     ├─ WAIT for frontend ready     │ (5) Register all listeners
     │                              │     in parallel
     │                              ├──────────────────────────▶
     │◀─ "system-ready" event       │ (6) Send "ready" signal
     │                              │
     │  (7) NOW send events         │
     ├──────────────────────────────▶│ (8) Receive buffered events
     │  - metrics-connected         │     - All events queued
     │  - session-start             │     - All events received
     │  - metrics-update            │     - UI consistent
     │  - transcription             │
     │  - state-change              │
     │                              │


WHAT ACTUALLY HAPPENS (Current Broken Design):

Tauri Backend                 React Frontend
     │                              │
     │  (1) Start setup()           │
     │  └──────────────────────────▶│ (2) Start React mount
     │                              │
     │  (3) Spawn socket listener   │
     │      (returns immediately)   │
     │                              │
     │  (4) Socket connects         │ (5) Mounting...
     │      and starts emitting     │
     │  ├─ metrics-connected        │
     │  │  🔴 NO LISTENER YET!      │
     │  │  EVENT LOST!              │
     │  │                           │
     │  ├─ session-start            │
     │  │  🔴 NO LISTENER YET!      │ (6) Still mounting...
     │  │  EVENT LOST!              │
     │  │                           │
     │  ├─ metrics-update           │
     │  │  🔴 NO LISTENER YET!      │
     │  │  EVENT LOST!              │
     │  │                           │
     │  └─ transcription            │
     │     🔴 NO LISTENER YET!      │ (7) Finally: useEffect runs
     │     EVENT LOST!              │     Listeners registering...
     │                              │
     │  (8) More events...          │ (8) Still registering...
     │      But listeners exist     │
     │      now (mostly)            │
     │                              │
     └──────────────────────────────▶│ (9) Ready, but too late!
```

## Why This Happens

The problem stems from **asynchronous execution in multiple threads/event loops:**

```
Rust (Tokio Runtime):
  - spawned task runs on async executor
  - doesn't block main thread
  - main thread returns immediately

Browser (JavaScript Event Loop):
  - React mounts synchronously
  - useEffect runs asynchronously (microtask)
  - Listener registration awaits Promises

IPC Events (Tauri):
  - Events can be emitted anytime after socket connects
  - No ordering guarantee with React lifecycle
  - Lost if no listener registered

Result: **RACE CONDITION** - who initializes first?
  If socket → React: Events lost 🔴
  If React → socket: Works correctly ✓ (but unpredictable)
```

## Vulnerable Timeline

```
BEST CASE SCENARIO:
React mounts (50ms) < Socket connects (100ms)
→ Listeners ready when events arrive ✓

NORMAL CASE SCENARIO:
Socket connects (100ms) < React useEffect completes (200ms)
→ 100ms window of lost events 🔴

WORST CASE SCENARIO:
Socket is super fast (10ms) < React mounts (50ms)
→ ALL initial events lost! 🔴
```

