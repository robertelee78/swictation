#!/bin/bash
# Test 1.1B Parakeet-TDT model with direct ONNX Runtime
# Requires ONNX Runtime 1.22+ via ORT_DYLIB_PATH

set -e

echo "=========================================="
echo "1.1B Parakeet-TDT Model Test"
echo "=========================================="
echo ""

# Set ONNX Runtime library path
echo "Setting up ONNX Runtime environment..."
export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")

if [ ! -f "$ORT_DYLIB_PATH" ]; then
    echo "❌ ERROR: ONNX Runtime library not found at: $ORT_DYLIB_PATH"
    echo ""
    echo "Please install onnxruntime-gpu:"
    echo "  pip3 install onnxruntime-gpu"
    exit 1
fi

echo "✓ Using ONNX Runtime: $ORT_DYLIB_PATH"
echo ""

# Run the test
cd /opt/swictation/rust-crates
cargo run --release --example test_1_1b
