# macOS Toggle Command Hang - Root Cause Analysis

## Investigation Date
2025-11-27

## Symptoms
- `swictation toggle` command hangs intermittently on macOS
- Works fine on fresh daemon start (0.26s-0.46s response)
- After some recording sessions, daemon becomes unresponsive
- CLI hangs waiting for IPC response
- All subsequent IPC commands hang
- Daemon restart fixes it temporarily

## Code Path Analysis

### 1. IPC Request Flow
```
CLI (swictation toggle)
  ↓ Unix socket (/tmp/swictation.sock)
main.rs:595 → ipc_server.accept()
  ↓
ipc.rs:86 → handle_connection()
ipc.rs:100 → daemon.toggle().await
  ↓
main.rs:111 → async fn toggle()
```

### 2. Toggle Function Lock Acquisition Order

**Location:** `rust-crates/swictation-daemon/src/main.rs:111-171`

```rust
async fn toggle(&self) -> Result<String> {
    // LOCK 1: State (RwLock write)
    let mut state = self.state.write().await;

    // LOCK 2: Pipeline (RwLock write)
    let mut pipeline = self.pipeline.write().await;

    // LOCK 3: Session ID (RwLock write)
    let mut session_id = self.session_id.write().await;

    match *state {
        DaemonState::Recording => {
            // CRITICAL PATH:
            pipeline.stop_recording().await?;  // Line 144

            // LOCK 4: Metrics (std::sync::Mutex)
            let metrics = pipeline.get_metrics();
            let session_metrics = metrics.lock().unwrap().end_session()?;  // Line 152

            // LOCK 5: Broadcaster (async operations)
            self.broadcaster.end_session(sid).await;  // Line 156
            self.broadcaster.broadcast_state_change(...).await;  // Line 161-163
        }
    }
}
```

**CRITICAL ISSUE**: This function holds THREE RwLock write locks while performing async operations!

### 3. stop_recording() Lock Chain

**Location:** `rust-crates/swictation-daemon/src/pipeline.rs:630-780`

The `#[allow(clippy::await_holding_lock)]` annotation at line 629 is a **RED FLAG** - it explicitly suppresses warnings about holding locks across await points.

```rust
#[allow(clippy::await_holding_lock)]  // WARNING: Suppressed deadlock warning!
pub async fn stop_recording(&mut self) -> Result<()> {
    // LOCK 6: Audio (std::sync::Mutex)
    self.audio.lock().unwrap().stop()?;  // Line 636

    // LOCK 7: VAD (std::sync::Mutex)
    if let Some(...) = self.vad.lock().unwrap().flush() {  // Line 642

        // LOCK 8: STT (std::sync::Mutex) - HELD FOR LONG TIME
        let mut stt_lock = self.stt.lock()?;  // Line 663
        let result = stt_lock.recognize(&speech_samples)?;  // Line 672 - SLOW!
        // stt_lock held during expensive STT inference

        // LOCK 9: Session ID (std::sync::Mutex)
        let current_session_id = *self.session_id.lock().unwrap();  // Line 723

        // LOCK 10: Metrics (std::sync::Mutex)
        self.metrics.lock().unwrap().add_segment(segment)?;  // Line 747

        // LOCK 11: Broadcaster (std::sync::Mutex)
        if let Some(ref broadcaster_ref) = *self.broadcaster.lock().unwrap() {  // Line 752
            // Spawns async task while holding broadcaster lock
            tokio::spawn(async move { ... }).await;  // Line 754-767
        }

        // LOCK 12: Transcription channel (async send)
        self.tx.send(Ok(capitalized)).await?;  // Line 772 - AWAIT POINT!
    }
}
```

## Root Cause: Lock Ordering Deadlock

### Deadlock Scenario

**Thread 1 (IPC Handler - toggle request):**
```
1. Acquires state.write() lock
2. Acquires pipeline.write() lock
3. Acquires session_id.write() lock
4. Calls pipeline.stop_recording().await
5. Inside stop_recording:
   - Acquires audio.lock()
   - Acquires vad.lock()
   - Acquires stt.lock() ← BLOCKS HERE if STT is busy
   - Tries to acquire metrics.lock()
   - Tries to acquire broadcaster.lock()
```

**Thread 2 (Metrics updater - background task):**
```
1. Runs every 1 second (main.rs:401-424)
2. Acquires metrics.lock() for update_system_metrics()
3. Acquires metrics.lock() for update_recording_duration()
4. Acquires metrics.lock() for get_realtime_metrics()
5. Tries to acquire daemon_state.read() ← BLOCKS because Thread 1 holds write lock!
6. Needs to acquire broadcaster lock for broadcast
```

**Circular dependency:**
- Thread 1: Holds `pipeline.write()` → Waiting for `metrics.lock()`
- Thread 2: Holds `metrics.lock()` → Waiting for `daemon_state.read()` (blocked by Thread 1's write lock)

### Why It's Intermittent

The deadlock only occurs when:
1. A toggle command arrives (Thread 1 starts)
2. **AND** the metrics updater fires at the same time (Thread 2)
3. **AND** there's flushed audio to process (makes Thread 1 hold locks longer)
4. **AND** STT inference is slow (increases window for race condition)

After some recording sessions, the probability increases because:
- More system load (CPU/GPU busy)
- STT model cache effects
- CoreAudio stream state on macOS

## macOS-Specific Factors

### CoreAudio Stream Management

**Location:** `rust-crates/swictation-audio/src/capture.rs:274-467`

The audio stream is created via `cpal` which uses CoreAudio on macOS:

```rust
pub fn start(&mut self) -> Result<()> {
    // ...
    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // This runs in CoreAudio's real-time thread!
            // ...
            callback(chunk);  // Triggers audio_tx.try_send()
        },
        |err| { eprintln!("Audio stream error: {}", err); },
        None,
    )?;

    stream.play()?;
    self.stream = Some(stream);  // Line 460
}

pub fn stop(&mut self) -> Result<()> {
    self.is_recording.store(false, Ordering::Relaxed);

    // Stop and drop stream
    if let Some(stream) = self.stream.take() {
        drop(stream);  // Line 575 - Synchronous drop!
    }
}
```

### Potential macOS CoreAudio Hang

The `drop(stream)` call at line 575 synchronously tears down the CoreAudio stream. On macOS, this can block if:

1. **CoreAudio callback is running**: The audio thread might be in the middle of processing
2. **Stream state transition**: CoreAudio HAL requires proper state machine transitions
3. **Hardware buffer drain**: Waiting for audio hardware to finish
4. **Framework lock contention**: CoreAudio framework internal locks

This is especially problematic because it happens **while holding the `audio.lock()` mutex** inside `stop_recording()`, which is called **while holding multiple RwLocks** in `toggle()`.

## Lock Holding Analysis

### Locks Held Across Await Points

**Violation:** `toggle()` holds THREE tokio::sync::RwLock write locks while calling async functions:

1. `state.write().await` - Held from line 112 until function end
2. `pipeline.write().await` - Held from line 113 until function end
3. `session_id.write().await` - Held from line 114 until function end

**Consequence:** Any other task trying to read daemon state (metrics updater, IPC status request) will block.

### Locks Held During Expensive Operations

**Violation:** `stop_recording()` holds `stt.lock()` during STT inference (line 663-683):

```rust
let mut stt_lock = self.stt.lock()?;
let result = stt_lock.recognize(&speech_samples)?;  // Expensive! 50-500ms
```

**Consequence:** Any audio processing task needing STT access will block.

## Additional Issues Found

### 1. std::sync::Mutex in Async Context

The codebase mixes `std::sync::Mutex` with async/await:
- `pipeline.metrics: Arc<Mutex<MetricsCollector>>`
- `pipeline.audio: Arc<Mutex<AudioCapture>>`
- `pipeline.vad: Arc<Mutex<VadDetector>>`
- `pipeline.stt: Arc<Mutex<SttEngine>>`

**Problem:** `std::sync::Mutex::lock()` is blocking and can cause executor thread starvation in async contexts.

**Should use:** `tokio::sync::Mutex` or separate blocking operations to dedicated thread pool.

### 2. Suppressed Clippy Warning

The `#[allow(clippy::await_holding_lock)]` at `pipeline.rs:629` suppresses a critical warning about holding locks across await points. This is the exact pattern that causes the deadlock.

### 3. Broadcaster Lock Contention

**Location:** `broadcaster.rs:210-214`

```rust
pub async fn add_transcription(...) {
    self.transcription_buffer.write().await.push(segment);  // Lock held
    if let Err(e) = self.client_manager.broadcast(&event).await {  // Network I/O!
        // ...
    }
}
```

The broadcaster holds locks while doing network I/O to multiple clients. If a client is slow or disconnected, this blocks the entire broadcaster.

### 4. Channel Backpressure

**Location:** `pipeline.rs:772`

```rust
if let Err(e) = self.tx.send(Ok(capitalized)).await {
    // Bounded channel - this can block if consumer is slow!
}
```

This `await` point happens while holding the `stop_recording()` context, which includes all the locks mentioned above.

## Recommended Fix Approach

### Priority 1: Remove Lock Holding Across Await Points

**In `main.rs:toggle()`:**
```rust
async fn toggle(&self) -> Result<String> {
    // Acquire locks only for reading/modifying state
    let current_state = {
        let state = self.state.read().await;
        *state
    };  // Lock released immediately

    match current_state {
        DaemonState::Idle => {
            // Start recording logic without holding locks
            let sid = {
                let pipeline = self.pipeline.read().await;
                let metrics = pipeline.get_metrics();
                metrics.lock().unwrap().start_session()?
            };

            // Now acquire write lock only for state change
            {
                let mut pipeline = self.pipeline.write().await;
                pipeline.set_session_id(sid);
                pipeline.start_recording().await?;
            }

            // Update state last
            {
                let mut state = self.state.write().await;
                *state = DaemonState::Recording;
            }

            // Broadcast without holding locks
            self.broadcaster.start_session(sid).await;
            // ...
        }
        // Similar for Recording → Idle transition
    }
}
```

### Priority 2: Replace std::sync::Mutex with tokio::sync::Mutex

In `pipeline.rs`, change:
```rust
// FROM:
audio: Arc<Mutex<AudioCapture>>,
vad: Arc<Mutex<VadDetector>>,
stt: Arc<Mutex<SttEngine>>,
metrics: Arc<Mutex<MetricsCollector>>,

// TO:
audio: Arc<tokio::sync::Mutex<AudioCapture>>,
vad: Arc<tokio::sync::Mutex<VadDetector>>,
stt: Arc<tokio::sync::Mutex<SttEngine>>,
metrics: Arc<tokio::sync::Mutex<MetricsCollector>>,
```

### Priority 3: Offload STT Inference to Blocking Thread Pool

```rust
let stt_result = tokio::task::spawn_blocking({
    let stt = self.stt.clone();
    let samples = speech_samples.clone();
    move || {
        let mut stt_lock = stt.blocking_lock();
        stt_lock.recognize(&samples)
    }
}).await??;
```

### Priority 4: Fix CoreAudio Stream Drop

```rust
// In AudioCapture::stop()
pub fn stop(&mut self) -> Result<Vec<f32>> {
    self.is_recording.store(false, Ordering::Relaxed);

    // Give CoreAudio time to finish current callback
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Stop and drop stream on dedicated thread to avoid blocking
    if let Some(stream) = self.stream.take() {
        std::thread::spawn(move || {
            drop(stream);  // Drop in dedicated thread
        });
    }

    // ...
}
```

### Priority 5: Minimize Lock Scope in stop_recording()

Remove the `#[allow(clippy::await_holding_lock)]` and fix the actual issue:

```rust
pub async fn stop_recording(&mut self) -> Result<()> {
    if !self.is_recording {
        return Ok(());
    }

    self.is_recording = false;

    // Stop audio without holding lock
    {
        self.audio.lock().await.stop()?;
    }

    // Flush VAD without holding lock
    let speech_samples = {
        self.vad.lock().await.flush()
    };

    if let Some(VadResult::Speech { samples, .. }) = speech_samples {
        // Process on blocking thread pool (doesn't hold any locks)
        let text = tokio::task::spawn_blocking({
            let stt = self.stt.clone();
            let samples = samples.clone();
            move || {
                let mut stt_lock = stt.blocking_lock();
                stt_lock.recognize(&samples)
            }
        }).await??;

        // Only lock metrics when actually adding segment
        {
            let segment = /* build segment */;
            self.metrics.lock().await.add_segment(segment)?;
        }

        // Send to channel without holding any locks
        self.tx.send(Ok(text)).await?;
    }

    Ok(())
}
```

## Testing Recommendations

### Reproduce the Deadlock

1. **Stress test:**
   ```bash
   while true; do
     swictation toggle &
     sleep 0.1
     swictation toggle &
     sleep 0.5
   done
   ```

2. **With recording load:**
   ```bash
   # Terminal 1: Continuous recording
   while true; do
     swictation toggle
     sleep 5  # Record for 5 seconds
     swictation toggle
     sleep 1
   done

   # Terminal 2: Concurrent status checks
   while true; do
     swictation status
     sleep 0.1
   done
   ```

3. **Monitor with:**
   ```bash
   # Check for hung processes
   ps aux | grep swictation

   # Check daemon logs
   journalctl -u swictation -f

   # Check socket connections
   lsof /tmp/swictation.sock
   ```

### Verify Fix

After implementing fixes, run the stress test for **at least 30 minutes** without hangs.

## Specific Code Locations to Fix

1. **main.rs:111-171** - `toggle()` function lock ordering
2. **pipeline.rs:629** - Remove `#[allow(clippy::await_holding_lock)]`
3. **pipeline.rs:630-780** - `stop_recording()` lock minimization
4. **pipeline.rs:24-52** - Change `std::sync::Mutex` to `tokio::sync::Mutex`
5. **capture.rs:564-590** - `AudioCapture::stop()` async drop handling
6. **main.rs:401-424** - Metrics updater lock acquisition order

## Severity Assessment

**Severity:** CRITICAL

**Impact:**
- Complete daemon hang requiring restart
- Data loss (in-progress recording lost)
- Poor user experience (intermittent failures)
- macOS-specific issue affects all macOS users

**Likelihood:** MODERATE-HIGH
- Increases with system load
- Increases with longer recording sessions
- Race condition timing-dependent

**Priority:** P0 - Fix immediately before next release

## Conclusion

The root cause is a **lock ordering deadlock** between:
1. IPC handler holding multiple RwLock write locks during toggle
2. Background metrics updater trying to read state while holding metrics lock
3. Audio stream teardown blocking in CoreAudio framework

The fix requires:
1. Minimize lock scope - acquire and release immediately
2. Never hold locks across await points
3. Use appropriate async-aware primitives (tokio::sync::Mutex)
4. Offload blocking operations to dedicated thread pool
5. Handle CoreAudio stream lifecycle on separate thread

The `#[allow(clippy::await_holding_lock)]` annotation was hiding this critical bug. **Never suppress this warning** without careful analysis.
