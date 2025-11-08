#!/usr/bin/env python3
"""
Convert NVIDIA Parakeet-TDT-1.1B from .nemo format to ONNX
This is the PROPER way to get the 1.1B model working with ONNX Runtime
"""

import os
import sys
import argparse
from pathlib import Path

def check_dependencies():
    """Check if NeMo is installed"""
    try:
        import nemo
        import nemo.collections.asr as nemo_asr
        return True
    except ImportError:
        print("ERROR: NeMo is not installed!")
        print("Please install it with:")
        print("  pip install nemo_toolkit[all]")
        print("Or for minimal installation:")
        print("  pip install nemo_toolkit[asr]")
        return False

def download_nemo_model(output_dir: Path):
    """Download the Parakeet-TDT-1.1B .nemo file from HuggingFace"""
    import urllib.request

    model_url = "https://huggingface.co/nvidia/parakeet-tdt-1.1b/resolve/main/parakeet-tdt-1.1b.nemo"
    nemo_path = output_dir / "parakeet-tdt-1.1b.nemo"

    if nemo_path.exists():
        print(f"Model already exists at {nemo_path}")
        return nemo_path

    print(f"Downloading Parakeet-TDT-1.1B model (4.28 GB)...")
    print(f"From: {model_url}")
    print(f"To: {nemo_path}")

    try:
        def download_progress(block_num, block_size, total_size):
            downloaded = block_num * block_size
            percent = min(downloaded * 100 / total_size, 100)
            mb_downloaded = downloaded / (1024 * 1024)
            mb_total = total_size / (1024 * 1024)
            sys.stdout.write(f'\rDownloading: {percent:.1f}% ({mb_downloaded:.1f}/{mb_total:.1f} MB)')
            sys.stdout.flush()

        urllib.request.urlretrieve(model_url, nemo_path, download_progress)
        print("\nDownload complete!")
        return nemo_path
    except Exception as e:
        print(f"\nERROR downloading model: {e}")
        if nemo_path.exists():
            nemo_path.unlink()  # Remove partial download
        sys.exit(1)

def convert_to_onnx(nemo_path: Path, output_dir: Path, use_gpu: bool = True):
    """Convert the .nemo model to ONNX format"""
    import nemo.collections.asr as nemo_asr

    print(f"\nLoading model from {nemo_path}...")

    # Load the model
    model = nemo_asr.models.EncDecRNNTBPEModel.restore_from(str(nemo_path))

    # Prepare for export
    print("Preparing model for export...")
    model.eval()
    model.freeze()  # Freeze weights for inference

    # Move to appropriate device
    device = "cuda" if use_gpu else "cpu"
    if use_gpu:
        try:
            import torch
            if not torch.cuda.is_available():
                print("WARNING: CUDA not available, falling back to CPU export")
                device = "cpu"
        except:
            print("WARNING: PyTorch CUDA not available, using CPU export")
            device = "cpu"

    print(f"Using device: {device}")
    model.to(device)

    # For TDT models, we may need to set specific export configuration
    # TDT models export as RNNT by default (encoder + decoder_joint)
    print("\nConfiguring export for TDT model...")

    # Export to ONNX
    onnx_path = output_dir / "parakeet-tdt-1.1b.onnx"
    print(f"Exporting to {onnx_path}...")

    try:
        # This will create encoder-model.onnx and decoder_joint-model.onnx
        model.export(str(onnx_path))
        print("Export successful!")

        # List the generated files
        print("\nGenerated ONNX files:")
        for f in output_dir.glob("*.onnx*"):
            size_mb = f.stat().st_size / (1024 * 1024)
            print(f"  {f.name}: {size_mb:.1f} MB")

        # Also export vocabulary
        if hasattr(model, 'tokenizer'):
            vocab_path = output_dir / "vocab.txt"
            print(f"\nExtracting vocabulary to {vocab_path}...")
            # Get vocabulary from tokenizer
            vocab = model.tokenizer.vocab
            with open(vocab_path, 'w', encoding='utf-8') as f:
                for token in sorted(vocab.keys(), key=lambda x: vocab[x]):
                    f.write(f"{token}\n")
            print(f"Vocabulary saved: {len(vocab)} tokens")

        return True

    except Exception as e:
        print(f"\nERROR during export: {e}")
        print("\nTroubleshooting:")
        print("1. Ensure you have the latest NeMo version")
        print("2. Try with use_gpu=False if GPU export fails")
        print("3. Check that you have enough disk space")
        return False

def main():
    parser = argparse.ArgumentParser(
        description="Convert NVIDIA Parakeet-TDT-1.1B to ONNX format"
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="/opt/swictation/models/parakeet-tdt-1.1b-onnx",
        help="Directory to save the ONNX model"
    )
    parser.add_argument(
        "--no-gpu",
        action="store_true",
        help="Export using CPU instead of GPU"
    )
    parser.add_argument(
        "--skip-download",
        action="store_true",
        help="Skip downloading if .nemo file already exists"
    )

    args = parser.parse_args()

    # Check dependencies first
    if not check_dependencies():
        sys.exit(1)

    # Create output directory
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Output directory: {output_dir}")

    # Download the .nemo model
    if not args.skip_download:
        nemo_path = download_nemo_model(output_dir)
    else:
        nemo_path = output_dir / "parakeet-tdt-1.1b.nemo"
        if not nemo_path.exists():
            print(f"ERROR: Model not found at {nemo_path}")
            print("Run without --skip-download to download it")
            sys.exit(1)

    # Convert to ONNX
    success = convert_to_onnx(nemo_path, output_dir, use_gpu=not args.no_gpu)

    if success:
        print("\n✅ Conversion complete!")
        print(f"ONNX models saved to: {output_dir}")
        print("\nNext steps:")
        print("1. Update config to use this model path")
        print("2. Test with swictation-daemon")
    else:
        print("\n❌ Conversion failed!")
        sys.exit(1)

if __name__ == "__main__":
    main()