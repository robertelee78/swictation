# Changelog

All notable changes to Swictation will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-11-13

### Added
- **Real model test-loading during installation** - Installation now attempts to test-load the recommended AI model (30-60 seconds)
  - Validates that the model can be loaded successfully before service starts
  - Catches VRAM allocation errors at install time instead of runtime
  - Can be skipped with `SKIP_MODEL_TEST=1` for CI/headless environments
  - Provides immediate feedback if model is too large for available VRAM
- **Interactive config migration with pacman/apt-style prompts** - Handles upgrades with conflicting config files
  - Offers options: [K]eep, [N]ew, [M]erge, [D]iff, [S]kip
  - Defaults to "Keep" in non-interactive mode (CI/automation-friendly)
  - Creates backups of old configs before replacement
- **Old service cleanup on upgrades** - Automatically removes conflicting service files from previous installations
  - Cleans up old Python-based services before installing new Node.js services
  - Prevents "already registered" errors during systemd installation
  - Stops and disables old services gracefully

### Changed
- **Intelligent VRAM-based model selection with verification**
  - Now attempts to verify model can be loaded, not just VRAM size check
  - More conservative thresholds based on empirical testing:
    - 1.1B model: Requires 6GB+ VRAM (was 4GB+)
    - 0.6B model: Requires 4GB+ VRAM (minimum)
    - CPU-only: Falls back if <4GB VRAM
- **Fixed model memory thresholds** - Based on real-world testing on RTX A1000 (4GB VRAM)
  - 0.6B model: ~3.5GB VRAM usage (validated safe on 4GB)
  - 1.1B model: ~6GB VRAM usage (needs 8GB+ for safety)
  - Previous thresholds were too optimistic and caused allocation failures

### Fixed
- **Model selection failure on limited VRAM GPUs** - RTX A1000 (4GB) now correctly selects 0.6B model
- **Service installation conflicts during upgrades** - Old services are now cleaned up before new services are installed
- **Config file conflicts on updates** - Users can now choose how to handle config differences

### Technical Details
- Test-loading uses 30-second timeout to prevent hanging
- GPU detection enhanced with VRAM verification
- Graceful fallback if test-loading fails (doesn't block installation)
- Environment variable `SKIP_MODEL_TEST=1` disables test-loading for automation

### Migration Notes
When upgrading from v0.3.0:
1. Old service files will be automatically cleaned up
2. You'll be prompted about config changes (or default to "Keep" in CI)
3. Installation will test-load your recommended model (~30-60s)
4. If model fails to load, service will still be installed but may need manual config

---

## [0.3.0] - 2025-11-13

### Added
- **Secretary Mode v0.3.0** - Complete natural dictation with 60+ transformation rules
  - Punctuation commands (comma, period, question mark, exclamation point)
  - Smart quotes with stateful toggle
  - Number conversion ("number forty two" → 42)
  - Abbreviations (mister → Mr., doctor → Dr.)
  - Formatting (new line, new paragraph, tab)
  - Capitalization modes (caps on/off, all caps, capital letter)
  - Automatic capitalization (I pronoun, sentence starts, after quotes)
- **27/27 tests passing** with real voice samples
- **MidStream text-transform** integration (pure Rust, ~1µs latency)
- **GPU acceleration support** with CUDA 11.8+
  - Silero VAD v6 with ONNX Runtime
  - Parakeet-TDT models (0.6B and 1.1B)
  - Adaptive model selection based on GPU VRAM

### Fixed
- GPU CUDA environment fixes for systemd services
- cuDNN support in ONNX Runtime with cuda-12.9 lib path
- Service file conflicts between old and new installations

---

## [0.2.x] - 2025-11-12

### Added
- Pure Rust implementation (zero Python runtime)
- VAD-triggered segmentation with auto-transcription
- Wayland native (wtype text injection)
- systemd integration with auto-start
- Hotkey control ($mod+Shift+d toggle)

### Technical
- ort 2.0.0-rc.10 with modern CUDA support
- Silero VAD v6 (August 2024, 16% less errors)
- Parakeet-TDT-1.1B (5.77% WER)
- Sub-second latency with GPU optimization

---

## Version History

- **0.3.1** - Real model test-loading, service cleanup, config migration
- **0.3.0** - Secretary Mode with 60+ rules, GPU acceleration
- **0.2.x** - Pure Rust implementation, VAD-triggered segmentation
- **0.1.x** - Initial Python-based implementation

---

## Links

- [Repository](https://github.com/robertelee78/swictation)
- [Issues](https://github.com/robertelee78/swictation/issues)
- [Secretary Mode Documentation](docs/secretary-mode.md)
