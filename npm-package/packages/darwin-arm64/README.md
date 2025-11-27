# @agidreams/darwin-arm64

macOS ARM64 (Apple Silicon) binaries for [Swictation](https://github.com/robertelee78/swictation) - cross-platform voice-to-text dictation with CoreML acceleration.

## What is this package?

This package contains pre-compiled macOS ARM64 binaries for Swictation. **You should not install this package directly.** It is automatically installed as an optional dependency when you install the main `swictation` package on a macOS ARM64 (Apple Silicon) system.

## Installation

Install the main package instead:

```bash
npm install -g swictation
```

The main package will automatically install `@agidreams/darwin-arm64` on macOS ARM64 systems via npm's `optionalDependencies` mechanism.

## Package Contents

This package includes:

- `bin/swictation-daemon` - Main voice recognition daemon (Rust binary, Mach-O arm64)
- `bin/swictation-ui` - Tauri-based UI application (Rust binary, Mach-O arm64)
- `lib/libonnxruntime.dylib` - ONNX Runtime library for ML inference with CoreML support

## System Requirements

- **OS**: macOS 11.0 (Big Sur) or later
- **Architecture**: ARM64 (Apple Silicon - M1, M2, M3, M4)
- **Processor**: Apple Silicon required (not compatible with Intel Macs)

## CoreML Acceleration

Swictation automatically uses CoreML on Apple Silicon Macs for hardware-accelerated ML inference:

- **Neural Engine**: Utilizes Apple's Neural Engine for optimal performance
- **GPU Acceleration**: Falls back to Metal GPU acceleration when Neural Engine unavailable
- **CPU Fallback**: CPU inference as last resort (slower but always available)

CoreML provides excellent performance on Apple Silicon without requiring any manual configuration.

## Version Information

- **Distribution Version**: 0.7.9
- **Daemon Version**: 0.7.5
- **UI Version**: 0.1.0
- **ONNX Runtime**: 1.22.0 (with CoreML support)

## Architecture

This package is part of Swictation's platform-specific distribution strategy:

```
swictation (main package)
├── @agidreams/linux-x64 (Linux binaries)
└── @agidreams/darwin-arm64 (macOS binaries) - THIS PACKAGE
```

When you install `swictation`, npm automatically installs the correct platform package for your system.

## Intel Mac Support

**Note:** This package is for Apple Silicon Macs only. Intel Macs are not currently supported. If you have an Intel Mac, please check the main repository for compatibility information.

## License

Apache-2.0 - See [LICENSE](https://github.com/robertelee78/swictation/blob/main/LICENSE) for details.

## Links

- [Main Repository](https://github.com/robertelee78/swictation)
- [Documentation](https://github.com/robertelee78/swictation/blob/main/README.md)
- [Issue Tracker](https://github.com/robertelee78/swictation/issues)
- [NPM Package](https://www.npmjs.com/package/swictation)
