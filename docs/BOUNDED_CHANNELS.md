# Bounded Channels and Backpressure Implementation

## Overview

This document describes the implementation of bounded channels and backpressure mechanisms to address Issues #1 and #2 in the Swictation audio processing pipeline.

## Problem Statement

**Memory Safety Issues:**
1. **Unbounded Channels (Issue #1):** Audio and transcription channels had no capacity limits, allowing memory exhaustion if consumers couldn't keep up with producers
2. **No Backpressure (Issue #2):** Failed channel sends were silently ignored, providing no feedback to slow down production

**Risk Scenarios:**
- Fast speaker generates audio faster than STT can process → unbounded memory growth
- Network lag delays text injection → transcription queue grows unboundedly
- System under load → both pipelines accumulate data without bounds

## Solution

### Approach: Bounded Channels with Smart Backpressure

We implemented **bounded channels** with **capacity limits** and **graceful degradation**:

1. ✅ **Memory-bounded** - Hard limits prevent unbounded growth
2. ✅ **Backpressure-aware** - Producers slow down when consumers fall behind
3. ✅ **Observable** - Metrics track dropped chunks and queue pressure
4. ✅ **Graceful degradation** - Audio drops chunks rather than blocking the thread

### Channel Capacities

| Channel | Old | New | Rationale |
|---------|-----|-----|-----------|
| Transcription results | Unbounded | 100 results | ~1-2 minutes of buffering at typical speech rate (60 WPM) |
| Audio chunks | Unbounded | 20 chunks | 10 seconds buffer (20 × 0.5s chunks) |

### Backpressure Mechanisms

#### Audio Callback (Producer)

**Challenge:** Audio capture runs in a real-time thread that **cannot block**. Blocking would cause audio glitches.

**Solution:** Use `try_send()` for non-blocking sends with dropped chunk tracking:

```rust
// Audio callback (cpal real-time thread)
audio.set_chunk_callback(move |chunk| {
    match audio_tx_clone.try_send(chunk) {
        Ok(_) => {
            // Success: chunk queued for processing
        }
        Err(mpsc::error::TrySendError::Full(_)) => {
            // Backpressure: drop chunk and track metric
            dropped_chunks_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            eprintln!("WARNING: Audio chunk dropped (processing too slow)");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            // Recording stopped
        }
    }
});
```

**Why `try_send()` instead of `send().await`:**
- Audio callback is synchronous (not async)
- Must never block (real-time constraints)
- Dropping audio chunks is better than blocking and causing glitches

#### VAD/STT Processing (Consumer → Producer)

**Challenge:** VAD/STT processing reads from bounded audio channel and writes to bounded transcription channel.

**Solution:** Use `send().await` for backpressure propagation:

```rust
// Send transcription (bounded channel - will block if consumer is slow)
// This provides natural backpressure to VAD/STT processing
if let Err(e) = tx.send(Ok(final_text)).await {
    eprintln!("Failed to send transcription (consumer dropped): {}", e);
}
```

**Why `send().await` instead of `try_send()`:**
- VAD/STT processing is async (can safely await)
- Blocking here slows down transcription → slows down audio chunk consumption
- Creates backpressure chain: slow text injection → slow transcription → audio drops

### Monitoring and Observability

#### Dropped Chunk Tracking

```rust
// Atomic counter for lock-free updates from audio callback
let dropped_chunks = Arc::new(std::sync::atomic::AtomicU64::new(0));

// Background monitoring task
tokio::spawn(async move {
    let mut last_count = 0u64;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let current = dropped_monitor.load(Ordering::Relaxed);
        if current > last_count {
            eprintln!("⚠️  BACKPRESSURE: Dropped {} chunks in last 5s", current - last_count);
            last_count = current;
        }
    }
});
```

**Metrics Tracked:**
- Total dropped chunks (lifetime counter)
- Dropped chunks per 5-second window
- Warnings printed to stderr when chunks are dropped

## Implementation Details

### Modified Components

**1. Channel Declarations (pipeline.rs:259-285)**

```rust
// Old: Unbounded channels
let (tx, rx) = mpsc::unbounded_channel();
let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();

// New: Bounded channels with capacities
let (tx, rx) = mpsc::channel(100); // Transcription results
let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<f32>>(20); // Audio chunks
```

**2. Audio Callback Backpressure (pipeline.rs:292-339)**

```rust
// Non-blocking try_send() with dropped chunk tracking
audio.set_chunk_callback(move |chunk| {
    match audio_tx_clone.try_send(chunk) {
        Ok(_) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            dropped_chunks_clone.fetch_add(1, Ordering::Relaxed);
            eprintln!("WARNING: Audio chunk dropped...");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {}
    }
});
```

**3. Async Send Operations (pipeline.rs:505-507, 651-653)**

```rust
// Old: Ignores send errors (no backpressure)
let _ = tx.send(Ok(final_text));

// New: Async send with error handling (backpressure)
if let Err(e) = tx.send(Ok(final_text)).await {
    eprintln!("Failed to send transcription (consumer dropped): {}", e);
}
```

**4. Thread-Safety Fixes**

**Problem:** Holding `std::sync::MutexGuard` across `.await` points makes futures not `Send`, preventing `tokio::spawn`.

**Solution:** Scope all mutex locks to ensure they're dropped before any `.await`:

```rust
// Old: Lock held across await (compiler error!)
let current_session_id = *session_id.lock().unwrap();
// ... later ...
tx.send(Ok(final_text)).await; // ERROR: MutexGuard held across await

// New: Lock scoped to ensure drop before await
let current_session_id = {
    *session_id.lock().unwrap()
}; // Lock dropped here
// ... later ...
tx.send(Ok(final_text)).await; // ✅ OK: No locks held
```

**Applied to:**
- `session_id` lock (line 446-450)
- `metrics` lock (line 473-477)
- `broadcaster` lock (line 480-484)
- `vad` lock (line 372-381)
- `stt` lock (line 393-414)

### Type Changes

**File:** `rust-crates/swictation-daemon/src/main.rs`

```rust
// Old return type
fn start_recording() -> Result<mpsc::UnboundedReceiver<Result<String>>>

// New return type
fn start_recording() -> Result<mpsc::Receiver<Result<String>>>
```

**File:** `rust-crates/swictation-daemon/src/pipeline.rs`

```rust
// Old field type
struct Pipeline {
    tx: mpsc::UnboundedSender<Result<String>>,
    // ...
}

// New field type
struct Pipeline {
    tx: mpsc::Sender<Result<String>>,
    // ...
}
```

## Backpressure Flow

### Normal Operation

```
Audio Capture (cpal)
    │ try_send() → OK
    ▼
Audio Channel [20 chunks]
    │ recv().await
    ▼
VAD Processing
    │ (lock scoped, dropped before await)
    ▼
STT Processing
    │ (lock scoped, dropped before await)
    │ send().await → OK
    ▼
Transcription Channel [100 results]
    │ recv().await
    ▼
Text Injection
```

### Backpressure Scenario (Slow Text Injection)

```
Audio Capture (cpal)
    │ try_send() → FULL (queue: 20/20)
    │ ❌ Drops chunk, increments counter
    │ Prints warning every 5s
    ▼
Audio Channel [FULL: 20/20 chunks]
    │ recv().await (blocked waiting for space)
    │ ⏸️  Slow because STT is waiting...
    ▼
VAD Processing ⏸️
    ▼
STT Processing ⏸️
    │ send().await → BLOCKED (queue: 100/100)
    │ ⏸️  Waiting for text injection to consume
    ▼
Transcription Channel [FULL: 100/100 results]
    │ recv().await (slow - keyboard busy)
    ▼
Text Injection ⏳ (bottleneck!)
```

**Outcome:**
1. Text injection slows down → transcription channel fills
2. Transcription send blocks → STT processing slows
3. STT slows → audio channel consumption slows
4. Audio channel fills → audio callback drops chunks
5. User sees backpressure warnings in logs

## Performance Impact

✅ **Minimal impact under normal conditions:**
- Bounded channels add negligible overhead (~nanoseconds for bounds check)
- Audio callback remains non-blocking (same latency as before)
- Backpressure only activates when system is overloaded

✅ **Improved stability under load:**
- Memory usage bounded (no unbounded growth)
- Graceful degradation (drop audio chunks vs. OOM crash)
- Observable metrics (know when system is struggling)

## Testing

### Verify Bounded Behavior

```bash
# Start daemon
swictation start

# Monitor logs for backpressure warnings
tail -f ~/.local/share/swictation/swictation.log | grep "BACKPRESSURE"

# Simulate slow consumer (hold keys to delay text injection)
# Speak continuously while holding Ctrl+Z (or similar)
# Expected: See "Audio chunk dropped" and "BACKPRESSURE" warnings
```

### Test Scenarios

1. **Fast Speech → Slow Typing**
   - Speak rapidly for 30+ seconds
   - Observe: Audio chunks may drop if transcription can't keep up
   - Verify: No memory growth, warnings printed

2. **Normal Speech**
   - Speak at normal pace
   - Observe: No dropped chunks, smooth operation
   - Verify: Same latency as before

3. **System Under Load**
   - Run CPU-intensive tasks while dictating
   - Observe: Backpressure activates if STT slows down
   - Verify: System remains stable, no crashes

## Future Enhancements

1. **Adaptive Channel Capacities**
   - Dynamically adjust based on system load
   - Increase buffer when CPU available, decrease under pressure

2. **Priority Queue**
   - High-priority segments (e.g., commands) bypass queue
   - Low-priority (e.g., dictation) can be dropped

3. **Jitter Buffer**
   - Smooth out temporary slowdowns
   - Allow brief spikes without dropping chunks

4. **Metrics Dashboard**
   - Real-time visualization of queue depths
   - Historical backpressure events
   - Performance graphs (latency, throughput)

## References

- Tokio MPSC Channels: https://docs.rs/tokio/latest/tokio/sync/mpsc/
- Send Bounds in Async Rust: https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
- Audio Real-time Constraints: https://docs.rs/cpal/latest/cpal/

## Related Issues

- Issue #1: Unbounded Channels ✅ Fixed
- Issue #2: No Audio Backpressure ✅ Fixed
- Issue #4: Sequential VAD/STT Processing (Next)
- Issue #5: Unix Socket Security ✅ Fixed

---

**Status:** ✅ Implemented and tested
**Date:** 2025-11-17
**Author:** Archon (AI Assistant)
