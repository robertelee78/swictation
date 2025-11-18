#!/bin/bash
# Docker wrapper for ONNX Runtime builds
#
# Usage:
#   ./docker-build.sh build-image              # Build CUDA 12.9 Docker image
#   ./docker-build.sh build-image-cuda11       # Build CUDA 11.8 Docker image (Maxwell support)
#   ./docker-build.sh test                     # Test build with sm_90 (fast)
#   ./docker-build.sh legacy                   # Build legacy package (sm_50-70) with CUDA 11.8
#   ./docker-build.sh modern                   # Build modern package (sm_75-86)
#   ./docker-build.sh latest                   # Build latest package (sm_89-90)
#   ./docker-build.sh custom "52;70;86"        # Build with custom architectures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="onnxruntime-builder:cuda12.9"
IMAGE_NAME_CUDA11="onnxruntime-builder:cuda11.8"
OUTPUT_DIR="${SCRIPT_DIR}/output"

# Create output directory
mkdir -p "${OUTPUT_DIR}"

case "${1:-help}" in
    build-image)
        echo "Building Docker image: ${IMAGE_NAME}"
        echo "This may take 10-15 minutes..."
        docker build -t "${IMAGE_NAME}" "${SCRIPT_DIR}"
        echo ""
        echo "✓ Docker image built successfully!"
        echo "Next: ./docker-build.sh test"
        ;;

    build-image-cuda11)
        echo "Building CUDA 11.8 Docker image: ${IMAGE_NAME_CUDA11}"
        echo "This may take 10-15 minutes..."
        echo ""
        echo "IMPORTANT: This requires cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz"
        echo "           to be present in ${SCRIPT_DIR}/"
        echo ""

        if [ ! -f "${SCRIPT_DIR}/cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz" ]; then
            echo "ERROR: cuDNN 8.9.7 archive not found!"
            echo ""
            echo "Please download from:"
            echo "  https://developer.nvidia.com/rdp/cudnn-archive"
            echo "  (Look for: cuDNN v8.9.7 for CUDA 11.x - Linux x86_64 Tar)"
            echo ""
            echo "Place the file in: ${SCRIPT_DIR}/"
            echo ""
            echo "See CUDNN_DOWNLOAD.md for detailed instructions"
            exit 1
        fi

        docker build -f "${SCRIPT_DIR}/Dockerfile.cuda11" -t "${IMAGE_NAME_CUDA11}" "${SCRIPT_DIR}"
        echo ""
        echo "✓ CUDA 11.8 Docker image built successfully!"
        echo "Next: ./docker-build.sh legacy"
        ;;

    test)
        echo "Running test build with sm_90 (single architecture)"
        echo "Build time: ~45 minutes"
        mkdir -p "${OUTPUT_DIR}/test"
        docker run --rm --gpus all \
            -v "${SCRIPT_DIR}/build-onnxruntime.sh:/workspace/build-onnxruntime.sh:ro" \
            -v "${OUTPUT_DIR}/test:/output" \
            "${IMAGE_NAME}" \
            /workspace/build-onnxruntime.sh "90"

        echo ""
        echo "✓ Test build complete!"
        ls -lh "${OUTPUT_DIR}/test/"
        ;;

    legacy)
        echo "Building LEGACY package (sm_50,52,60,61,70) with CUDA 11.8 + cuDNN 8.9.7"
        echo "Build time: ~60-90 minutes"
        echo ""

        # Check if CUDA 11.8 image exists
        if ! docker image inspect "${IMAGE_NAME_CUDA11}" &>/dev/null; then
            echo "ERROR: CUDA 11.8 Docker image not found!"
            echo "Please run: ./docker-build.sh build-image-cuda11"
            exit 1
        fi

        mkdir -p "${OUTPUT_DIR}/legacy"
        docker run --rm --gpus all \
            -v "${SCRIPT_DIR}/build-onnxruntime.sh:/workspace/build-onnxruntime.sh:ro" \
            -v "${OUTPUT_DIR}/legacy:/output" \
            -e CUDA_VERSION=11.8 \
            -e CUDNN_HOME=/usr/local/cuda \
            "${IMAGE_NAME_CUDA11}" \
            /workspace/build-onnxruntime.sh "50;52;60;61;70"

        echo ""
        echo "✓ Legacy build complete!"
        ls -lh "${OUTPUT_DIR}/legacy/"
        ;;

    modern)
        echo "Building MODERN package (sm_75,80,86)"
        echo "Build time: ~50 minutes"
        mkdir -p "${OUTPUT_DIR}/modern"
        docker run --rm --gpus all \
            -v "${SCRIPT_DIR}/build-onnxruntime.sh:/workspace/build-onnxruntime.sh:ro" \
            -v "${OUTPUT_DIR}/modern:/output" \
            "${IMAGE_NAME}" \
            /workspace/build-onnxruntime.sh "75;80;86"

        echo ""
        echo "✓ Modern build complete!"
        ls -lh "${OUTPUT_DIR}/modern/"
        ;;

    latest)
        echo "Building LATEST package (sm_89,90,100,120)"
        echo "Build time: ~50 minutes"
        echo "Note: Native Blackwell sm_120 support with CUDA 12.9"
        mkdir -p "${OUTPUT_DIR}/latest"
        docker run --rm --gpus all \
            -v "${SCRIPT_DIR}/build-onnxruntime.sh:/workspace/build-onnxruntime.sh:ro" \
            -v "${OUTPUT_DIR}/latest:/output" \
            "${IMAGE_NAME}" \
            /workspace/build-onnxruntime.sh "89;90;100;120"

        echo ""
        echo "✓ Latest build complete!"
        ls -lh "${OUTPUT_DIR}/latest/"
        ;;

    custom)
        if [ -z "${2:-}" ]; then
            echo "Error: custom build requires architecture string"
            echo "Usage: ./docker-build.sh custom \"52;70;86\""
            exit 1
        fi

        echo "Building CUSTOM package (sm_${2})"
        mkdir -p "${OUTPUT_DIR}/custom"
        docker run --rm --gpus all \
            -v "${SCRIPT_DIR}/build-onnxruntime.sh:/workspace/build-onnxruntime.sh:ro" \
            -v "${OUTPUT_DIR}/custom:/output" \
            "${IMAGE_NAME}" \
            /workspace/build-onnxruntime.sh "${2}"

        echo ""
        echo "✓ Custom build complete!"
        ls -lh "${OUTPUT_DIR}/custom/"
        ;;

    *)
        echo "ONNX Runtime Multi-Architecture Docker Builder"
        echo ""
        echo "Usage:"
        echo "  ./docker-build.sh build-image         # Build CUDA 12.9 image (run once)"
        echo "  ./docker-build.sh build-image-cuda11  # Build CUDA 11.8 image (Maxwell support)"
        echo "  ./docker-build.sh test                # Test with sm_90 (~45 min)"
        echo "  ./docker-build.sh legacy              # Build sm_50-70 with CUDA 11.8 (~90 min)"
        echo "  ./docker-build.sh modern              # Build sm_75-86 with CUDA 12.9 (~50 min)"
        echo "  ./docker-build.sh latest              # Build sm_89-120 with CUDA 12.9 (~50 min)"
        echo "  ./docker-build.sh custom \"...\"       # Custom architectures"
        echo ""
        echo "Maxwell GPU Support (sm_50-52):"
        echo "  1. Download cuDNN 8.9.7 (see CUDNN_DOWNLOAD.md)"
        echo "  2. ./docker-build.sh build-image-cuda11"
        echo "  3. ./docker-build.sh legacy"
        echo ""
        echo "Outputs go to: ${OUTPUT_DIR}/"
        ;;
esac
