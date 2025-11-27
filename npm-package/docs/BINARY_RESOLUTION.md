# Binary Resolution & GPU Library Management

## Overview

The `resolve-binary.js` module works together with `postinstall.js` to ensure correct binaries and GPU-specific libraries are used.

## Two-Stage Installation

### Stage 1: Platform Package Installation (Automatic)

When user runs `npm install -g swictation`:

1. npm detects platform (Linux x64 or macOS ARM64)
2. npm automatically installs the correct platform package via `optionalDependencies`:
   - `@agidreams/linux-x64` → Linux users
   - `@agidreams/darwin-arm64` → macOS users
3. Platform package contains:
   - Binaries: `swictation-daemon`, `swictation-ui`
   - Base ONNX Runtime library
   - Package structure: `bin/`, `lib/`

### Stage 2: GPU Library Detection & Download (postinstall.js)

After platform package is installed, `postinstall.js` runs:

```javascript
const { resolveBinaryPaths, getLibraryDirectory } = require('./src/resolve-binary.js');

// Find where platform package was installed
const paths = resolveBinaryPaths();
// {
//   platform: 'linux',
//   arch: 'x64',
//   packageDir: '/usr/local/lib/node_modules/@agidreams/linux-x64',
//   binDir: '/usr/local/lib/node_modules/@agidreams/linux-x64/bin',
//   libDir: '/usr/local/lib/node_modules/@agidreams/linux-x64/lib',  // GPU libs go here
//   daemon: '/usr/local/lib/node_modules/@agidreams/linux-x64/bin/swictation-daemon',
//   ui: '/usr/local/lib/node_modules/@agidreams/linux-x64/bin/swictation-ui'
// }

// Now detect GPU and download appropriate libraries to paths.libDir
```

## GPU Library Download Logic (Preserved)

### Linux (`@agidreams/linux-x64`)

postinstall.js continues to:

1. **Detect GPU:**
   ```javascript
   const gpuInfo = detectGPU(); // NVIDIA, AMD, Intel, or none
   ```

2. **Check VRAM:**
   ```javascript
   const vram = getGPUMemory(); // e.g., 24GB
   ```

3. **Check System RAM:**
   ```javascript
   const systemRAM = getSystemMemory(); // e.g., 64GB
   ```

4. **Download GPU-Specific Libraries to `paths.libDir`:**
   ```javascript
   const libDir = getLibraryDirectory(); // from resolve-binary.js

   if (gpuInfo.vendor === 'NVIDIA') {
     // Download to libDir:
     // - libcudart.so (CUDA 11.8 or 12.9 based on GPU generation)
     // - libcublas.so
     // - libcudnn.so (9.15.1)
     // - libonnxruntime_providers_cuda.so
     downloadCUDALibraries(libDir, gpuInfo.computeCapability);
   }

   if (gpuInfo.vendor === 'AMD') {
     // Download to libDir:
     // - ROCm libraries
     // - libonnxruntime_providers_rocm.so
     downloadROCmLibraries(libDir, gpuInfo.rocmVersion);
   }

   if (gpuInfo.vendor === 'Intel') {
     // Download to libDir:
     // - OpenVINO libraries
     // - libonnxruntime_providers_openvino.so
     downloadOpenVINOLibraries(libDir);
   }
   ```

5. **Recommend Model Size:**
   ```javascript
   if (vram >= 24) {
     console.log('Recommended: parakeet-tdt-1.1b (24GB VRAM)');
   } else if (vram >= 8) {
     console.log('Recommended: parakeet-tdt-0.6b (8GB VRAM)');
   } else {
     console.log('Recommended: CPU mode with smaller model');
   }
   ```

### macOS (`@agidreams/darwin-arm64`)

postinstall.js verifies:

1. **CoreML Support:**
   - ONNX Runtime 1.22.0 with CoreML already included
   - No additional libraries needed
   - Uses Neural Engine automatically

2. **System Check:**
   ```javascript
   const systemRAM = getSystemMemory();
   console.log(`Detected ${systemRAM}GB RAM - CoreML will use Neural Engine`);
   ```

## Directory Structure After Installation

### Linux Installation
```
/usr/local/lib/node_modules/
├── swictation/                          (main package)
│   ├── bin/swictation                   (CLI wrapper)
│   ├── src/resolve-binary.js            (finds platform package)
│   └── postinstall.js                   (GPU detection & library download)
└── @agidreams/
    └── linux-x64/                       (platform package)
        ├── bin/
        │   ├── swictation-daemon        (Rust binary)
        │   └── swictation-ui            (Rust binary)
        └── lib/
            ├── libonnxruntime.so        (base ONNX Runtime)
            ├── libcudart.so.12          (downloaded by postinstall)
            ├── libcublas.so.12          (downloaded by postinstall)
            ├── libcudnn.so.9            (downloaded by postinstall)
            └── libonnxruntime_providers_cuda.so (downloaded by postinstall)
```

### macOS Installation
```
/usr/local/lib/node_modules/
├── swictation/                          (main package)
│   ├── bin/swictation                   (CLI wrapper)
│   ├── src/resolve-binary.js            (finds platform package)
│   └── postinstall.js                   (verifies CoreML)
└── @agidreams/
    └── darwin-arm64/                    (platform package)
        ├── bin/
        │   ├── swictation-daemon        (Rust binary, Mach-O arm64)
        │   └── swictation-ui            (Rust binary, Mach-O arm64)
        └── lib/
            └── libonnxruntime.dylib     (with CoreML support)
```

## CLI Integration

The CLI wrapper (`bin/swictation`) uses the resolver:

```javascript
#!/usr/bin/env node
const { resolveBinaryPaths } = require('../src/resolve-binary.js');

try {
  const paths = resolveBinaryPaths();

  // Spawn daemon with correct paths
  const { spawn } = require('child_process');
  const daemon = spawn(paths.daemon, process.argv.slice(2), {
    env: {
      ...process.env,
      // Set library path for GPU libraries
      LD_LIBRARY_PATH: `${paths.libDir}:${process.env.LD_LIBRARY_PATH || ''}`,
      DYLD_LIBRARY_PATH: `${paths.libDir}:${process.env.DYLD_LIBRARY_PATH || ''}`
    },
    stdio: 'inherit'
  });

  daemon.on('exit', code => process.exit(code));
} catch (err) {
  console.error('Error:', err.message);
  process.exit(1);
}
```

## Error Handling

### Platform Not Supported
```
Error: Unsupported platform: win32-x64. Swictation supports linux-x64 and darwin-arm64 only.
```

### Platform Package Not Found
```
Error: Platform package @agidreams/linux-x64 not found.
This usually means:
  1. The package failed to install (check npm install output)
  2. You're on an unsupported platform (linux-x64)
  3. npm's optionalDependencies installation failed

Try reinstalling: npm install -g swictation --force
```

### Binaries Missing
```
Error: Platform package @agidreams/linux-x64 is installed but binaries are missing:
  Missing: swictation-daemon, swictation-ui
  Expected in: /usr/local/lib/node_modules/@agidreams/linux-x64/bin

This means the platform package was not built correctly.
Try reinstalling: npm install -g swictation --force
```

## GPU Detection Preserved

**All existing GPU detection logic is preserved:**

✅ NVIDIA GPU detection (CUDA 11.8 vs 12.9)
✅ AMD GPU detection (ROCm support)
✅ Intel GPU detection (OpenVINO support)
✅ VRAM detection and model recommendations
✅ System RAM checks
✅ Automatic library downloads
✅ Smart model size recommendations

The only difference: GPU libraries download to the platform package's `lib/` directory instead of the main package's `lib/native/` directory.

## Benefits

1. **Cleaner Architecture:** Binaries and libraries grouped by platform
2. **Smaller Downloads:** Users only download binaries for their platform
3. **Better CI/CD:** Each platform package built independently
4. **Preserved Logic:** All GPU detection and smart download logic remains
5. **Same User Experience:** `npm install -g swictation` still "just works"
