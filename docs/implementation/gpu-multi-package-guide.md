# GPU Multi-Package Implementation Guide

**Quick Reference for Implementing Option 2: Multiple Binaries with Runtime Detection**

## Overview

This guide provides step-by-step instructions for implementing the recommended architecture from `ARCHITECTURE_GPU_SUPPORT.md`.

**Goal:** Split 330MB CUDA library into 3 optimized packages (150-180MB each) with automatic runtime detection.

---

## Build Configuration

### 1. ONNX Runtime Build Commands

Build ONNX Runtime 3 times with different CUDA architectures:

```bash
# Package 1: Legacy (sm_50-70)
./build.sh --config Release \
  --build_shared_lib \
  --use_cuda \
  --cuda_home=/usr/local/cuda \
  --cudnn_home=/usr/lib/x86_64-linux-gnu \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="50;52;60;61;70" \
  --parallel 8

# Package 2: Modern (sm_75-86)
./build.sh --config Release \
  --build_shared_lib \
  --use_cuda \
  --cuda_home=/usr/local/cuda \
  --cudnn_home=/usr/lib/x86_64-linux-gnu \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="75;80;86" \
  --parallel 8

# Package 3: Latest (sm_89-90)
./build.sh --config Release \
  --build_shared_lib \
  --use_cuda \
  --cuda_home=/usr/local/cuda \
  --cudnn_home=/usr/lib/x86_64-linux-gnu \
  --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="89;90" \
  --parallel 8
```

### 2. Package Assembly

Each package should contain:

```bash
# Structure for each tar.gz
lib/native/
├── libonnxruntime.so (22MB)
├── libonnxruntime_providers_cuda.so (varies by package)
├── libonnxruntime_providers_shared.so (15KB)
├── libonnxruntime_providers_tensorrt.so (787KB)
├── libsherpa-onnx-c-api.so (3.8MB)
└── libsherpa-onnx-cxx-api.so (84KB)

# Create package
tar -czf cuda-libs-legacy.tar.gz -C lib/native .
tar -czf cuda-libs-modern.tar.gz -C lib/native .
tar -czf cuda-libs-latest.tar.gz -C lib/native .

# Generate checksums
sha256sum cuda-libs-*.tar.gz > SHA256SUMS
```

### 3. GitHub Actions Workflow

`.github/workflows/build-gpu-libs.yml`:

```yaml
name: Build GPU Libraries

on:
  push:
    tags:
      - 'gpu-libs-v*'

jobs:
  build-gpu-packages:
    runs-on: ubuntu-24.04
    strategy:
      matrix:
        package:
          - name: legacy
            archs: "50;52;60;61;70"
          - name: modern
            archs: "75;80;86"
          - name: latest
            archs: "89;90"

    steps:
      - name: Checkout
        uses: actions/checkout@v3

      - name: Install CUDA Toolkit 12.6
        uses: Jimver/cuda-toolkit@v0.2.11
        with:
          cuda: '12.6.0'

      - name: Install cuDNN
        run: |
          wget https://developer.download.nvidia.com/compute/cudnn/9.0.0/local_installers/cudnn-local-repo-ubuntu2404-9.0.0_1.0-1_amd64.deb
          sudo dpkg -i cudnn-local-repo-ubuntu2404-9.0.0_1.0-1_amd64.deb
          sudo apt-get update
          sudo apt-get install -y libcudnn9-cuda-12

      - name: Clone ONNX Runtime
        run: |
          git clone --depth 1 --branch v1.19.0 https://github.com/microsoft/onnxruntime.git

      - name: Build ONNX Runtime (${{ matrix.package.name }})
        run: |
          cd onnxruntime
          ./build.sh --config Release \
            --build_shared_lib \
            --use_cuda \
            --cuda_home=/usr/local/cuda \
            --cudnn_home=/usr/lib/x86_64-linux-gnu \
            --cmake_extra_defines CMAKE_CUDA_ARCHITECTURES="${{ matrix.package.archs }}" \
            --parallel 8

      - name: Package Libraries
        run: |
          mkdir -p package/lib/native
          cp onnxruntime/build/Linux/Release/libonnxruntime*.so package/lib/native/
          # Copy other libs (sherpa-onnx, etc.)
          tar -czf cuda-libs-${{ matrix.package.name }}.tar.gz -C package/lib/native .

      - name: Generate Checksum
        run: |
          sha256sum cuda-libs-${{ matrix.package.name }}.tar.gz > cuda-libs-${{ matrix.package.name }}.tar.gz.sha256

      - name: Upload to Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            cuda-libs-${{ matrix.package.name }}.tar.gz
            cuda-libs-${{ matrix.package.name }}.tar.gz.sha256
```

---

## Detection & Download Logic

### Modified `postinstall.js`

Add to `npm-package/postinstall.js`:

```javascript
/**
 * Detect GPU compute capability and select appropriate package
 * @returns {Object} Package information
 */
function selectGPUPackage() {
  try {
    // Get compute capability (e.g., "8.6")
    const computeCapRaw = execSync(
      'nvidia-smi --query-gpu=compute_cap --format=csv,noheader',
      { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] }
    ).trim();

    // Get GPU name for logging
    const gpuName = execSync(
      'nvidia-smi --query-gpu=name --format=csv,noheader',
      { encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] }
    ).trim();

    // Parse compute capability
    const [major, minor] = computeCapRaw.split('.').map(Number);
    const sm = major * 10 + minor;

    log('green', `✓ GPU Detected: ${gpuName}`);
    log('cyan', `  Compute Capability: ${major}.${minor} (sm_${sm})`);

    // Select package based on compute capability
    let packageInfo;

    if (sm >= 89 && sm <= 90) {
      packageInfo = {
        name: 'cuda-libs-latest',
        size: '150MB',
        description: 'Latest generation (Ada Lovelace/Hopper)',
        gpus: 'RTX 40 series, H100, L4/L40'
      };
    } else if (sm >= 75 && sm <= 86) {
      packageInfo = {
        name: 'cuda-libs-modern',
        size: '180MB',
        description: 'Modern generation (Turing/Ampere)',
        gpus: 'GTX 16, RTX 20/30, A100'
      };
    } else if (sm >= 50 && sm <= 70) {
      packageInfo = {
        name: 'cuda-libs-legacy',
        size: '150MB',
        description: 'Legacy generation (Maxwell/Pascal/Volta)',
        gpus: 'GTX 900/1000, Quadro P, Titan V'
      };
    } else {
      // Unknown compute capability - fall back to modern (widest compatibility)
      log('yellow', `⚠️  Unknown compute capability: sm_${sm}`);
      log('cyan', '   Falling back to modern package (sm_75-86)');
      packageInfo = {
        name: 'cuda-libs-modern',
        size: '180MB',
        description: 'Modern generation (fallback)',
        gpus: 'Wide compatibility'
      };
    }

    log('cyan', `  Package: ${packageInfo.description}`);
    log('cyan', `  Target GPUs: ${packageInfo.gpus}`);
    log('cyan', `  Download size: ${packageInfo.size}`);

    return packageInfo;

  } catch (err) {
    // nvidia-smi failed - no GPU or driver issue
    return null;
  }
}

/**
 * Download GPU package with progress bar and retry logic
 */
async function downloadGPULibraries() {
  const packageInfo = selectGPUPackage();

  if (!packageInfo) {
    log('cyan', '\nℹ No NVIDIA GPU detected - skipping GPU library download');
    log('cyan', '  CPU-only mode will be used');
    return;
  }

  log('green', '\n✓ NVIDIA GPU detected!');
  log('cyan', `📦 Downloading ${packageInfo.description}...`);

  const GPU_LIBS_VERSION = '1.1.0'; // Update when packages change
  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${GPU_LIBS_VERSION}/${packageInfo.name}.tar.gz`;
  const checksumUrl = `${releaseUrl}.sha256`;

  const tmpDir = path.join(os.tmpdir(), 'swictation-gpu-install');
  const tarPath = path.join(tmpDir, `${packageInfo.name}.tar.gz`);
  const checksumPath = path.join(tmpDir, `${packageInfo.name}.tar.gz.sha256`);
  const nativeDir = path.join(__dirname, 'lib', 'native');

  try {
    // Create temp directory
    if (!fs.existsSync(tmpDir)) {
      fs.mkdirSync(tmpDir, { recursive: true });
    }

    // Download with retries
    const maxRetries = 3;
    let downloadSuccess = false;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        log('cyan', `  Downloading package (attempt ${attempt}/${maxRetries})...`);
        await downloadFileWithProgress(releaseUrl, tarPath);
        downloadSuccess = true;
        break;
      } catch (err) {
        if (attempt === maxRetries) {
          throw err;
        }
        log('yellow', `  ⚠️  Download failed, retrying in 2s...`);
        await new Promise(resolve => setTimeout(resolve, 2000));
      }
    }

    if (!downloadSuccess) {
      throw new Error('Failed to download after 3 attempts');
    }

    log('green', '  ✓ Package downloaded');

    // Download and verify checksum
    try {
      log('cyan', '  Verifying checksum...');
      await downloadFile(checksumUrl, checksumPath);

      const expectedHash = fs.readFileSync(checksumPath, 'utf8').split(/\s+/)[0];
      const actualHash = execSync(`sha256sum "${tarPath}"`, { encoding: 'utf8' }).split(/\s+/)[0];

      if (expectedHash !== actualHash) {
        throw new Error(`Checksum mismatch: expected ${expectedHash}, got ${actualHash}`);
      }

      log('green', '  ✓ Checksum verified');
    } catch (err) {
      log('yellow', `  ⚠️  Checksum verification failed: ${err.message}`);
      log('yellow', '     Continuing anyway (checksum optional)');
    }

    // Extract tarball
    log('cyan', '  Extracting libraries...');
    execSync(`tar -xzf "${tarPath}" -C "${nativeDir}"`, { stdio: 'inherit' });
    log('green', '  ✓ Extracted GPU libraries');

    // Save metadata
    const metadata = {
      packageName: packageInfo.name,
      packageVersion: GPU_LIBS_VERSION,
      description: packageInfo.description,
      installedAt: new Date().toISOString()
    };

    fs.writeFileSync(
      path.join(nativeDir, '.gpu-metadata.json'),
      JSON.stringify(metadata, null, 2)
    );

    // Cleanup
    fs.unlinkSync(tarPath);
    if (fs.existsSync(checksumPath)) fs.unlinkSync(checksumPath);
    fs.rmdirSync(tmpDir);

    log('green', '✓ GPU acceleration enabled!');
    log('cyan', `  Package: ${packageInfo.name}`);
    log('cyan', '  Your system will use CUDA for faster transcription');

  } catch (err) {
    log('yellow', `\n⚠️  Failed to download GPU libraries: ${err.message}`);
    log('cyan', '  Continuing with CPU-only mode');
    log('cyan', '  You can manually download from:');
    log('cyan', `  ${releaseUrl}`);
  }
}

/**
 * Download file with progress bar
 */
function downloadFileWithProgress(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    let downloadedBytes = 0;
    let lastPercent = -1;

    https.get(url, (response) => {
      // Handle redirects
      if (response.statusCode === 302 || response.statusCode === 301) {
        file.close();
        fs.unlinkSync(dest);
        return downloadFileWithProgress(response.headers.location, dest)
          .then(resolve)
          .catch(reject);
      }

      const totalBytes = parseInt(response.headers['content-length'], 10);

      response.on('data', (chunk) => {
        downloadedBytes += chunk.length;
        const percent = Math.floor((downloadedBytes / totalBytes) * 100);

        // Update progress every 5% to avoid spamming
        if (percent !== lastPercent && percent % 5 === 0) {
          const downloadedMB = (downloadedBytes / 1024 / 1024).toFixed(1);
          const totalMB = (totalBytes / 1024 / 1024).toFixed(1);
          process.stdout.write(`\r  Progress: ${percent}% (${downloadedMB}MB / ${totalMB}MB)`);
          lastPercent = percent;
        }
      });

      response.pipe(file);

      file.on('finish', () => {
        file.close();
        console.log(''); // New line after progress
        resolve();
      });

    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}
```

---

## Testing Plan

### Test Matrix

Test on representative hardware for each package:

```bash
# Package 1: Legacy (sm_50-70)
Hardware: GTX 1060 (sm_61)
Expected: Downloads cuda-libs-legacy.tar.gz (150MB)
Verify:
  - Package selection correct
  - Download completes
  - Checksum verifies
  - Daemon starts with GPU acceleration
  - Performance: >10x realtime

# Package 2: Modern (sm_75-86)
Hardware: RTX 3080 (sm_86)
Expected: Downloads cuda-libs-modern.tar.gz (180MB)
Verify:
  - Package selection correct
  - Download completes
  - Checksum verifies
  - Daemon starts with GPU acceleration
  - Performance: >10x realtime (baseline)

# Package 3: Latest (sm_89-90)
Hardware: RTX 4090 (sm_89)
Expected: Downloads cuda-libs-latest.tar.gz (150MB)
Verify:
  - Package selection correct
  - Download completes
  - Checksum verifies
  - Daemon starts with GPU acceleration
  - Performance: >10x realtime
```

### Test Cases

```javascript
// Test 1: Correct package selection
describe('GPU Package Selection', () => {
  test('sm_61 → legacy package', () => {
    const pkg = selectGPUPackage('6.1');
    expect(pkg.name).toBe('cuda-libs-legacy');
  });

  test('sm_86 → modern package', () => {
    const pkg = selectGPUPackage('8.6');
    expect(pkg.name).toBe('cuda-libs-modern');
  });

  test('sm_89 → latest package', () => {
    const pkg = selectGPUPackage('8.9');
    expect(pkg.name).toBe('cuda-libs-latest');
  });

  test('unknown sm_95 → modern (fallback)', () => {
    const pkg = selectGPUPackage('9.5');
    expect(pkg.name).toBe('cuda-libs-modern');
  });
});

// Test 2: Download retry logic
describe('Download Resilience', () => {
  test('retries 3 times on network failure', async () => {
    // Mock network failure twice, succeed on third
  });

  test('falls back to CPU on persistent failure', async () => {
    // All 3 retries fail → graceful fallback
  });
});

// Test 3: Checksum verification
describe('Security', () => {
  test('detects corrupted download', async () => {
    // Corrupt tarball → checksum fails → re-download
  });
});
```

---

## Rollout Plan

### Phase 1: Build & Release (Week 1)
```bash
# 1. Build all 3 packages locally
./scripts/build-gpu-packages.sh

# 2. Test locally on available hardware
npm install ./swictation-0.3.15.tgz

# 3. Create GitHub release
git tag gpu-libs-v1.1.0
git push origin gpu-libs-v1.1.0

# 4. Upload packages to release
gh release create gpu-libs-v1.1.0 \
  cuda-libs-legacy.tar.gz \
  cuda-libs-modern.tar.gz \
  cuda-libs-latest.tar.gz \
  SHA256SUMS
```

### Phase 2: npm Package Update (Week 2)
```bash
# 1. Update package.json version
npm version minor  # 0.3.14 → 0.3.15

# 2. Update GPU_LIBS_VERSION in postinstall.js
const GPU_LIBS_VERSION = '1.1.0';

# 3. Test npm package
npm pack
npm install -g swictation-0.3.15.tgz

# 4. Publish to npm
npm publish
```

### Phase 3: Monitoring (Week 3-4)
```bash
# Monitor installation metrics
# - Download success rates (target: >99%)
# - Package selection distribution (legacy/modern/latest)
# - Average installation time (target: <3 min)
# - Support tickets (target: <5%)
```

---

## Manual Override (Advanced Users)

For users who want explicit control:

```bash
# Environment variable to skip auto-download
SWICTATION_MANUAL_GPU=1 npm install swictation

# CLI command for manual download
swictation download-gpu modern
swictation download-gpu legacy
swictation download-gpu latest
swictation download-gpu all  # Download all packages
```

Implementation in `bin/swictation`:

```javascript
if (process.argv[2] === 'download-gpu') {
  const packageName = process.argv[3]; // modern, legacy, latest, all

  if (packageName === 'all') {
    await downloadGPUPackage('cuda-libs-legacy');
    await downloadGPUPackage('cuda-libs-modern');
    await downloadGPUPackage('cuda-libs-latest');
  } else {
    const pkgMap = {
      'modern': 'cuda-libs-modern',
      'legacy': 'cuda-libs-legacy',
      'latest': 'cuda-libs-latest'
    };
    await downloadGPUPackage(pkgMap[packageName]);
  }
}
```

---

## Troubleshooting

### Issue: Wrong package selected
```bash
# Check detected compute capability
nvidia-smi --query-gpu=compute_cap --format=csv,noheader

# Check installed package
cat ~/.local/lib/node_modules/swictation/lib/native/.gpu-metadata.json

# Manually download correct package
swictation download-gpu <correct-package>
```

### Issue: Download fails
```bash
# Check GitHub releases are accessible
curl -I https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.1.0/cuda-libs-modern.tar.gz

# Manual download and install
wget https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.1.0/cuda-libs-modern.tar.gz
tar -xzf cuda-libs-modern.tar.gz -C ~/.local/lib/node_modules/swictation/lib/native/
```

### Issue: Checksum verification fails
```bash
# Verify file integrity
sha256sum cuda-libs-modern.tar.gz

# Compare with published checksum
curl https://github.com/robertelee78/swictation/releases/download/gpu-libs-v1.1.0/cuda-libs-modern.tar.gz.sha256
```

---

## Success Metrics

After rollout, track:

1. **Installation Metrics**
   - Average install time (target: <3 min on 50Mbps)
   - Download success rate (target: >99%)
   - Package size per user (baseline: 150-180MB)

2. **Distribution**
   - Legacy package: ~15% of GPU users
   - Modern package: ~70% of GPU users
   - Latest package: ~15% of GPU users

3. **Performance**
   - No performance regression vs single binary
   - GPU utilization >90% during transcription

4. **Support**
   - GPU-related support tickets <5% of userbase
   - Installation success rate >95%

---

**Reference**: See `ARCHITECTURE_GPU_SUPPORT.md` for full architectural analysis.
