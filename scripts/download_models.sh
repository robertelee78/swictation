#!/bin/bash
# Download Parakeet-TDT ONNX models for adaptive selection
# Models will be cached in ~/.cache/huggingface/hub/

set -e  # Exit on error

echo "================================================"
echo "Downloading Parakeet-TDT ONNX Models"
echo "================================================"
echo ""

# Check if huggingface-cli is installed
if ! command -v huggingface-cli &> /dev/null; then
    echo "❌ huggingface-cli not found!"
    echo ""
    echo "Install with:"
    echo "  pip install --upgrade huggingface_hub"
    exit 1
fi

echo "✓ huggingface-cli found"
echo ""

# Download 0.6B model (official sherpa-onnx conversion)
echo "================================================"
echo "1/2: Downloading Parakeet-TDT 0.6B (sherpa-onnx)"
echo "================================================"
echo "Repository: csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3"
echo "Size: ~800MB"
echo "Target: /opt/swictation/models/parakeet-tdt-0.6b-v3-onnx"
echo ""

huggingface-cli download \
    csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3 \
    --local-dir /opt/swictation/models/parakeet-tdt-0.6b-v3-onnx

echo ""
echo "✓ 0.6B model downloaded successfully"
echo ""

# Download 1.1B model (your custom export)
echo "================================================"
echo "2/2: Downloading Parakeet-TDT 1.1B (custom ONNX)"
echo "================================================"
echo "Repository: jenerallee78/parakeet-tdt-1.1b-onnx"
echo "Size: ~1.8GB"
echo "Target: /opt/swictation/models/parakeet-tdt-1.1b-onnx"
echo ""

huggingface-cli download \
    jenerallee78/parakeet-tdt-1.1b-onnx \
    --local-dir /opt/swictation/models/parakeet-tdt-1.1b-onnx

echo ""
echo "✓ 1.1B model downloaded successfully"
echo ""

# Verify downloads
echo "================================================"
echo "Verifying Downloads"
echo "================================================"
echo ""

check_model() {
    local model_dir=$1
    local model_name=$2

    echo "Checking $model_name..."

    if [ ! -d "$model_dir" ]; then
        echo "  ❌ Directory not found: $model_dir"
        return 1
    fi

    # Check for required files (either .onnx or .int8.onnx)
    local has_encoder=0
    local has_decoder=0
    local has_joiner=0
    local has_tokens=0

    if [ -f "$model_dir/encoder.onnx" ] || [ -f "$model_dir/encoder.int8.onnx" ]; then
        has_encoder=1
    fi

    if [ -f "$model_dir/decoder.onnx" ] || [ -f "$model_dir/decoder.int8.onnx" ]; then
        has_decoder=1
    fi

    if [ -f "$model_dir/joiner.onnx" ] || [ -f "$model_dir/joiner.int8.onnx" ]; then
        has_joiner=1
    fi

    if [ -f "$model_dir/tokens.txt" ]; then
        has_tokens=1
    fi

    if [ $has_encoder -eq 1 ] && [ $has_decoder -eq 1 ] && [ $has_joiner -eq 1 ] && [ $has_tokens -eq 1 ]; then
        echo "  ✓ All required files present"
        ls -lh "$model_dir"/*.onnx "$model_dir"/tokens.txt 2>/dev/null | awk '{print "    " $9 " (" $5 ")"}'
        return 0
    else
        echo "  ❌ Missing files:"
        [ $has_encoder -eq 0 ] && echo "    - encoder.onnx or encoder.int8.onnx"
        [ $has_decoder -eq 0 ] && echo "    - decoder.onnx or decoder.int8.onnx"
        [ $has_joiner -eq 0 ] && echo "    - joiner.onnx or joiner.int8.onnx"
        [ $has_tokens -eq 0 ] && echo "    - tokens.txt"
        return 1
    fi
}

echo ""
check_model "/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx" "0.6B model"
echo ""
check_model "/opt/swictation/models/parakeet-tdt-1.1b-onnx" "1.1B model"
echo ""

echo "================================================"
echo "✅ Download Complete!"
echo "================================================"
echo ""
echo "Models are cached in: ~/.cache/huggingface/hub/"
echo "Symlinked to: /opt/swictation/models/"
echo ""
echo "Next steps:"
echo "  1. Build daemon: cd rust-crates && cargo build --release"
echo "  2. Run daemon: ./rust-crates/target/release/swictation-daemon"
echo ""
