# Changelog

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

