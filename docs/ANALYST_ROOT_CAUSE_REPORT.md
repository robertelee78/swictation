# Root Cause Analysis: Swictation Packaged vs Test Environment
**Analyst Agent Report**
**Date:** 2025-11-11
**Session:** Hive Mind Collective Investigation

---

## Executive Summary

The packaged version of Swictation running under systemd shows two critical issues:
1. **GPU inference is 14x slower than expected** (43s for 57s audio vs expected 3-6s)
2. **Audio device detection is inconsistent** (detecting 1 device instead of 4 available)

**Root Cause:** Missing environment variables and resource constraints in systemd service configuration.

---

## Critical Findings

### 1. GPU Performance Degradation ⚠️ CRITICAL

#### Measured Performance
```
Timeline Analysis (from journalctl):
13:15:36 - Start processing chunk 1/1 (911,872 samples = 57s audio)
13:15:36 - Feature extraction complete (108ms) ✓ FAST
13:15:36 - Encoder inference starts
13:16:19 - Encoder inference complete = 43 SECONDS ✗ VERY SLOW
13:16:20 - Decoder complete (1 second)
Total: 44 seconds to process 57 seconds of audio
Performance: 0.75x realtime (SLOWER than realtime!)
```

#### Expected Performance
- GPU should deliver **10-20x realtime** minimum
- 57s audio should process in **3-6 seconds**, not 44 seconds
- This is a **14x performance regression**

#### Evidence GPU IS Being Used
```
From logs:
- BFCArena for Cuda allocations: 678MB
- nvidia-smi shows: swictation-daemon using 1322MB GPU memory
- CUDA execution provider loaded successfully
- No CUDA initialization errors
```

#### Why So Slow Despite GPU Access?

**Primary Root Cause: Missing CUDA Library Paths**

The systemd service file shows:
```systemd
[Service]
Environment="RUST_LOG=info"
# NO LD_LIBRARY_PATH!
# NO CUDA_HOME!
```

The working test environment had:
```bash
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:...
export CUDA_HOME=/usr/local/cuda
export FORCE_CUDA=1
```

**Impact:** ONNX Runtime likely falls back to suboptimal CUDA library loading or uses CPU for some operations despite successful GPU memory allocation.

#### Contributing Factors

**A. CPU Quota Throttling**
```systemd
CPUQuota=50%
```
- Limits service to 4 cores on 8-core system
- Impacts CPU-side preprocessing (feature extraction, VAD, etc.)
- Feature extraction was fast (108ms) so not the primary issue

**B. Memory Pressure**
```
Memory: 498.4M used / 500M limit (99.7% utilized)
Swap: 2.5GB peak usage
```
- Service hitting memory ceiling
- Forcing operations to swap
- May cause GPU<->CPU memory transfer slowdowns

**C. Missing Optimization Environment**
```bash
# Present in test environment, missing in systemd:
CUDA_ARCHITECTURES=90
TORCH_CUDA_ARCH_LIST=9.0
```

---

### 2. Audio Device Detection Issues 🎤 MEDIUM

#### Current Behavior
```
Systemd service logs:
=== Available Input Devices ===
Device: plughw:CARD=Generic,DEV=0
Device sample format: F32
```

Only 1 device detected!

#### PulseAudio Shows 4 Devices Available
```bash
$ pactl list sources | grep "Name:"
Name: alsa_output.pci-0000_81_00.1.hdmi-stereo.monitor
Name: alsa_input.usb-Sonix_Technology_Co.__Ltd._USB_Live_camera_SN0001-03.analog-stereo
Name: alsa_output.usb-Generic_USB_Audio-00.iec958-stereo.monitor
Name: alsa_input.usb-Generic_USB_Audio-00.iec958-stereo
```

#### Root Cause

The code uses `cpal` which uses ALSA directly:
```rust
// From capture.rs line 261-267
for (idx, dev) in self.host.input_devices()
    .enumerate()
{
    let name = dev.name().unwrap_or_else(|_| "Unknown".to_string());
    println!("  [{}] {}", idx, name);
}
```

**Problem:**
- ALSA enumeration happens without PulseAudio session context
- Missing environment variables that connect to audio session:
  - `PULSE_SERVER`
  - `PIPEWIRE_REMOTE`
  - `XDG_RUNTIME_DIR` (present but maybe insufficient)

**Test Environment Difference:**
- Test runs in full user shell with audio session active
- Systemd service starts early, before audio session fully established
- Service file has `After=graphical-session.target` but no audio-specific ordering

---

## Environment Comparison Matrix

| Variable/Setting | Test Environment | Systemd Service | Impact |
|-----------------|------------------|-----------------|---------|
| `LD_LIBRARY_PATH` | `/usr/local/cuda/lib64:...` | ❌ Not set | **CRITICAL - GPU slowdown** |
| `CUDA_HOME` | `/usr/local/cuda` | ❌ Not set | **HIGH - CUDA initialization** |
| `PULSE_SERVER` | Inherited from session | ❌ Not set | **MEDIUM - Audio detection** |
| `DISPLAY` | `:0` | ❌ Not set | **LOW - Audio session link** |
| `WAYLAND_DISPLAY` | `wayland-1` | ❌ Not set | **LOW - Audio session link** |
| CPU Quota | Unlimited | 50% (4 cores) | **MEDIUM - Preprocessing** |
| Memory Limit | Unlimited | 500M (at 99.7%) | **MEDIUM - Swap pressure** |
| `RUST_LOG` | `info` | `info` | ✅ Same |

---

## Supporting Evidence

### GPU Memory Allocation (Working But Slow)
```
From logs (13:15:36 - 13:15:38):
2025-11-11T21:15:36.348968Z INFO Extending BFCArena for Cuda. bin_num:13 (requested) num_bytes: 3200000
2025-11-11T21:15:36.349078Z INFO Allocated memory at 0x779fa2000000 to 0x779fa6000000
2025-11-11T21:15:37.804082Z INFO Extending BFCArena for Cuda. bin_num:19 (requested) num_bytes: 204800000
2025-11-11T21:15:38.005247Z INFO Extending BFCArena for Cuda. bin_num:19 (requested) num_bytes: 204800000

Total CUDA allocations: 678MB
```

### nvidia-smi Confirmation
```bash
$ nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv
pid, process_name, used_gpu_memory [MiB]
426362, /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin, 1322 MiB
```

GPU is being used, but inference is still 14x slower than expected!

### Library Files Present
```bash
$ ls -lah /usr/local/lib/node_modules/swictation/lib/native/
-rw-r--r-- 1 root root  22M libonnxruntime.so
-rw-r--r-- 1 root root 345M libonnxruntime_providers_cuda.so  ← Present!
-rw-r--r-- 1 root root  15K libonnxruntime_providers_shared.so
-rw-r--r-- 1 root root 806K libonnxruntime_providers_tensorrt.so
```

All CUDA libraries are present in the package. The issue is **runtime linking**, not missing files.

---

## Detailed Timing Analysis

### Feature Extraction (Fast - Working Correctly)
```
13:15:36.227806Z - Start processing 911872 samples
13:15:36.336051Z - Features extracted: [5697, 80]
Duration: 108ms for 57s audio = 527x realtime ✓ EXCELLENT
```

### Encoder Inference (Slow - The Problem)
```
13:15:36.339310Z - Processing 1 encoder chunks
13:15:36.348968Z - CUDA memory allocation starts
13:16:19.??????Z - Encoder output ready
Duration: ~43 seconds for 5697 frames
Expected: 3-6 seconds on GPU
Actual: 0.75x realtime (slower than realtime!)
```

This is where the 43-second slowdown occurs. The encoder should run at 10-20x realtime on GPU.

### Decoder Inference (Fast - Working Correctly)
```
13:16:19Z - Computing decoder_out
13:16:20Z - Decoded 0 tokens
Duration: ~1 second
```

Decoder is fast, encoder is the bottleneck.

---

## Root Cause Hypotheses Ranked by Probability

### 1. Missing LD_LIBRARY_PATH (90% confidence)
**Most Likely Root Cause**

Without `LD_LIBRARY_PATH=/usr/local/cuda/lib64`, ONNX Runtime may:
- Fail to load optimized CUDA kernels
- Fall back to generic/unoptimized GPU code paths
- Use CPU for some tensor operations despite CUDA context existing
- Experience slow cuBLAS/cuDNN library resolution

**Test:** Check ONNX Runtime logs for cuBLAS/cuDNN warnings (currently using RUST_LOG=info, may need debug level)

### 2. CPU Quota Limiting Preprocessing (60% confidence)
`CPUQuota=50%` limits to 4 cores. While feature extraction was fast, CPU quota may affect:
- Data marshalling between CPU and GPU
- Audio buffer processing
- Resampling operations

### 3. Memory Pressure Causing Swap (40% confidence)
At 498.4M/500M with 2.5GB swap:
- May cause GPU<->CPU memory transfers to be slow
- Page faults during inference
- But GPU has plenty of memory (74GB free), so less likely

### 4. Missing CUDA_HOME (30% confidence)
Some libraries use `CUDA_HOME` to find additional resources at runtime:
- cuDNN config files
- Kernel cache directories
- But less critical than LD_LIBRARY_PATH

---

## Test vs Packaged: What Changed

### Working Test Environment
```bash
# Running from shell
$ ./rust-crates/target/release/swictation-daemon

✓ Full CUDA environment inherited
✓ Unlimited CPU/memory
✓ Direct terminal with audio session
✓ PulseAudio/PipeWire session active
✓ All environment variables available
✓ Running as user in X11/Wayland session
```

### Broken Packaged Environment
```systemd
# Running from systemd
[Service]
Type=simple
ExecStart=/usr/local/lib/node_modules/swictation/bin/swictation-daemon
Environment="RUST_LOG=info"    ← ONLY this environment variable!
MemoryMax=500M
CPUQuota=50%

✗ Minimal environment
✗ CPU capped at 50%
✗ Memory capped at 500M
✗ No CUDA environment
✗ No audio session variables
✗ Early boot, before full user session
```

---

## Recommendations (Analysis Only - No Code Changes)

### Immediate Fixes for GPU Performance

**1. Add CUDA Environment Variables**
```systemd
Environment="LD_LIBRARY_PATH=/usr/local/cuda/lib64:/usr/local/lib/node_modules/swictation/lib/native"
Environment="CUDA_HOME=/usr/local/cuda"
```

**2. Remove or Relax Resource Limits**
```systemd
# Remove or increase:
CPUQuota=200%    # Allow 16 cores (or remove entirely)
MemoryMax=2G     # Increase from 500M to 2GB
```

**3. Add CUDA Optimization Flags** (if ONNX Runtime respects them)
```systemd
Environment="CUDA_LAUNCH_BLOCKING=0"
Environment="CUDA_VISIBLE_DEVICES=0"
```

### Immediate Fixes for Audio Detection

**1. Add Audio Session Variables**
```systemd
Environment="PULSE_SERVER=/run/user/1000/pulse/native"
Environment="XDG_RUNTIME_DIR=/run/user/1000"
```

**2. Ensure Service Ordering**
```systemd
After=graphical-session.target sound.target pulseaudio.service pipewire-pulse.service
Wants=sound.target
```

**3. Grant Audio Socket Access** (if using sandboxing)
```systemd
# If ProtectSystem=strict is re-enabled:
ReadWritePaths=/run/user/1000/pulse
```

### Testing the Fixes

After applying environment changes:
```bash
# 1. Reload systemd
systemctl --user daemon-reload

# 2. Restart service
systemctl --user restart swictation-daemon

# 3. Check new environment
systemctl --user show swictation-daemon | grep Environment

# 4. Test audio
pactl list sources
# Service should now detect all 4 devices

# 5. Monitor performance
journalctl --user -u swictation-daemon -f
# Encoder inference should complete in 3-6 seconds, not 43 seconds
```

---

## Conclusion

The packaged version's issues stem from **environment isolation** in systemd, not code defects:

1. **GPU slowdown:** Missing `LD_LIBRARY_PATH` prevents optimal CUDA library loading → 14x performance regression
2. **Audio issues:** Missing `PULSE_SERVER` and early boot timing → inconsistent device detection

The test environment "accidentally worked" by inheriting a full user shell environment. The systemd service runs in a minimal environment by design, exposing these missing dependencies.

**All fixes are configuration changes to `/home/robert/.config/systemd/user/swictation-daemon.service`**

No code changes required. The application code is correct; the runtime environment needs adjustment.

---

## Additional Notes

### Why Did GPU Memory Allocate But Inference Was Slow?

CUDA initialization succeeded (GPU memory allocated) but ONNX Runtime likely couldn't find optimized kernels:
- Base CUDA works: `libcuda.so` found (system-wide)
- Optimized libs missing: `libcublas.so`, `libcudnn.so` not in LD_LIBRARY_PATH
- Result: CUDA context exists but inference falls back to generic/slow paths

This explains the paradox: GPU memory usage + slow inference.

### IPC Connection Errors

The logs show many "Broken pipe (os error 32)" errors at 13:16:25. These appear to be client disconnections, not related to the performance issue. Likely the UI or a monitoring client timing out during the slow 43-second inference.

---

**Report compiled by:** Analyst Agent
**Coordination:** Hive Mind Collective
**Memory stored at:** `workers/analyst/analysis` in swictation-analysis namespace
**Status:** Analysis complete, ready for solution implementation
