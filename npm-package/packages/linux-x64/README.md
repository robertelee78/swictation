# @swictation/linux-x64

Linux x64 binaries for [Swictation](https://github.com/robertelee78/swictation) - cross-platform voice-to-text dictation with GPU acceleration.

## What is this package?

This package contains pre-compiled Linux x64 binaries for Swictation. **You should not install this package directly.** It is automatically installed as an optional dependency when you install the main `swictation` package on a Linux x64 system.

## Installation

Install the main package instead:

```bash
npm install -g swictation
```

The main package will automatically install `@swictation/linux-x64` on Linux x64 systems via npm's `optionalDependencies` mechanism.

## Package Contents

This package includes:

- `bin/swictation-daemon` - Main voice recognition daemon (Rust binary)
- `bin/swictation-ui` - Tauri-based UI application (Rust binary)
- `lib/libonnxruntime.so` - ONNX Runtime library for ML inference
- `lib/libonnxruntime_providers_shared.so` - ONNX Runtime GPU providers

## System Requirements

- **OS**: Linux (GNU/Linux 3.2.0+)
- **Architecture**: x86_64 (64-bit)
- **GLIBC**: 2.27 or later
- **GPU** (optional):
  - NVIDIA GPU with CUDA 11.8+ or 12.9+ support
  - AMD GPU with ROCm support
  - Intel GPU with OpenVINO support

## GPU Acceleration

Swictation automatically detects your GPU and downloads the appropriate acceleration libraries during installation:

- **NVIDIA**: CUDA and cuDNN libraries
- **AMD**: ROCm libraries
- **Intel**: OpenVINO libraries
- **CPU-only**: Falls back to CPU inference (slower but works everywhere)

## Version Information

- **Distribution Version**: 0.7.9
- **Daemon Version**: 0.7.5
- **UI Version**: 0.1.0
- **ONNX Runtime**: 1.23.2 (GPU), 1.22.0 (CPU)

## Architecture

This package is part of Swictation's platform-specific distribution strategy:

```
swictation (main package)
├── @swictation/linux-x64 (Linux binaries) - THIS PACKAGE
└── @swictation/darwin-arm64 (macOS binaries)
```

When you install `swictation`, npm automatically installs the correct platform package for your system.

## License

Apache-2.0 - See [LICENSE](https://github.com/robertelee78/swictation/blob/main/LICENSE) for details.

## Links

- [Main Repository](https://github.com/robertelee78/swictation)
- [Documentation](https://github.com/robertelee78/swictation/blob/main/README.md)
- [Issue Tracker](https://github.com/robertelee78/swictation/issues)
- [NPM Package](https://www.npmjs.com/package/swictation)
