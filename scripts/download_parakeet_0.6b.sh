#!/bin/bash
# Download Parakeet-TDT-0.6B-v3 ONNX model (multilingual, 25 languages)

MODEL_DIR="/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx"

echo "Downloading Parakeet-TDT-0.6B-v3 ONNX model..."
echo "Note: This model is approximately 2.5GB in size and supports 25 languages"

# Create directory if not exists
mkdir -p "$MODEL_DIR"

# Download model files from Hugging Face (istupakov's ONNX conversion)
BASE_URL="https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main"

echo "Downloading encoder model..."
wget -c "$BASE_URL/encoder-model.onnx" -O "$MODEL_DIR/encoder-model.onnx"

echo "Downloading encoder model data..."
wget -c "$BASE_URL/encoder-model.onnx.data" -O "$MODEL_DIR/encoder-model.onnx.data"

echo "Downloading decoder-joint model..."
wget -c "$BASE_URL/decoder_joint-model.onnx" -O "$MODEL_DIR/decoder_joint-model.onnx"

echo "Downloading vocabulary..."
wget -c "$BASE_URL/vocab.txt" -O "$MODEL_DIR/vocab.txt"

echo "Model download complete!"
echo "Location: $MODEL_DIR"
ls -lh "$MODEL_DIR"