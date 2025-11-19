# Testing STT Inference Standalone (Outside Daemon)

## Problem
When testing STT inference with standalone test programs (examples), the encoder hangs indefinitely even though the daemon works perfectly.

## Root Cause
The standalone test programs don't have the required environment variables set for ONNX Runtime to find:
1. The main ONNX Runtime library (`libonnxruntime.so`)
2. The CUDA provider libraries (`libonnxruntime_providers_cuda.so`)

Without these libraries properly loaded, CUDA operations hang instead of failing cleanly.

## Solution
Set these environment variables before running standalone tests:

```bash
export ORT_DYLIB_PATH=/home/robert/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.so
export LD_LIBRARY_PATH=/home/robert/.local/share/swictation/gpu-libs:/usr/local/cuda/lib64
```

On dad's system (Maxwell GPU), use the legacy ONNX Runtime:
```bash
export ORT_DYLIB_PATH=/home/jrl/.local/share/swictation/gpu-libs/libonnxruntime.so
export LD_LIBRARY_PATH=/home/jrl/.local/share/swictation/gpu-libs:/usr/local/cuda/lib64
```

## Verified Working Test

```bash
# With proper environment
export ORT_DYLIB_PATH=/home/robert/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.so
export LD_LIBRARY_PATH=/home/robert/.local/share/swictation/gpu-libs:/usr/local/cuda/lib64

# Run test
/opt/swictation/rust-crates/target/release/examples/test_simple_samples
```

**Result**: Successfully transcribed "Ask not what your country can do for you. Ask what you can do for your country."

## Key Learnings

1. **Daemon vs Standalone**: The daemon has environment variables set by systemd service file, standalone tests don't
2. **Symptoms of Missing Libs**: Encoder hangs indefinitely (100% CPU) instead of failing with error
3. **CHUNK_FRAMES Size**: Not the issue - daemon successfully processes 10000-frame chunks
4. **WAV Format**: Also not the issue - 24kHz files get resampled correctly to 16kHz
5. **recognize_samples() vs recognize_file()**: Both methods work identically when environment is correct

## How Daemon Gets Environment

Check daemon service file:
```bash
systemctl --user show swictation-daemon.service | grep Environment
```

Shows:
```
Environment=RUST_LOG=info
  ORT_DYLIB_PATH=/home/robert/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.so
  LD_LIBRARY_PATH=/home/robert/.local/share/swictation/gpu-libs:/usr/local/cuda/lib64:...
```

## Testing Checklist

Before testing standalone STT programs:
- [ ] Set ORT_DYLIB_PATH to correct libonnxruntime.so
- [ ] Set LD_LIBRARY_PATH to include gpu-libs and cuda lib64
- [ ] Verify files exist: `ls -lh $ORT_DYLIB_PATH`
- [ ] Verify GPU libs exist: `ls -lh ~/.local/share/swictation/gpu-libs/`
- [ ] Build test in release mode: `cargo build --example test_simple_samples --release`
- [ ] Run with timeout: `timeout 30 ./target/release/examples/test_simple_samples`

## Maxwell GPU Specific Notes

Maxwell GPUs (Quadro M2200, compute 5.2) require:
- ONNX Runtime 1.23.2 (CUDA 11.8 + cuDNN 8.9.7) - in gpu-libs
- NOT the npm bundled ONNX Runtime 1.22.0 (CUDA 12)

Path on dad's system:
```bash
export ORT_DYLIB_PATH=/home/jrl/.local/share/swictation/gpu-libs/libonnxruntime.so
export LD_LIBRARY_PATH=/home/jrl/.local/share/swictation/gpu-libs:/usr/local/cuda-11.8/lib64
```
