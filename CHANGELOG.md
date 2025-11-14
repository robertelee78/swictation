# Changelog

All notable changes to Swictation will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.15] - 2025-11-14

### Added
- **Multi-architecture GPU support with optimized library packages**
  - Three architecture-specific packages reduce download size by 65-74%
  - **LEGACY** (sm_50-70): Maxwell, Pascal, Volta GPUs (GTX 750-Titan V, Quadro M/P series)
  - **MODERN** (sm_75-86): Turing, Ampere GPUs (GTX 16, RTX 20/30, A100, RTX A1000-A6000)
  - **LATEST** (sm_89-120): Ada, Hopper, Blackwell GPUs (RTX 4090, H100, B100/B200, RTX PRO 6000 Blackwell, RTX 50 series)
  - Automatic GPU compute capability detection via nvidia-smi
  - Downloads only the libraries needed for your specific GPU architecture
  - Package size: ~1.5GB compressed vs 500-700MB for universal binary
- **Native Blackwell (sm_120) support** - Built with CUDA 12.9 for RTX PRO 6000 and RTX 50 series
- **Restored sm_50 support** - Custom-built ONNX Runtime supporting Maxwell GPUs (GTX 750/900, Quadro M series)
- **Automatic package variant selection** - Zero configuration required, works out of the box
- **Package metadata tracking** - Saves installed variant info to `~/.config/swictation/gpu-package-info.json`

### Changed
- **GPU library download system completely redesigned**
  - Now downloads from GitHub release `gpu-libs-v1.1.0` with three separate packages
  - Libraries installed to `~/.local/share/swictation/gpu-libs/` (user-specific, no sudo required)
  - CUDA runtime libraries included in packages (libcublas, libcudnn, libcudart, etc.)
  - Downloads happen during npm postinstall automatically
- **Prioritized library search paths**
  - User's GPU libs directory checked first (`~/.local/share/swictation/gpu-libs`)
  - System CUDA installations used as fallback
  - LD_LIBRARY_PATH in systemd service updated accordingly

### Fixed
- **GPU library loading failures on older GPUs** - sm_50 (Maxwell) support restored
  - Fixes "All GPU models failed to load" on Quadro M2200 and similar cards
  - Custom ONNX Runtime build with CMAKE_CUDA_ARCHITECTURES includes sm_50-52
- **GPU library loading failures on newest GPUs** - Native sm_120 (Blackwell) support
  - No PTX forward compatibility hacks - native compilation for RTX PRO 6000 and RTX 50 series
  - CUDA 12.9 supports full range: sm_50 through sm_121
- **Download size optimization** - Users no longer download libraries for GPU architectures they don't have
  - 65-74% smaller downloads compared to universal binary approach
  - Faster installation, especially on slower connections

### Technical Details
- Built with Docker for reproducible environment (CUDA 12.9, cuDNN 9.x, CMake 3.28+)
- ONNX Runtime v1.23.2 built from source with custom architecture flags
- Parallel builds on 32-thread Threadripper (3 variants in ~51 minutes)
- Architecture verification with cuobjdump confirms all compute capabilities present
- Comprehensive documentation: `/opt/swictation/docs/GPU_LIBRARY_PACKAGES.md`

### GPU Architecture Support
| Package | Compute Caps | Example GPUs | Download Size |
|---------|-------------|--------------|---------------|
| LEGACY | sm_50-70 | Maxwell, Pascal, Volta | ~1.5GB |
| MODERN | sm_75-86 | Turing, Ampere | ~1.5GB |
| LATEST | sm_89-120 | Ada, Hopper, Blackwell | ~1.5GB |

### Migration Notes
When upgrading to v0.3.15:
- Old universal GPU libraries will be replaced with architecture-specific packages
- First install will download ~1.5GB package appropriate for your GPU
- GPU detection runs automatically during `npm install`
- No configuration changes required - works out of the box
- If upgrading from v0.3.1-v0.3.14, old libraries will be cleaned up automatically

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
