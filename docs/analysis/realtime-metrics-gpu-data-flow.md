# Complete Data Flow: RealtimeMetrics GPU Memory Fields

## Executive Summary

**CRITICAL FINDING**: GPU memory fields in `RealtimeMetrics` are **ALWAYS ZERO** because `GpuMonitor::update()` returns `None` for all memory fields, regardless of GPU provider.

## Complete Data Flow Trace

### 1. Data Structure Definition
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/models.rs:243-292`

```rust
pub struct RealtimeMetrics {
    // ... other fields ...

    // Resource usage (Lines 258-262)
    pub gpu_memory_current_mb: f64,    // ← Always 0.0
    pub gpu_memory_total_mb: f64,      // ← Always 0.0
    pub gpu_memory_percent: f64,       // ← Always 0.0
    pub cpu_percent_current: f64,

    // ... other fields ...
}
```

**Default Values** (Lines 271-291):
```rust
impl Default for RealtimeMetrics {
    fn default() -> Self {
        Self {
            // ...
            gpu_memory_current_mb: 0.0,   // ← Default is 0.0
            gpu_memory_total_mb: 0.0,     // ← Default is 0.0
            gpu_memory_percent: 0.0,      // ← Default is 0.0
            // ...
        }
    }
}
```

---

### 2. GPU Monitor Implementation (THE ROOT CAUSE)
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/gpu.rs:54-83`

```rust
pub fn update(&mut self) -> GpuMetrics {
    // CPU provider has no GPU metrics
    if self.provider == "cpu" {
        return GpuMetrics {
            gpu_name: self.gpu_name.clone(),
            provider: self.provider.clone(),
            utilization_percent: None,
            memory_used_mb: None,      // ← Returns None
            memory_total_mb: None,     // ← Returns None
            temperature_c: None,
        };
    }

    // For CUDA/DirectML/CoreML, return basic info
    // Real metrics require platform-specific APIs (nvidia-ml-sys, Windows, Metal)
    GpuMetrics {
        gpu_name: self.gpu_name.clone(),
        provider: self.provider.clone(),
        utilization_percent: None,     // Would need NVML/platform APIs
        memory_used_mb: None,          // ← Returns None (even for CUDA!)
        memory_total_mb: None,         // ← Returns None (even for CUDA!)
        temperature_c: None,           // Would need NVML
    }
}
```

**ISSUE**: The `update()` method **ALWAYS** returns `None` for memory fields, regardless of provider type (CPU, CUDA, DirectML, CoreML).

---

### 3. GPU Monitoring Initialization
**File**: `/opt/swictation/rust-crates/swictation-daemon/src/pipeline.rs:254-257`

```rust
// Enable GPU monitoring if provider is available
if let Some(ref provider) = gpu_provider {
    metrics.enable_gpu_monitoring(provider);  // ← Called with "cuda", "directml", etc.
}
```

**File**: `/opt/swictation/rust-crates/swictation-metrics/src/collector.rs:75-79`

```rust
pub fn enable_gpu_monitoring(&self, provider: &str) {
    let monitor = GpuMonitor::new(provider);  // ← Creates monitor
    *self.gpu_monitor.lock().unwrap() = Some(monitor);
    info!("GPU monitoring enabled for provider: {}", provider);
}
```

**Status**: GPU monitor is **properly initialized** with the correct provider string ("cuda", "directml", "coreml").

---

### 4. Periodic System Metrics Update
**File**: `/opt/swictation/rust-crates/swictation-daemon/src/main.rs:312-334`

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;

        // Update internal metrics (Line 318)
        metrics.lock().unwrap().update_system_metrics();

        // Get realtime metrics and update daemon state (Line 321)
        let mut realtime = metrics.lock().unwrap().get_realtime_metrics();

        // Broadcast to connected clients (Line 332)
        broadcaster.update_metrics(&realtime).await;
    }
})
```

**Frequency**: Every **1 second**

---

### 5. System Metrics Collection
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/collector.rs:305-344`

```rust
pub fn update_system_metrics(&self) {
    // Refresh system info (Lines 306-310)
    let mut system = self.system.lock().unwrap();
    system.refresh_cpu_all();
    system.refresh_memory();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, false);

    // Get CPU usage (global average) (Line 313)
    let cpu_percent = system.global_cpu_usage();

    // Update realtime metrics (Line 323)
    self.update_cpu_usage(cpu_percent as f64);

    // Update GPU metrics if available (Lines 326-333)
    if let Some(ref mut monitor) = *self.gpu_monitor.lock().unwrap() {
        let gpu_metrics = monitor.update();  // ← Calls GpuMonitor::update()

        // Update GPU memory if available (Lines 330-332)
        if let (Some(used), Some(total)) = (gpu_metrics.memory_used_mb, gpu_metrics.memory_total_mb) {
            self.update_gpu_memory(used as f64, total as f64);  // ← NEVER CALLED (both are None)
        }
    }

    // ... CPU mean calculation ...
}
```

**CRITICAL**: The `if let (Some(used), Some(total))` condition is **NEVER TRUE** because `GpuMonitor::update()` always returns `None` for both fields.

---

### 6. GPU Memory Update Function (Never Called)
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/collector.rs:253-272`

```rust
pub fn update_gpu_memory(&self, current_mb: f64, total_mb: f64) {
    let mut realtime = self.realtime.lock().unwrap();
    realtime.gpu_memory_current_mb = current_mb;   // ← Would set value
    realtime.gpu_memory_total_mb = total_mb;       // ← Would set value
    realtime.gpu_memory_percent = if total_mb > 0.0 {
        (current_mb / total_mb) * 100.0            // ← Would calculate percentage
    } else {
        0.0
    };

    // Track peak in session
    if let Some(ref mut session) = *self.current_session.lock().unwrap() {
        session.gpu_memory_peak_mb = session.gpu_memory_peak_mb.max(current_mb);
    }

    // Check threshold (Lines 269-271)
    if self.warnings_enabled && realtime.gpu_memory_percent > self.gpu_memory_threshold_percent {
        info!("⚠️  High GPU memory usage: {:.1}%", realtime.gpu_memory_percent);
    }
}
```

**Status**: This function is **correctly implemented** but **NEVER CALLED** because the precondition in `update_system_metrics()` is never met.

---

### 7. Realtime Metrics Retrieval
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/collector.rs:294-296`

```rust
pub fn get_realtime_metrics(&self) -> RealtimeMetrics {
    self.realtime.lock().unwrap().clone()  // ← Clones current state
}
```

**Result**: Returns `RealtimeMetrics` with:
- `gpu_memory_current_mb: 0.0` (unchanged from default)
- `gpu_memory_total_mb: 0.0` (unchanged from default)
- `gpu_memory_percent: 0.0` (unchanged from default)

---

### 8. Broadcasting to Clients
**File**: `/opt/swictation/rust-crates/swictation-broadcaster/src/broadcaster.rs:221-240`

```rust
pub async fn update_metrics(&self, realtime: &RealtimeMetrics) {
    let state = Self::daemon_state_to_string(&realtime.current_state);

    let event = BroadcastEvent::MetricsUpdate {
        state,
        session_id: realtime.current_session_id,
        segments: realtime.segments_this_session,
        words: realtime.words_this_session,
        wpm: realtime.wpm_this_session,
        duration_s: realtime.recording_duration_s,
        latency_ms: realtime.last_segment_latency_ms,
        gpu_memory_mb: realtime.gpu_memory_current_mb,      // ← Always 0.0
        gpu_memory_percent: realtime.gpu_memory_percent,    // ← Always 0.0
        cpu_percent: realtime.cpu_percent_current,
    };

    if let Err(e) = self.client_manager.broadcast(&event).await {
        tracing::error!("Failed to broadcast metrics_update: {}", e);
    }
}
```

**Result**: Broadcasts event with GPU memory fields set to 0.0.

---

### 9. JSON Serialization
**File**: `/opt/swictation/rust-crates/swictation-broadcaster/src/events.rs:32-44`

```rust
#[serde(rename = "metrics_update")]
MetricsUpdate {
    state: String,
    session_id: Option<i64>,
    segments: i32,
    words: i32,
    wpm: f64,
    duration_s: f64,
    latency_ms: f64,
    gpu_memory_mb: f64,         // ← Serialized as 0.0
    gpu_memory_percent: f64,    // ← Serialized as 0.0
    cpu_percent: f64,
}
```

**JSON Output** (example):
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
  "gpu_memory_mb": 0.0,        ← ALWAYS ZERO
  "gpu_memory_percent": 0.0,   ← ALWAYS ZERO
  "cpu_percent": 23.1
}
```

---

### 10. Tauri UI Reception
**File**: `/opt/swictation/tauri-ui/src/types.ts:114-125`

```typescript
| {
    type: 'metrics_update';
    state: DaemonState;
    session_id?: number;
    segments: number;
    words: number;
    wpm: number;
    duration_s: number;
    latency_ms: number;
    gpu_memory_mb: number;        // ← Receives 0.0
    gpu_memory_percent: number;   // ← Receives 0.0
    cpu_percent: number;
  }
```

**Result**: TypeScript receives valid JSON but with zero values for GPU memory.

---

## Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ INITIALIZATION (daemon startup)                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  pipeline.rs:255    if let Some(ref provider) = gpu_provider               │
│  pipeline.rs:256        metrics.enable_gpu_monitoring(provider)            │
│                                │                                             │
│                                ▼                                             │
│  collector.rs:76   GpuMonitor::new(provider)  // "cuda", "directml", etc.  │
│  collector.rs:77   *self.gpu_monitor = Some(monitor)                       │
│                                │                                             │
│                                ✓ GPU Monitor initialized successfully       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ PERIODIC UPDATE LOOP (every 1 second)                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  main.rs:318       metrics.update_system_metrics()                         │
│                                │                                             │
│                                ▼                                             │
│  collector.rs:326  if let Some(ref mut monitor) = gpu_monitor {            │
│  collector.rs:327      let gpu_metrics = monitor.update()                  │
│                                │                                             │
│                                ▼                                             │
│  gpu.rs:60         pub fn update(&mut self) -> GpuMetrics {                │
│  gpu.rs:78             memory_used_mb: None,     ← ALWAYS None             │
│  gpu.rs:79             memory_total_mb: None,    ← ALWAYS None             │
│                        }                                                     │
│                                │                                             │
│                                ▼                                             │
│  collector.rs:330  if let (Some(used), Some(total)) = (...) {              │
│                        ❌ CONDITION NEVER TRUE (both are None)              │
│                        ❌ update_gpu_memory() NEVER CALLED                  │
│                    }                                                         │
│                                │                                             │
│                                ▼                                             │
│  collector.rs:294  get_realtime_metrics()                                  │
│                    Returns RealtimeMetrics with:                            │
│                    - gpu_memory_current_mb: 0.0                             │
│                    - gpu_memory_total_mb: 0.0                               │
│                    - gpu_memory_percent: 0.0                                │
│                                │                                             │
│                                ▼                                             │
│  main.rs:332       broadcaster.update_metrics(&realtime)                   │
│                                │                                             │
│                                ▼                                             │
│  broadcaster.rs:224 BroadcastEvent::MetricsUpdate {                        │
│                        gpu_memory_mb: realtime.gpu_memory_current_mb, // 0.0│
│                        gpu_memory_percent: realtime.gpu_memory_percent, // 0.0│
│                     }                                                        │
│                                │                                             │
│                                ▼                                             │
│  events.rs:66      event.to_json_line()                                    │
│                    Serializes to JSON with gpu_memory_mb: 0.0               │
│                                │                                             │
│                                ▼                                             │
│  Unix Socket       {"type":"metrics_update","gpu_memory_mb":0.0,...}       │
│                                │                                             │
│                                ▼                                             │
│  Tauri UI          Receives JSON with gpu_memory_mb: 0.0                   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Root Cause Analysis

### The Problem
**File**: `/opt/swictation/rust-crates/swictation-metrics/src/gpu.rs:54-83`

The `GpuMonitor::update()` method is a **stub implementation** that:

1. **Returns `None` for CPU provider** (expected behavior)
2. **Returns `None` for ALL providers including CUDA/DirectML/CoreML** (bug!)

### Why It Returns None
The code contains this comment on lines 74-75:
```rust
// For CUDA/DirectML/CoreML, return basic info
// Real metrics require platform-specific APIs (nvidia-ml-sys, Windows, Metal)
```

This indicates the GPU monitoring was **intentionally left unimplemented** as a placeholder for future enhancement.

---

## Evidence Files

| File | Line(s) | Evidence |
|------|---------|----------|
| `models.rs` | 243-292 | `RealtimeMetrics` struct definition with GPU fields |
| `models.rs` | 282-284 | Default values: all GPU fields = 0.0 |
| `gpu.rs` | 60-83 | `GpuMonitor::update()` returns `None` for all memory fields |
| `gpu.rs` | 74-75 | Comment stating "Real metrics require platform-specific APIs" |
| `collector.rs` | 253-272 | `update_gpu_memory()` function (correctly implemented but never called) |
| `collector.rs` | 326-333 | Conditional check that's never true |
| `collector.rs` | 330 | `if let (Some(used), Some(total))` - fails because both are `None` |
| `main.rs` | 318 | Periodic call to `update_system_metrics()` every 1 second |
| `broadcaster.rs` | 232-233 | Broadcasting zero values to clients |
| `events.rs` | 41-42 | JSON serialization of zero values |
| `types.ts` | 122-123 | TypeScript interface receiving zero values |

---

## Summary

### Data Flow Path
```
GpuMonitor::update() → Returns None for memory fields
                    ↓
MetricsCollector::update_system_metrics() → Skips update_gpu_memory() call
                    ↓
MetricsCollector::get_realtime_metrics() → Returns default zeros
                    ↓
MetricsBroadcaster::update_metrics() → Broadcasts zeros
                    ↓
BroadcastEvent::MetricsUpdate → Serializes zeros to JSON
                    ↓
Unix Socket → Sends {"gpu_memory_mb": 0.0, ...}
                    ↓
Tauri UI → Receives zeros
```

### The Fix
To properly populate GPU memory values, implement platform-specific APIs in `GpuMonitor::update()`:

1. **NVIDIA/CUDA**: Use `nvidia-ml-sys` crate (NVML bindings)
2. **DirectML**: Use Windows Memory APIs
3. **CoreML**: Use Metal Performance APIs
4. **CPU**: Keep returning `None` (correct current behavior)

Once implemented, the existing data flow infrastructure will work correctly without any other changes needed.
