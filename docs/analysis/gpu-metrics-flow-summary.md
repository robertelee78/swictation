# GPU Metrics Data Flow - Quick Reference

## TL;DR - Root Cause

**Problem**: GPU memory fields (`gpu_memory_current_mb`, `gpu_memory_percent`) are always 0.0 in UI

**Root Cause**: `GpuMonitor::update()` returns `None` for memory fields (stub implementation)

**File**: `/opt/swictation/rust-crates/swictation-metrics/src/gpu.rs:78-79`

```rust
memory_used_mb: None,      // ← TODO: Needs NVML/platform APIs
memory_total_mb: None,     // ← TODO: Needs NVML/platform APIs
```

---

## Visual Data Flow

```
┌──────────────────────────────────────────────────────────────────┐
│ START: Every 1 second (main.rs:318)                              │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ MetricsCollector::update_system_metrics()                        │
│ (collector.rs:305)                                               │
│                                                                   │
│  1. Update CPU metrics ✓                                         │
│  2. if gpu_monitor.is_some():                                    │
│     - Call monitor.update()                                      │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ GpuMonitor::update() → GpuMetrics                                │
│ (gpu.rs:60-83)                                                   │
│                                                                   │
│  Returns:                                                         │
│  GpuMetrics {                                                     │
│    provider: "cuda",        ✓                                    │
│    gpu_name: "NVIDIA GPU",  ✓                                    │
│    memory_used_mb: None,    ← ❌ PROBLEM                         │
│    memory_total_mb: None,   ← ❌ PROBLEM                         │
│  }                                                                │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ Check if memory values exist (collector.rs:330)                  │
│                                                                   │
│  if let (Some(used), Some(total)) = (                            │
│      gpu_metrics.memory_used_mb,    ← None                       │
│      gpu_metrics.memory_total_mb    ← None                       │
│  ) {                                                              │
│      // ❌ NEVER ENTERS THIS BLOCK                               │
│      update_gpu_memory(used, total); // Never called             │
│  }                                                                │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ RealtimeMetrics remains at default values (collector.rs:294)     │
│                                                                   │
│  RealtimeMetrics {                                                │
│    gpu_memory_current_mb: 0.0,   ← Default, never updated        │
│    gpu_memory_total_mb: 0.0,     ← Default, never updated        │
│    gpu_memory_percent: 0.0,      ← Default, never updated        │
│    ...                                                            │
│  }                                                                │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ Broadcast to UI (broadcaster.rs:232-233, main.rs:332)            │
│                                                                   │
│  BroadcastEvent::MetricsUpdate {                                 │
│    gpu_memory_mb: 0.0,        ← Zero value                       │
│    gpu_memory_percent: 0.0,   ← Zero value                       │
│  }                                                                │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ JSON over Unix socket                                             │
│                                                                   │
│  {"type":"metrics_update","gpu_memory_mb":0.0,...}               │
└────────────────┬─────────────────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ Tauri UI displays 0 MB / 0%                                       │
└──────────────────────────────────────────────────────────────────┘
```

---

## Code References by Component

### 1. Data Structure Definition
```
File: rust-crates/swictation-metrics/src/models.rs

Lines 243-292: RealtimeMetrics struct definition
Lines 259-261: GPU memory field declarations
Lines 282-284: Default values (all 0.0)
```

### 2. GPU Monitoring (Root Cause)
```
File: rust-crates/swictation-metrics/src/gpu.rs

Lines 8-16:   GpuMetrics struct (has Option<u64> for memory)
Lines 32-35:  GpuMonitor struct
Lines 39-52:  GpuMonitor::new() - initialization (works correctly)
Lines 60-83:  GpuMonitor::update() - THE PROBLEM (returns None)
Lines 74-75:  Comment explaining it's a stub
```

### 3. Metrics Collection
```
File: rust-crates/swictation-metrics/src/collector.rs

Lines 17-39:  MetricsCollector struct definition
Lines 75-79:  enable_gpu_monitoring() - initialization (works)
Lines 253-272: update_gpu_memory() - NEVER CALLED
Lines 305-344: update_system_metrics() - main update loop
Lines 326-333: GPU monitor update attempt (fails at line 330)
Lines 294-296: get_realtime_metrics() - returns zeros
```

### 4. Daemon Integration
```
File: rust-crates/swictation-daemon/src/main.rs

Lines 308-335: Metrics update task (every 1 second)
Line 318:      Call to update_system_metrics()
Line 321:      Get realtime metrics
Line 332:      Broadcast to clients
```

```
File: rust-crates/swictation-daemon/src/pipeline.rs

Lines 254-257: GPU monitoring initialization
```

### 5. Broadcasting
```
File: rust-crates/swictation-broadcaster/src/broadcaster.rs

Lines 14-22:  MetricsBroadcaster struct
Lines 221-240: update_metrics() - broadcasts zeros
Lines 232-233: GPU fields copied from realtime
```

```
File: rust-crates/swictation-broadcaster/src/events.rs

Lines 32-44:  MetricsUpdate event definition
Lines 41-42:  GPU memory fields in event
Lines 66-69:  to_json_line() serialization
```

### 6. UI Reception
```
File: tauri-ui/src/types.ts

Lines 114-125: BroadcastEvent::MetricsUpdate type
Lines 122-123: GPU memory fields in TypeScript
```

---

## Fixing the Issue

### Option 1: Implement NVML for CUDA (Recommended for NVIDIA)

**File**: `rust-crates/swictation-metrics/src/gpu.rs`

Add dependency:
```toml
[dependencies]
nvml-wrapper = "0.9"  # NVIDIA Management Library bindings
```

Modify `update()`:
```rust
pub fn update(&mut self) -> GpuMetrics {
    if self.provider == "cuda" {
        // Use NVML to get real GPU stats
        match nvml_wrapper::Nvml::init() {
            Ok(nvml) => {
                if let Ok(device) = nvml.device_by_index(0) {
                    let mem_info = device.memory_info().ok();
                    return GpuMetrics {
                        gpu_name: self.gpu_name.clone(),
                        provider: self.provider.clone(),
                        memory_used_mb: mem_info.map(|m| m.used / 1024 / 1024),
                        memory_total_mb: mem_info.map(|m| m.total / 1024 / 1024),
                        utilization_percent: device.utilization_rates().ok()
                            .map(|u| u.gpu as f32),
                        temperature_c: device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).ok()
                            .map(|t| t as f32),
                    };
                }
            }
            Err(_) => {}
        }
    }

    // Fallback for other providers
    GpuMetrics::default()
}
```

### Option 2: Use MemoryMonitor (Already Available!)

**Current System**: There's already a `MemoryMonitor` that tracks VRAM!

**File**: `rust-crates/swictation-daemon/src/main.rs:337-400`

The daemon already has a separate memory pressure monitor that:
- Checks VRAM every 5 seconds
- Has access to GPU memory stats
- Uses platform-specific APIs

**Quick Fix**: Share the `MemoryMonitor`'s VRAM data with `MetricsCollector`:

```rust
// In main.rs, share memory stats with metrics collector
let memory_stats = memory_monitor.get_stats();
if let Some(vram) = memory_stats.vram {
    metrics.lock().unwrap().update_gpu_memory(
        vram.used_mb as f64,
        vram.total_mb as f64
    );
}
```

### Option 3: Remove Fields from UI (Quick Workaround)

If GPU metrics aren't critical:
1. Keep fields in `RealtimeMetrics` (for future use)
2. Hide/remove GPU display from Tauri UI
3. Document that GPU monitoring requires platform APIs

---

## Testing the Fix

After implementing any fix, verify the data flow:

1. **Check GpuMonitor output**:
   ```rust
   let metrics = monitor.update();
   assert!(metrics.memory_used_mb.is_some());
   assert!(metrics.memory_total_mb.is_some());
   ```

2. **Check RealtimeMetrics update**:
   ```rust
   collector.update_system_metrics();
   let realtime = collector.get_realtime_metrics();
   assert!(realtime.gpu_memory_current_mb > 0.0);
   assert!(realtime.gpu_memory_percent > 0.0);
   ```

3. **Check broadcasted event**:
   Monitor Unix socket output:
   ```bash
   socat - UNIX-CONNECT:/tmp/swictation_metrics.sock
   ```
   Should see:
   ```json
   {"type":"metrics_update","gpu_memory_mb":1823.4,"gpu_memory_percent":45.2,...}
   ```

4. **Check UI display**:
   Verify Tauri UI shows non-zero GPU memory values

---

## Related Issues

This analysis may help resolve:
- UI showing "0 MB / 0%" for GPU memory
- Missing VRAM statistics in real-time metrics
- Incomplete GPU monitoring implementation
- Integration with existing MemoryMonitor system

---

## Key Takeaways

1. ✅ **Data structure is correct** - `RealtimeMetrics` has all needed fields
2. ✅ **Initialization works** - GPU monitor is created with correct provider
3. ✅ **Periodic updates work** - Called every 1 second
4. ✅ **Broadcasting works** - JSON serialization and socket transmission OK
5. ✅ **UI reception works** - TypeScript types match Rust structs
6. ❌ **GPU data collection fails** - `GpuMonitor::update()` is a stub
7. ✅ **Alternative exists** - `MemoryMonitor` already has VRAM data

**Bottom Line**: The entire pipeline is working correctly except for ONE function (`GpuMonitor::update()`) that needs platform-specific API implementation.
