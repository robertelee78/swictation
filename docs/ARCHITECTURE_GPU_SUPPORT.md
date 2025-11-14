# GPU Support Architecture for Swictation

**Status**: Architecture Decision Record (ADR)
**Date**: 2025-11-13
**Author**: System Architecture Designer
**Project ID**: fbeae03f-cd20-47a1-abf2-c9be91af34ca

## Executive Summary

This document analyzes four architectural approaches for supporting multiple NVIDIA GPU generations (compute capabilities sm_50 through sm_90+) in the swictation npm package, and recommends **Approach 2: Multiple binaries with runtime detection** as the optimal solution.

**Current State:**
- 330MB `libonnxruntime_providers_cuda.so` excluded from npm package
- GPU libraries downloaded from GitHub releases during postinstall (gpu-libs-v1.0.1)
- Limited compute capability coverage (sm_80, sm_86, sm_89 detected)
- Excellent user experience: automatic GPU detection and library download

**Recommended Solution:**
- Maintain current GitHub release download approach
- Create **3 separate GPU library packages** for different compute capability ranges
- Runtime detection selects and downloads appropriate package
- **Total size reduction: 330MB → ~110-150MB per user** (only downloads what they need)

---

## Problem Statement

### Requirements

1. **GPU Generation Coverage**: Support NVIDIA GPUs from Maxwell (2014) through Ada Lovelace/Hopper (2024+)
   - sm_50, sm_52: Maxwell (GTX 900 series)
   - sm_60, sm_61: Pascal (GTX 10 series)
   - sm_70, sm_75: Volta/Turing (GTX 16/RTX 20 series)
   - sm_80, sm_86: Ampere (RTX 30 series)
   - sm_89, sm_90: Ada Lovelace/Hopper (RTX 40 series, H100)

2. **Package Size Constraints**
   - npm practical limit: ~100-200MB unpacked (larger packages work but cause slow installs)
   - GitHub release asset limit: 2GB per file (not a constraint)
   - Current CUDA provider: 330MB for limited compute capabilities

3. **User Experience Goals**
   - Zero configuration for most users
   - Automatic GPU detection
   - Fast installation (< 2 minutes on typical connections)
   - No build tools required
   - Compatible with Ubuntu 24.04+ (GLIBC 2.39+)

4. **Performance Requirements**
   - GPU acceleration: 10-62x realtime transcription speed
   - No runtime overhead from abstraction layers
   - Optimal CUDA kernel selection for each GPU

### Current Architecture

```
npm install swictation
  ↓
postinstall.js runs
  ↓
Detects NVIDIA GPU (nvidia-smi)
  ↓
Downloads gpu-libs-v1.0.1 tarball from GitHub releases (~350MB)
  ↓
Extracts to lib/native/:
  - libonnxruntime.so (22MB)
  - libonnxruntime_providers_cuda.so (330MB) ← PROBLEM: Limited compute capabilities
  - libonnxruntime_providers_shared.so (15KB)
  - libonnxruntime_providers_tensorrt.so (787KB)
  - libsherpa-onnx-c-api.so (3.8MB)
  - libsherpa-onnx-cxx-api.so (84KB)
```

**Strengths:**
- ✅ GPU libraries excluded from npm package (keeps base package small)
- ✅ Automatic detection and download (excellent UX)
- ✅ Only downloads if GPU detected (saves bandwidth for CPU users)
- ✅ Version independent (gpu-libs-v1.0.1 can update independently)

**Weaknesses:**
- ❌ Limited compute capability coverage (only sm_80, sm_86, sm_89)
- ❌ 330MB download even for users with older GPUs
- ❌ No optimization for specific GPU generations

---

## Architecture Options Analysis

### Option 1: Single Binary with Multiple Compute Capabilities

**Description:**
Compile `libonnxruntime_providers_cuda.so` with all compute capabilities (sm_50,52,60,61,70,75,80,86,89,90) in one library.

#### Architecture Diagram
```
libonnxruntime_providers_cuda.so (estimated 500-700MB)
├── CUDA kernels for sm_50 (Maxwell)
├── CUDA kernels for sm_52 (Maxwell)
├── CUDA kernels for sm_60 (Pascal)
├── CUDA kernels for sm_61 (Pascal)
├── CUDA kernels for sm_70 (Volta)
├── CUDA kernels for sm_75 (Turing)
├── CUDA kernels for sm_80 (Ampere)
├── CUDA kernels for sm_86 (Ampere)
├── CUDA kernels for sm_89 (Ada Lovelace)
└── CUDA kernels for sm_90 (Hopper)

Runtime: CUDA driver selects appropriate kernels based on GPU
```

#### Detailed Analysis

**Package Size Impact:**
- Current (3 capabilities): 330MB
- Estimated (10 capabilities): **500-700MB**
- Size increase: ~2x current size
- **Each compute capability adds ~50-70MB** of compiled kernels

**Implementation:**
```bash
# Build command
nvcc -gencode arch=compute_50,code=sm_50 \
     -gencode arch=compute_52,code=sm_52 \
     -gencode arch=compute_60,code=sm_60 \
     -gencode arch=compute_61,code=sm_61 \
     -gencode arch=compute_70,code=sm_70 \
     -gencode arch=compute_75,code=sm_75 \
     -gencode arch=compute_80,code=sm_80 \
     -gencode arch=compute_86,code=sm_86 \
     -gencode arch=compute_89,code=sm_89 \
     -gencode arch=compute_90,code=sm_90 \
     libonnxruntime_providers_cuda.so
```

**Pros:**
- ✅ **Simplest architecture**: One library for all GPUs
- ✅ **No runtime detection needed**: CUDA driver handles selection
- ✅ **Zero user configuration**: Works automatically
- ✅ **Guaranteed compatibility**: All GPUs supported out of the box
- ✅ **Easy maintenance**: Single build pipeline

**Cons:**
- ❌ **Massive download size**: 500-700MB for every user
- ❌ **Wasted bandwidth**: GTX 1060 user downloads RTX 4090 kernels they'll never use
- ❌ **Slow installs**: 5-10 minutes on slower connections
- ❌ **Storage waste**: Each user stores kernels for 9 GPU generations they don't have
- ❌ **May exceed npm best practices**: Though technically possible, 700MB packages are problematic

**User Experience:**
```bash
$ npm install swictation
# Downloads 700MB of GPU kernels
# User with GTX 1060 (sm_61): Uses 70MB, wastes 630MB
# User with RTX 4090 (sm_89): Uses 70MB, wastes 630MB
```

**Performance:**
- ✅ **Zero runtime overhead**: CUDA driver selects optimal kernels
- ✅ **Full optimization**: Each GPU gets architecture-specific kernels

**Maintenance:**
- ✅ **Simple CI/CD**: One build configuration
- ✅ **Easy testing**: Single artifact to validate
- ❌ **Long build times**: Compiling 10 compute capabilities takes hours

**Verdict:** ❌ **NOT RECOMMENDED**
- Simplicity doesn't justify 500-700MB download for all users
- Wasted bandwidth and storage on unused code
- Better solutions exist

---

### Option 2: Multiple Binaries with Runtime Detection ⭐ RECOMMENDED

**Description:**
Create 3 separate GPU library packages for different generation ranges. Runtime detection identifies user's GPU and downloads only the appropriate package.

#### Architecture Diagram
```
GPU Detection Layer (postinstall.js)
├── nvidia-smi --query-gpu=compute_cap --format=csv,noheader
│   ↓
├── sm_50-70 detected → Download cuda-libs-legacy.tar.gz (150MB)
│   └── Maxwell/Pascal/Volta: GTX 900/1000, Quadro P-series
│
├── sm_75-86 detected → Download cuda-libs-modern.tar.gz (180MB)
│   └── Turing/Ampere: GTX 16/RTX 20/30 series, A100
│
└── sm_89-90 detected → Download cuda-libs-latest.tar.gz (150MB)
    └── Ada Lovelace/Hopper: RTX 40 series, H100

Each package contains:
  - libonnxruntime_providers_cuda.so (optimized for range)
  - libonnxruntime.so (22MB, shared)
  - libonnxruntime_providers_shared.so (15KB, shared)
  - Other libs (sherpa-onnx, tensorrt)
```

#### Package Breakdown

**Package 1: cuda-libs-legacy.tar.gz (sm_50-70)**
```
Target GPUs:
  - GTX 980 Ti, GTX 1080 Ti, Titan X (Pascal)
  - Quadro P6000, Tesla P100
  - Titan V (Volta)

Compute capabilities: sm_50,52,60,61,70
Size: ~150MB
User base: ~15% of active NVIDIA users (legacy hardware)
```

**Package 2: cuda-libs-modern.tar.gz (sm_75-86)**
```
Target GPUs:
  - GTX 1660, RTX 2060/2070/2080 (Turing)
  - RTX 3060/3070/3080/3090 (Ampere)
  - A100, A6000 (Data center Ampere)

Compute capabilities: sm_75,80,86
Size: ~180MB
User base: ~70% of active NVIDIA users (mainstream)
```

**Package 3: cuda-libs-latest.tar.gz (sm_89-90)**
```
Target GPUs:
  - RTX 4060/4070/4080/4090 (Ada Lovelace)
  - L4, L40 (Data center Ada)
  - H100, H200 (Hopper)

Compute capabilities: sm_89,90
Size: ~150MB
User base: ~15% of active NVIDIA users (latest hardware)
```

#### Implementation

**Modified postinstall.js:**
```javascript
async function detectAndDownloadGPU() {
  // Detect compute capability
  const computeCap = execSync(
    'nvidia-smi --query-gpu=compute_cap --format=csv,noheader',
    { encoding: 'utf8' }
  ).trim();

  const [major, minor] = computeCap.split('.').map(Number);
  const sm = major * 10 + minor;

  // Select appropriate package
  let packageName, packageVersion;

  if (sm >= 50 && sm <= 70) {
    packageName = 'cuda-libs-legacy';
    packageVersion = GPU_LIBS_VERSION;
    log('cyan', `Detected legacy GPU (sm_${sm}) - downloading optimized package`);
  } else if (sm >= 75 && sm <= 86) {
    packageName = 'cuda-libs-modern';
    packageVersion = GPU_LIBS_VERSION;
    log('cyan', `Detected modern GPU (sm_${sm}) - downloading optimized package`);
  } else if (sm >= 89 && sm <= 90) {
    packageName = 'cuda-libs-latest';
    packageVersion = GPU_LIBS_VERSION;
    log('cyan', `Detected latest GPU (sm_${sm}) - downloading optimized package`);
  } else {
    log('yellow', `Unknown compute capability: sm_${sm}`);
    log('cyan', 'Falling back to modern package (sm_75-86)');
    packageName = 'cuda-libs-modern';
    packageVersion = GPU_LIBS_VERSION;
  }

  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${packageVersion}/${packageName}.tar.gz`;

  // Download and extract
  await downloadFile(releaseUrl, tarPath);
  execSync(`tar -xzf "${tarPath}" -C "${nativeDir}"`, { stdio: 'inherit' });

  log('green', `✓ GPU libraries installed (optimized for sm_${sm})`);
}
```

**GitHub Releases Structure:**
```
Releases:
  gpu-libs-v1.1.0/
    ├── cuda-libs-legacy.tar.gz   (150MB, sm_50-70)
    ├── cuda-libs-modern.tar.gz   (180MB, sm_75-86)
    └── cuda-libs-latest.tar.gz   (150MB, sm_89-90)

Total release size: 480MB (but users only download one)
```

#### Detailed Analysis

**Pros:**
- ✅ **Optimal size per user**: 150-180MB vs 500-700MB (65-74% reduction)
- ✅ **Better user experience**: Faster downloads, less storage
- ✅ **Architecture-optimized**: Each package contains only relevant kernels
- ✅ **Backward compatible**: Legacy GPUs fully supported
- ✅ **Future-proof**: Easy to add sm_100 package when new architecture launches
- ✅ **Transparent to users**: Detection and download automatic
- ✅ **Graceful fallback**: Unknown GPUs get "modern" package (best compatibility)
- ✅ **Easy A/B testing**: Can measure download/performance by GPU generation

**Cons:**
- ⚠️ **Moderate complexity**: Need to build 3 packages instead of 1
- ⚠️ **Requires nvidia-smi**: Must parse compute capability (already required)
- ⚠️ **3x CI/CD builds**: Build pipeline needs to generate 3 artifacts
- ⚠️ **Testing overhead**: Should test all 3 packages on representative GPUs

**User Experience:**

```bash
# User with RTX 3080 (sm_86)
$ npm install swictation

Detecting GPU...
✓ NVIDIA GeForce RTX 3080 (sm_86)
Downloading optimized libraries (180MB)...
✓ GPU acceleration enabled!

Total download: 180MB (vs 500-700MB with Option 1)
```

**Performance:**
- ✅ **Zero runtime overhead**: Pre-compiled for specific range
- ✅ **Optimal kernels**: Each GPU generation gets tailored code
- ✅ **Better cache utilization**: Smaller binary = better instruction cache hits

**Maintenance:**
- ⚠️ **Medium complexity**: 3 build configurations
- ✅ **Parallel builds**: Can build all 3 simultaneously in CI/CD
- ✅ **Independent versioning**: Can update modern package without rebuilding legacy
- ✅ **Clear separation**: Each package has well-defined target GPUs

**Cost-Benefit Analysis:**
```
Cost:
  - Build time: 3x (but parallelizable)
  - Testing: 3 test matrices (legacy/modern/latest)
  - Storage: 480MB total on GitHub (vs 700MB single)
  - Code complexity: +150 lines in postinstall.js

Benefit:
  - User bandwidth: 65-74% reduction (150-180MB vs 500-700MB)
  - User storage: Same reduction per installation
  - Install time: 50-60% faster (2-3 min vs 5-7 min on slow connections)
  - Better performance: Architecture-specific optimizations
```

**Verdict:** ✅ **RECOMMENDED**
- Best balance of user experience, performance, and maintenance
- Significant bandwidth/storage savings
- Acceptable complexity increase
- Proven pattern (similar to Node.js native modules, Electron)

---

### Option 3: Separate Download Packages (User-Driven)

**Description:**
Don't auto-download GPU libraries. Instead, provide manual download commands and let users choose their package.

#### Architecture Diagram
```
npm install swictation (base package only, no GPU libs)
  ↓
User runs detection: swictation detect-gpu
  ↓
Output:
  "RTX 3080 detected (sm_86)"
  "Run: swictation download-gpu modern"
  ↓
User manually runs: swictation download-gpu modern
  ↓
Downloads cuda-libs-modern.tar.gz (180MB)
```

#### Implementation

**Modified installation flow:**
```javascript
// postinstall.js - NO automatic GPU download
async function main() {
  checkPlatform();
  ensureBinaryPermissions();
  createDirectories();

  // Detect GPU but don't download
  const gpuInfo = detectGPUVRAM();

  if (gpuInfo.hasGPU) {
    log('green', '✓ NVIDIA GPU detected!');
    log('yellow', '\nTo enable GPU acceleration, run:');

    if (gpuInfo.computeCap >= 89) {
      log('cyan', '  swictation download-gpu latest');
    } else if (gpuInfo.computeCap >= 75) {
      log('cyan', '  swictation download-gpu modern');
    } else {
      log('cyan', '  swictation download-gpu legacy');
    }

    log('cyan', '\nOr download all packages:');
    log('cyan', '  swictation download-gpu all');
  } else {
    log('cyan', 'No GPU detected - CPU mode will be used');
  }

  generateSystemdService(null); // No ORT path yet
}
```

**CLI commands:**
```javascript
// bin/swictation additions
commands:
  detect-gpu        Show GPU capabilities and recommended package
  download-gpu      Download GPU libraries (legacy|modern|latest|all)

Examples:
  swictation detect-gpu
    Output:
      GPU: NVIDIA GeForce RTX 3080
      Compute Capability: 8.6 (sm_86)
      Recommended: modern

  swictation download-gpu modern
    Downloads: cuda-libs-modern.tar.gz (180MB)
    Installs to: lib/native/
```

#### Detailed Analysis

**Pros:**
- ✅ **Zero automatic downloads**: Users in full control
- ✅ **Bandwidth conscious**: Download only when user explicitly wants GPU
- ✅ **Multiple package support**: Advanced users can download "all" for testing
- ✅ **Transparent sizing**: User sees exactly what they're downloading
- ✅ **Offline-friendly**: Can download packages on one machine, transfer to another
- ✅ **CI/CD friendly**: No surprise downloads in build pipelines

**Cons:**
- ❌ **Poor user experience**: Extra manual step required
- ❌ **Common mistake**: Users forget to download, wonder why GPU doesn't work
- ❌ **Documentation burden**: Must clearly explain download process
- ❌ **Support burden**: "GPU not working" → "Did you download GPU libraries?"
- ❌ **Inconsistent behavior**: Some users get GPU, some don't (based on manual step)
- ❌ **Higher barrier to entry**: Requires understanding of package system

**User Experience:**

```bash
# New user workflow
$ npm install swictation
✓ Installed!
⚠ GPU detected but libraries not downloaded
  Run: swictation download-gpu modern

$ swictation start
ERROR: GPU libraries not found
  Run: swictation download-gpu modern

$ swictation download-gpu modern
Downloading cuda-libs-modern.tar.gz (180MB)...
✓ GPU libraries installed

$ swictation start
✓ Started with GPU acceleration
```

**Performance:**
- ✅ **Same as Option 2**: Architecture-specific packages
- ✅ **No overhead**: Pre-compiled binaries

**Maintenance:**
- ✅ **Same as Option 2**: 3 build configurations
- ⚠️ **Additional CLI commands**: Need detect-gpu, download-gpu
- ⚠️ **More documentation**: User guide for manual download

**Verdict:** ⚠️ **NOT RECOMMENDED FOR GENERAL USE**
- Adds friction to user experience
- Increases support burden ("why isn't GPU working?")
- **Only useful for:**
  - Corporate environments with restricted internet
  - CI/CD pipelines that want explicit control
  - Users with bandwidth caps who want to defer download

**Alternative:** Could offer this as **opt-in behavior** via environment variable:
```bash
SWICTATION_MANUAL_GPU=1 npm install swictation
# Skips auto-download, user must run: swictation download-gpu
```

---

### Option 4: Build from Source During Postinstall

**Description:**
Detect user's GPU during postinstall and compile CUDA kernels only for that specific compute capability.

#### Architecture Diagram
```
npm install swictation
  ↓
postinstall.js detects GPU (nvidia-smi)
  ↓
RTX 3080 (sm_86) detected
  ↓
Downloads CUDA toolkit + ONNX Runtime source (~2GB)
  ↓
Compiles with: -gencode arch=compute_86,code=sm_86
  ↓
Generates: libonnxruntime_providers_cuda.so (~70MB, sm_86 only)
  ↓
Install complete (took 15-30 minutes)
```

#### Implementation

**postinstall.js:**
```javascript
async function buildFromSource() {
  log('cyan', 'Building ONNX Runtime CUDA provider from source...');

  // Check prerequisites
  const required = ['nvcc', 'cmake', 'g++', 'git'];
  for (const tool of required) {
    if (!hasCommand(tool)) {
      throw new Error(`Required tool missing: ${tool}`);
    }
  }

  // Detect compute capability
  const sm = detectComputeCapability();
  log('cyan', `Detected GPU: sm_${sm}`);

  // Clone ONNX Runtime
  execSync('git clone --depth 1 https://github.com/microsoft/onnxruntime.git',
           { stdio: 'inherit' });

  // Build with specific compute capability
  const buildCmd = `
    cd onnxruntime
    ./build.sh --config Release --build_shared_lib \
      --use_cuda --cuda_home=/usr/local/cuda \
      --cudnn_home=/usr/lib/x86_64-linux-gnu \
      --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES=${sm}
  `;

  execSync(buildCmd, { stdio: 'inherit' }); // Takes 15-30 minutes

  // Copy built library
  fs.copyFileSync(
    'onnxruntime/build/Linux/Release/libonnxruntime_providers_cuda.so',
    path.join(__dirname, 'lib/native/libonnxruntime_providers_cuda.so')
  );

  log('green', '✓ Built ONNX Runtime CUDA provider for sm_${sm}');
}
```

#### Detailed Analysis

**Pros:**
- ✅ **Minimal final size**: Only ~70MB for user's specific GPU
- ✅ **Perfectly optimized**: Exact architecture match
- ✅ **No wasted code**: Zero unused compute capability kernels
- ✅ **Flexible**: Can compile with any CUDA version user has installed
- ✅ **Latest features**: Can build from onnxruntime main branch

**Cons:**
- ❌ **Extremely slow installation**: 15-30 minutes compile time
- ❌ **Requires build tools**: nvcc, cmake, g++, git (heavy dependencies)
- ❌ **CUDA Toolkit required**: 3-4GB download + installation
- ❌ **Complex dependencies**: Different distros have different CUDA paths
- ❌ **High failure rate**: Build errors, missing dependencies, CUDA version mismatches
- ❌ **Non-deterministic builds**: Different CUDA versions produce different binaries
- ❌ **Poor CI/CD**: Can't test what users will build
- ❌ **Massive support burden**: "Build failed" issues across distros/CUDA versions
- ❌ **Storage requirements**: ~5-10GB during build (ONNX source + build artifacts)

**User Experience:**

```bash
$ npm install swictation

Checking prerequisites...
✗ ERROR: nvcc not found
  Install CUDA Toolkit: https://developer.nvidia.com/cuda-downloads
  (3.5GB download + 4GB disk space)

# User installs CUDA Toolkit (30 minutes)

$ npm install swictation

Building ONNX Runtime from source...
Cloning onnxruntime... (5 minutes)
Running CMake... (3 minutes)
Compiling CUDA kernels... (15 minutes)
Linking... (2 minutes)

Total install time: 25-30 minutes
vs Option 2: 2-3 minutes
```

**Performance:**
- ✅ **Perfectly optimized**: Single architecture build
- ⚠️ **Build quality varies**: Depends on user's CUDA version, compiler flags

**Maintenance:**
- ❌ **Impossible to support**: Too many variables (distros, CUDA versions, hardware)
- ❌ **Can't reproduce user builds**: Each build is unique
- ❌ **Documentation nightmare**: Need guides for Ubuntu/Fedora/Arch × CUDA 11/12/13

**Verdict:** ❌ **STRONGLY NOT RECOMMENDED**
- Completely violates "no build tools required" requirement
- Installation time 10x longer than download (25-30 min vs 2-3 min)
- High failure rate will generate massive support burden
- Used by Python packages (pip install onnxruntime-gpu) but they have different expectations

**When This Works:**
- Research environments where users already have CUDA development environment
- Niche use cases requiring bleeding-edge ONNX Runtime features
- Not appropriate for general-purpose npm package

---

## Comparison Matrix

| Criterion | Option 1: Single Binary | Option 2: Multiple Binaries ⭐ | Option 3: User Download | Option 4: Build Source |
|-----------|------------------------|-------------------------------|------------------------|----------------------|
| **User download size** | 500-700MB | 150-180MB | 150-180MB | ~5GB (CUDA + source) |
| **Install time (typical)** | 5-7 minutes | 2-3 minutes | 1 min + manual step | 25-30 minutes |
| **Storage waste** | ~630MB unused | ~0MB (optimal) | ~0MB (optimal) | ~0MB (optimal) |
| **User experience** | ⭐⭐⭐⭐⭐ Automatic | ⭐⭐⭐⭐⭐ Automatic | ⭐⭐ Manual step | ⭐ Complex setup |
| **Setup complexity** | Zero config | Zero config | Manual command | Install CUDA Toolkit |
| **Build complexity** | ⭐ Simple | ⭐⭐ Medium | ⭐⭐ Medium | ⭐⭐⭐⭐⭐ Very complex |
| **Maintenance burden** | ⭐⭐ Low | ⭐⭐⭐ Medium | ⭐⭐⭐ Medium | ⭐⭐⭐⭐⭐ Very high |
| **Support burden** | ⭐⭐ Low | ⭐⭐ Low | ⭐⭐⭐⭐ High | ⭐⭐⭐⭐⭐ Very high |
| **Performance** | ⭐⭐⭐⭐⭐ Optimal | ⭐⭐⭐⭐⭐ Optimal | ⭐⭐⭐⭐⭐ Optimal | ⭐⭐⭐⭐⭐ Optimal |
| **Backward compat** | ⭐⭐⭐⭐⭐ All GPUs | ⭐⭐⭐⭐⭐ All GPUs | ⭐⭐⭐⭐⭐ All GPUs | ⭐⭐⭐⭐ User's GPU only |
| **CI/CD friendly** | ⭐⭐⭐⭐⭐ Yes | ⭐⭐⭐⭐ Yes | ⭐⭐⭐⭐⭐ Yes | ❌ No |
| **Bandwidth efficiency** | ⭐ Poor | ⭐⭐⭐⭐⭐ Excellent | ⭐⭐⭐⭐⭐ Excellent | ⭐ Poor (initial) |
| **Future-proof** | ⭐⭐⭐ Must rebuild for new arch | ⭐⭐⭐⭐⭐ Add new package | ⭐⭐⭐⭐⭐ Add new package | ⭐⭐⭐⭐ User rebuilds |
| **Offline installation** | ⭐⭐⭐⭐ Download once | ⭐⭐⭐⭐ Download once | ⭐⭐⭐⭐⭐ Explicit download | ❌ Needs internet |

**Legend:** ⭐⭐⭐⭐⭐ Excellent, ⭐⭐⭐⭐ Good, ⭐⭐⭐ Acceptable, ⭐⭐ Poor, ⭐ Very Poor

---

## Recommendation: Option 2 with Enhancements

### Primary Recommendation

**Adopt Option 2: Multiple Binaries with Runtime Detection**

This provides the best balance of:
1. **User Experience**: Automatic, fast, zero-configuration
2. **Efficiency**: 65-74% size reduction vs single binary
3. **Performance**: Architecture-optimized kernels
4. **Maintainability**: Moderate complexity, well-defined builds
5. **Scalability**: Easy to add sm_100 when Blackwell launches

### Enhanced Architecture

#### Package Structure
```
GitHub Releases (gpu-libs-v1.1.0):
  ├── cuda-libs-legacy.tar.gz     (sm_50-70: 150MB)
  │   Target: GTX 900/1000, Quadro P, Titan V
  │   User base: ~15% (legacy gamers, workstations)
  │
  ├── cuda-libs-modern.tar.gz     (sm_75-86: 180MB)
  │   Target: GTX 16, RTX 20/30, A100
  │   User base: ~70% (mainstream, most popular)
  │
  └── cuda-libs-latest.tar.gz     (sm_89-90: 150MB)
      Target: RTX 40, H100, L4/L40
      User base: ~15% (enthusiasts, data centers)

Total: 480MB storage, but users download 150-180MB max
```

#### Detection Logic (Enhanced)
```javascript
async function detectAndDownloadGPU() {
  // Step 1: Detect compute capability
  const computeCapRaw = execSync(
    'nvidia-smi --query-gpu=compute_cap --format=csv,noheader',
    { encoding: 'utf8' }
  ).trim();

  const [major, minor] = computeCapRaw.split('.').map(Number);
  const sm = major * 10 + minor;

  // Step 2: Get GPU name for user-friendly messaging
  const gpuName = execSync(
    'nvidia-smi --query-gpu=name --format=csv,noheader',
    { encoding: 'utf8' }
  ).trim();

  // Step 3: Select package with fallback logic
  let packageName, packageSize, packageDesc;

  if (sm >= 89 && sm <= 90) {
    packageName = 'cuda-libs-latest';
    packageSize = '150MB';
    packageDesc = 'Latest generation (Ada Lovelace/Hopper)';
  } else if (sm >= 75 && sm <= 86) {
    packageName = 'cuda-libs-modern';
    packageSize = '180MB';
    packageDesc = 'Modern generation (Turing/Ampere)';
  } else if (sm >= 50 && sm <= 70) {
    packageName = 'cuda-libs-legacy';
    packageSize = '150MB';
    packageDesc = 'Legacy generation (Maxwell/Pascal/Volta)';
  } else {
    log('yellow', `⚠️  Unknown compute capability: sm_${sm}`);
    log('cyan', '   Falling back to modern package (widest compatibility)');
    packageName = 'cuda-libs-modern';
    packageSize = '180MB';
    packageDesc = 'Modern generation (fallback)';
  }

  log('green', `✓ GPU Detected: ${gpuName} (sm_${sm})`);
  log('cyan', `  Package: ${packageDesc}`);
  log('cyan', `  Download size: ${packageSize}`);

  // Step 4: Download with retry logic
  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${GPU_LIBS_VERSION}/${packageName}.tar.gz`;

  const maxRetries = 3;
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      log('cyan', `  Downloading (attempt ${attempt}/${maxRetries})...`);
      await downloadFileWithProgress(releaseUrl, tarPath);
      log('green', '  ✓ Download complete');
      break;
    } catch (err) {
      if (attempt === maxRetries) {
        throw new Error(`Failed to download after ${maxRetries} attempts: ${err.message}`);
      }
      log('yellow', `  ⚠️  Download failed, retrying in 2s...`);
      await sleep(2000);
    }
  }

  // Step 5: Verify checksum (optional but recommended)
  // TODO: Add SHA256 verification

  // Step 6: Extract
  log('cyan', '  Extracting...');
  execSync(`tar -xzf "${tarPath}" -C "${nativeDir}"`, { stdio: 'inherit' });
  log('green', `  ✓ GPU libraries installed (optimized for sm_${sm})`);

  // Step 7: Save metadata for runtime verification
  const gpuMetadata = {
    computeCapability: sm,
    gpuName: gpuName,
    packageName: packageName,
    packageVersion: GPU_LIBS_VERSION,
    installedAt: new Date().toISOString()
  };

  fs.writeFileSync(
    path.join(__dirname, 'lib', 'native', '.gpu-metadata.json'),
    JSON.stringify(gpuMetadata, null, 2)
  );
}

function downloadFileWithProgress(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    let downloadedBytes = 0;

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        return downloadFileWithProgress(response.headers.location, dest)
          .then(resolve).catch(reject);
      }

      const totalBytes = parseInt(response.headers['content-length'], 10);

      response.on('data', (chunk) => {
        downloadedBytes += chunk.length;
        const percent = ((downloadedBytes / totalBytes) * 100).toFixed(1);
        process.stdout.write(`\r  Progress: ${percent}% (${(downloadedBytes / 1024 / 1024).toFixed(1)}MB / ${(totalBytes / 1024 / 1024).toFixed(1)}MB)`);
      });

      response.pipe(file);

      file.on('finish', () => {
        file.close();
        console.log(''); // New line after progress
        resolve();
      });

    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}
```

#### Build Pipeline (GitHub Actions)

```yaml
name: Build GPU Libraries

on:
  push:
    tags:
      - 'gpu-libs-v*'

jobs:
  build-legacy:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v3

      - name: Install CUDA 12.x
        uses: Jimver/cuda-toolkit@v0.2.11
        with:
          cuda: '12.6.0'

      - name: Build ONNX Runtime (sm_50-70)
        run: |
          ./build.sh --config Release --build_shared_lib \
            --use_cuda --cuda_home=/usr/local/cuda \
            --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;52;60;61;70"

      - name: Package libraries
        run: |
          mkdir -p package
          cp build/Linux/Release/libonnxruntime*.so package/
          tar -czf cuda-libs-legacy.tar.gz -C package .

      - name: Upload to release
        uses: actions/upload-release-asset@v1
        with:
          asset_path: cuda-libs-legacy.tar.gz
          asset_name: cuda-libs-legacy.tar.gz

  build-modern:
    # Similar to legacy, but CMAKE_CUDA_ARCHITECTURES="75;80;86"

  build-latest:
    # Similar to legacy, but CMAKE_CUDA_ARCHITECTURES="89;90"
```

---

## Implementation Plan

### Phase 1: Build Infrastructure (Week 1)

**Tasks:**
1. Create GitHub Actions workflow for 3-package builds
2. Build and test all 3 packages locally
3. Verify package sizes (target: legacy/latest 150MB, modern 180MB)
4. Create SHA256 checksums for verification

**Deliverables:**
- `.github/workflows/build-gpu-libs.yml`
- Three tar.gz files on GitHub releases
- Checksum file (SHA256SUMS)

### Phase 2: Detection & Download Logic (Week 2)

**Tasks:**
1. Implement compute capability detection
2. Add package selection logic with fallbacks
3. Implement download with progress bar and retries
4. Add checksum verification
5. Store metadata for runtime verification

**Deliverables:**
- Updated `npm-package/postinstall.js`
- Metadata file: `.gpu-metadata.json`
- Unit tests for detection logic

### Phase 3: Testing & Validation (Week 3)

**Hardware test matrix:**
- Legacy: GTX 1060 (sm_61)
- Modern: RTX 3080 (sm_86)
- Latest: RTX 4090 (sm_89)

**Tests:**
1. Package selection correctness
2. Download retry on network failure
3. Checksum verification
4. Fallback to modern package for unknown GPUs
5. Performance benchmarks (ensure no regression)

**Deliverables:**
- Test results document
- Performance comparison (old vs new packages)

### Phase 4: Documentation & Release (Week 4)

**Tasks:**
1. Update README with package details
2. Document manual override (environment variable)
3. Create troubleshooting guide
4. Release gpu-libs-v1.1.0 with 3 packages
5. Update npm package to v0.3.15

**Deliverables:**
- Updated README.md
- docs/GPU_PACKAGES.md (technical details)
- docs/TROUBLESHOOTING_GPU.md
- Release notes

---

## Risk Mitigation

### Risk 1: Download Failures
**Impact**: Medium (users can't get GPU acceleration)

**Mitigation:**
- Retry logic (3 attempts with 2s delay)
- Fallback to CPU-only mode if all downloads fail
- Clear error messages with manual download instructions
- Manual download command: `swictation download-gpu modern`

### Risk 2: Package Selection Errors
**Impact**: Low (wrong package downloaded, but should still work)

**Mitigation:**
- Conservative fallback (modern package has wide compatibility)
- Runtime verification (daemon checks GPU matches package)
- Metadata logging for debugging
- Clear logging of detected GPU and selected package

### Risk 3: Build Complexity
**Impact**: Medium (CI/CD takes longer, more failure points)

**Mitigation:**
- Parallel builds in GitHub Actions (all 3 at once)
- Separate GPU library versioning (independent from npm package)
- Pre-built packages (don't rebuild on every npm version bump)
- Automated testing before release

### Risk 4: Storage Costs
**Impact**: Low (GitHub releases free for open source)

**Mitigation:**
- 480MB total is well within GitHub limits
- Can delete old gpu-libs versions (keep last 3)
- Consider mirror on CDN if GitHub bandwidth becomes issue

---

## Alternative Considerations

### Hybrid Approach: Option 2 + Option 3

For power users who want explicit control:

```bash
# Environment variable to skip auto-download
SWICTATION_MANUAL_GPU=1 npm install swictation

# Then manually download desired package
swictation download-gpu modern
```

**Benefits:**
- Gives advanced users control
- Useful for CI/CD pipelines
- Allows offline installation workflows

**Implementation:**
```javascript
// postinstall.js
if (process.env.SWICTATION_MANUAL_GPU === '1') {
  log('cyan', 'SWICTATION_MANUAL_GPU=1 detected');
  log('yellow', 'Skipping automatic GPU library download');
  log('cyan', 'Run "swictation download-gpu <package>" when ready');
  return;
}
```

### CDN Caching

If GitHub bandwidth becomes an issue:

```javascript
// Try CDN first, fall back to GitHub
const cdnUrl = `https://cdn.swictation.io/gpu-libs-v${version}/${packageName}.tar.gz`;
const githubUrl = `https://github.com/.../releases/download/gpu-libs-v${version}/${packageName}.tar.gz`;

try {
  await downloadFile(cdnUrl, tarPath);
} catch {
  log('cyan', 'CDN unavailable, trying GitHub...');
  await downloadFile(githubUrl, tarPath);
}
```

---

## Success Metrics

### User Experience
- Installation time: < 3 minutes on 50Mbps connection
- Download success rate: > 99%
- GPU detection accuracy: > 99.5%
- Support tickets re: GPU setup: < 5% of userbase

### Performance
- No performance regression vs current implementation
- GPU utilization: > 90% during transcription
- Memory footprint: < 2GB for 1.1B model

### Maintenance
- Build time: < 30 minutes for all 3 packages
- CI/CD success rate: > 95%
- Time to add new architecture (sm_100): < 1 day

---

## Conclusion

**Recommendation: Implement Option 2 (Multiple Binaries with Runtime Detection)**

This architecture provides:
1. ✅ **65-74% size reduction** per user (150-180MB vs 500-700MB)
2. ✅ **Automatic, zero-config** user experience
3. ✅ **Architecture-optimized** performance for each GPU generation
4. ✅ **Backward compatible** with all NVIDIA GPUs since 2014
5. ✅ **Future-proof** design (easy to add new architectures)
6. ✅ **Maintainable** complexity (moderate increase vs significant benefit)

**Implementation timeline:** 4 weeks
**Estimated effort:** 60-80 hours
**Risk level:** Low-Medium (well-understood pattern, clear fallbacks)

### Next Steps

1. ✅ Get approval from maintainer (@robertelee78)
2. Create Archon task for implementation tracking
3. Set up GPU library build pipeline
4. Implement detection and download logic
5. Test on representative hardware (GTX 1060, RTX 3080, RTX 4090)
6. Release gpu-libs-v1.1.0 with 3 packages
7. Update npm package (v0.3.15) with new postinstall logic

---

**Document Status**: ✅ Ready for Review
**Archon Project**: fbeae03f-cd20-47a1-abf2-c9be91af34ca
**Author**: System Architecture Designer
**Last Updated**: 2025-11-13
