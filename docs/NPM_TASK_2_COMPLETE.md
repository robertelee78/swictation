# Task 2 Complete: Automated Model Download Command

## ✅ Status: COMPLETE

**Task ID**: b6e065ed-0045-4bdb-af78-eee6273f5541
**Completion Date**: 2025-11-11
**Blocker Status**: RESOLVED ✓

## 🎯 Objective

Implement `swictation download-models` command to automate downloading 9.43 GB of AI models from HuggingFace, eliminating the manual download blocker for npm distribution.

## 📝 Implementation Summary

### Files Created
- `/opt/swictation/npm-package/lib/model-downloader.js` (260 lines)
  - Uses modern `hf` CLI (not deprecated `huggingface-cli`)
  - Downloads from HuggingFace repositories
  - Progress tracking and error handling
  - Model verification before download

### Files Modified
- `/opt/swictation/npm-package/bin/swictation`
  - Added `download-models` command handler
  - Integrated with CLI argument parsing
  - Updated help menu with download options

- `/opt/swictation/npm-package/postinstall.js`
  - Added `hf` CLI dependency check
  - Updated installation instructions
  - Enhanced next steps guidance

## 🚀 Features Implemented

### Command Interface
```bash
swictation download-models [--model=<0.6b|1.1b|both>] [--force]

Options:
  --model=0.6b   Download 0.6B model only (2.47GB, CPU-friendly)
  --model=1.1b   Download 1.1B model only (6.96GB, GPU-optimized)
  --model=both   Download both models (9.43GB, default)
  --force        Re-download even if models exist
```

### Models Supported
1. **Silero VAD v6** (656 KB)
   - Repository: `snakers4/silero-vad`
   - File: `files/silero_vad.onnx`

2. **Parakeet-TDT 0.6B** (2.47 GB)
   - Repository: `csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3`
   - Files: `encoder.onnx`, `decoder.onnx`, `joiner.onnx`, `tokens.txt`

3. **Parakeet-TDT 1.1B INT8** (6.96 GB)
   - Repository: `jenerallee78/parakeet-tdt-1.1b-onnx`
   - Files: `encoder.int8.onnx`, `decoder.int8.onnx`, `joiner.int8.onnx`, `tokens.txt`

### Smart Features
- ✅ Skip already downloaded models (unless `--force`)
- ✅ XDG-compliant target: `~/.local/share/swictation/models/`
- ✅ Dependency check (requires `hf` CLI from `huggingface_hub[cli]`)
- ✅ Clear error messages with installation instructions
- ✅ Post-download processing (VAD model file copy)

## 📦 Dependencies

### Required (for model downloads)
- `hf` CLI from `huggingface_hub[cli]` package
  - Install: `pip install "huggingface_hub[cli]"`
  - Or: `pipx install "huggingface_hub[cli]"`

### Why `hf` CLI?
- Modern replacement for deprecated `huggingface-cli`
- Better performance and features
- Official HuggingFace recommendation

## 🧪 Testing

### Validation Performed
```bash
# Help menu verification
node bin/swictation help
# ✓ Shows download-models command
# ✓ Lists all flags and options

# Dependency check
node lib/model-downloader.js
# ✓ Detects missing hf CLI
# ✓ Shows clear installation instructions

# Integration test
node bin/swictation download-models
# ✓ Properly routes to model downloader
# ✓ Maintains error handling
```

## 📊 Impact Assessment

### Before This Task
- ❌ Users had to manually run `/opt/swictation/scripts/download_models.sh`
- ❌ Required understanding of script location
- ❌ Hardcoded paths incompatible with npm
- ❌ No integration with npm workflow

### After This Task
- ✅ Single command: `swictation download-models`
- ✅ Integrated into npm installation flow
- ✅ XDG-compliant user directories
- ✅ Clear error messages and guidance
- ✅ Model selection flexibility

## 🔗 Integration with npm Flow

### User Installation Journey
```bash
# 1. Install via npm
npm install -g swictation

# 2. Install hf CLI (if needed)
pip install "huggingface_hub[cli]"

# 3. Download models
swictation download-models --model=0.6b  # CPU-friendly option

# 4. Setup and run
swictation setup
swictation start
```

## 📈 Progress on npm Distribution

### Blocker Resolution Status
1. ✅ **DONE**: Package metadata updated
2. ✅ **DONE**: Hardcoded model paths fixed (XDG)
3. ✅ **DONE**: Automated model download ← This task
4. ⏳ **TODO**: Fix missing libsherpa-onnx-c-api.so
5. ⏳ **TODO**: VM testing
6. ⏳ **TODO**: Dry-run validation
7. ⏳ **TODO**: npm publish

**Overall Progress**: 3/7 tasks complete (43%)

## 🎓 Technical Decisions

### Why `hf download` over Direct HTTP?
1. **Authentication**: HuggingFace now requires auth for many models
2. **Reliability**: Built-in retry, resume, and verification
3. **Caching**: Automatic caching in `~/.cache/huggingface/`
4. **Maintenance**: Official tool, less likely to break

### Model File Organization
```
~/.local/share/swictation/models/
├── silero-vad/
│   └── silero_vad.onnx
├── sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx/
│   ├── encoder.onnx
│   ├── decoder.onnx
│   ├── joiner.onnx
│   └── tokens.txt
└── parakeet-tdt-1.1b-onnx/
    ├── encoder.int8.onnx
    ├── decoder.int8.onnx
    ├── joiner.int8.onnx
    └── tokens.txt
```

## 🚧 Known Limitations

1. **Python Dependency**: Requires Python + pip for `hf` CLI
   - Acceptable: Most ML users already have Python
   - Alternative: Could bundle pre-downloaded models (9.43 GB npm package!)

2. **Download Time**: 9.43 GB takes time on slow connections
   - Mitigated: User can choose `--model=0.6b` (2.47 GB) for CPU-only

3. **Internet Required**: No offline installation option
   - Future: Consider torrent/mirror support

## 📚 Documentation Updates Needed

- [ ] Add "Model Download" section to README
- [ ] Document `--model` flag options
- [ ] Explain CPU vs GPU model selection
- [ ] Troubleshooting guide for `hf` CLI issues

## 🎉 Success Criteria Met

- ✅ Single command model download
- ✅ XDG-compliant paths
- ✅ Clear dependency messages
- ✅ Model selection flexibility
- ✅ Integration with npm workflow
- ✅ Uses modern `hf` CLI

## 🔜 Next Task

**Task 4**: Fix missing `libsherpa-onnx-c-api.so` shared library
- Options: Static linking OR bundle .so file
- Impact: Makes binary truly portable
- Priority: P0 (Critical Blocker)
