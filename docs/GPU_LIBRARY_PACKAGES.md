# GPU Library Packages - Multi-Architecture Support

## Overview

Swictation v0.3.15+ includes automatic GPU detection and optimized library downloads. Instead of downloading a single large package, the system detects your GPU's compute capability and downloads only the libraries optimized for your GPU architecture.

## Supported GPU Architectures

### Legacy Package (sm_50-70) - ~1.5GB
**Target GPUs:** Maxwell, Pascal, Volta (2014-2017)
- **Architectures:** sm_50, sm_52, sm_60, sm_61, sm_70
- **Example GPUs:**
  - GTX 750/900/1000 series
  - Quadro M2200, M4000, M5000, M6000
  - Quadro P series
  - Titan V, V100

**Download:** `cuda-libs-legacy.tar.gz`

### Modern Package (sm_75-86) - ~1.5GB
**Target GPUs:** Turing, Ampere (2018-2021)
- **Architectures:** sm_75, sm_80, sm_86
- **Example GPUs:**
  - GTX 1650, 1660 series
  - RTX 2060/2070/2080/2090 series
  - RTX 3060/3070/3080/3090 series
  - A100, A30, A40
  - RTX A1000, A2000, A4000, A5000, A6000

**Download:** `cuda-libs-modern.tar.gz`

### Latest Package (sm_89-120) - ~1.5GB
**Target GPUs:** Ada Lovelace, Hopper, Blackwell (2022-2024)
- **Architectures:** sm_89, sm_90, sm_100, sm_120
- **Example GPUs:**
  - RTX 4090, 4080, 4070 series
  - H100, H200
  - B100, B200 (Blackwell data center)
  - RTX PRO 6000 Blackwell
  - RTX 50 series (5090, 5080, upcoming)

**Download:** `cuda-libs-latest.tar.gz`

## Automatic Detection (npm install)

When you install swictation via npm, the postinstall script automatically:

1. Detects your GPU via `nvidia-smi`
2. Reads compute capability (e.g., "5.2", "8.6", "12.0")
3. Maps to appropriate package variant
4. Downloads from GitHub release `gpu-libs-v1.1.0`
5. Extracts to `~/.local/share/swictation/gpu-libs/`

### Example Output

```bash
npm install -g swictation

# Output:
✓ NVIDIA GPU detected!
📦 Detecting GPU architecture and downloading optimized libraries...

✓ Detected GPU: NVIDIA RTX PRO 6000 Blackwell
  Compute Capability: 12.0 (sm_120)

📦 Selected Package: LATEST
   Architectures: sm_89-120
   Description: Ada Lovelace, Hopper, Blackwell GPUs (2022-2024)
   Examples: RTX 4090, H100, B100/B200, RTX PRO 6000 Blackwell, RTX 50 series

  Downloading latest package...
  ✓ Downloaded latest package (~1.5GB)
  ✓ Extracted 20 libraries to ~/.local/share/swictation/gpu-libs

✅ GPU acceleration enabled!
   Architecture: sm_89-120
   Libraries: /home/user/.local/share/swictation/gpu-libs
   Your system will use CUDA for faster transcription
```

## Manual Installation

If automatic detection fails or you need to install manually:

### Step 1: Check Your GPU

```bash
nvidia-smi --query-gpu=name,compute_cap --format=csv
```

### Step 2: Download Appropriate Package

Visit: https://github.com/robertelee78/swictation/releases/tag/gpu-libs-v1.1.0

Download the appropriate variant:
- **sm_50-70:** `cuda-libs-legacy.tar.gz`
- **sm_75-86:** `cuda-libs-modern.tar.gz`
- **sm_89-120:** `cuda-libs-latest.tar.gz`

### Step 3: Extract Libraries

```bash
# Create directory
mkdir -p ~/.local/share/swictation/gpu-libs

# Extract (replace <variant> with legacy/modern/latest)
tar -xzf cuda-libs-<variant>.tar.gz
cd <variant>/libs
cp *.so ~/.local/share/swictation/gpu-libs/

# Verify
ls -lh ~/.local/share/swictation/gpu-libs/
```

## Package Contents

Each package includes:

### ONNX Runtime 1.23.2
- `libonnxruntime.so` - Core runtime library
- `libonnxruntime_providers_cuda.so` - CUDA execution provider (architecture-specific)
- `libonnxruntime_providers_shared.so` - Shared provider utilities

### CUDA 12.9 Runtime Libraries
- `libcublas.so.12` (101MB) - CUDA Basic Linear Algebra Subroutines
- `libcublasLt.so.12` (713MB) - cuBLAS with tensor cores
- `libcudart.so.12` (720KB) - CUDA runtime
- `libcufft.so.11` (277MB) - CUDA Fast Fourier Transform
- `libcurand.so.10` (160MB) - CUDA random number generation
- `libnvrtc.so.12` (102MB) - NVIDIA runtime compilation

### cuDNN 9.15.1 Deep Learning Libraries
- `libcudnn.so.9` - Main cuDNN library
- `libcudnn_adv.so.9` (259MB) - Advanced operations
- `libcudnn_cnn.so.9` (4MB) - Convolutional neural networks
- `libcudnn_engines_precompiled.so.9` (479MB) - Precompiled kernels
- `libcudnn_engines_runtime_compiled.so.9` (28MB) - Runtime compilation
- `libcudnn_graph.so.9` (4MB) - Graph API
- `libcudnn_heuristic.so.9` (53MB) - Heuristics engine
- `libcudnn_ops.so.9` (102MB) - Core operations

**Total Uncompressed Size:** ~2.3GB per package

## Technical Details

### Build Configuration
- **Base Image:** NVIDIA CUDA 12.9.0-devel-ubuntu22.04
- **ONNX Runtime:** v1.23.2 (built from source)
- **CMake:** 3.28.3
- **Compiler:** GCC 11.4.0, nvcc 12.9

### Why CUDA 12.9?
CUDA 12.9 is the "sweet spot" for multi-architecture support:
- ✅ Supports sm_50 through sm_121 (Maxwell 2014 → Blackwell 2024)
- ✅ Native Blackwell sm_120 compilation
- ✅ Last CUDA version supporting sm_50 (CUDA 13.0+ drops sm_50)

### Architecture-Specific Sizes

| Package | CUDA Provider Size | Total Uncompressed |
|---------|-------------------|-------------------|
| Legacy (5 archs) | 223 MB | 2.3 GB |
| Modern (3 archs) | 368 MB | 2.3 GB |
| Latest (4 archs) | 682 MB | 2.3 GB |

The CUDA provider library size increases with more architectures because it contains compiled kernels for each sm_XX version.

## Verification

### Verify Installed Libraries

```bash
ls -lh ~/.local/share/swictation/gpu-libs/
```

You should see:
- 3 ONNX Runtime libraries (~22MB total)
- 6 CUDA runtime libraries (~1.5GB total)
- 8 cuDNN libraries (~900MB total)

### Verify Package Metadata

```bash
cat ~/.config/swictation/gpu-package-info.json
```

This file contains:
- Detected GPU name and compute capability
- Selected package variant
- Installation timestamp

### Test GPU Acceleration

```bash
# Start swictation daemon
swictation start

# Check logs for CUDA provider loading
journalctl --user -u swictation-daemon -n 50 | grep -i cuda

# You should see:
# "CUDA provider loaded successfully"
# "Using GPU: [Your GPU Name]"
```

## Troubleshooting

### Library Not Found Errors

If you see errors like `libcublas.so.12: cannot open shared object file`:

1. Check library installation:
   ```bash
   ls -lh ~/.local/share/swictation/gpu-libs/libcublas.so.12
   ```

2. Verify systemd service has correct LD_LIBRARY_PATH:
   ```bash
   systemctl --user cat swictation-daemon.service | grep LD_LIBRARY_PATH
   ```

3. Re-run setup:
   ```bash
   swictation setup
   ```

### Wrong Package Downloaded

If automatic detection selected the wrong package:

1. Check detected GPU:
   ```bash
   cat ~/.config/swictation/gpu-package-info.json
   ```

2. Manually install correct package (see Manual Installation above)

### GPU Not Detected

If you have an NVIDIA GPU but it's not detected:

1. Install nvidia-driver and nvidia-smi:
   ```bash
   sudo ubuntu-drivers autoinstall
   nvidia-smi  # Should show GPU info
   ```

2. Re-run npm postinstall:
   ```bash
   cd /usr/local/lib/node_modules/swictation
   sudo -E npm run postinstall
   ```

## Minimum Requirements

- **CUDA Driver:** 12.0+ (NVIDIA driver ≥525.60.13)
- **ONNX Runtime:** 1.23.0+
- **Linux:** Ubuntu 20.04+, similar distributions
- **Disk Space:** ~2.5GB for GPU libraries

## Support

- **GitHub Release:** https://github.com/robertelee78/swictation/releases/tag/gpu-libs-v1.1.0
- **Build Instructions:** See `docker/onnxruntime-builder/README.md`
- **Issues:** https://github.com/robertelee78/swictation/issues
