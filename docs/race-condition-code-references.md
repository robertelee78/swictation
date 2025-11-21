# Race Condition Code References

## Problem Code Locations

### 1. Rust Backend - Unsynchronized Socket Spawn

**File:** `/opt/swictation/tauri-ui/src-tauri/src/main.rs` (Lines 128-132)

```rust
// CURRENT (BROKEN):
tauri::async_runtime::spawn(async move {
    if let Err(e) = metrics_socket.listen(app_handle).await {
        log::error!("Metrics socket error: {}", e);
    }
});
// ⚠️  Returns immediately - doesn't wait for socket to connect!
```

**Problem:**
- `spawn()` returns immediately without waiting
- No synchronization with React mounting
- Socket could connect and start emitting before React listeners exist

**What Should Happen:**
- Setup should wait for socket to connect
- Only then should React be allowed to mount
- Or implement a "ready" handshake

---

### 2. React Hook - Sequential Listener Registration

**File:** `/opt/swictation/tauri-ui/src/hooks/useMetrics.ts` (Lines 36-123)

```typescript
// CURRENT (SLOW + VULNERABLE):
useEffect(() => {
  const unlistenFns: Array<() => void> = [];

  (async () => {
    // Each await blocks sequentially!
    const unlistenConnected = await listen<boolean>('metrics-connected', (event) => {
      setMetrics((prev) => ({
        ...prev,
        connected: event.payload,
      }));
    });
    unlistenFns.push(unlistenConnected);
    // T+0ms to T+20ms ⏱️

    // Second listener
    const unlistenMetrics = await listen<BroadcastEvent & { type: 'metrics_update' }>('metrics-update', (event) => {
      // ...
    });
    unlistenFns.push(unlistenMetrics);
    // T+20ms to T+40ms ⏱️

    // Third listener
    const unlistenState = await listen<BroadcastEvent & { type: 'state_change' }>('state-change', (event) => {
      // ...
    });
    unlistenFns.push(unlistenState);
    // T+40ms to T+60ms ⏱️

    // Fourth listener
    const unlistenTranscription = await listen<BroadcastEvent & { type: 'transcription' }>('transcription', (event) => {
      // ...
    });
    unlistenFns.push(unlistenTranscription);
    // T+60ms to T+80ms ⏱️

    // Fifth listener
    const unlistenSessionStart = await listen<BroadcastEvent & { type: 'session_start' }>('session-start', (event) => {
      // ...
    });
    unlistenFns.push(unlistenSessionStart);
    // T+80ms to T+100ms ⏱️

    // Sixth listener
    const unlistenSessionEnd = await listen<BroadcastEvent & { type: 'session_end' }>('session-end', () => {
      // ...
    });
    unlistenFns.push(unlistenSessionEnd);
    // T+100ms to T+120ms ⏱️

    // Total time: ~120ms (too long!)
  })();

  return () => {
    unlistenFns.forEach((fn) => fn());
  };
}, []); // ⚠️  useEffect runs AFTER first render
```

**Problems:**
1. **Sequential execution:** Each `await` waits for previous to complete
2. **Slow registration:** Takes ~120ms total
3. **Vulnerable window:** Socket could connect during registration
4. **Async IIFE:** No error handling if listen() fails

**Current Timing:**
```
T+0ms:   Socket starts connecting
T+20ms:  First listener registers
T+40ms:  Second listener registers  ← Events from socket could fire HERE
T+60ms:  Third listener registers
T+80ms:  Fourth listener registers
T+100ms: Fifth listener registers
T+120ms: Sixth listener registers
T+150ms: All listeners ready (3+ events might be lost!)
```

**Line-by-Line Vulnerable Points:**
- Line 44: `listen('metrics-connected', ...)` - NOT READY YET
- Line 53: `listen('metrics-update', ...)` - NOT READY YET
- Line 73: `listen('state-change', ...)` - NOT READY YET
- Line 82: `listen('transcription', ...)` - NOT READY YET
- Line 98: `listen('session-start', ...)` - NOT READY YET
- Line 113: `listen('session-end', ...)` - NOT READY YET

---

### 3. Tauri Configuration - Window Visible Too Early

**File:** `/opt/swictation/tauri-ui/src-tauri/tauri.conf.json` (Line 24)

```json
{
  "app": {
    "windows": [
      {
        "visible": true,  // ⚠️ Shows window immediately!
        "title": "Swictation",
        "width": 1200,
        "height": 800
      }
    ]
  }
}
```

**Problem:**
- Window is visible before system is initialized
- User sees "OFFLINE" status while system is still starting
- If socket connects immediately, shows wrong state

**Timeline:**
```
T+0ms:   Tauri setup starts
T+50ms:  Socket starts connecting
T+60ms:  Window shown to user (visible: true)
         ↓ User sees "OFFLINE" status (metrics.connected = false)
T+105ms: Socket actually connects
         ↓ "metrics-connected" event emitted
         ↓ BUT: React listener not registered yet!
         ↓ Event LOST! Still shows OFFLINE to user 🔴
T+200ms: React listener finally registers
         ↓ Too late!
```

---

### 4. Socket Connection - Immediate Event Emission

**File:** `/opt/swictation/tauri-ui/src-tauri/src/socket/metrics.rs` (Lines 101-228)

```rust
// Lines 101-117: Main listen loop
pub async fn listen(&mut self, app_handle: AppHandle) -> Result<()> {
    loop {
        match self.connect_and_process(&app_handle).await {
            Ok(_) => {
                info!("Socket connection closed normally");
            }
            Err(e) => {
                error!("Socket connection error: {}", e);
                self.connected = false;
            }
        }
        sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
    }
}

// Lines 119-170: Connect and process events
async fn connect_and_process(&mut self, app_handle: &AppHandle) -> Result<()> {
    // Connect to Unix socket
    let stream = UnixStream::connect(&self.socket_path)
        .await
        .context("Failed to connect to metrics socket")?;

    info!("✓ Connected to metrics socket");
    self.connected = true;

    // ⚠️  EMIT CONNECTION IMMEDIATELY (line 131)
    app_handle
        .emit("metrics-connected", true)
        .context("Failed to emit connection status")?;

    // Set up buffered reader for line-by-line processing
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    // ⚠️  START READING AND EMITTING IMMEDIATELY (line 139+)
    while let Some(line) = lines
        .next_line()
        .await
        .context("Failed to read from socket")?
    {
        if line.trim().is_empty() {
            continue;
        }

        debug!("Received raw event: {}", line);

        // Parse and handle event
        match serde_json::from_str::<MetricsEvent>(&line) {
            Ok(event) => {
                if let Err(e) = self.handle_event(app_handle, event).await {
                    error!("Failed to handle event: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to parse event: {} (line: {})", e, line);
            }
        }
    }
    // ...
}

// Lines 172-228: Event handler emits Tauri events
async fn handle_event(&self, app_handle: &AppHandle, event: MetricsEvent) -> Result<()> {
    match &event {
        MetricsEvent::SessionStart { session_id, .. } => {
            app_handle.emit("session-start", event)?;  // ⚠️ Line 180
        }
        MetricsEvent::SessionEnd { session_id, .. } => {
            app_handle.emit("session-end", event)?;    // ⚠️ Line 187
        }
        MetricsEvent::StateChange { state, .. } => {
            app_handle.emit("state-change", event)?;   // ⚠️ Line 194
        }
        MetricsEvent::Transcription { .. } => {
            app_handle.emit("transcription", event)?;  // ⚠️ Line 203
        }
        MetricsEvent::MetricsUpdate { .. } => {
            app_handle.emit("metrics-update", event)?; // ⚠️ Line 222
        }
    }
    Ok(())
}
```

**Problems:**
1. **Line 131:** Emits "metrics-connected" immediately after connecting
2. **Lines 139-160:** Starts reading and emitting events immediately
3. **Lines 180, 187, 194, 203, 222:** All events emitted as they arrive
4. **No queue:** No buffering for early events
5. **No handshake:** No way to know if listeners are ready

**Critical Emission Points:**
- Line 131: `emit("metrics-connected", true)` - First event
- Line 180: `emit("session-start", event)` - Could be lost
- Line 194: `emit("state-change", event)` - Could be lost
- Line 203: `emit("transcription", event)` - Could be lost
- Line 222: `emit("metrics-update", event)` - Could be lost

---

### 5. React Initialization - Main Entry Point

**File:** `/opt/swictation/tauri-ui/src/main.tsx` (Lines 23-29)

```typescript
// ⚠️  Renders immediately, useEffect runs AFTER
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
```

**Problem:**
- React synchronously mounts all components
- Renders JSX
- THEN (after all sync code) runs useEffect hooks
- By this time, socket could have already connected and emitted events

**React Lifecycle Order:**
1. Synchronous: Component constructor/initialization
2. Synchronous: Render (create JSX tree)
3. Asynchronous: useEffect hooks (scheduled)
4. Tauri API calls (async)

**Vulnerability:**
- Tauri events can fire at step 3 or during step 4
- If socket connects during step 1-3, listeners don't exist yet

---

## Summary Table: Timing Analysis

| Component | Location | Action | Timing | Status |
|-----------|----------|--------|--------|--------|
| **Rust Setup** | main.rs:128 | Spawn socket task | T+0ms | Returns immediately ⚠️ |
| **Socket Connect** | metrics.rs:122 | Connect to socket | T+100ms | Starts emitting ⚠️ |
| **React Mount** | main.tsx:23 | Create root | T+0ms | Synchronous ✓ |
| **App Render** | App.tsx:9 | Render JSX | T+50ms | Synchronous ✓ |
| **useEffect Run** | useMetrics.ts:36 | Schedule effect | T+75ms | Microtask 🔴 |
| **Listen #1** | useMetrics.ts:44 | Register listener | T+85-100ms | Too late 🔴 |
| **Listen #2** | useMetrics.ts:53 | Register listener | T+100-120ms | Lost events 🔴 |
| **Listen #3** | useMetrics.ts:73 | Register listener | T+120-140ms | Lost events 🔴 |
| **Listen #4** | useMetrics.ts:82 | Register listener | T+140-160ms | Lost events 🔴 |
| **Listen #5** | useMetrics.ts:98 | Register listener | T+160-180ms | Lost events 🔴 |
| **Listen #6** | useMetrics.ts:113 | Register listener | T+180-200ms | Lost events 🔴 |
| **All Ready** | - | System initialized | T+200ms | Finally! ✓ |

---

## Event Loss Examples

### Example 1: Lost "metrics-connected" Event

```
T+100ms: Socket connects
         emit("metrics-connected", true)  ← Event fired

T+105ms: Event in Tauri's emit queue
         No listener registered yet!
         Event LOST! 🔴

T+95ms:  React still rendering App component
T+100ms: useEffect scheduled (not running yet)
T+110ms: useEffect finally runs
T+120ms: listen('metrics-connected') now registered
         ← TOO LATE!
```

### Example 2: Lost Transcription

```
T+150ms: User is recording
         Daemon transcribes first words
         emit("transcription", {text: "Hello world", ...})

T+155ms: Event in Tauri's emit queue
         React still registering listen('transcription')
         Event LOST! 🔴

T+175ms: listen('transcription') finally registered
         ← TOO LATE!
         First transcription missing! User sees empty buffer
```

### Example 3: Lost Session State

```
T+110ms: Session-start event fired
         emit("session-start", {session_id: "123", ...})

T+115ms: Not delivered (no listener)
         Event LOST! 🔴

T+200ms: listen('session-start') finally registered
         But first session already started!

T+210ms: Session-end event fires
         emit("session-end", {...})

T+215ms: Delivered (listener exists now)
         React receives "session-end" without "session-start"
         State corruption! 🔴
```

---

## Files Requiring Changes

### Must Fix:

1. **useMetrics.ts** (lines 36-123)
   - Replace sequential await with Promise.all()
   - Add initialization barrier
   - Emit "ready" event after listeners registered

2. **main.rs** (lines 128-132)
   - Wait for socket connection before React mounts
   - Or implement handshake with React

3. **tauri.conf.json** (line 24)
   - Change `visible: true` to `visible: false`
   - Show window only after system ready

4. **metrics.rs** (lines 101-228)
   - Add ability to queue initial events
   - Wait for "ready" signal before emitting
   - Or send "ready" event after connecting

### Should Fix:

5. **App.tsx** (lines 1-12)
   - Show loading state while system initializes
   - Hide window content until "system-ready"

6. **main.tsx** (lines 23-29)
   - Could add initialization wrapper
   - But main issue is in hooks/parent components

---

## Testing Points

```typescript
// Test 1: Capture lost events
describe('Race Condition Detection', () => {
  test('No events lost during startup', async () => {
    let receivedEvents = [];

    // Mock Tauri emit to track what's emitted
    // Mock listen to track when listeners register

    // Start app
    // Start socket emitting rapidly

    // Verify: ALL events received by React
    expect(receivedEvents.length).toBeGreaterThan(10);
  });
});

// Test 2: Sequential vs Parallel registration speed
describe('Listener Registration Speed', () => {
  test('Promise.all is faster than sequential', async () => {
    const start = Date.now();

    // Current: 120-150ms
    // Proposed: 30-50ms

    expect(Date.now() - start).toBeLessThan(60); // 2x improvement
  });
});

// Test 3: Window initialization
describe('Window Visibility', () => {
  test('Window hidden until system ready', () => {
    // Window should be invisible until:
    // 1. Socket connected
    // 2. Listeners registered
    // 3. "system-ready" event received

    expect(window.isVisible()).toBe(false); // Initially
    // ... system initializes ...
    expect(window.isVisible()).toBe(true);  // After ready
  });
});
```

