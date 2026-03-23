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
    """Convert an ONNX model to FP16 (mixed precision)."""
    try:
        import onnx
        from onnxconverter_common import float16
    except ImportError:
        print("ERROR: Required packages not installed.")
        print("Run: pip install onnx onnxconverter-common")
        sys.exit(1)

    print(f"Converting {input_path} -> {output_path}")
    model = onnx.load(input_path)

    # Convert to FP16 with mixed precision (keeps some ops in FP32 for stability)
    model_fp16 = float16.convert_float_to_float16(
        model,
        min_positive_val=1e-7,
        max_finite_val=1e4,
        keep_io_types=True,  # Keep input/output as FP32 for compatibility
        disable_shape_infer=False,
        op_block_list=['Softmax', 'LayerNormalization'],  # Keep these in FP32 for numerical stability
    )

    onnx.save(model_fp16, output_path)

    # Report size reduction
    orig_size = os.path.getsize(input_path) / (1024 * 1024)
    new_size = os.path.getsize(output_path) / (1024 * 1024)
    reduction = (1 - new_size / orig_size) * 100
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
            # Find the source file (could be .onnx or .int8.onnx)
            source = os.path.join(model_dir, f'{component}.onnx')
            if not os.path.exists(source):
                source = os.path.join(model_dir, f'{component}.int8.onnx')
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
