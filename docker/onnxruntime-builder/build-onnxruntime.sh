#!/bin/bash
# ONNX Runtime Multi-Architecture Build Script
#
# Usage:
#   ./build-onnxruntime.sh sm_90                    # Single architecture
#   ./build-onnxruntime.sh "50;52;60;61;70"        # Multiple architectures (legacy)
#   ./build-onnxruntime.sh "75;80;86"              # Multiple architectures (modern)
#
# Environment variables:
#   CUDA_ARCHITECTURES - Override architectures (default: from arg1)
#   BUILD_CONFIG       - Release or Debug (default: Release)
#   SKIP_TESTS         - Set to 1 to skip tests (default: 1)

set -e  # Exit on error
set -u  # Exit on undefined variable

# Configuration
CUDA_ARCHITECTURES="${1:-90}"
BUILD_CONFIG="${BUILD_CONFIG:-Release}"
SKIP_TESTS="${SKIP_TESTS:-1}"

echo "================================================"
echo "ONNX Runtime Multi-Architecture Builder"
echo "================================================"
echo "CUDA Architectures: ${CUDA_ARCHITECTURES}"
echo "Build Config: ${BUILD_CONFIG}"
echo "Skip Tests: ${SKIP_TESTS}"
echo "================================================"

# Navigate to ONNX Runtime directory
cd /workspace/onnxruntime

# Clean previous builds
echo "Cleaning previous build artifacts..."
rm -rf build/Linux/${BUILD_CONFIG}

# Run ONNX Runtime build script
echo "Starting ONNX Runtime build..."
echo "This will take 45-70 minutes depending on number of architectures..."

BUILD_ARGS=(
    --config "${BUILD_CONFIG}"
    --build_shared_lib
    --parallel
    --use_cuda
    --cuda_version 12.9
    --cuda_home /usr/local/cuda
    --cudnn_home /usr
    --cmake_extra_defines "CMAKE_CUDA_ARCHITECTURES=${CUDA_ARCHITECTURES}"
    --cmake_extra_defines "CMAKE_CUDA_COMPILER=/usr/local/cuda/bin/nvcc"
    --allow_running_as_root
)

# Skip tests if requested (saves 30-50% build time)
if [ "${SKIP_TESTS}" = "1" ]; then
    BUILD_ARGS+=(--skip_tests)
fi

./build.sh "${BUILD_ARGS[@]}"

echo "================================================"
echo "Build Complete!"
echo "================================================"

# Show build output location
BUILD_DIR="build/Linux/${BUILD_CONFIG}"
echo "Build directory: ${BUILD_DIR}"
echo ""
echo "CUDA Provider library:"
ls -lh "${BUILD_DIR}/libonnxruntime_providers_cuda.so" 2>/dev/null || echo "  Not found"
echo ""
echo "All ONNX Runtime libraries:"
ls -lh "${BUILD_DIR}"/libonnxruntime*.so

echo ""
echo "Verifying CUDA architectures..."
if command -v cuobjdump &> /dev/null; then
    echo "PTX files in library:"
    cuobjdump --list-ptx "${BUILD_DIR}/libonnxruntime_providers_cuda.so" | grep "PTX file" | head -10
else
    echo "cuobjdump not available, skipping verification"
fi

echo ""
echo "CUDA dependencies:"
ldd "${BUILD_DIR}/libonnxruntime_providers_cuda.so" | grep cuda

echo "================================================"
echo "Build artifacts ready for extraction"
echo "================================================"

# Copy artifacts to /output if directory exists (mounted from host)
if [ -d "/output" ]; then
    echo ""
    echo "Copying artifacts to /output..."
    cp "${BUILD_DIR}"/libonnxruntime*.so /output/ 2>/dev/null || true
    echo "✓ Artifacts copied to /output/"
    ls -lh /output/libonnxruntime*.so 2>/dev/null || echo "  (copy may have failed - check manually)"
fi
