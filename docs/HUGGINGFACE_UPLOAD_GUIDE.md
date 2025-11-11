# Hugging Face Upload Guide

## Step 1: Login to Hugging Face
```bash
huggingface-cli login
# Enter your HF token (from https://huggingface.co/settings/tokens)
```

## Step 2: Create a New Model Repository
Visit: https://huggingface.co/new

**Suggested repo name**: `parakeet-tdt-1.1b-onnx`
- Type: Model
- License: Same as NVIDIA's (Apache 2.0)
- Make it public so others can use it

## Step 3: Upload the Models
```bash
cd /opt/swictation/models/parakeet-tdt-1.1b

# Upload all files (this will take a while - 12GB)
huggingface-cli upload YOUR_USERNAME/parakeet-tdt-1.1b-onnx . \
  --repo-type model \
  --commit-message "Add verified working ONNX export of Parakeet-TDT 1.1B"
```

## Step 4: Add Model Card (README.md on HF)
The model card below will be automatically uploaded if you save it as README.md in the directory.

## Alternative: Upload Specific Files Only
If you want to upload just the INT8 quantized models (smaller):
```bash
huggingface-cli upload YOUR_USERNAME/parakeet-tdt-1.1b-onnx \
  encoder.int8.onnx encoder.int8.weights \
  decoder.int8.onnx \
  joiner.int8.onnx \
  tokens.txt \
  --repo-type model
```

## File Size Breakdown
- encoder.weights: 4.0GB (FP32 - optional)
- encoder.int8.weights: 2.0GB (quantized - recommended)
- encoder.int8.onnx: 1.1GB
- Layer files: ~6GB (639 files)
- Total: ~12GB

**Recommendation**: Upload everything so users have both FP32 and INT8 options.
