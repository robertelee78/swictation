# cuDNN 8.9.7 Download Instructions for Maxwell GPU Support

## Why cuDNN 8.9.7?

Maxwell GPUs (sm_50-52) require cuDNN 8.9.7 with CUDA 11.8 for full RNN/LSTM support:
- **cuDNN 9.x**: Dropped Maxwell support (requires compute capability ≥6.0)
- **cuDNN 8.9.7**: Official NVIDIA recommendation for Maxwell GPUs
- **CUDA 11.8**: Last stable CUDA 11.x with full Maxwell support

## Download Steps

### 1. Login to NVIDIA Developer

Visit: https://developer.nvidia.com/rdp/cudnn-archive

**Required**: NVIDIA Developer account (free registration)

### 2. Find cuDNN v8.9.7

Scroll to **"Download cuDNN v8.9.7 (December 5th, 2023), for CUDA 11.x"**

### 3. Download the Correct File

Click on: **"Local Installer for Linux x86_64 (Tar)"**

**Direct link** (requires login):
```
https://developer.nvidia.com/downloads/compute/cudnn/secure/8.9.7/local_installers/11.x/cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz
```

**File details**:
- Name: `cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz`
- Size: ~600 MB
- SHA256: (verify on NVIDIA website)

### 4. Place in Build Directory

```bash
# Move downloaded file to onnxruntime-builder directory
mv ~/Downloads/cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz \
   /opt/swictation/docker/onnxruntime-builder/

# Verify file exists
ls -lh /opt/swictation/docker/onnxruntime-builder/cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz
```

### 5. Build Docker Image

```bash
cd /opt/swictation/docker/onnxruntime-builder
./docker-build.sh build-image-cuda11
```

## Alternative: Manual Download via wget (if you have NVIDIA API key)

If you have NVIDIA NGC API key:

```bash
# This won't work without proper authentication
# NVIDIA requires interactive login for cuDNN downloads
```

## Verification

After Docker image builds successfully:

```bash
docker run --rm onnxruntime-builder:cuda11.8 \
  bash -c "strings /usr/local/cuda/lib64/libcudnn.so.8 | grep '8.9.7'"

# Expected output: 8.9.7
```

## Troubleshooting

**Error: "cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz not found"**
- Download the file from NVIDIA Developer (step 1-3)
- Place it in `/opt/swictation/docker/onnxruntime-builder/`
- Ensure exact filename matches

**Error: "Permission denied"**
```bash
chmod 644 cudnn-linux-x86_64-8.9.7.29_cuda11-archive.tar.xz
```

**Download fails / Access denied**
- Login to NVIDIA Developer account
- Accept cuDNN license agreement
- Try download again

## Why Manual Download?

NVIDIA requires:
1. Developer account registration
2. License agreement acceptance
3. Interactive authentication

Automated downloads are not supported for cuDNN archives.

## References

- [NVIDIA cuDNN Archive](https://developer.nvidia.com/rdp/cudnn-archive)
- [cuDNN 8.9.7 Documentation](https://docs.nvidia.com/deeplearning/cudnn/archives/cudnn-897/index.html)
- [cuDNN 8.9.7 Installation Guide](https://docs.nvidia.com/deeplearning/cudnn/archives/cudnn-897/install-guide/index.html)
