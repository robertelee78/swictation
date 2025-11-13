# Changelog

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

