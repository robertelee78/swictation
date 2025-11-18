#!/bin/bash
# Package CUDA runtime libraries with ONNX Runtime builds
#
# This script collects all required CUDA runtime libraries and creates
# distributable tar.gz archives for each architecture package.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/output"
IMAGE_NAME="onnxruntime-builder:cuda12.9"
IMAGE_NAME_CUDA11="onnxruntime-builder:cuda11.8"

echo "================================================"
echo "CUDA Runtime Library Packaging Script"
echo "================================================"

# Function to package a specific variant
package_variant() {
    local VARIANT=$1
    local VARIANT_DIR="${OUTPUT_DIR}/${VARIANT}"
    local USE_IMAGE="${2:-${IMAGE_NAME}}"

    echo ""
    echo "Packaging ${VARIANT} variant..."
    echo "Using image: ${USE_IMAGE}"

    if [ ! -d "${VARIANT_DIR}" ]; then
        echo "Error: ${VARIANT_DIR} does not exist"
        return 1
    fi

    # Create libs subdirectory
    mkdir -p "${VARIANT_DIR}/libs"

    # Determine CUDA version from image name
    if [[ "${USE_IMAGE}" == *"cuda11"* ]]; then
        # CUDA 11.8 + cuDNN 8.9.7 libraries
        echo "Extracting CUDA 11.8 runtime libraries..."
        docker run --rm \
            -v "${VARIANT_DIR}:/output" \
            "${USE_IMAGE}" \
            bash -c '
                # Copy CUDA 11.8 runtime libraries (follow symlinks with -L)
                echo "Copying CUDA 11.8 runtime libraries..."
                cp -L /usr/local/cuda/lib64/libcublas.so.11 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcublasLt.so.11 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcudart.so.11.0 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcufft.so.10 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcurand.so.10 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libnvrtc.so.11.2 /output/libs/ 2>/dev/null || true

                # Copy cuDNN 8.9.7 libraries (follow symlinks)
                echo "Copying cuDNN 8.9.7 libraries..."
                cp -L /usr/local/cuda/lib64/libcudnn.so.8 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcudnn_*.so.8 /output/libs/ 2>/dev/null || true

                # Show what was copied
                echo "Libraries copied:"
                ls -lh /output/libs/
            '
    else
        # CUDA 12.9 + cuDNN 9.15.1 libraries
        echo "Extracting CUDA 12.9 runtime libraries..."
        docker run --rm \
            -v "${VARIANT_DIR}:/output" \
            "${USE_IMAGE}" \
            bash -c '
                # Copy CUDA runtime libraries (follow symlinks with -L)
                echo "Copying CUDA runtime libraries..."
                cp -L /usr/local/cuda/lib64/libcublas.so.12 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcublasLt.so.12 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcudart.so.12 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcufft.so.11 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libcurand.so.10 /output/libs/ 2>/dev/null || true
                cp -L /usr/local/cuda/lib64/libnvrtc.so.12 /output/libs/ 2>/dev/null || true

                # Copy cuDNN libraries (follow symlinks)
                echo "Copying cuDNN libraries..."
                cp -L /usr/lib/x86_64-linux-gnu/libcudnn.so.9 /output/libs/ 2>/dev/null || true
                cp -L /usr/lib/x86_64-linux-gnu/libcudnn_*.so.9 /output/libs/ 2>/dev/null || true

                # Show what was copied
                echo "Libraries copied:"
                ls -lh /output/libs/
            '
    fi

    # Move ONNX Runtime libraries to libs directory
    echo "Organizing ONNX Runtime libraries..."
    mv "${VARIANT_DIR}"/*.so "${VARIANT_DIR}/libs/" 2>/dev/null || true

    # Create archive
    echo "Creating tar.gz archive..."
    cd "${OUTPUT_DIR}"
    tar -czf "cuda-libs-${VARIANT}.tar.gz" "${VARIANT}/libs/"

    # Show archive size
    echo "Archive created:"
    ls -lh "cuda-libs-${VARIANT}.tar.gz"

    # Show total package size
    echo "Total package contents:"
    du -sh "${VARIANT}/libs/"
}

# Package all three variants
# Legacy uses CUDA 11.8 image, modern/latest use CUDA 12.9 image
package_variant "legacy" "${IMAGE_NAME_CUDA11}"
package_variant "modern" "${IMAGE_NAME}"
package_variant "latest" "${IMAGE_NAME}"

echo ""
echo "================================================"
echo "Packaging Complete!"
echo "================================================"
echo ""
echo "Created packages:"
ls -lh "${OUTPUT_DIR}"/cuda-libs-*.tar.gz
echo ""
echo "Package contents:"
echo "  - ONNX Runtime libraries (libonnxruntime*.so)"
echo "  - CUDA runtime libraries (libcublas, libcudart, etc.)"
echo "  - cuDNN libraries (libcudnn*.so)"
echo ""
echo "Ready for GitHub release upload!"
