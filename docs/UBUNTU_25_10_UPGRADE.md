# Ubuntu 25.10 Upgrade Guide

## Issue Summary

After upgrading to Ubuntu 25.10, swictation service failed to start with CUDA library incompatibility errors:

```
ImportError: /usr/local/lib/python3.13/dist-packages/torch/lib/../../nvidia/cusparse/lib/libcusparse.so.12:
undefined symbol: __nvJitLinkCreate_12_8, version libnvJitLink.so.12
```

## Root Cause

Ubuntu 25.10 ships with:
- NVIDIA Driver 580.x with CUDA 13.0
- The system had PyTorch 2.9.0 built for CUDA 12.x
- CUDA library version mismatch between PyTorch bundled libraries and system CUDA

## Solution

### 1. Reinstall PyTorch with CUDA 13.0 Support

```bash
sudo python3 -m pip uninstall --break-system-packages -y torch torchaudio triton \
    nvidia-cublas-cu12 nvidia-cuda-cupti-cu12 nvidia-cuda-nvrtc-cu12 \
    nvidia-cuda-runtime-cu12 nvidia-cudnn-cu12 nvidia-cufft-cu12 \
    nvidia-cufile-cu12 nvidia-curand-cu12 nvidia-cusolver-cu12 \
    nvidia-cusparse-cu12 nvidia-cusparselt-cu12 nvidia-nccl-cu12 \
    nvidia-nvjitlink-cu12 nvidia-nvshmem-cu12 nvidia-nvtx-cu12

sudo python3 -m pip install --break-system-packages torch torchaudio \
    --index-url https://download.pytorch.org/whl/cu130
```

### 2. Install Missing NeMo Dependencies

Several dependencies became required with NeMo 2.5.2 but were not in requirements.txt:

```bash
sudo python3 -m pip install --break-system-packages \
    lightning \
    fiddle \
    nv_one_logger_core \
    nv_one_logger_training_telemetry \
    nv_one_logger_pytorch_lightning_integration \
    braceexpand \
    webdataset \
    jiwer \
    editdistance
```

### 3. Install nemo_toolkit with [asr] Extra

This ensures all ASR dependencies are installed:

```bash
sudo python3 -m pip install --break-system-packages 'nemo_toolkit[asr]==2.5.2'
```

### 4. Upgrade ml_dtypes for ONNX Compatibility

```bash
sudo python3 -m pip install --break-system-packages 'ml_dtypes>=0.5.0' --upgrade
```

## Verification

After the fixes, verify PyTorch CUDA support:

```bash
python3 -c "import torch; print(f'PyTorch: {torch.__version__}'); \
    print(f'CUDA: {torch.version.cuda}'); \
    print(f'Available: {torch.cuda.is_available()}')"
```

Expected output:
```
PyTorch: 2.9.0+cu130
CUDA: 13.0
Available: True
```

Start the service:

```bash
systemctl --user restart swictation.service
systemctl --user status swictation.service
```

## Updated Files

The following files were updated to reflect these changes:

1. **requirements.txt**
   - Updated installation instructions for CUDA 13.0
   - Changed `nemo_toolkit==2.5.2` to `nemo_toolkit[asr]==2.5.2`
   - Added missing dependencies: fiddle, lightning, nv_one_logger packages, braceexpand, webdataset, jiwer, editdistance
   - Updated fsspec version constraint
   - Documented torch/torchaudio must be installed separately with --index-url

2. **scripts/install.sh**
   - Updated CUDA 13.0 detection (driver version >= 555)
   - Added Ubuntu 25.10 notes
   - Removed torchvision (not needed by swictation)

## Key Lessons

1. **PyTorch CUDA Version Must Match System CUDA**: Always install PyTorch with the `--index-url` flag matching your CUDA version
2. **Driver Version Matters**: Ubuntu 25.10+ uses driver 580.x which requires CUDA 13.0 support
3. **NeMo Has Hidden Dependencies**: Using `nemo_toolkit[asr]` extra ensures all ASR dependencies are installed
4. **Library Compatibility**: ml_dtypes, fiddle, and other packages need specific versions for Python 3.13 + CUDA 13.0

## CUDA Version Reference

| Ubuntu Version | NVIDIA Driver | CUDA Version | PyTorch Index URL |
|----------------|---------------|--------------|-------------------|
| 25.10+         | 580.x+        | 13.0         | cu130             |
| 24.10          | 560.x+        | 12.9         | cu129             |
| 24.04 LTS      | 535.x+        | 12.4-12.9    | cu124 or cu129    |
| 22.04 LTS      | 520.x+        | 11.8-12.x    | cu118 or cu121    |

## Testing

System tested and verified working:
- Ubuntu 25.10
- NVIDIA Driver 580.95.05
- CUDA 13.0
- GPU: NVIDIA RTX PRO 6000 Blackwell Workstation Edition
- Python 3.13.7
- PyTorch 2.9.0+cu130
- NeMo 2.5.2

Service status: ✅ Active and running
