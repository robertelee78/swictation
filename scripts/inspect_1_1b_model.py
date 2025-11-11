#!/usr/bin/env python3.12
"""
Inspect Parakeet-TDT 1.1B model configuration to understand vocab size mismatch.

The joiner outputs 1030 dimensions but vocabulary has 1025 tokens.
Need to understand why there's a +5 difference.
"""

import nemo.collections.asr as nemo_asr
import torch

def main():
    print("="*80)
    print("Inspecting Parakeet-TDT 1.1B Model Configuration")
    print("="*80)

    # Load model on CPU
    print("\nLoading model...")
    asr_model = nemo_asr.models.ASRModel.from_pretrained(
        model_name="nvidia/parakeet-tdt-1.1b",
        map_location=torch.device('cpu')
    )

    # Vocabulary info
    print("\n" + "="*80)
    print("VOCABULARY INFO")
    print("="*80)
    vocab = asr_model.joint.vocabulary
    print(f"Vocabulary size: {len(vocab)}")
    print(f"First 10 tokens: {vocab[:10]}")
    print(f"Last 10 tokens: {vocab[-10:]}")

    # Check if there's a tokenizer with more info
    if hasattr(asr_model, 'tokenizer'):
        print(f"\nTokenizer vocab size: {asr_model.tokenizer.vocab_size}")

    # Joint/Joiner configuration
    print("\n" + "="*80)
    print("JOINT/JOINER CONFIGURATION")
    print("="*80)
    print(f"Joint type: {type(asr_model.joint).__name__}")
    print(f"Joint num_classes: {asr_model.joint.num_classes if hasattr(asr_model.joint, 'num_classes') else 'N/A'}")
    print(f"Joint vocab_size: {asr_model.joint.vocab_size if hasattr(asr_model.joint, 'vocab_size') else 'N/A'}")

    # Check for TDT-specific attributes
    if hasattr(asr_model.joint, 'num_extra_outputs'):
        print(f"Joint num_extra_outputs: {asr_model.joint.num_extra_outputs}")
    if hasattr(asr_model.joint, 'duration_prediction'):
        print(f"Joint duration_prediction: {asr_model.joint.duration_prediction}")

    # Decoder configuration
    print("\n" + "="*80)
    print("DECODER CONFIGURATION")
    print("="*80)
    print(f"Decoder type: {type(asr_model.decoder).__name__}")
    print(f"Decoder vocab_size: {asr_model.decoder.vocab_size if hasattr(asr_model.decoder, 'vocab_size') else 'N/A'}")
    print(f"Decoder blank_id: {asr_model.decoder.blank_id if hasattr(asr_model.decoder, 'blank_id') else 'N/A'}")

    # Full model config
    print("\n" + "="*80)
    print("MODEL CONFIG (relevant fields)")
    print("="*80)
    if hasattr(asr_model, 'cfg'):
        cfg = asr_model.cfg
        if hasattr(cfg, 'joint') or hasattr(cfg, 'decoder'):
            print("Joint config:")
            if hasattr(cfg, 'joint'):
                for key in dir(cfg.joint):
                    if not key.startswith('_') and 'vocab' in key.lower() or 'num' in key.lower():
                        print(f"  joint.{key}: {getattr(cfg.joint, key, 'N/A')}")
            print("\nDecoder config:")
            if hasattr(cfg, 'decoder'):
                for key in dir(cfg.decoder):
                    if not key.startswith('_') and 'vocab' in key.lower() or 'num' in key.lower():
                        print(f"  decoder.{key}: {getattr(cfg.decoder, key, 'N/A')}")

    # Test forward pass to see actual output dimensions
    print("\n" + "="*80)
    print("TESTING JOINER OUTPUT DIMENSIONS")
    print("="*80)
    print("Creating dummy inputs to test joiner...")

    # Create dummy encoder output (batch=1, time=10, dim=1024)
    encoder_out = torch.randn(1, 10, 1024)

    # Create dummy decoder output (batch=1, dim=640)
    decoder_out = torch.randn(1, 1, 640)

    # Run joiner
    with torch.no_grad():
        joiner_out = asr_model.joint(encoder_out, decoder_out)

    print(f"Joiner output shape: {joiner_out.shape}")
    print(f"Expected vocab shape: (batch, time, {len(vocab) + 1}) = (batch, time, 1025)")
    print(f"Actual output dim: {joiner_out.shape[-1]}")
    print(f"Difference: {joiner_out.shape[-1] - (len(vocab) + 1)} extra dimensions")

    if joiner_out.shape[-1] != len(vocab) + 1:
        print(f"\n⚠️  MISMATCH CONFIRMED!")
        print(f"   Joiner outputs {joiner_out.shape[-1]} dimensions")
        print(f"   Vocabulary has {len(vocab)} tokens + 1 blank = {len(vocab) + 1} total")
        print(f"   Extra dimensions: {joiner_out.shape[-1] - (len(vocab) + 1)}")
        print(f"\n   This is likely TDT-specific (duration tokens or special handling)")

    print("\n" + "="*80)
    print("CONCLUSION")
    print("="*80)
    print("The joiner output dimension mismatch needs to be handled in the export.")
    print("Possible solutions:")
    print("  1. Export only the first 1025 dimensions of joiner output")
    print("  2. Add the extra tokens to tokens.txt")
    print("  3. Use a different export method that handles TDT correctly")

if __name__ == "__main__":
    main()
