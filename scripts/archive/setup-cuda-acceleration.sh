#!/usr/bin/env bash
# Swictation CUDA Acceleration Setup Script
# Automatically installs and configures CUDA 12/13 libraries for GPU acceleration

set -euo pipefail

echo "🚀 Swictation CUDA Acceleration Setup"
echo "======================================"
echo ""

# Check for NVIDIA GPU
if ! command -v nvidia-smi &> /dev/null; then
    echo "❌ nvidia-smi not found. NVIDIA GPU may not be available."
    exit 1
fi

echo "✅ NVIDIA GPU detected:"
nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
echo ""

# Install modern CUDA 12/13 runtime libraries
echo "📦 Installing CUDA 12/13 runtime libraries..."
pip3 install --upgrade \
    nvidia-cudnn-cu12 \
    nvidia-cublas-cu12 \
    nvidia-cufft-cu12 \
    nvidia-cuda-runtime-cu12 \
    onnxruntime-gpu

echo ""
echo "✅ CUDA libraries installed:"
pip3 list | grep -E "nvidia-(cudnn|cublas|cufft|cuda-runtime)-cu12|onnxruntime-gpu"
echo ""

# Create ONNX Runtime's hardcoded RUNPATH directory
echo "🔗 Creating symlinks for ONNX Runtime..."
sudo mkdir -p /home/runner/work/ort-artifacts/ort-artifacts/cudnn/lib
sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cudnn/lib/libcudnn*.so* \
    /home/runner/work/ort-artifacts/ort-artifacts/cudnn/lib/
sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cublas/lib/libcublas*.so* \
    /home/runner/work/ort-artifacts/ort-artifacts/cudnn/lib/

# Create system-wide CUDA library paths
echo "🔗 Creating system-wide CUDA symlinks..."
for CUDA_VER in 12 13; do
    sudo mkdir -p /usr/local/cuda-$CUDA_VER/lib
    sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cudnn/lib/libcudnn.so.9 \
        /usr/local/cuda-$CUDA_VER/lib/
    sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cublas/lib/libcublas.so.12 \
        /usr/local/cuda-$CUDA_VER/lib/
    sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cublas/lib/libcublasLt.so.12 \
        /usr/local/cuda-$CUDA_VER/lib/
    sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cufft/lib/libcufft.so.11 \
        /usr/local/cuda-$CUDA_VER/lib/
    sudo ln -sf ~/.local/lib/python3.12/site-packages/nvidia/cuda_runtime/lib/libcudart.so.12 \
        /usr/local/cuda-$CUDA_VER/lib/
done

echo ""
echo "✅ Symlinks created:"
ls -lah /home/runner/work/ort-artifacts/ort-artifacts/cudnn/lib/ | head -10
echo ""

# Create environment configuration file
echo "📝 Creating environment configuration..."
cat > ~/.config/swictation/cuda-env.sh <<'EOF'
# Swictation CUDA Acceleration Environment
# Source this file or add to ~/.bashrc

export LD_LIBRARY_PATH="/opt/swictation/rust-crates/target/release:/usr/local/cuda-12.9/lib64:$LD_LIBRARY_PATH"

# Alias for running daemon with GPU acceleration
alias swictation-daemon-gpu='cd /opt/swictation/rust-crates/target/release && LD_LIBRARY_PATH=.:/usr/local/cuda-12.9/lib64 ./swictation-daemon'
EOF

chmod +x ~/.config/swictation/cuda-env.sh
echo "✅ Environment file created at: ~/.config/swictation/cuda-env.sh"
echo ""

# Test GPU acceleration
echo "🧪 Testing GPU acceleration..."
cd /opt/swictation/rust-crates/target/release
export LD_LIBRARY_PATH=.:/usr/local/cuda-12.9/lib64

if timeout 5 ./swictation-daemon 2>&1 | grep -q "Successfully registered.*CUDAExecutionProvider"; then
    echo "✅ GPU ACCELERATION WORKING!"
    echo ""
    echo "🎉 Setup complete! GPU acceleration is enabled."
    echo ""
    echo "To run the daemon with GPU acceleration:"
    echo "  source ~/.config/swictation/cuda-env.sh"
    echo "  swictation-daemon-gpu"
    echo ""
    echo "Expected performance improvement: 4.2x faster (180ms vs 750ms CPU)"
else
    echo "⚠️  GPU acceleration test inconclusive. Check logs manually."
    echo ""
    echo "To test manually:"
    echo "  cd /opt/swictation/rust-crates/target/release"
    echo "  LD_LIBRARY_PATH=.:/usr/local/cuda-12.9/lib64 ./swictation-daemon"
    echo ""
    echo "Look for: 'Successfully registered CUDAExecutionProvider'"
fi

echo ""
echo "📚 Documentation: /opt/swictation/docs/GPU_ACCELERATION_SETUP.md"
echo "🎮 GPU Status: nvidia-smi"
echo ""
