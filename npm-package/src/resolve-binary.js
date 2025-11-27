/**
 * Binary Path Resolution Module
 *
 * Finds the correct binaries and libraries from the installed platform package.
 * Works with postinstall.js GPU detection to ensure correct library paths.
 *
 * Platform packages:
 * - @agidreams/linux-x64: Linux x86_64 binaries + ONNX Runtime
 * - @agidreams/darwin-arm64: macOS ARM64 binaries + CoreML ONNX Runtime
 *
 * After platform package is found, postinstall.js will:
 * - Detect GPU (NVIDIA/AMD/Intel)
 * - Check VRAM and system RAM
 * - Download GPU-specific libraries to platformPackage.libDir
 * - Recommend appropriate model sizes
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

/**
 * Platform detection
 */
function detectPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === 'linux' && arch === 'x64') {
    return {
      platform: 'linux',
      arch: 'x64',
      packageName: '@agidreams/linux-x64',
      binaries: {
        daemon: 'swictation-daemon',
        ui: 'swictation-ui'
      }
    };
  }

  if (platform === 'darwin' && arch === 'arm64') {
    return {
      platform: 'darwin',
      arch: 'arm64',
      packageName: '@agidreams/darwin-arm64',
      binaries: {
        daemon: 'swictation-daemon',
        ui: 'swictation-ui'
      }
    };
  }

  // Unsupported platform
  return {
    platform,
    arch,
    supported: false,
    error: `Unsupported platform: ${platform}-${arch}. Swictation supports linux-x64 and darwin-arm64 only.`
  };
}

/**
 * Find the platform package in node_modules
 *
 * Searches upward through parent directories to handle:
 * - Global installs: npm install -g swictation
 * - Local installs: npm install swictation
 * - Development: working within the repo
 */
function findPlatformPackage(packageName) {
  // Start from current directory
  let currentDir = __dirname;

  // Try up to 10 levels up (should be more than enough)
  for (let i = 0; i < 10; i++) {
    // Check for node_modules at this level
    const nodeModulesDir = path.join(currentDir, 'node_modules');

    if (fs.existsSync(nodeModulesDir)) {
      // Check for @agidreams scope directory
      const scopeDir = path.join(nodeModulesDir, '@agidreams');

      if (fs.existsSync(scopeDir)) {
        // Extract package name without scope (@agidreams/linux-x64 -> linux-x64)
        const packageShortName = packageName.replace('@agidreams/', '');
        const packageDir = path.join(scopeDir, packageShortName);

        if (fs.existsSync(packageDir)) {
          // Verify it has a package.json
          const packageJsonPath = path.join(packageDir, 'package.json');
          if (fs.existsSync(packageJsonPath)) {
            return packageDir;
          }
        }
      }
    }

    // Move up one directory
    const parentDir = path.dirname(currentDir);

    // If we've reached the root, stop
    if (parentDir === currentDir) {
      break;
    }

    currentDir = parentDir;
  }

  return null;
}

/**
 * Verify binaries exist in the platform package
 */
function verifyBinaries(binDir, binaries) {
  const missing = [];

  for (const [name, filename] of Object.entries(binaries)) {
    const binaryPath = path.join(binDir, filename);
    if (!fs.existsSync(binaryPath)) {
      missing.push(filename);
    }
  }

  return missing;
}

/**
 * Resolve all paths for the current platform
 *
 * Returns:
 * {
 *   platform: 'linux' | 'darwin',
 *   arch: 'x64' | 'arm64',
 *   packageName: '@agidreams/linux-x64' | '@agidreams/darwin-arm64',
 *   packageDir: '/path/to/node_modules/@agidreams/linux-x64',
 *   binDir: '/path/to/node_modules/@agidreams/linux-x64/bin',
 *   libDir: '/path/to/node_modules/@agidreams/linux-x64/lib',
 *   daemon: '/path/to/node_modules/@agidreams/linux-x64/bin/swictation-daemon',
 *   ui: '/path/to/node_modules/@agidreams/linux-x64/bin/swictation-ui'
 * }
 *
 * libDir is where postinstall.js will download GPU-specific libraries:
 * - Linux: libcudart.so, libcublas.so, etc. (based on GPU detection)
 * - macOS: CoreML libraries (already included in ONNX Runtime)
 */
function resolveBinaryPaths() {
  // Detect platform
  const platformInfo = detectPlatform();

  if (platformInfo.supported === false) {
    throw new Error(platformInfo.error);
  }

  // Find platform package
  const packageDir = findPlatformPackage(platformInfo.packageName);

  if (!packageDir) {
    throw new Error(
      `Platform package ${platformInfo.packageName} not found.\n` +
      `This usually means:\n` +
      `  1. The package failed to install (check npm install output)\n` +
      `  2. You're on an unsupported platform (${platformInfo.platform}-${platformInfo.arch})\n` +
      `  3. npm's optionalDependencies installation failed\n\n` +
      `Try reinstalling: npm install -g swictation --force`
    );
  }

  // Build paths
  const binDir = path.join(packageDir, 'bin');
  const libDir = path.join(packageDir, 'lib');

  // Verify binaries exist
  const missingBinaries = verifyBinaries(binDir, platformInfo.binaries);

  if (missingBinaries.length > 0) {
    throw new Error(
      `Platform package ${platformInfo.packageName} is installed but binaries are missing:\n` +
      `  Missing: ${missingBinaries.join(', ')}\n` +
      `  Expected in: ${binDir}\n\n` +
      `This means the platform package was not built correctly.\n` +
      `Try reinstalling: npm install -g swictation --force`
    );
  }

  return {
    platform: platformInfo.platform,
    arch: platformInfo.arch,
    packageName: platformInfo.packageName,
    packageDir,
    binDir,
    libDir, // GPU libraries will be downloaded here by postinstall.js
    daemon: path.join(binDir, platformInfo.binaries.daemon),
    ui: path.join(binDir, platformInfo.binaries.ui)
  };
}

/**
 * Get library directory for GPU libraries
 *
 * Used by postinstall.js to determine where to download:
 * - CUDA libraries (Linux with NVIDIA GPU)
 * - ROCm libraries (Linux with AMD GPU)
 * - OpenVINO libraries (Linux with Intel GPU)
 * - CoreML libraries (macOS - already in ONNX Runtime)
 */
function getLibraryDirectory() {
  const paths = resolveBinaryPaths();
  return paths.libDir;
}

/**
 * Check if platform package is installed (for postinstall checks)
 */
function isPlatformPackageInstalled() {
  try {
    resolveBinaryPaths();
    return true;
  } catch (err) {
    return false;
  }
}

module.exports = {
  resolveBinaryPaths,
  getLibraryDirectory,
  isPlatformPackageInstalled,
  detectPlatform
};
