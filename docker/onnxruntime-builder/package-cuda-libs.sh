#!/bin/bash
# Package CUDA runtime libraries with ONNX Runtime builds
#
# This script collects all required CUDA runtime libraries and creates
# distributable tar.gz archives for each architecture package.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/output"
IMAGE_NAME="onnxruntime-builder:cuda12.9"

echo "================================================"
echo "CUDA Runtime Library Packaging Script"
echo "================================================"

# Function to package a specific variant
package_variant() {
    local VARIANT=$1
    local VARIANT_DIR="${OUTPUT_DIR}/${VARIANT}"

    echo ""
    echo "Packaging ${VARIANT} variant..."

    if [ ! -d "${VARIANT_DIR}" ]; then
        echo "Error: ${VARIANT_DIR} does not exist"
        return 1
    fi

    # Create libs subdirectory
    mkdir -p "${VARIANT_DIR}/libs"

    # Extract CUDA runtime libraries from container
    echo "Extracting CUDA runtime libraries..."
    docker run --rm \
        -v "${VARIANT_DIR}:/output" \
        "${IMAGE_NAME}" \
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
package_variant "legacy"
package_variant "modern"
package_variant "latest"

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
