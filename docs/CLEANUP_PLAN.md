# Repository Cleanup Plan - Parakeet TDT 1.1B

## ✅ Verified Working Script
- **Script**: `/opt/swictation/scripts/export_parakeet_tdt_1.1b.py`
- **Status**: Produces IDENTICAL models to working version (MD5 verified)
- **Metadata**: Correct `feat_dim: 80`, `vocab_size: 1024`

## 📋 Cleanup Tasks

### 1. Remove Incorrect Export Script
```bash
# Remove the older/incorrect version
rm /opt/swictation/scripts/export_parakeet_1.1b.py
```

### 2. Update Documentation
Files to review and update:
- `/opt/swictation/models/parakeet-tdt-1.1b/README.md` - Update to reference HF download
- `/opt/swictation/models/parakeet-tdt-1.1b/AHA_EXPORT_BUG_ANALYSIS.md` - Mark as historical
- Remove or consolidate other AHA/analysis docs that contradict working solution

### 3. Clean Up Contradictory Markdown Files
Files in `models/parakeet-tdt-1.1b/` to clean:
```
- AHA_EXPORT_BUG_ANALYSIS.md (archive or remove - historical debugging)
- AHA_PHYSICAL_ACOUSTIC_VALIDATION.md (archive or remove)
- TESTING_NOTES.md (update with current status)
```

### 4. Add Models to .gitignore
```bash
# Prevent accidentally committing 12GB models again
echo "" >> .gitignore
echo "# Model files (download from Hugging Face or export with script)" >> .gitignore
echo "models/parakeet-tdt-1.1b/*.onnx" >> .gitignore
echo "models/parakeet-tdt-1.1b/*.weights" >> .gitignore
echo "models/parakeet-tdt-1.1b/layers.*" >> .gitignore
echo "models/parakeet-tdt-1.1b/onnx__*" >> .gitignore
echo "models/parakeet-tdt-1.1b/Constant_*" >> .gitignore
```

### 5. Create Simple README for Model Directory
Replace existing README with:
```markdown
# Parakeet-TDT 1.1B ONNX Models

## Quick Start - Download from Hugging Face (Recommended)

```bash
# Download pre-exported ONNX models (12GB)
huggingface-cli download YOUR_USERNAME/parakeet-tdt-1.1b-onnx --local-dir models/parakeet-tdt-1.1b
```

## Alternative - Export Yourself

If you prefer to export from source:

```bash
cd /opt/swictation
docker run --rm -v $(pwd):/workspace -w /workspace/scripts \
  nvcr.io/nvidia/nemo:25.07 \
  bash -c "pip install onnxruntime && python3 export_parakeet_tdt_1.1b.py"
```

## Files Included

- `encoder.int8.onnx` (1.1GB) - Quantized encoder
- `encoder.weights` (4.0GB) - FP32 encoder weights  
- `encoder.int8.weights` (2.0GB) - INT8 encoder weights
- `decoder.int8.onnx` (7.0MB) - Quantized decoder
- `joiner.int8.onnx` (1.7MB) - Quantized joiner
- `tokens.txt` (11KB) - Vocabulary (1025 tokens)
- 639 layer weight files

## Model Details

- **Source**: nvidia/parakeet-tdt-1.1b (Hugging Face)
- **Parameters**: 1.1 billion
- **Features**: 80 mel filterbank
- **Vocab Size**: 1024 tokens + blank
- **Architecture**: Token-and-Duration Transducer (TDT)

## Verified Working

These models have been tested and produce correct transcriptions with the Rust implementation.
```
