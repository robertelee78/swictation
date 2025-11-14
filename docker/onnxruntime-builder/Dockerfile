# ONNX Runtime Multi-Architecture Builder
# Supports building with custom CMAKE_CUDA_ARCHITECTURES for sm_50-90

FROM nvidia/cuda:12.9.0-devel-ubuntu22.04

LABEL maintainer="robert@agidreams.us"
LABEL description="ONNX Runtime builder with CUDA 12.6 for multi-architecture support"

# Prevent interactive prompts during apt install
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install build dependencies
RUN apt-get update && apt-get install -y \
    git \
    wget \
    curl \
    build-essential \
    software-properties-common \
    libssl-dev \
    zlib1g-dev \
    python3.10 \
    python3.10-dev \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install CMake 3.28+ (ONNX Runtime requires 3.27+)
RUN wget https://github.com/Kitware/CMake/releases/download/v3.28.3/cmake-3.28.3-linux-x86_64.tar.gz \
    && tar -xzf cmake-3.28.3-linux-x86_64.tar.gz -C /opt \
    && rm cmake-3.28.3-linux-x86_64.tar.gz \
    && ln -s /opt/cmake-3.28.3-linux-x86_64/bin/cmake /usr/local/bin/cmake \
    && ln -s /opt/cmake-3.28.3-linux-x86_64/bin/ctest /usr/local/bin/ctest

# Install Python packages needed for ONNX Runtime build
RUN pip3 install --no-cache-dir \
    numpy \
    packaging \
    wheel \
    setuptools

# Install cuDNN 9.x (required for ONNX Runtime 1.23+)
# Note: This will be downloaded from NVIDIA container runtime
# Alternative: Mount cuDNN from host system
RUN apt-get update && apt-get install -y \
    libcudnn9-dev-cuda-12 \
    libcudnn9-cuda-12 \
    && rm -rf /var/lib/apt/lists/*

# Set up CUDA paths
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=$CUDA_HOME/bin:$PATH
ENV LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH

# Clone ONNX Runtime repository
WORKDIR /workspace
RUN git clone --recursive https://github.com/microsoft/onnxruntime.git \
    && cd onnxruntime \
    && git checkout v1.23.2 \
    && git submodule update --init --recursive

# Create build directory
WORKDIR /workspace/onnxruntime
RUN mkdir -p build

# Build script will be mounted from host
# Usage: docker run -v $(pwd)/build.sh:/workspace/build.sh onnxruntime-builder

# Default command
CMD ["/bin/bash"]
