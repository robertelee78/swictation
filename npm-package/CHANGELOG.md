# Changelog

## [0.7.25] - 2025-11-29

### Fixed - macOS Permission Handling & Path Resolution
This release fixes the fundamental issues causing repeated permission prompts and library loading failures.

- **Microphone Permission at Startup** - Daemon now requests microphone permission immediately
  - `macos_audio_permission` module now enabled and called at daemon startup
  - Shows system permission dialog for microphone access BEFORE audio capture
  - Clear user guidance if permission denied or restricted
  - Single permission prompt approach - request once, works forever

- **Runtime Path Resolution** - Wrapper script no longer hardcodes paths
  - Previously: JavaScript template literals caused paths to be hardcoded at install time
  - Now: Paths resolved at runtime using `SCRIPT_DIR` and `PACKAGE_DIR`
  - Gracefully handles npm reinstalls and path changes
  - Falls back to install-time paths only as last resort

- **Entitlements for Signed Binaries** - Proper entitlements for hardened runtime
  - Added `daemon.entitlements` with `com.apple.security.device.audio-input`
  - Added entitlements for library loading and JIT (needed by ONNX Runtime/CoreML)
  - CI now signs binaries WITH entitlements file
  - Enables microphone permission dialog for Developer ID signed apps

- **UI Entitlements** - Added entitlements for Tauri UI
  - Proper entitlements for WebView and library loading
  - Consistent signing with hardened runtime

### Installation
```bash
npm install -g swictation@0.7.25
```

### Platforms
- macOS ARM64 (Apple Silicon) - `@agidreams/darwin-arm64` (Full Developer ID signing + entitlements)
- Linux x86_64 (NVIDIA CUDA/CPU) - `@agidreams/linux-x64`

---

## [0.7.24] - 2025-11-29

### Fixed - macOS Hardened Runtime Library Loading
This release fixes the ONNX Runtime library loading issue with hardened runtime.

- **Developer ID Signed ONNX Runtime Library** - Library now has matching Team ID
  - `libonnxruntime.dylib` signed with same Developer ID as daemon
  - Fixes "different Team IDs" error when loading library with hardened runtime
  - All dylibs in platform package are now signed in CI

- **Library Resolution Priority** - postinstall prefers signed platform library
  - First checks `@agidreams/darwin-arm64/lib/` for pre-signed library
  - Falls back to download only if platform library not found
  - Ensures hardened runtime compatibility out of the box

### Installation
```bash
npm install -g swictation@0.7.24
```

### Platforms
- macOS ARM64 (Apple Silicon) - `@agidreams/darwin-arm64` (Full Developer ID signing)
- Linux x86_64 (NVIDIA CUDA/CPU) - `@agidreams/linux-x64`

---

## [0.7.23] - 2025-11-29

### Added - macOS Developer ID Code Signing
This release adds proper code signing for macOS binaries, solving the repeated permission prompt issue.

- **Developer ID Application Signing** - All macOS binaries are now signed with Apple Developer ID
  - Daemon and UI signed with "Developer ID Application: ROBERT E LEE"
  - Hardened runtime enabled for Gatekeeper compatibility
  - Stable identity in macOS TCC (Transparency, Consent, and Control) database

### Fixed - macOS Permission Prompts
- **One-time permission grant** - Accessibility permission now persists across updates
  - No more repeated "swictation-daemon would like to control this computer" prompts
  - Single permission grant covers all future versions
  - Clean permission dialogs showing developer identity

### Infrastructure
- **CI/CD Code Signing** - Automated signing in GitHub Actions
  - Certificate imported into temporary keychain during build
  - Both daemon and UI signed automatically
  - Secrets-based configuration for security

### Installation
```bash
npm install -g swictation@0.7.23
```

### Platforms
- macOS ARM64 (Apple Silicon) - `@agidreams/darwin-arm64` (Developer ID signed)
- Linux x86_64 (NVIDIA CUDA/CPU) - `@agidreams/linux-x64`

---

## [0.7.22] - 2025-11-28

### Fixed
- **IPC Response Handling** - Fixed toggle hang on macOS by spawning response writes
- **Cross-platform Paths** - Added `swictation-paths` crate for unified path handling

---

## [0.7.21] - 2025-11-27

### Fixed - macOS Support
This release focuses on completing macOS Apple Silicon support with critical bug fixes.

- **UI Menu Bar Behavior** - Fixed macOS UI to behave as a proper menu bar app
  - Set `ActivationPolicy::Accessory` to hide dock icon entirely
  - Window starts hidden by default (`visible: false` in tauri.conf.json)
  - Tray icon appears immediately on launch
  - Middle-click tray toggles window visibility
  - Close button hides window (tray remains active)

- **Daemon Deadlock** - Resolved lock ordering deadlock causing toggle hang on macOS
  - Fixed mutex acquisition order to prevent deadlock during recording toggle
  - Daemon now responds reliably to toggle commands via socket and hotkey

- **CoreML Library Path** - Fixed ORT_DYLIB_PATH for macOS CoreML acceleration
  - Corrected library path from `.so` to `.dylib` extension
  - CoreML GPU acceleration now works properly on Apple Silicon

### Installation
```bash
npm install -g swictation@0.7.21
```

### Platforms
- macOS ARM64 (Apple Silicon) - `@agidreams/darwin-arm64`
- Linux x86_64 (NVIDIA CUDA/CPU) - `@agidreams/linux-x64`

---

## [0.7.20] - 2025-11-27

### Fixed
- **macOS ORT_DYLIB_PATH** - Corrected ONNX Runtime library path for CoreML support
- **PostInstall improvements** - Better platform and compositor detection

---

## [0.7.19] - 2025-11-27

### Fixed
- **PostInstall compositor detection** - System-level wlroots detection for Wayland compositors
- **Sway/wlroots support** - Improved environment variable handling for tray integration

---

## [0.4.7] - 2025-11-16

### Added
- **MidStream SSH Integration** - Updated midstream submodule to use SSH URL for better development workflows

### Fixed
- **Number Conversion with Year Patterns** - Comprehensive number-to-digit conversion with year pattern support
- **NVIDIA Hibernation Detection** - Proper GPU hibernation check using `/proc/driver/nvidia/params`

### Documentation
- Restructured README to eliminate redundancy
- Added comprehensive task implementation documentation

## [0.4.6] - 2025-11-16

### Fixed
- **NVIDIA hibernation detection**: Fixed `isNvidiaConfigured()` to check `/proc/driver/nvidia/params` first (preferred location, always available when driver loaded), then fall back to `/sys/module/nvidia/parameters/` (not always available). Version 0.4.5 incorrectly reported "NOT CONFIGURED" on systems where the parameter was set but only visible in `/proc/driver/nvidia/params`.

## [0.4.5] - 2025-11-16

### Added
- **NVIDIA hibernation support detection**: Automatic detection of laptops with NVIDIA GPUs
  - Detects laptop systems via battery presence in `/sys/class/power_supply/`
  - Checks for NVIDIA GPU using `nvidia-smi`
  - Verifies current hibernation configuration status
  - Phase 7 added to postinstall process for automatic detection
  - Warns users during installation if configuration needed

- **Interactive NVIDIA hibernation setup**: `swictation setup` now configures NVIDIA hibernation
  - Prompts user to configure NVIDIA power management
  - Creates `/etc/modprobe.d/nvidia-power-management.conf`
  - Sets `NVreg_PreserveVideoMemoryAllocations=1` kernel parameter
  - Updates initramfs automatically (Ubuntu/Debian/Fedora/Arch support)
  - Notifies user that reboot is required

- **New system detection utilities**: `npm-package/src/utils/system-detect.js`
  - `isLaptop()` - Battery-based laptop detection
  - `hasNvidiaGpu()` - NVIDIA GPU detection
  - `isNvidiaConfigured()` - Kernel parameter verification
  - `detectDistribution()` - Auto-detect Ubuntu/Debian/Fedora/Arch
  - `nvidiaModprobeConfigExists()` - Config file check

- **Comprehensive test suite**: `npm-package/tests/test-nvidia-hibernation.js`
  - Tests all detection functions
  - Validates logic consistency
  - Provides diagnostic information
  - 6/6 tests passing (100% success rate)

- **Full documentation**: `docs/nvidia-hibernation-support.md`
  - Problem description and root cause analysis
  - Manual configuration steps
  - Verification procedures
  - Distribution-specific notes
  - Comprehensive troubleshooting guide

### Fixed
- Prevents GPU defunct state after laptop hibernation (CUDA errors 719/999)
- Automatically preserves GPU memory allocations during suspend/hibernation cycles

### Changed
- Updated `package.json` to include `tests/` directory in published files

## [0.3.14] - 2025-11-13

### Fixed
- **User-local npm installations broken**: Fixed package directory structure for user-local npm installs
  - Previously: npm files array listed individual files without parent directories
  - Issue: `lib/native/` directory wasn't created, causing GPU libs extraction to fail
  - Now: Added `.gitkeep` placeholder to ensure `lib/native/` directory structure is preserved
  - Fixes installations with `npm config set prefix ~/.npm-global` (non-sudo installs)
  - Fixes nvm-based installations
  - Error was: `tar: /home/user/.npm-global/lib/node_modules/swictation/lib/native: Cannot open: No such file or directory`

### Changed
- Updated package.json files array to include `lib/native/.gitkeep` for directory preservation
- Maintains small package size (8 MB) - GPU libs still downloaded separately during postinstall

## [0.3.13] - 2025-11-13

### Fixed
- **UI service hardcoded paths**: UI service now uses template with auto-detected paths
  - Previously: Hardcoded `/usr/local/lib/node_modules/swictation/` path
  - Now: Auto-detects installation directory (`__INSTALL_DIR__` placeholder)
  - Also auto-detects `DISPLAY` and `WAYLAND_DISPLAY` environment variables
  - Fixes UI service failures on user-local and nvm installations

### Changed
- Removed `config/swictation-ui.service` (replaced with template)
- Added `templates/swictation-ui.service.template` with placeholders
- Updated postinstall to template UI service like daemon service

## [0.3.12] - 2025-11-13

### Fixed
- **GPU acceleration broken**: Updated to gpu-libs-v1.0.1 with complete library set
  - gpu-libs-v1.0.0 was missing libonnxruntime.so (main ONNX Runtime library)
  - gpu-libs-v1.0.0 was missing libsherpa-onnx-c-api.so and libsherpa-onnx-cxx-api.so
  - This caused "ONNX Runtime not found" warning and model test-loading failures
  - All GPU acceleration fell back to CPU-only mode
  - **Now includes all 6 required libraries** (218 MB total)

### Changed
- Updated GPU_LIBS_VERSION from 1.0.0 to 1.0.1 in postinstall.js

## [0.3.11] - 2025-11-13

### Fixed
- **Confusing model test-loading documentation**: README now accurately reflects default behavior
  - Model testing IS enabled by default when GPU detected (not optional)
  - Removed misleading `TEST_MODEL_LOADING=1` variable that didn't do anything
  - Clarified: Default = model testing (GPU systems), SKIP_MODEL_TEST=1 = disable it
  - Updated installation instructions to reflect actual behavior

### Documentation
- Updated README Installation Options section for clarity
- Removed confusing "Default: Fast install, no model test-loading" language
- Emphasized that model testing verifies GPU setup works correctly

## [0.3.1] - 2025-11-13

### Fixed
- **Old service cleanup during upgrades**: Automatically detects and removes conflicting service files from previous installations
  - Stops and disables old services (both user and system-wide)
  - Removes old service files to prevent conflicts
  - Reloads systemd daemon after cleanup
- **Interactive config migration**: Pacman/apt-style prompts for handling config file conflicts
  - Options to keep, replace, or compare existing config with new template
  - Intelligent detection of config differences
  - Backup creation before replacement
- **Intelligent GPU VRAM detection**: Improved model recommendation based on available GPU memory
  - Detects NVIDIA GPU VRAM using nvidia-smi
  - Fixed VRAM thresholds: 6GB+ for 1.1B model, 4GB+ for 0.6B model
  - Clear warnings for systems with <4GB VRAM
  - Prevents loading models that exceed available VRAM
- **Robust error handling**: Enhanced error recovery throughout postinstall script
  - Graceful fallback when GPU detection fails
  - Better handling of missing dependencies
  - Informative error messages for troubleshooting
- **Optional model test-loading**: Environment variable support for CI/headless installs
  - `SKIP_MODEL_TEST=1` to skip model testing entirely
  - `TEST_MODEL_LOADING=1` to explicitly enable model testing
  - Default: No model testing (faster, more reliable installs)

### Added
- Support for headless/CI installations without interactive prompts
- Better service file generation with template system
- UI service installation alongside daemon service

### Changed
- ONNX Runtime library detection now prioritizes bundled GPU-enabled library over Python installations
- Improved logging and status messages throughout installation
- More informative warnings about Python ONNX Runtime fallbacks

## [0.3.0] - 2025-11-12

### Added
- **Secretary Mode v0.3.0**: Complete natural dictation with 60+ transformation rules
  - Smart punctuation (comma, period, colon, semicolon, dash, ellipsis)
  - Brackets & parentheses with plural support
  - Stateful quote handling with auto-capitalization
  - Special symbols ($, %, @, &, *, #, /, \, +, =, ×)
  - Abbreviations (Mr., Dr., etc.)
  - Number conversion ("number forty two"→42)
  - Formatting (new line, new paragraph, tab)
  - Capitalization modes
  - 27/27 tests passing with real-world voice samples

### Fixed
- GPU CUDA environment fixes in systemd service files
- cuDNN support with cuda-12.9 lib path for ONNX Runtime
- Complete GPU acceleration fixes

## [0.2.2] - 2025-11-11

### Fixed
- **CRITICAL VAD FIX**: Replaced incorrect Silero VAD model with k2-fsa/sherpa-onnx pre-converted version
  - Previous model from onnx-community/silero-vad had wrong tensor format (input/state/sr)
  - New model from k2-fsa uses correct format (x/h/c) matching our ONNX Runtime code
  - Fixes "Invalid input name: x" errors that prevented VAD from working
  - Model size: 629 KB (down from 2.2 MB)
  - Source: https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx

### Added
- Direct URL download support in model-downloader.js for non-HuggingFace models
- Uses curl for fast direct downloads when model.directUrl is specified

## [0.2.1] - 2025-11-11

### Added
- Sway/Wayland environment detection in UI launcher
- Qt6 swaybar + Tauri hybrid UI for Sway environments
- Python tray UI with PySide6 for better system tray integration
- Intelligent UI variant selection based on compositor

### Fixed
- UI service now correctly launches hybrid mode on Sway
- systemd service dependencies for proper UI auto-start

## [0.2.0] - 2025-11-11

### Added
- NVIDIA GPU acceleration with bundled CUDA 12/13 providers
- Rust-based architecture with ort ONNX Runtime
- Parakeet-TDT 0.6B and 1.1B model support
- Silero VAD v6 integration
- systemd user service integration
- HuggingFace model auto-download

### Changed
- Complete rewrite from Python to Rust
- Migrated from sherpa-rs to direct ort 2.0 integration
- Modern ONNX Runtime with GPU acceleration support

