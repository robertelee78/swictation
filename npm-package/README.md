# Swictation

Voice-to-text dictation for Linux and macOS with GPU acceleration. Pure Rust daemon with VAD-triggered auto-transcription, sub-second latency, and complete privacy.

Supported platforms:

- Linux x64 (Ubuntu 24.04+, GLIBC 2.39+)
- macOS Apple Silicon (M1/M2/M3/M4/M5), macOS 14 Sonoma+

## Install

```bash
npm install -g swictation --foreground-scripts
```

The `--foreground-scripts` flag shows installation progress. Postinstall automatically:

1. Detects your GPU and downloads optimized acceleration libraries
2. Downloads and test-loads AI models (~30-60s on first install)
3. Sets up the background service (systemd on Linux, launchd on macOS)
4. Shows platform-specific setup instructions

## Platform Requirements

**Linux x64**
- Ubuntu 24.04+ (GLIBC 2.39+), Node.js 18+
- Optional: NVIDIA GPU with 4GB+ VRAM for CUDA acceleration (CPU fallback available)

**macOS Apple Silicon (M1-M5)**
- macOS 14 Sonoma or later, Node.js 18+
- 8GB+ RAM (16GB+ recommended for the 1.1B model)
- Accessibility permissions granted during setup

## Commands

```bash
swictation start      # Start the daemon
swictation stop       # Stop the daemon
swictation status     # Show service status
swictation toggle     # Toggle recording on/off
swictation --version  # Show version
```

## Documentation

Full documentation, configuration reference, and troubleshooting guides:
https://github.com/robertelee78/swictation

## Links

- Source: https://github.com/robertelee78/swictation
- Issues: https://github.com/robertelee78/swictation/issues

## License

Apache-2.0