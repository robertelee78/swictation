# Swictation Rust Migration Roadmap

## Overview

Migration from Python/PyTorch to Pure Rust ONNX architecture with NPM distribution.

**Goal**: Single-command installation (`npm install -g swictation`) with 5-7x performance improvements.

## Current State (Python/PyTorch)

- **Model**: nvidia/canary-1b-flash (1.48-5.2% WER)
- **Runtime**: Python 3.12 + PyTorch + NeMo
- **Memory**: 4.5GB RAM, 1.8GB VRAM (FP16)
- **Startup**: 15 seconds
- **Distribution**: DEB package (Debian/Ubuntu only)
- **UI**: Python/Qt (PySide6 + QML)

## Target State (Pure Rust ONNX)

- **Model**: Parakeet-TDT-0.6B-V3 (6.05% WER, #1 ONNX model)
- **Runtime**: Pure Rust + ONNX Runtime (no Python)
- **Memory**: 800MB RAM, 640MB VRAM (INT8)
- **Startup**: 2-3 seconds
- **Distribution**: NPM (`npm install -g swictation`)
- **UI**: Tauri (Rust + React/Web)

## Performance Improvements

| Metric | Before (Python) | After (Rust) | Improvement |
|--------|-----------------|--------------|-------------|
| Startup Time | 15s | 2-3s | **5-7x faster** |
| Memory (RAM) | 4.5GB | 800MB | **82% reduction** |
| VRAM Usage | 1.8GB | 640MB | **64% reduction** |
| WER | 1.48-5.2% | ~6.05% | ~1% degradation |
| Installation | Multi-step | 1 command | **Seamless** |
| Platforms | Linux only | Linux/Mac/Win | **Cross-platform** |

## Migration Phases

### ✅ Phase 0: Research & Planning (COMPLETE)
- [x] Codebase analysis
- [x] ONNX model research
- [x] Architecture design
- [x] Migration checklist (70+ items)
- [x] Archon task planning

**Branch**: `rust-migration` (created)
**Rollback Tag**: `pre-rust-migration` (safe rollback point)

### 📍 Phase 1: Audio Pipeline (Weeks 1-2) - IN PROGRESS

**Archon Task**: `e2b2e87f-272f-4069-8e5c-b0ea5596398b`

**Components**:
- Lock-free circular buffer (ringbuf) ✅ COMPLETE
- Error types ✅ COMPLETE
- Audio capture with cpal ⏳ TODO
- Resampler (rubato) ⏳ TODO
- 70+ item API compatibility checklist

**Location**: `/opt/swictation/rust-crates/swictation-audio/`

**Goal**: Replace Python audio_capture.py (474 lines) with zero-copy Rust implementation.

### Phase 2: Model Export & Validation (Weeks 1-2)

**Archon Task**: `82a583e3-4fb5-4310-986b-89e6182d4ced`

**Tasks**:
- [x] Model selection (Parakeet-TDT-0.6B-V3)
- [ ] Download pre-exported ONNX model from sherpa-onnx
- [ ] Validate WER on LibriSpeech test set
- [ ] Benchmark inference latency
- [ ] Verify Rust compatibility (ort crate)

**Model Details**:
- Source: `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2`
- Size: ~640MB (INT8 quantized)
- Languages: 25 European languages
- WER: 6.05% (Open ASR Leaderboard #1 for ONNX)

### Phase 3: VAD Migration (Weeks 2-3)

**Archon Task**: `adeb99aa-7f85-41b7-b9e9-dd24891b0220`

**Components**:
- Silero VAD ONNX model (2MB)
- silero-vad-rs Rust crate integration
- Speech segmentation logic
- 512ms detection windows

**Location**: `/opt/swictation/rust-crates/swictation-audio/src/vad.rs`

**Benefits**:
- Remove 500MB+ PyTorch dependency
- 5x faster inference (<10ms vs ~50ms)
- 96% memory reduction (20MB vs 500MB)

### Phase 4: STT Crate (Weeks 3-5)

**Archon Task**: `c037d51d-37ef-4b23-bd1d-c64734b1b23f`

**Components**:
- Model loading (sherpa-onnx or ort)
- Inference engine (Parakeet ONNX)
- VAD integration
- Streaming state machine
- Callbacks for partial/final results

**Location**: `/opt/swictation/rust-crates/swictation-stt/`

**Modules**:
- `model.rs` - ONNX model management
- `inference.rs` - STT inference
- `vad.rs` - Speech segmentation
- `streaming.rs` - Real-time state machine

**Performance Targets**:
- Latency: <250ms total pipeline
- VRAM: <700MB (Parakeet + VAD)
- WER: <7% on LibriSpeech

### Phase 5: Build System (Weeks 5-6)

**Archon Task**: `03749a35-d244-4acb-83e1-a2f713aede32`

**Components**:
- Unified Cargo workspace
- ONNX model download scripts
- Build pipeline for all crates
- Platform-specific binaries
- Automated testing

**Deliverables**:
- Single Rust binary: `swictation-daemon`
- No Python runtime required
- Model packaging automation
- CI/CD for multi-platform builds

### Phase 6: Daemon Rewrite (Weeks 7-8)

**Components**:
- Rewrite swictationd.py (1531 lines) in Rust
- Unix socket server
- Metrics broadcasting
- Integration with audio/VAD/STT crates
- Text transformation (MidStream - already Rust)

**Location**: `/opt/swictation/rust-crates/swictation-daemon/`

**Architecture**:
```
swictation-daemon (Pure Rust)
├── Audio capture (cpal)
├── VAD (silero-vad-rs)
├── STT (sherpa-onnx + Parakeet)
├── Text transform (MidStream)
├── Metrics collector
└── Unix socket server
```

### Phase 7: Tauri UI Migration (Weeks 7-9)

**Archon Task**: `8aeaee3e-7624-4f2b-a62e-0491938479b6`

**Goal**: Replace Python/Qt UI with Tauri for NPM distribution

**Components**:
- Tauri setup (Rust + Web frontend)
- System tray integration
- Metrics display (React/Svelte)
- Socket client (connect to daemon)
- SQLite database access

**Features to Preserve**:
- System tray with recording indicator
- Live metrics (WPM, latency, GPU, CPU)
- Transcription display
- Session history
- Copy-to-clipboard

**Benefits**:
- NPM distributable
- Cross-platform (Linux/macOS/Windows)
- Tiny bundle (3-5MB vs 100MB+ Electron)
- Native system webview

### Phase 8: NPM Packaging (Weeks 8-9)

**Components**:
- Platform-specific binary packages
- NPM launcher script
- Cross-platform detection
- Desktop integration

**Package Structure**:
```
swictation (npm)
├── @swictation/linux-x64
├── @swictation/darwin-x64
├── @swictation/darwin-arm64
└── swictation (main package)
```

**User Experience**:
```bash
# Single command installation
npm install -g swictation

# Run
swictation
```

### Phase 9: Testing & Validation (Weeks 9-10)

**Test Categories**:
- Unit tests (Rust crates)
- Integration tests (end-to-end)
- Performance benchmarks
- WER validation
- Cross-platform testing
- 24-hour stress tests

**Validation Criteria**:
- WER <7% on LibriSpeech
- Latency <250ms total pipeline
- VRAM <700MB
- Memory stable over 24 hours
- All platforms install via npm

### Phase 10: Production Rollout (Week 10)

**Steps**:
1. Publish npm packages
2. Update documentation
3. Migration guide for existing users
4. Side-by-side testing (Rust vs Python)
5. Gradual rollout with monitoring
6. Python deprecation notice

## Technology Stack

### Core Dependencies

**Rust Crates**:
- `cpal` - Audio I/O (PipeWire/ALSA)
- `ringbuf` - Lock-free circular buffer
- `rubato` - Audio resampling
- `sherpa-onnx` - ONNX ASR inference
- `silero-vad` - Voice activity detection
- `ort` - ONNX Runtime (alternative to sherpa-onnx)
- `tauri` - Cross-platform UI framework
- `tokio` - Async runtime

**ONNX Models**:
- Parakeet-TDT-0.6B-V3 (STT) - 640MB INT8
- Silero VAD (Voice detection) - 2MB

**Frontend** (Tauri UI):
- React or Svelte (web framework)
- Vite (build tool)
- Tauri API (system integration)

### Removed Dependencies

**No Longer Needed**:
- ❌ Python runtime (except for legacy UI during transition)
- ❌ PyTorch (500MB+)
- ❌ NVIDIA NeMo
- ❌ sounddevice
- ❌ numpy (for audio processing)
- ❌ 40+ Python packages from requirements.txt

**Replaced By**:
- ✅ Rust binary (~15MB)
- ✅ ONNX Runtime
- ✅ Tauri UI (~3-5MB)

## File Structure

```
/opt/swictation/
├── rust-crates/              # Pure Rust implementation
│   ├── Cargo.toml            # Workspace root ✅
│   ├── swictation-audio/     # Audio capture ⏳
│   ├── swictation-stt/       # ONNX STT engine
│   ├── swictation-vad/       # VAD integration
│   └── swictation-daemon/    # Main binary
├── swictation-ui/            # Tauri UI (NEW)
│   ├── src-tauri/            # Tauri Rust backend
│   └── src/                  # Web frontend (React)
├── models/                   # ONNX models
│   ├── parakeet-tdt-0.6b-v3/ # STT model
│   └── silero_vad.onnx       # VAD model
├── external/
│   └── midstream/            # Text transform ✅
├── src/                      # Legacy Python (deprecate)
├── docs/
│   ├── research/
│   │   └── PURE_RUST_ONNX_RESEARCH.md ✅
│   └── MIGRATION_ROADMAP.md  # This file
└── package.json              # NPM package definition
```

## Git Workflow

**Main Branch**: `main` (stable Python version)
**Migration Branch**: `rust-migration` (active development)
**Rollback Tag**: `pre-rust-migration` (safe restore point)

**Workflow**:
```bash
# Work on Rust migration
git checkout rust-migration

# Test changes
cargo test --workspace

# Commit progress
git commit -m "feat: implement X"
git push origin rust-migration

# When ready to merge
git checkout main
git merge rust-migration

# If issues, instant rollback
git checkout pre-rust-migration
```

## Success Metrics

### Performance
- [x] Startup time: <3s (target: 2-3s vs current 15s)
- [ ] Memory: <1GB RAM (target: 800MB vs current 4.5GB)
- [ ] VRAM: <1GB (target: 640MB vs current 1.8GB)
- [ ] Latency: <300ms (target: 250ms vs current 600-800ms)

### Accuracy
- [ ] WER: <7% (target: 6.05% vs current 1.48-5.2%)
- Acceptable trade-off: ~1% worse for massive performance gains

### Distribution
- [ ] Single-command install: `npm install -g swictation`
- [ ] Cross-platform: Linux, macOS (Intel + ARM), Windows
- [ ] Bundle size: <15MB (vs current 2GB+ with dependencies)

### Quality
- [ ] All 70+ checklist items implemented
- [ ] Zero API breaks (drop-in replacement)
- [ ] 24-hour stability test passing
- [ ] Multi-language support (25 languages vs current 4)

## Risk Mitigation

**Risk 1: WER Degradation**
- Mitigation: Side-by-side testing, user feedback period
- Fallback: Keep Python version available during transition

**Risk 2: ONNX Model Issues**
- Mitigation: Extensive validation on LibriSpeech
- Fallback: Whisper-Large-V3 as alternative model

**Risk 3: Cross-Platform Bugs**
- Mitigation: CI/CD for all platforms, extensive testing
- Fallback: Platform-specific fixes, gradual rollout

**Risk 4: NPM Distribution Issues**
- Mitigation: Test on clean systems, clear documentation
- Fallback: Keep DEB package as alternative

## Timeline Summary

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1-2 | Audio + Models | Audio crate, ONNX models validated |
| 2-3 | VAD | VAD integration complete |
| 3-5 | STT | STT crate with streaming support |
| 5-6 | Build System | Unified build pipeline |
| 7-8 | Daemon | Rust daemon replacing Python |
| 7-9 | Tauri UI | Web-based UI with Tauri |
| 8-9 | NPM | Platform packages published |
| 9-10 | Testing | Full validation and benchmarks |
| 10 | Rollout | Production deployment |

**Total Duration**: 10 weeks

## Next Steps

1. ✅ Create migration branch (`rust-migration`)
2. ✅ Tag rollback point (`pre-rust-migration`)
3. ✅ Update Archon tasks with Parakeet model
4. ⏳ Complete audio capture implementation
5. ⏳ Download and validate Parakeet ONNX model
6. ⏳ Implement STT crate with sherpa-onnx

## Documentation

- [x] Migration Roadmap (this document)
- [x] Pure Rust ONNX Research (`docs/research/PURE_RUST_ONNX_RESEARCH.md`)
- [x] Audio Migration Checklist (`rust-crates/swictation-audio/MIGRATION_CHECKLIST.md`)
- [ ] Tauri Migration Guide (future)
- [ ] NPM Distribution Guide (future)
- [ ] User Migration Guide (future)

## Questions?

For questions about the migration:
- See Archon tasks for detailed implementation plans
- Check `docs/research/PURE_RUST_ONNX_RESEARCH.md` for technical deep dive
- Review individual crate README files

---

**Last Updated**: 2025-11-06
**Status**: Phase 1 in progress
**Branch**: `rust-migration`
