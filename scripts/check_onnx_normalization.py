#!/usr/bin/env python3
"""
Check ONNX model metadata for normalization parameters.

This script examines whether the exported Parakeet-TDT model contains
the fixed mean/std values that were used during training.
"""

import sys
import onnxruntime as ort
import numpy as np
from pathlib import Path

def check_model_metadata(model_path: str):
    """Check ONNX model for normalization parameters."""
    print(f"📦 Loading ONNX model: {model_path}")

    try:
        session = ort.InferenceSession(str(model_path), providers=['CPUExecutionProvider'])
        metadata = session.get_modelmeta()
    except Exception as e:
        print(f"❌ Failed to load model: {e}")
        return

    print(f"✓ Model loaded successfully\n")

    # Check metadata properties
    custom_meta = metadata.custom_metadata_map
    print(f"📊 Found {len(custom_meta)} custom metadata properties:\n")

    # Store all metadata
    normalization_found = False

    for key, value in custom_meta.items():
        # Check for normalization-related keys
        key_lower = key.lower()
        if any(keyword in key_lower for keyword in ['mean', 'std', 'normalize', 'norm', 'fbank']):
            print(f"🔍 NORMALIZATION RELATED:")
            print(f"   Key: {key}")

            # Try to parse as array if it looks like one
            if value.startswith('[') or ',' in value:
                try:
                    # Try to parse as numpy array
                    if value.startswith('['):
                        arr = eval(value)
                    else:
                        arr = [float(x.strip()) for x in value.split(',') if x.strip()]

                    print(f"   Type: Array")
                    print(f"   Length: {len(arr)}")
                    print(f"   First 5 values: {arr[:5]}")
                    print(f"   Last 5 values: {arr[-5:]}")
                    print(f"   Min: {min(arr):.4f}, Max: {max(arr):.4f}, Mean: {np.mean(arr):.4f}")
                except:
                    print(f"   Value: {value[:100]}..." if len(value) > 100 else value)
            else:
                print(f"   Value: {value}")

            print()
            normalization_found = True
        else:
            # Print non-normalization metadata
            print(f"   {key}: {value[:80]}..." if len(value) > 80 else f"   {key}: {value}")

    print("\n" + "="*60)

    if normalization_found:
        print("✅ NORMALIZATION PARAMETERS FOUND!")
        print("\nNext steps:")
        print("1. Extract the mean/std arrays from metadata")
        print("2. Store them in RecognizerOrt struct")
        print("3. Use them in audio.rs instead of computing from input")
    else:
        print("❌ NO normalization parameters found in metadata")
        print("\nPossible reasons:")
        print("1. Model was exported without normalization stats")
        print("2. Need to re-export using sherpa-onnx export script")
        print("3. Stats might be in a different model file")

    print("="*60)

if __name__ == "__main__":
    # Check 1.1B model
    model_dir = Path("/opt/swictation/models/parakeet-tdt-1.1b-exported")

    if not model_dir.exists():
        print(f"❌ Model directory not found: {model_dir}")
        sys.exit(1)

    print("🔍 Checking Parakeet-TDT 1.1B Model for Normalization Parameters\n")
    print("="*60 + "\n")

    # Check encoder
    encoder_path = model_dir / "encoder.int8.onnx"
    if encoder_path.exists():
        print("📋 ENCODER MODEL:")
        print("-" * 60)
        check_model_metadata(str(encoder_path))
        print()

    # Check decoder
    decoder_path = model_dir / "decoder.onnx"
    if decoder_path.exists():
        print("\n📋 DECODER MODEL:")
        print("-" * 60)
        check_model_metadata(str(decoder_path))
        print()

    # Check joiner
    joiner_path = model_dir / "joiner.int8.onnx"
    if joiner_path.exists():
        print("\n📋 JOINER MODEL:")
        print("-" * 60)
        check_model_metadata(str(joiner_path))
