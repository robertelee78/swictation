#!/usr/bin/env python3
"""
Export Parakeet-TDT 1.1B to ONNX with proper metadata.
Based on sherpa-onnx reference implementation.
"""

from pathlib import Path
from typing import Dict
import os

import nemo.collections.asr as nemo_asr
import onnx
import torch
from onnxruntime.quantization import QuantType, quantize_dynamic


def add_meta_data(filename: str, meta_data: Dict[str, str]):
    """Add meta data to an ONNX model. It is changed in-place.

    Args:
      filename:
        Filename of the ONNX model to be changed.
      meta_data:
        Key-value pairs.
    """
    model = onnx.load(filename)
    while len(model.metadata_props):
        model.metadata_props.pop()

    for key, value in meta_data.items():
        meta = model.metadata_props.add()
        meta.key = key
        meta.value = str(value)

    if filename == "encoder.onnx":
        external_filename = "encoder"
        onnx.save(
            model,
            filename,
            save_as_external_data=True,
            all_tensors_to_one_file=True,
            location=external_filename + ".weights",
        )
    else:
        onnx.save(model, filename)


@torch.no_grad()
def main():
    print("=" * 70)
    print("Exporting Parakeet-TDT 1.1B to ONNX")
    print("=" * 70)

    # Load the 1.1B model
    if Path("./parakeet-tdt-1.1b.nemo").is_file():
        print("\n📦 Loading from local .nemo file...")
        asr_model = nemo_asr.models.ASRModel.restore_from(
            restore_path="./parakeet-tdt-1.1b.nemo"
        )
    else:
        print("\n📦 Downloading from HuggingFace...")
        asr_model = nemo_asr.models.ASRModel.from_pretrained(
            model_name="nvidia/parakeet-tdt-1.1b"
        )

    asr_model.eval()

    # Extract vocabulary
    print("\n📝 Extracting vocabulary...")
    with open("./tokens.txt", "w", encoding="utf-8") as f:
        for i, s in enumerate(asr_model.joint.vocabulary):
            f.write(f"{s} {i}\n")
        f.write(f"<blk> {i+1}\n")
    print(f"   ✓ Saved {len(asr_model.joint.vocabulary)} tokens + <blk> to tokens.txt")

    # Export ONNX models
    print("\n🔧 Exporting ONNX models...")
    asr_model.encoder.export("encoder.onnx")
    print("   ✓ encoder.onnx")
    asr_model.decoder.export("decoder.onnx")
    print("   ✓ decoder.onnx")
    asr_model.joint.export("joiner.onnx")
    print("   ✓ joiner.onnx")

    os.system("ls -lh *.onnx")

    # CRITICAL: Read normalize_type from model config
    normalize_type = asr_model.cfg.preprocessor.normalize
    if normalize_type == "NA":
        normalize_type = ""

    print(f"\n📊 Model configuration:")
    print(f"   vocab_size: {asr_model.decoder.vocab_size}")
    print(f"   normalize_type: '{normalize_type}'")
    print(f"   pred_rnn_layers: {asr_model.decoder.pred_rnn_layers}")
    print(f"   pred_hidden: {asr_model.decoder.pred_hidden}")

    # CRITICAL: Add comprehensive metadata
    meta_data = {
        "vocab_size": asr_model.decoder.vocab_size,
        "normalize_type": normalize_type,
        "pred_rnn_layers": asr_model.decoder.pred_rnn_layers,
        "pred_hidden": asr_model.decoder.pred_hidden,
        "subsampling_factor": 8,
        "model_type": "EncDecRNNTBPEModel",
        "version": "2",
        "model_author": "NeMo",
        "url": "https://huggingface.co/nvidia/parakeet-tdt-1.1b",
        "comment": "Only the transducer branch is exported. 1.1B parameter model.",
        "feat_dim": 128,  # CRITICAL: 1.1B uses 128 features
    }

    # Quantize to INT8
    print("\n⚙️  Quantizing to INT8...")
    for m in ["encoder", "decoder", "joiner"]:
        quantize_dynamic(
            model_input=f"./{m}.onnx",
            model_output=f"./{m}.int8.onnx",
            weight_type=QuantType.QUInt8 if m == "encoder" else QuantType.QInt8,
        )
        print(f"   ✓ {m}.int8.onnx")

    os.system("ls -lh *.onnx")

    # Add metadata to both fp32 and int8 models
    print("\n📋 Adding metadata...")
    add_meta_data("encoder.int8.onnx", meta_data)
    add_meta_data("encoder.onnx", meta_data)
    print("   ✓ Metadata added to encoder models")

    print("\n" + "=" * 70)
    print("✅ Export complete!")
    print("=" * 70)
    print("\nMetadata added:")
    for key, value in meta_data.items():
        print(f"  {key}: {value}")

    print("\n📦 Output files:")
    print("  - encoder.onnx (fp32)")
    print("  - encoder.int8.onnx")
    print("  - decoder.onnx (fp32)")
    print("  - decoder.int8.onnx")
    print("  - joiner.onnx (fp32)")
    print("  - joiner.int8.onnx")
    print("  - tokens.txt")


if __name__ == "__main__":
    main()
