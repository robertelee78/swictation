# GPU Library Download Failure - Root Cause Analysis

## Executive Summary

**Issue:** The GPU library download fails with HTTP 404 when attempting to download `cuda-libs-latest.tar.gz` for Blackwell RTX PRO 6000 (sm_120).

**Root Cause:** Missing GitHub release assets. Only `cuda-libs-legacy.tar.gz` exists in the v1.2.0 release, while the code expects three variants (`legacy`, `modern`, `latest`).

**Impact:** Blackwell GPU users (sm_89-120) cannot download GPU libraries automatically, falling back to CPU-only mode.

---

## System Information

### User's Hardware
- **GPU:** NVIDIA RTX PRO 6000 Blackwell Workstation Edition
- **VRAM:** 96GB (97887MB)
- **Driver:** 580.105.08
- **CUDA:** 13.0
- **Compute Capability:** 12.0 (sm_120)

### Detection Results
```
✓ Detected GPU: NVIDIA RTX PRO 6000 Blackwell Workstation Edition
  Compute Capability: 12.0 (sm_120)
```

---

## Root Cause Analysis

### 1. Variant Selection Logic (Lines 460-495)

The code correctly identifies Blackwell GPUs:

```javascript
function selectGPUPackageVariant(smVersion) {
  // smVersion = 120 for Blackwell RTX PRO 6000

  if (smVersion >= 89 && smVersion <= 121) {
    // ✅ CORRECTLY MATCHES sm_120
    return {
      variant: 'latest',  // ← This is returned
      architectures: 'sm_89-120',
      description: 'Ada Lovelace, Hopper, Blackwell GPUs (2022-2024)',
      examples: 'RTX 4090, H100, B100/B200, RTX PRO 6000 Blackwell, RTX 50 series'
    };
  }
}
```

**Result:** `variant = 'latest'` ✅ Correct

### 2. URL Construction (Line 638)

```javascript
const GPU_LIBS_VERSION = '1.2.0';
const variant = packageInfo.variant;  // 'latest'
const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${GPU_LIBS_VERSION}/cuda-libs-${variant}.tar.gz`;
// ↓
// https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz
```

**Result:** URL construction is correct ✅

### 3. GitHub Release Assets (The Problem)

```bash
$ curl -sL "https://api.github.com/repos/robertelee78/swictation/releases/tags/gpu-libs-v1.2.0" | jq -r '.assets[].name'
cuda-libs-legacy.tar.gz
```

**Available Assets:**
- ✅ `cuda-libs-legacy.tar.gz` (sm_50-70: Maxwell, Pascal, Volta)
- ❌ `cuda-libs-modern.tar.gz` (sm_75-86: Turing, Ampere) - **MISSING**
- ❌ `cuda-libs-latest.tar.gz` (sm_89-120: Ada, Hopper, Blackwell) - **MISSING**

**Expected Assets (per RELEASE_NOTES.md):**
```markdown
1. cuda-libs-legacy.tar.gz (sm_50-70) ✅ EXISTS
2. cuda-libs-modern.tar.gz (sm_75-86) ❌ MISSING
3. cuda-libs-latest.tar.gz (sm_89-120) ❌ MISSING
```

### 4. Download Failure Chain

```
User's GPU: sm_120 (Blackwell)
     ↓
selectGPUPackageVariant(120) → variant='latest'
     ↓
URL: https://github.com/.../cuda-libs-latest.tar.gz
     ↓
HTTP 404 - File Not Found
     ↓
downloadFile() writes "Not Found" text to tarball
     ↓
tar -xzf fails: "gzip: stdin: not in gzip format"
```

---

## Evidence

### Error Output
```
⚠️  Failed to download GPU libraries: Error: Command failed: tar -xzf "/tmp/swictation-gpu-install/cuda-libs-latest.tar.gz" -C "/tmp/swictation-gpu-install"

gzip: stdin: not in gzip format
tar: Child returned status 1
tar: Error is not recoverable: exiting now
```

### Downloaded File Content
```bash
$ file /tmp/swictation-gpu-install/cuda-libs-latest.tar.gz
/tmp/swictation-gpu-install/cuda-libs-latest.tar.gz: ASCII text, with no line terminators

$ cat /tmp/swictation-gpu-install/cuda-libs-latest.tar.gz
Not Found
```

### HTTP Response
```bash
$ curl -sL "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz" | head -c 200
Not Found
```

---

## Impact Analysis

### Affected GPU Architectures

| Architecture | Compute Cap | Variant | Status |
|--------------|-------------|---------|--------|
| Maxwell, Pascal, Volta | sm_50-70 | `legacy` | ✅ Working |
| Turing, Ampere | sm_75-86 | `modern` | ❌ 404 Error |
| Ada, Hopper, Blackwell | sm_89-120 | `latest` | ❌ 404 Error |

### User Impact
- **Blackwell GPUs** (RTX PRO 6000, RTX 50 series) - Cannot use GPU acceleration
- **Hopper GPUs** (H100, H200) - Cannot use GPU acceleration
- **Ada Lovelace GPUs** (RTX 4090, 4080, 4070) - Cannot use GPU acceleration
- **Ampere GPUs** (RTX 3090, 3080, A100, A6000) - Cannot use GPU acceleration
- **Turing GPUs** (RTX 2080 Ti, Quadro RTX) - Cannot use GPU acceleration

**Estimate:** 80%+ of modern NVIDIA GPUs are affected (everything sm_75+)

---

## Why This Happened

### Hypothesis: Incomplete Release Upload

The v1.2.0 release was likely uploaded incompletely:
1. ✅ Release tag created (`gpu-libs-v1.2.0`)
2. ✅ `cuda-libs-legacy.tar.gz` uploaded (1.5GB)
3. ❌ `cuda-libs-modern.tar.gz` **NOT uploaded** (1.5GB)
4. ❌ `cuda-libs-latest.tar.gz` **NOT uploaded** (1.5GB)

### Possible Reasons
- Build failures for modern/latest variants
- Upload interruption (network/timeout)
- Missing build artifacts
- Manual upload process incomplete

---

## Recommended Fix

### Option 1: Complete the v1.2.0 Release (Recommended)

**Build and upload missing packages:**

```bash
cd /opt/swictation/docker/onnxruntime-builder

# Build missing variants
./docker-build.sh modern  # sm_75-86 (Turing, Ampere)
./docker-build.sh latest  # sm_89-120 (Ada, Hopper, Blackwell)

# Package CUDA libraries
./package-cuda-libs.sh modern
./package-cuda-libs.sh latest

# Upload to GitHub release gpu-libs-v1.2.0
gh release upload gpu-libs-v1.2.0 \
  cuda-libs-modern.tar.gz \
  cuda-libs-latest.tar.gz
```

**Verification:**
```bash
# Test all three variants
curl -I "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-legacy.tar.gz"
curl -I "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-modern.tar.gz"
curl -I "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz"
```

### Option 2: Temporary Fallback (Quick Fix)

**Modify `postinstall.js` to use v1.1.0 for missing variants:**

```javascript
async function downloadGPULibraries() {
  // ... existing code ...

  const GPU_LIBS_VERSION = '1.2.0';
  const variant = packageInfo.variant;

  // TEMPORARY: Use v1.1.0 for modern/latest until v1.2.0 is complete
  const effectiveVersion = (variant === 'modern' || variant === 'latest') ? '1.1.0' : GPU_LIBS_VERSION;

  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${effectiveVersion}/cuda-libs-${variant}.tar.gz`;

  // ... rest of download logic ...
}
```

**Note:** This is a workaround. Option 1 is preferred.

### Option 3: Use Legacy Package for All GPUs (Not Recommended)

Force all GPUs to use `cuda-libs-legacy.tar.gz`:
- ❌ Wrong CUDA provider architectures
- ❌ No native Blackwell sm_120 support
- ❌ Degrades performance on modern GPUs

---

## What Should Happen Next

### Immediate (Fix the Release)
1. **Build missing packages** using Docker build system
2. **Upload to GitHub release** `gpu-libs-v1.2.0`
3. **Verify all three URLs** return HTTP 200

### Short-term (Improve Robustness)
1. **Add HTTP status check** in `downloadFile()` to detect 404 early
2. **Fallback to v1.1.0** if v1.2.0 asset is missing
3. **Better error messages** explaining the missing release

### Long-term (Prevention)
1. **Automated build CI/CD** for GPU library releases
2. **Pre-release verification** that all three variants exist
3. **Checksum validation** of downloaded files

---

## Code Analysis

### Detection Flow (✅ Working Correctly)

```javascript
// Step 1: Detect GPU compute capability
nvidia-smi --query-gpu=compute_cap,name --format=csv,noheader
// Result: "12.0, NVIDIA RTX PRO 6000 Blackwell Workstation Edition"

// Step 2: Parse compute capability
const [major, minor] = "12.0".split('.').map(n => parseInt(n));
const smVersion = major * 10 + minor;  // = 120

// Step 3: Select variant
if (smVersion >= 89 && smVersion <= 121) {
  return { variant: 'latest', architectures: 'sm_89-120', ... };
}
// Result: variant = 'latest' ✅ CORRECT
```

### URL Construction (✅ Working Correctly)

```javascript
const GPU_LIBS_VERSION = '1.2.0';  // ✅ Correct
const variant = 'latest';           // ✅ Correct
const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${GPU_LIBS_VERSION}/cuda-libs-${variant}.tar.gz`;
// Result: https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz
// ✅ URL is CORRECT
```

### Download Function (⚠️ Missing Error Handling)

```javascript
async function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      // ❌ NO STATUS CODE CHECK
      // Should check: response.statusCode === 200

      if (response.statusCode === 302 || response.statusCode === 301) {
        // Handles redirects
      } else {
        // ❌ Writes ANY response to file (including 404 HTML)
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }
    }).on('error', reject);
  });
}
```

**Problem:** The `downloadFile()` function writes the HTTP 404 "Not Found" text to the `.tar.gz` file, then `tar -xzf` fails because it's not a valid gzip archive.

**Fix:**
```javascript
async function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      // ✅ CHECK STATUS CODE
      if (response.statusCode === 404) {
        file.close();
        fs.unlink(dest, () => {});
        reject(new Error(`File not found (HTTP 404): ${url}`));
        return;
      }

      if (response.statusCode !== 200 && response.statusCode !== 301 && response.statusCode !== 302) {
        file.close();
        fs.unlink(dest, () => {});
        reject(new Error(`HTTP ${response.statusCode}: ${url}`));
        return;
      }

      // ... rest of function
    });
  });
}
```

---

## Correct URL (After Fix)

For your Blackwell RTX PRO 6000 (sm_120), the **correct URL should be:**

```
https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz
```

**Package Contents (Expected):**
- **ONNX Runtime 1.23.2** with CUDA 12.9 provider
- **Architecture support:** sm_89, sm_90, sm_100, sm_120
- **Size:** ~1.5GB compressed, 2.3GB uncompressed
- **CUDA libraries:** cuBLAS, cuDNN 9.15.1, CUDA Runtime 12.9

**This file does NOT exist yet** - it needs to be built and uploaded.

---

## Verification Steps

After the release is fixed, verify with:

```bash
# Check URL returns 200 OK
curl -I "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz"

# Download and verify
curl -L -o /tmp/test.tar.gz "https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.2.0/cuda-libs-latest.tar.gz"

# Verify it's a valid gzip archive
file /tmp/test.tar.gz
# Expected: "gzip compressed data"

# Extract and check contents
tar -tzf /tmp/test.tar.gz | head -20
# Expected: latest/libs/libonnxruntime.so, libcudnn.so.9, etc.

# Verify CUDA architectures in ONNX Runtime provider
tar -xzf /tmp/test.tar.gz latest/libs/libonnxruntime_providers_cuda.so
docker run --rm -v $(pwd):/data nvidia/cuda:12.9.0-devel-ubuntu22.04 \
  cuobjdump --list-elf /data/latest/libs/libonnxruntime_providers_cuda.so | grep sm_

# Expected output:
# sm_89
# sm_90
# sm_100
# sm_120
```

---

## Summary

| Component | Status | Notes |
|-----------|--------|-------|
| GPU Detection | ✅ Working | Correctly detects sm_120 |
| Variant Selection | ✅ Working | Correctly selects 'latest' |
| URL Construction | ✅ Working | URL is correct |
| GitHub Release | ❌ **BROKEN** | Missing `cuda-libs-latest.tar.gz` |
| Error Handling | ⚠️ Inadequate | Should detect 404 earlier |

**The code is correct. The GitHub release is incomplete.**

**Action Required:** Build and upload `cuda-libs-modern.tar.gz` and `cuda-libs-latest.tar.gz` to the v1.2.0 release.
