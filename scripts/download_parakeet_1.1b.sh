#!/bin/bash
# Download Parakeet-TDT-1.1B ONNX model

MODEL_DIR="/opt/swictation/models/nvidia-parakeet-tdt-1.1b"

echo "Downloading Parakeet-TDT-1.1B model..."
echo "Note: This model is approximately 4.4GB in size"

# Create directory if not exists
mkdir -p "$MODEL_DIR"

# Download model files from Hugging Face
# These are the ONNX-exported versions of nvidia/parakeet-tdt-1.1b
BASE_URL="https://huggingface.co/nvidia/parakeet-tdt-1.1b/resolve/main/onnx"

echo "Downloading encoder model..."
wget -c "$BASE_URL/encoder-model.onnx" -O "$MODEL_DIR/encoder-model.onnx"

echo "Downloading decoder-joint model..."
wget -c "$BASE_URL/decoder_joint-model.onnx" -O "$MODEL_DIR/decoder_joint-model.onnx"

echo "Downloading vocabulary..."
wget -c "https://huggingface.co/nvidia/parakeet-tdt-1.1b/resolve/main/vocab.txt" -O "$MODEL_DIR/vocab.txt"

echo "Downloading tokenizer configuration..."
wget -c "https://huggingface.co/nvidia/parakeet-tdt-1.1b/resolve/main/tokenizer_config.json" -O "$MODEL_DIR/tokenizer.json"

echo "Model download complete!"
echo "Location: $MODEL_DIR"
ls -lh "$MODEL_DIR"