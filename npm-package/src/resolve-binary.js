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
const { execSync } = require('child_process');

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
 * Searches multiple locations to handle:
 * - Global installs: npm install -g swictation (sibling packages)
 * - Local installs: npm install swictation (nested in node_modules)
 * - Development: working within the repo
 * - nvm installations
 * - Custom npm prefix configurations
 */
function findPlatformPackage(packageName) {
  const packageShortName = packageName.replace('@agidreams/', '');

  // Helper to check if package exists at a given node_modules path
  function checkNodeModules(nodeModulesDir) {
    const scopeDir = path.join(nodeModulesDir, '@agidreams');
    if (fs.existsSync(scopeDir)) {
      const packageDir = path.join(scopeDir, packageShortName);
      if (fs.existsSync(packageDir)) {
        const packageJsonPath = path.join(packageDir, 'package.json');
        if (fs.existsSync(packageJsonPath)) {
          return packageDir;
        }
      }
    }
    return null;
  }

  // Strategy 1: Use npm root -g to find the definitive global path
  // This handles nvm, custom prefixes, and all npm configurations correctly
  try {
    const npmRoot = execSync('npm root -g', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }).trim();
    if (npmRoot && fs.existsSync(npmRoot)) {
      const result = checkNodeModules(npmRoot);
      if (result) return result;
    }
  } catch (e) {
    // npm root -g failed, continue with other strategies
  }

  // Strategy 2: Check for sibling package based on __dirname
  // If we're at /path/to/node_modules/swictation/src,
  // the platform package would be at /path/to/node_modules/@agidreams/linux-x64
  let currentDir = __dirname;
  for (let i = 0; i < 5; i++) {
    const parentDir = path.dirname(currentDir);
    // Check if parent is a node_modules directory
    if (path.basename(parentDir) === 'node_modules') {
      const result = checkNodeModules(parentDir);
      if (result) return result;
    }
    currentDir = parentDir;
    if (parentDir === currentDir) break;
  }

  // Strategy 3: Search upward for nested node_modules (local installs)
  currentDir = __dirname;
  for (let i = 0; i < 10; i++) {
    const nodeModulesDir = path.join(currentDir, 'node_modules');
    if (fs.existsSync(nodeModulesDir)) {
      const result = checkNodeModules(nodeModulesDir);
      if (result) return result;
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) break;
    currentDir = parentDir;
  }

  // Strategy 4: Check common global npm locations and nvm paths
  const globalPaths = [
    '/usr/local/lib/node_modules',
    '/usr/lib/node_modules',
    path.join(os.homedir(), '.npm-global', 'lib', 'node_modules'),
    path.join(os.homedir(), 'node_modules')
  ];

  // Add nvm paths if NVM_DIR is set
  if (process.env.NVM_DIR) {
    // Try to find the current node version's node_modules
    try {
      const nodeVersion = process.version;
      globalPaths.unshift(path.join(process.env.NVM_DIR, 'versions', 'node', nodeVersion, 'lib', 'node_modules'));
    } catch (e) {
      // Ignore
    }
  }

  // Also check paths containing literal $HOME (broken npm config edge case)
  const literalHomePath = path.join(os.homedir(), '$HOME', '.npm-global', 'lib', 'node_modules');
  if (fs.existsSync(literalHomePath)) {
    globalPaths.unshift(literalHomePath);
  }

  for (const globalPath of globalPaths) {
    if (fs.existsSync(globalPath)) {
      const result = checkNodeModules(globalPath);
      if (result) return result;
    }
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
