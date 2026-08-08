#!/usr/bin/env python3.12
"""
Fix metadata on existing 1.1B ONNX models.

CRITICAL FIXES:
1. vocab_size: 1025 → 1024 (should NOT include blank token!)
2. feat_dim: 128 → 80 (1.1B uses 80 mel features)
"""

import onnx
from pathlib import Path

def update_metadata(model_path: str, updates: dict):
    """Update metadata in ONNX model."""
    print(f"\nUpdating {model_path}...")
    model = onnx.load(model_path)

    # Show current metadata
    print("  Current metadata:")
    for prop in sorted(model.metadata_props, key=lambda x: x.key):
        print(f"    {prop.key}: {prop.value}")

    # Update metadata
    for prop in model.metadata_props:
        if prop.key in updates:
            old_val = prop.value
            prop.value = str(updates[prop.key])
            print(f"  ✏️  {prop.key}: {old_val} → {prop.value}")

    # Save
    if "encoder" in model_path and "int8" in model_path:
        # INT8 encoder needs external data
        onnx.save(
            model,
            model_path,
            save_as_external_data=True,
            all_tensors_to_one_file=True,
            location="encoder.int8.weights",
        )
    elif "encoder" in model_path:
        # FP32 encoder needs external data
        onnx.save(
            model,
            model_path,
            save_as_external_data=True,
            all_tensors_to_one_file=True,
            location="encoder.weights",
        )
    else:
        onnx.save(model, model_path)

    print(f"  ✅ Saved!")

def main():
    model_dir = Path("/opt/swictation/models/parakeet-tdt-1.1b")

    print("="*80)
    print("Fixing Parakeet-TDT 1.1B Metadata")
    print("="*80)
    print("\nCRITICAL FIXES:")
    print("  1. vocab_size: 1025 → 1024 (sherpa-onnx adds blank internally)")
    print("  2. feat_dim: 128 → 80 (1.1B uses 80 mel features)")

    # Metadata fixes
    fixes = {
        "vocab_size": 1024,  # Vocabulary only, excluding blank!
        "feat_dim": 80,      # 1.1B uses 80, not 128!
    }

    # Update both encoder versions
    for encoder_file in ["encoder.onnx", "encoder.int8.onnx"]:
        encoder_path = model_dir / encoder_file
        if encoder_path.exists():
            update_metadata(str(encoder_path), fixes)

    print("\n" + "="*80)
    print("✅ Metadata fixes applied!")
    print("="*80)
    print("\nNext: Test with sherpa-onnx validation script")

if __name__ == "__main__":
    main()
