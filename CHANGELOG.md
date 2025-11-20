# Changelog

All notable changes to Swictation will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.27] - 2025-11-20

### Fixed
- **CLI --version Command** - Completed implementation of --version/-V flags
  - Added --version, -V, and version commands to CLI wrapper
  - Shows npm package version, daemon details (if available), platform, and Node.js version
  - Works even before binaries are installed (doesn't require checkBinaries())
  - Updated help text to include --version command
  - File: npm-package/bin/swictation

### Note
- This completes task cbde1aa4-6605-47fa-b706-2609926ddfcb which was partially implemented in v0.4.24 (daemon only)

## [0.4.26] - 2025-11-20

### Fixed
- **Tauri UI Launch from Tray Icon** - Fixed middle-click and "Show Metrics" menu item not working
  - Tray app now correctly locates Tauri UI binary in npm package installations
  - Added smart path detection with development fallback
  - Resolves "app not installed" error on Sway/Wayland when launching UI from tray
  - Both `src/ui/swictation_tray.py` and `npm-package/src/ui/swictation_tray.py` updated

## [0.4.25] - 2025-11-19

### Fixed
- **ONNX Runtime Version Compatibility** - Fixed daemon crash caused by version mismatch
  - Upgraded bundled ONNX Runtime from 1.22.0 to 1.23.2 for compatibility with CUDA providers
  - All GPU library packages (legacy/modern/latest) now use ONNX Runtime 1.23.2
  - Resolves segmentation fault on Blackwell GPUs (sm_120) and other architectures
  - Daemon now starts successfully with proper CUDA acceleration

### Changed
- **GPU Library Packages** - All variants updated to ONNX Runtime 1.23.2
  - Legacy (sm_50-70): CUDA 11.8 + cuDNN 8.9.7 + ONNX Runtime 1.23.2
  - Modern (sm_75-86): CUDA 12.9 + cuDNN 9.15.1 + ONNX Runtime 1.23.2
  - Latest (sm_89-120): CUDA 12.9 + cuDNN 9.15.1 + ONNX Runtime 1.23.2

### Technical Details
- Base ONNX Runtime library updated from 22MB (v1.22.0) to 27MB (v1.23.2)
- CUDA execution provider compatibility ensured across all GPU variants
- Blackwell architecture (sm_120) now fully supported with native CUDA 12.9

## [0.4.24] - 2025-11-19

### Added
- **Maxwell GPU Support (CUDA 11.8)** - Full support for older Maxwell architecture GPUs
  - Custom ONNX Runtime 1.23.2 built with CUDA 11.8 + cuDNN 8.9.7
  - Supports compute capability 5.0-5.3 (GTX 750 Ti, GTX 900 series, Quadro M2200, etc.)
  - Legacy GPU library package (cuda-libs-legacy.tar.gz) with sm_50-70 support
  - Automatic detection and library selection for Maxwell GPUs
  - Verified working on Quadro M2200 (4GB VRAM, sm_52) with 0.6b-gpu model
  - See `docs/maxwell_gpu_solution_plan.md` for technical details

- **Automatic AI Model Download** - Models now download automatically during npm install
  - Intelligent model detection: checks if recommended model already exists
  - Auto-downloads ONLY the recommended model based on GPU/VRAM detection
  - No more "Next steps" wall of text when model already present
  - Graceful fallback to manual instructions if auto-download fails
  - Uses existing ModelDownloader for consistent behavior
  - First install: Downloads VAD + recommended model (~3-6GB one-time)
  - Subsequent installs: Detects existing model, skips download

### Changed
- **Improved Daemon Error Messages** - Missing models now show helpful instructions
  - Detects model loading failures on daemon startup
  - Shows clear error message with appropriate download commands
  - Prevents cryptic "file not found" errors
  - Suggests correct model for user's GPU configuration

### Fixed
- **Maxwell GPU Inference** - Confirmed working with CUDA 11.8 + cuDNN 8.9.7
  - 0.6b-gpu model runs efficiently on 4GB VRAM Maxwell GPUs
  - Audio transcription accuracy verified with microphone settings
  - VRAM usage stable at 85-87% during inference
  - No compatibility issues with modern ONNX Runtime features

### Technical Details
- ONNX Runtime 1.23.2 built with:
  - CUDA 11.8.0 (last version supporting sm_50)
  - cuDNN 8.9.7.29 for CUDA 11.x
  - Compute capabilities: sm_50, sm_52, sm_53, sm_60, sm_61, sm_70
- GPU library selection logic:
  - Maxwell (sm_50-70) → cuda-libs-legacy.tar.gz (CUDA 11.8)
  - Turing/Ampere (sm_75-86) → cuda-libs-modern.tar.gz (CUDA 12.6)
  - Ada/Hopper/Blackwell (sm_89-120) → cuda-libs-latest.tar.gz (CUDA 12.6)
- Model download integration:
  - `isModelDownloaded()` - Verifies encoder.onnx, decoder.onnx, tokens.txt
  - `autoDownloadModel()` - Downloads via ModelDownloader if not present
  - Daemon startup check - Shows helpful error if models missing

### Documentation
- Added `docs/maxwell_gpu_investigation_2025-11-18.md` - Initial investigation findings
- Added `docs/maxwell_gpu_solution_plan.md` - Implementation plan and decisions
- Added `docs/testing_stt_inference_standalone.md` - Debugging guide for STT issues
- Updated `docs/phase1_implementation_plan.md` - Maxwell GPU implementation phases

## [0.4.9] - 2025-11-17

### Fixed
- **Sway Desktop Detection** - Fixed detection to use `SWAYSOCK` as primary indicator (XDG_CURRENT_DESKTOP often not set)
- **Stale Binary Issue** - Updated daemon binary that was accidentally stuck at commit d332855 in v0.4.8

### Added
- **Pre-release Build Script** - Automatic `prepublishOnly` hook ensures binaries are always fresh before npm publish
- **Sway Config Cleanup** - Daemon now removes old Swictation config blocks before adding new ones

### Technical
- Build script verifies binary integrity with SHA256 checksums
- Prevents duplicate/conflicting hotkey bindings from accumulating

## [0.4.8] - 2025-11-17

### Fixed
- **Sway Tray Icon State Sync** - Fixed tray icon not changing colors when toggling between idle/recording by updating socket path to use `$XDG_RUNTIME_DIR/swictation.sock` instead of hardcoded `/tmp/swictation.sock`

### Changed
- **Simplified Sway Config Generation** - Hotkey config now uses `swictation toggle` command instead of direct socket communication
- **Auto-Configure GNOME Wayland Hotkeys** - Added automatic gsettings configuration for GNOME Wayland hotkeys
- **Auto-Install System Dependencies** - Improved postinstall script to automatically detect and install missing system dependencies based on environment

## [0.4.7] - 2025-11-16

### Added
- **MidStream SSH Integration** - Updated midstream submodule to use SSH URL for better development workflows

### Fixed
- **Number Conversion with Year Patterns** - Implemented comprehensive number-to-digit conversion with support for year patterns (e.g., "twenty twenty-four" → "2024")
- **NVIDIA Hibernation Detection** - Fixed GPU hibernation check to properly read `/proc/driver/nvidia/params` instead of unreliable sysfs nodes

### Changed
- **Bounded Channels with Backpressure (Issue #1 & #2)**
  - Replaced unbounded channels with bounded capacity limits:
    - Transcription results: capacity 100 (prevents unbounded memory growth)
    - Audio chunks: capacity 20 (10 seconds buffer at 0.5s/chunk)
  - Implemented smart backpressure mechanism:
    - Audio callback uses `try_send()` for non-blocking drops (preserves real-time constraints)
    - VAD/STT processing uses `send().await` for backpressure propagation
    - Atomic counter tracks dropped chunks with 5-second warnings
  - Fixed thread-safety issues: scoped all mutex locks to ensure they're dropped before async operations
  - See `docs/BOUNDED_CHANNELS.md` for detailed implementation

- **Parallel VAD/STT Processing (Issue #4)**
  - Split sequential processing into two independent async tasks:
    - VAD Task: Continuously processes audio chunks and detects speech
    - STT Task: Processes speech segments in parallel with VAD
  - Benefits:
    - ~2x better CPU/GPU utilization (VAD uses CPU while STT uses GPU)
    - Lower perceived latency through pipelining
    - Consistent throughput with reduced variance
  - Created VAD→STT channel (capacity: 10 segments) for inter-task communication
  - VAD latency no longer tracked in metrics (runs asynchronously)
  - See `docs/PARALLEL_PROCESSING.md` for architecture details

### Technical Details
- Modified `pipeline.rs` to use bounded channels with backpressure
- Audio callback drops chunks when processing falls behind (prevents blocking)
- Transcription send blocks when consumer is slow (natural backpressure)
- All mutex guards are now properly scoped to avoid holding across `.await` points
- VAD and STT run as independent tokio tasks with channel-based communication
- Build status: ✅ Successful compilation with warnings only

## [0.3.21] - 2025-11-14

### Fixed
- **GPU libs package download URL** - Fixed GPU_LIBS_VERSION constant to correctly download gpu-libs-v1.1.1
  - v0.3.20 had GPU_LIBS_VERSION set to '1.1.0' instead of '1.1.1' in postinstall.js line 583
  - This caused postinstall to download the old package without the CUDA provider library
  - Daemon would crash with "cannot open shared object file: libonnxruntime_providers_cuda.so"
  - Now correctly downloads gpu-libs-v1.1.1 with all 15 libraries including CUDA provider

### Technical Details
- Updated GPU_LIBS_VERSION constant from '1.1.0' to '1.1.1'
- Verified gpu-libs-v1.1.1 release assets exist with correct naming (cuda-libs-modern.tar.gz)
- Tested on RTX A1000 (sm_86) - daemon starts successfully, no crashes
- All 15 libraries extracted including 330MB libonnxruntime_providers_cuda.so

### Migration Notes
This is a critical hotfix for v0.3.20. If you installed v0.3.20 and experienced daemon crashes with "libonnxruntime_providers_cuda.so: cannot open shared object file", upgrade to v0.3.21 to resolve the issue.

## [0.3.20] - 2025-11-14

### Fixed
- **CRITICAL: Missing ONNX Runtime CUDA provider library**
  - Added libonnxruntime_providers_cuda.so (345MB) to GPU libs packages v1.1.1
  - This library was missing from v1.1.0, causing daemon to crash with "cannot open shared object file" error
  - Now downloaded as part of GPU libs packages (not bundled in npm)
  - Fixes daemon crash on GPU systems

### Technical Details
- Created gpu-libs-v1.1.1 with 15 libraries total:
  - 14 CUDA runtime libraries (libcublas, libcudnn, etc.) - from v1.1.0
  - 1 ONNX Runtime CUDA provider (libonnxruntime_providers_cuda.so) - NEW
- Package size: ~1.7GB compressed per architecture (LEGACY, MODERN, LATEST)
- Removed CUDA provider from npm package (too large, 345MB)
- Updated postinstall to download from gpu-libs-v1.1.1

## [0.3.19] - 2025-11-14

### Fixed
- **Automatic systemd daemon-reload after service file updates**
  - postinstall now runs `systemctl --user daemon-reload` automatically after generating service files
  - Prevents systemd from using cached service files with old paths
  - Eliminates "EXEC status 203" errors when service paths change
  - Users no longer need to manually reload systemd after installation

### Technical Details
- Added daemon-reload step at end of Phase 4 (Service Installation)
- Executes after both daemon and UI service files are written
- Gracefully handles errors if systemd is not available
- Ensures services always use current installation paths

## [0.3.18] - 2025-11-14

### Fixed
- **CRITICAL: Missing native libraries in npm package** - Bundle libonnxruntime.so and related libraries
  - v0.3.15-v0.3.17 accidentally excluded native .so libraries from npm package
  - Restored libonnxruntime.so (22MB), libsherpa-onnx-c-api.so (3.8MB), and provider libs
  - Package size increases from 7.8MB to ~17MB (same as v0.3.8)
  - This was the root cause of "All GPU models failed to load" errors

### Technical Details
- Added back to package.json files list:
  - lib/native/libonnxruntime.so
  - lib/native/libonnxruntime_providers_shared.so
  - lib/native/libsherpa-onnx-c-api.so
  - lib/native/libsherpa-onnx-cxx-api.so
- v0.3.8 had these libraries, but they were accidentally removed in v0.3.15+
- Without these libraries, detectOrtLibrary() could never find them (neither in GPU libs nor npm package)

## [0.3.17] - 2025-11-14

### Fixed
- **ONNX Runtime detection order** - Check GPU libs directory first (reverted in v0.3.18)
  - detectOrtLibrary() now checks GPU libs directory FIRST (~/.local/share/swictation/gpu-libs/)
  - Previous version looked in npm package directory first, but ONNX Runtime is downloaded separately as part of multi-arch GPU packages
  - Fixes "GPU-enabled ONNX Runtime not found" error during installation
  - Fixes model test-loading failures (0.6b-gpu, 1.1b-gpu)

- **Expanded old installation cleanup paths**
  - Added /usr/lib/node_modules/swictation to cleanup list
  - Ensures all possible system-wide installations are removed during upgrade

### Technical Details
- ONNX Runtime library path detection priority:
  1. GPU libs directory: ~/.local/share/swictation/gpu-libs/libonnxruntime.so (PRIMARY)
  2. npm package directory: lib/native/libonnxruntime.so (FALLBACK)
  3. System Python installations: ~/.local/lib/python*/site-packages/onnxruntime/capi/ (LEGACY)
- This fix is critical - v0.3.16 could not load models due to wrong detection order

### Migration Notes
This is a hotfix release. If v0.3.16 failed to load models during installation, upgrading to v0.3.17 will resolve the issue.

## [0.3.16] - 2025-11-14

### Added
- **Automatic service shutdown before npm install/upgrade** - Prevents CUDA state corruption (error 999)
  - Stops existing swictation-daemon and swictation-ui services before any file modifications
  - 2-second grace period for services to release CUDA driver cleanly
  - Prevents "All GPU models failed to load" errors on upgrades

### Fixed
- **Upgrade installation failures on systems with existing installations**
  - Automatically removes old ONNX Runtime 1.20.x from Python site-packages (version conflicts with bundled 1.22.x)
  - Cleans up old system-wide npm installations that conflict with nvm installations
  - Auto-detects actual npm installation path (nvm vs system-wide) for service file generation
  - Fixed LD_LIBRARY_PATH in systemd service files to match actual npm installation location
  - Fixed ORT_DYLIB_PATH to use bundled library instead of old Python installations

- **4GB VRAM GPU model selection** - Now correctly recommends 0.6b-gpu instead of cpu-only
  - RTX A1000 (4GB VRAM) now gets 0.6b-gpu recommendation (verified working)
  - Adjusted VRAM threshold from 4GB to 3.5GB for GPU models
  - Based on real-world testing: 0.6b-gpu uses ~3.5GB VRAM (safe on 4GB GPUs)

### Changed
- **Postinstall script reorganized into phases**
  - Phase 0: Stop running services (new)
  - Phase 1: Clean up old service files
  - Phase 1.5: Clean up old installations (new)
  - Phase 2: Configuration migration
  - Phase 3: GPU detection and library download
  - Phase 3.5: Model verification
  - Phase 4: Service installation

### Technical Details
- Service shutdown uses both CLI (`swictation stop`) and systemctl fallback
- ONNX Runtime cleanup targets Python 3.10-3.13 site-packages directories
- npm installation detection handles both `/usr/local/lib/node_modules` and nvm paths
- Dynamic path detection ensures service files always point to actual installation

### Migration Notes
When upgrading from v0.3.15 or earlier:
- Services will be automatically stopped before upgrade (prevents CUDA corruption)
- Old conflicting installations will be cleaned up automatically
- No manual intervention required - upgrades now work smoothly

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
- **MidStream text-transform** integration (pure Rust, ~5µs latency)
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
