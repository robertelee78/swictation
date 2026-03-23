#!/usr/bin/env python3
"""Convert Swictation ONNX models to FP16 for Apple Silicon CoreML optimization.

Usage:
    python3 scripts/convert-to-fp16.py [--model-dir PATH]

Requirements:
    pip install onnx onnxruntime onnxconverter-common

This converts encoder.onnx, decoder.onnx, joiner.onnx to their .fp16.onnx variants.
The Rust model loader automatically prefers FP16 on macOS.
"""
import argparse
import os
import sys


def convert_to_fp16(input_path, output_path):
    """Convert an ONNX model to FP16 (mixed precision).

    Handles both self-contained models and models with external weight files.
    """
    try:
        import onnx
        from onnx.external_data_helper import convert_model_to_external_data
        from onnxconverter_common import float16
    except ImportError:
        print("ERROR: Required packages not installed.")
        print("Run: pip install onnx onnxconverter-common")
        sys.exit(1)

    print(f"Converting {input_path} -> {output_path}")
    model_dir = os.path.dirname(input_path)

    # Check if model has external weight files (.weights or .data)
    base_name = os.path.splitext(os.path.basename(input_path))[0]
    has_external = any(
        os.path.exists(os.path.join(model_dir, f"{base_name}.{ext}"))
        for ext in ['weights', 'data']
    )

    if has_external:
        # Load model with external data resolved from the model directory
        print(f"  Loading with external data from {model_dir}")
        model = onnx.load(input_path, load_external_data=True)
    else:
        model = onnx.load(input_path)

    # Convert to FP16 with mixed precision (keeps some ops in FP32 for stability)
    model_fp16 = float16.convert_float_to_float16(
        model,
        min_positive_val=1e-7,
        max_finite_val=1e4,
        keep_io_types=True,  # Keep input/output as FP32 for compatibility
        disable_shape_infer=True,  # Skip shape inference for large models
        op_block_list=['Softmax', 'LayerNormalization'],  # Keep in FP32 for numerical stability
    )

    if has_external:
        # Save with external data to avoid protobuf 2GB limit
        fp16_weights = output_path.replace('.fp16.onnx', '.fp16.weights')
        print(f"  Saving with external weights: {os.path.basename(fp16_weights)}")
        convert_model_to_external_data(
            model_fp16,
            all_tensors_to_one_file=True,
            location=os.path.basename(fp16_weights),
            size_threshold=1024,
        )
        onnx.save(model_fp16, output_path)
    else:
        onnx.save(model_fp16, output_path)

    # Report size
    orig_size = os.path.getsize(input_path) / (1024 * 1024)
    new_size = os.path.getsize(output_path) / (1024 * 1024)
    if has_external:
        orig_weights = input_path.replace('.onnx', '.weights')
        if os.path.exists(orig_weights):
            orig_size += os.path.getsize(orig_weights) / (1024 * 1024)
        fp16_weights_path = output_path.replace('.fp16.onnx', '.fp16.weights')
        if os.path.exists(fp16_weights_path):
            new_size += os.path.getsize(fp16_weights_path) / (1024 * 1024)
    reduction = (1 - new_size / orig_size) * 100 if orig_size > 0 else 0
    print(f"  {orig_size:.1f}MB -> {new_size:.1f}MB ({reduction:.1f}% smaller)")


def main():
    parser = argparse.ArgumentParser(description='Convert Swictation ONNX models to FP16')
    parser.add_argument('--model-dir', default=os.path.expanduser('~/.local/share/swictation/models'),
                        help='Model directory (default: ~/.local/share/swictation/models)')
    args = parser.parse_args()

    # Model directories to convert
    model_dirs = [
        'parakeet-tdt-0.6b-v3-onnx',
        'parakeet-tdt-1.1b-onnx',
    ]

    components = ['encoder', 'decoder', 'joiner']

    converted_count = 0

    for model_dir_name in model_dirs:
        model_dir = os.path.join(args.model_dir, model_dir_name)
        if not os.path.exists(model_dir):
            print(f"Skipping {model_dir_name} (not found at {model_dir})")
            continue

        print(f"\n=== Converting {model_dir_name} ===")
        for component in components:
            # Find the source file — ALWAYS prefer FP32 over INT8
            # INT8 → FP16 conversion produces broken type graphs
            source = os.path.join(model_dir, f'{component}.onnx')
            if not os.path.exists(source):
                print(f"  Skipping {component} (not found)")
                continue

            output = os.path.join(model_dir, f'{component}.fp16.onnx')
            if os.path.exists(output):
                print(f"  {component}.fp16.onnx already exists, skipping")
                continue

            convert_to_fp16(source, output)
            converted_count += 1

    print(f"\nConversion complete! Converted {converted_count} model(s).")
    print("The Rust model loader automatically prefers .fp16.onnx files on macOS.")


if __name__ == '__main__':
    main()
