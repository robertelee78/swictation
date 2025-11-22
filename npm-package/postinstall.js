#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');
const https = require('https');
const crypto = require('crypto');
const { checkNvidiaHibernationStatus } = require('./src/nvidia-hibernation-setup');

// Environment variable support for model test-loading
// By default, model testing runs when GPU is detected
// Set SKIP_MODEL_TEST=1 to disable (useful for CI/headless environments)
const SKIP_MODEL_TEST = process.env.SKIP_MODEL_TEST === '1';

// Colors for console output (basic implementation without chalk dependency)
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  red: '\x1b[31m'
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function checkPlatform() {
  if (process.platform !== 'linux') {
    log('yellow', 'Note: Swictation currently only supports Linux x64');
    log('yellow', 'Skipping postinstall for non-Linux platform');
    process.exit(0);
  }

  if (process.arch !== 'x64') {
    log('yellow', 'Note: Swictation currently only supports x64 architecture');
    process.exit(0);
  }

  // Check GLIBC version
  try {
    const glibcVersion = execSync('ldd --version 2>&1 | head -1', { encoding: 'utf8' });
    const versionMatch = glibcVersion.match(/(\d+)\.(\d+)/);
    if (versionMatch) {
      const major = parseInt(versionMatch[1]);
      const minor = parseInt(versionMatch[2]);

      if (major < 2 || (major === 2 && minor < 39)) {
        log('red', '\n⚠ INCOMPATIBLE GLIBC VERSION');
        log('yellow', `Detected GLIBC ${major}.${minor} (need 2.39+)`);
        log('yellow', 'Swictation requires Ubuntu 24.04 LTS or newer');
        log('yellow', 'Ubuntu 22.04 is NOT supported due to GLIBC 2.35');
        log('yellow', '\nSupported distributions:');
        log('cyan', '  - Ubuntu 24.04 LTS (Noble Numbat) or newer');
        log('cyan', '  - Debian 13+ (Trixie)');
        log('cyan', '  - Fedora 39+');
        log('yellow', '\nInstallation will continue but binaries may not work.');
      }
    }
  } catch (err) {
    log('yellow', 'Warning: Could not check GLIBC version');
  }
}

/**
 * Phase 1: Clean up old/conflicting service files from previous installations
 * This prevents conflicts between old Python-based services and new Node.js services
 */
/**
 * Stop currently running services before upgrade to prevent CUDA state corruption
 * This must run BEFORE any file modifications happen
 */
async function stopExistingServices() {
  log('cyan', '\n🛑 Stopping currently running services...');

  let stopped = false;

  try {
    // Method 1: Try using swictation CLI if available
    try {
      execSync('which swictation 2>/dev/null', { stdio: 'ignore' });
      execSync('swictation stop 2>/dev/null', { stdio: 'ignore' });
      log('green', '✓ Stopped swictation services via CLI');
      stopped = true;
      // Give services time to fully stop and release CUDA
      await new Promise(resolve => setTimeout(resolve, 2000));
    } catch (cliErr) {
      // swictation CLI not available, try systemctl
      try {
        execSync('systemctl --user stop swictation-daemon.service 2>/dev/null', { stdio: 'ignore' });
        execSync('systemctl --user stop swictation-ui.service 2>/dev/null', { stdio: 'ignore' });
        log('green', '✓ Stopped services via systemctl');
        stopped = true;
        await new Promise(resolve => setTimeout(resolve, 2000));
      } catch (systemctlErr) {
        log('cyan', 'ℹ No existing services to stop');
      }
    }
  } catch (err) {
    log('cyan', 'ℹ No existing services to stop');
  }

  return stopped;
}

/**
 * Clean up old ONNX Runtime libraries from Python pip installations
 * These cause version conflicts (1.20.1 vs 1.22.x)
 */
function cleanupOldOnnxRuntime() {
  log('cyan', '\n🧹 Checking for old ONNX Runtime libraries...');

  try {
    const homeDir = os.homedir();
    const pythonLibDirs = [
      path.join(homeDir, '.local', 'lib', 'python3.13', 'site-packages', 'onnxruntime'),
      path.join(homeDir, '.local', 'lib', 'python3.12', 'site-packages', 'onnxruntime'),
      path.join(homeDir, '.local', 'lib', 'python3.11', 'site-packages', 'onnxruntime'),
      path.join(homeDir, '.local', 'lib', 'python3.10', 'site-packages', 'onnxruntime'),
    ];

    let removedAny = false;

    for (const ortDir of pythonLibDirs) {
      if (fs.existsSync(ortDir)) {
        try {
          // Check if it's an old version that conflicts
          const capiDir = path.join(ortDir, 'capi');
          if (fs.existsSync(capiDir)) {
            const ortFiles = fs.readdirSync(capiDir).filter(f => f.includes('libonnxruntime.so'));
            if (ortFiles.length > 0 && ortFiles[0].includes('1.20')) {
              log('yellow', `⚠️  Found old ONNX Runtime 1.20.x at ${ortDir}`);
              log('cyan', `   Removing to prevent version conflicts...`);
              execSync(`rm -rf "${ortDir}"`, { stdio: 'ignore' });
              log('green', `✓ Removed old ONNX Runtime installation`);
              removedAny = true;
            }
          }
        } catch (err) {
          // Can't determine version, leave it alone
        }
      }
    }

    if (!removedAny) {
      log('green', '✓ No conflicting ONNX Runtime installations found');
    }
  } catch (err) {
    log('yellow', `⚠️  Error checking ONNX Runtime installations: ${err.message}`);
  }
}

/**
 * Remove old npm installations that conflict with new installation
 * Handles both system-wide and nvm installations
 */
function cleanupOldNpmInstallations() {
  log('cyan', '\n🧹 Checking for old npm installations...');

  const oldInstallPaths = [
    '/usr/local/lib/node_modules/swictation',
    '/usr/local/nodejs/lib/node_modules/swictation',
    '/usr/lib/node_modules/swictation',
  ];

  let removedAny = false;

  for (const oldPath of oldInstallPaths) {
    if (fs.existsSync(oldPath) && oldPath !== __dirname) {
      log('yellow', `⚠️  Found old npm installation at ${oldPath}`);
      log('cyan', `   Removing to prevent conflicts...`);
      try {
        execSync(`sudo rm -rf "${oldPath}" 2>/dev/null || rm -rf "${oldPath}"`, { stdio: 'ignore' });
        log('green', `✓ Removed old installation`);
        removedAny = true;
      } catch (err) {
        log('yellow', `⚠️  Could not remove ${oldPath}: ${err.message}`);
        log('yellow', `   You may need to run: sudo rm -rf "${oldPath}"`);
      }
    }
  }

  if (!removedAny) {
    log('green', '✓ No conflicting npm installations found');
  }
}

async function cleanOldServices() {
  log('cyan', '\n🧹 Checking for old service files...');

  const oldServiceLocations = [
    // Old system-wide service files (from Python version)
    '/usr/lib/systemd/user/swictation.service',
    '/usr/lib/systemd/system/swictation.service',
    // Old user service files that might conflict
    path.join(os.homedir(), '.config', 'systemd', 'user', 'swictation.service')
  ];

  let foundOldServices = false;

  for (const servicePath of oldServiceLocations) {
    if (fs.existsSync(servicePath)) {
      foundOldServices = true;
      log('yellow', `⚠️  Found old service file: ${servicePath}`);

      // Extract service name from path
      const serviceName = path.basename(servicePath);
      const isSystemService = servicePath.includes('/system/');

      try {
        // Try to stop the service if it's running
        const stopCmd = isSystemService
          ? `sudo systemctl stop ${serviceName} 2>/dev/null || true`
          : `systemctl --user stop ${serviceName} 2>/dev/null || true`;

        execSync(stopCmd, { stdio: 'ignore' });
        log('cyan', `  ✓ Stopped service: ${serviceName}`);

        // Disable the service
        const disableCmd = isSystemService
          ? `sudo systemctl disable ${serviceName} 2>/dev/null || true`
          : `systemctl --user disable ${serviceName} 2>/dev/null || true`;

        execSync(disableCmd, { stdio: 'ignore' });
        log('cyan', `  ✓ Disabled service: ${serviceName}`);

        // Remove the service file (requires sudo for system services)
        if (isSystemService) {
          try {
            execSync(`sudo rm -f "${servicePath}"`, { stdio: 'ignore' });
            log('green', `  ✓ Removed old service file: ${servicePath}`);
          } catch (err) {
            log('yellow', `  ⚠️  Could not remove ${servicePath} (may need manual cleanup)`);
          }
        } else {
          try {
            fs.unlinkSync(servicePath);
            log('green', `  ✓ Removed old service file: ${servicePath}`);
          } catch (err) {
            log('yellow', `  ⚠️  Could not remove ${servicePath}: ${err.message}`);
          }
        }

      } catch (err) {
        log('yellow', `  ⚠️  Error cleaning up ${serviceName}: ${err.message}`);
      }
    }
  }

  if (foundOldServices) {
    // Reload systemd to pick up changes
    try {
      execSync('systemctl --user daemon-reload 2>/dev/null', { stdio: 'ignore' });
      execSync('sudo systemctl daemon-reload 2>/dev/null || true', { stdio: 'ignore' });
      log('green', '✓ Reloaded systemd daemon');
    } catch (err) {
      log('yellow', '⚠️  Could not reload systemd daemon');
    }
  } else {
    log('green', '✓ No old service files found');
  }

  return foundOldServices;
}

function ensureBinaryPermissions() {
  const binDir = path.join(__dirname, 'bin');
  const daemonBinary = path.join(binDir, 'swictation-daemon');
  const uiBinary = path.join(binDir, 'swictation-ui');
  const cliBinary = path.join(binDir, 'swictation');
  const daemonBin = path.join(__dirname, 'lib', 'native', 'swictation-daemon.bin');

  // Make sure all binaries are executable
  const binaries = [daemonBinary, uiBinary, cliBinary, daemonBin];

  for (const binary of binaries) {
    if (fs.existsSync(binary)) {
      try {
        fs.chmodSync(binary, '755');
        log('green', `✓ Set execute permissions for ${path.basename(binary)}`);
      } catch (err) {
        log('yellow', `Warning: Could not set permissions for ${path.basename(binary)}: ${err.message}`);
      }
    }
  }
}

function createDirectories() {
  const dirs = [
    path.join(os.homedir(), '.config', 'swictation'),
    path.join(os.homedir(), '.local', 'share', 'swictation'),
    path.join(os.homedir(), '.local', 'share', 'swictation', 'models'),
    path.join(os.homedir(), '.cache', 'swictation')
  ];

  for (const dir of dirs) {
    if (!fs.existsSync(dir)) {
      try {
        fs.mkdirSync(dir, { recursive: true });
        log('green', `✓ Created directory: ${dir}`);
      } catch (err) {
        log('yellow', `Warning: Could not create ${dir}: ${err.message}`);
      }
    }
  }
}

/**
 * Detect system package manager
 * @returns {object} Package manager info with install command
 */
function detectPackageManager() {
  const managers = [
    { cmd: 'apt', name: 'apt', installCmd: 'sudo apt update && sudo apt install -y' },
    { cmd: 'dnf', name: 'dnf', installCmd: 'sudo dnf install -y' },
    { cmd: 'pacman', name: 'pacman', installCmd: 'sudo pacman -S --noconfirm' },
    { cmd: 'zypper', name: 'zypper', installCmd: 'sudo zypper install -y' }
  ];

  for (const manager of managers) {
    try {
      execSync(`which ${manager.cmd}`, { stdio: 'ignore' });
      return manager;
    } catch {
      // Try next manager
    }
  }

  return null;
}

/**
 * Install a package using the system package manager
 * @param {string} packageName - Package name to install
 * @param {string} displayName - Display name for logging
 * @returns {boolean} Success status
 */
function installPackage(packageName, displayName) {
  const pkgManager = detectPackageManager();

  if (!pkgManager) {
    log('yellow', `  ⚠️  No supported package manager found (apt/dnf/pacman/zypper)`);
    log('cyan', `  Please install ${displayName} manually`);
    return false;
  }

  log('cyan', `  Installing ${displayName} via ${pkgManager.name}...`);

  try {
    execSync(`${pkgManager.installCmd} ${packageName}`, { stdio: 'inherit' });
    log('green', `  ✓ ${displayName} installed successfully`);
    return true;
  } catch (err) {
    log('yellow', `  ⚠️  Failed to install ${displayName}: ${err.message}`);
    log('cyan', `  Install manually: ${pkgManager.installCmd} ${packageName}`);
    return false;
  }
}

function checkDependencies() {
  const optional = [];
  const required = [];

  // Check for required tools
  const tools = [
    { name: 'systemctl', type: 'optional', package: 'systemd' },
    { name: 'nc', type: 'optional', package: 'netcat' },
    { name: 'wtype', type: 'optional', package: 'wtype (for Wayland)' },
    { name: 'xdotool', type: 'optional', package: 'xdotool (for X11)' },
    { name: 'hf', type: 'optional', package: 'huggingface_hub[cli] (pip install huggingface_hub[cli])' }
  ];

  for (const tool of tools) {
    try {
      execSync(`which ${tool.name}`, { stdio: 'ignore' });
    } catch {
      if (tool.type === 'required') {
        required.push(tool);
      } else {
        optional.push(tool);
      }
    }
  }

  if (required.length > 0) {
    log('red', '\n⚠ Required dependencies missing:');
    for (const tool of required) {
      log('yellow', `  - ${tool.name} (install: ${tool.package})`);
    }
    log('red', '\nPlease install required dependencies before using Swictation');
    process.exit(1);
  }

  if (optional.length > 0) {
    log('yellow', '\n📦 Optional dependencies for full functionality:');
    for (const tool of optional) {
      log('cyan', `  - ${tool.name} (${tool.package})`);
    }
  }
}

function detectNvidiaGPU() {
  try {
    execSync('nvidia-smi', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Detect GPU compute capability (sm_XX architecture)
 * Returns { hasGPU: boolean, computeCap: string, smVersion: number }
 * @returns {object} GPU compute capability info
 */
function detectGPUComputeCapability() {
  const result = {
    hasGPU: false,
    computeCap: null,  // e.g., "5.2", "8.6", "12.0"
    smVersion: null,   // e.g., 52, 86, 120
    gpuName: null
  };

  if (!detectNvidiaGPU()) {
    return result;
  }

  result.hasGPU = true;

  try {
    // Get compute capability and GPU name
    const output = execSync(
      'nvidia-smi --query-gpu=compute_cap,name --format=csv,noheader',
      { encoding: 'utf8' }
    ).trim();

    const [computeCap, gpuName] = output.split(',').map(s => s.trim());
    result.computeCap = computeCap;
    result.gpuName = gpuName;

    // Convert "5.2" -> 52, "8.6" -> 86, "12.0" -> 120
    const [major, minor] = computeCap.split('.').map(n => parseInt(n));
    result.smVersion = major * 10 + minor;

    log('green', `✓ Detected GPU: ${gpuName}`);
    log('cyan', `  Compute Capability: ${computeCap} (sm_${result.smVersion})`);

  } catch (err) {
    log('yellow', `⚠️  Could not detect compute capability: ${err.message}`);
  }

  return result;
}

/**
 * Select appropriate GPU library package variant based on compute capability
 * @param {number} smVersion - Compute capability as integer (e.g., 52, 86, 120)
 * @returns {object} Package variant info
 */
function selectGPUPackageVariant(smVersion) {
  // Architecture mapping based on RELEASE_NOTES.md
  if (smVersion >= 50 && smVersion <= 70) {
    // sm_50-70: Maxwell, Pascal, Volta (2014-2017)
    return {
      variant: 'legacy',
      architectures: 'sm_50-70',
      description: 'Maxwell, Pascal, Volta GPUs (2014-2017)',
      examples: 'GTX 750/900/1000, Quadro M/P series, Titan V, V100'
    };
  } else if (smVersion >= 75 && smVersion <= 86) {
    // sm_75-86: Turing, Ampere (2018-2021)
    return {
      variant: 'modern',
      architectures: 'sm_75-86',
      description: 'Turing, Ampere GPUs (2018-2021)',
      examples: 'GTX 16/RTX 20/30 series, A100, RTX A1000-A6000'
    };
  } else if (smVersion >= 89 && smVersion <= 121) {
    // sm_89-120: Ada Lovelace, Hopper, Blackwell (2022-2024)
    return {
      variant: 'latest',
      architectures: 'sm_89-120',
      description: 'Ada Lovelace, Hopper, Blackwell GPUs (2022-2024)',
      examples: 'RTX 4090, H100, B100/B200, RTX PRO 6000 Blackwell, RTX 50 series'
    };
  } else {
    // Unsupported architecture
    return {
      variant: null,
      architectures: `sm_${smVersion}`,
      description: 'Unsupported GPU architecture',
      examples: 'GPU too old (<sm_50) or unknown architecture'
    };
  }
}

/**
 * Detect CUDA and cuDNN library paths dynamically
 * Returns an array of directories to include in LD_LIBRARY_PATH
 * Now includes ~/.local/share/swictation/gpu-libs as PRIMARY source
 */
/**
 * Detect actual npm installation path (handles nvm vs system-wide)
 * This is critical for service files to find the correct libraries
 */
function detectActualNpmInstallPath() {
  // __dirname is where this script is running from
  // For system-wide: /usr/local/lib/node_modules/swictation
  // For nvm: /home/user/.nvm/versions/node/vX.Y.Z/lib/node_modules/swictation

  // Return the actual installation directory
  return __dirname;
}

/**
 * Detect where npm global packages are installed
 * This helps find the native library path for LD_LIBRARY_PATH
 */
function detectNpmNativeLibPath() {
  const installDir = detectActualNpmInstallPath();
  return path.join(installDir, 'lib', 'native');
}

function detectCudaLibraryPaths() {
  const paths = [];

  // PRIORITY 1: User's GPU libs directory (our multi-architecture packages)
  const gpuLibsDir = path.join(os.homedir(), '.local', 'share', 'swictation', 'gpu-libs');
  if (fs.existsSync(gpuLibsDir)) {
    paths.push(gpuLibsDir);
  }

  // PRIORITY 2: Check common CUDA installation directories (system-wide fallback)
  const cudaDirs = [
    '/usr/local/cuda/lib64',
    '/usr/local/cuda/lib',
    '/usr/local/cuda-13/lib64',
    '/usr/local/cuda-13/lib',
    '/usr/local/cuda-12.9/lib64',
    '/usr/local/cuda-12.9/lib',
    '/usr/local/cuda-12/lib64',
    '/usr/local/cuda-12/lib',
  ];

  // Find directories that contain cuDNN or CUDA runtime
  for (const dir of cudaDirs) {
    try {
      if (fs.existsSync(dir)) {
        const files = fs.readdirSync(dir);
        // Check for cuDNN or CUDA runtime libraries
        if (files.some(f => f.startsWith('libcudnn.so') || f.startsWith('libcudart.so'))) {
          if (!paths.includes(dir)) {
            paths.push(dir);
          }
        }
      }
    } catch (err) {
      // Ignore errors from directories we can't read
    }
  }

  return paths;
}

async function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          redirectResponse.pipe(file);
          file.on('finish', () => {
            file.close();
            resolve();
          });
        }).on('error', (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
      } else {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

/**
 * Load expected checksums from checksums.txt
 * @returns {Map<string, string>} Map of filename -> sha512 hash
 */
function loadChecksums() {
  const checksumsPath = path.join(__dirname, 'checksums.txt');

  if (!fs.existsSync(checksumsPath)) {
    throw new Error('checksums.txt not found - package may be corrupted');
  }

  const content = fs.readFileSync(checksumsPath, 'utf8');
  const checksums = new Map();

  for (const line of content.split('\n')) {
    // Skip comments and empty lines
    if (line.trim().startsWith('#') || line.trim() === '') {
      continue;
    }

    // Parse "hash  filename" format
    const match = line.match(/^([a-f0-9]{128})\s+(.+)$/);
    if (match) {
      const [, hash, filename] = match;
      checksums.set(filename, hash);
    }
  }

  return checksums;
}

/**
 * Calculate SHA-512 checksum of a file
 * @param {string} filePath - Path to file
 * @returns {Promise<string>} SHA-512 hash in hex format
 */
function calculateChecksum(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha512');
    const stream = fs.createReadStream(filePath);

    stream.on('data', (chunk) => {
      hash.update(chunk);
    });

    stream.on('end', () => {
      resolve(hash.digest('hex'));
    });

    stream.on('error', (err) => {
      reject(err);
    });
  });
}

/**
 * Verify downloaded file checksum matches expected value
 * @param {string} filePath - Path to downloaded file
 * @param {string} filename - Original filename for lookup
 * @param {Map<string, string>} checksums - Expected checksums map
 * @throws {Error} If checksum doesn't match or file is missing from checksums
 */
async function verifyChecksum(filePath, filename, checksums) {
  const expectedChecksum = checksums.get(filename);

  if (!expectedChecksum) {
    throw new Error(`No checksum found for ${filename} - package may be corrupted`);
  }

  log('cyan', '  Verifying file integrity...');
  const actualChecksum = await calculateChecksum(filePath);

  if (actualChecksum !== expectedChecksum) {
    throw new Error(
      `SECURITY: Checksum mismatch for ${filename}!\n` +
      `  Expected: ${expectedChecksum}\n` +
      `  Actual:   ${actualChecksum}\n` +
      `  This could indicate a corrupted download or supply chain attack.\n` +
      `  DO NOT extract this file. Please report this issue.`
    );
  }

  log('green', `  ✓ Checksum verified (SHA-512)`);
}

async function downloadGPULibraries() {
  const hasGPU = detectNvidiaGPU();

  if (!hasGPU) {
    log('cyan', '\nℹ No NVIDIA GPU detected - skipping GPU library download');
    log('cyan', '  CPU-only mode will be used');
    return;
  }

  log('green', '\n✓ NVIDIA GPU detected!');
  log('cyan', '📦 Detecting GPU architecture and downloading optimized libraries...\n');

  // Detect GPU compute capability
  const gpuInfo = detectGPUComputeCapability();

  if (!gpuInfo.smVersion) {
    log('yellow', '⚠️  Could not detect GPU compute capability');
    log('cyan', '   Skipping GPU library download');
    log('cyan', '   You can manually download from:');
    log('cyan', '   https://github.com/robertelee78/swictation/releases/tag/gpu-libs-v1.2.0');
    return;
  }

  // Select appropriate package variant
  const packageInfo = selectGPUPackageVariant(gpuInfo.smVersion);

  if (!packageInfo.variant) {
    log('yellow', `⚠️  GPU architecture ${packageInfo.architectures} is not supported`);
    log('cyan', `   ${packageInfo.description}`);
    log('cyan', '   Supported architectures: sm_50 through sm_121');
    log('cyan', '   Your GPU may be too old or require a newer ONNX Runtime build');
    return;
  }

  log('cyan', `📦 Selected Package: ${packageInfo.variant.toUpperCase()}`);
  log('cyan', `   Architectures: ${packageInfo.architectures}`);
  log('cyan', `   Description: ${packageInfo.description}`);
  log('cyan', `   Examples: ${packageInfo.examples}\n`);

  // GPU libs v1.1.0: Multi-architecture CUDA support (sm_50-120)
  // ONNX Runtime 1.23.2, CUDA 12.9, cuDNN 9.15.1
  const GPU_LIBS_VERSION = '1.2.0';
  const variant = packageInfo.variant;
  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/gpu-libs-v${GPU_LIBS_VERSION}/cuda-libs-${variant}.tar.gz`;
  const tmpDir = path.join(os.tmpdir(), 'swictation-gpu-install');
  const tarPath = path.join(tmpDir, `cuda-libs-${variant}.tar.gz`);

  // Extract to user's home directory for GPU libs (shared across npm installs)
  const gpuLibsDir = path.join(os.homedir(), '.local', 'share', 'swictation', 'gpu-libs');

  try {
    // Load checksums for verification
    let checksums;
    try {
      checksums = loadChecksums();
      log('green', '  ✓ Loaded integrity checksums');
    } catch (err) {
      log('red', `  ✗ Failed to load checksums: ${err.message}`);
      throw new Error('Cannot proceed without checksums - package integrity cannot be verified');
    }

    // Create directories
    if (!fs.existsSync(tmpDir)) {
      fs.mkdirSync(tmpDir, { recursive: true });
    }
    if (!fs.existsSync(gpuLibsDir)) {
      fs.mkdirSync(gpuLibsDir, { recursive: true });
    }

    // Check if GPU libs are already installed by checking metadata file
    const configDir = path.join(os.homedir(), '.config', 'swictation');
    const gpuPackageInfoPath = path.join(configDir, 'gpu-package-info.json');

    let skipDownload = false;
    if (fs.existsSync(gpuPackageInfoPath)) {
      try {
        const existingMetadata = JSON.parse(fs.readFileSync(gpuPackageInfoPath, 'utf8'));
        if (existingMetadata.version === GPU_LIBS_VERSION && existingMetadata.variant === variant) {
          skipDownload = true;
          log('green', `  ✓ GPU libraries v${GPU_LIBS_VERSION} (${variant}) already installed`);
          log('cyan', `    Location: ${gpuLibsDir}`);
          log('cyan', `    Installed: ${existingMetadata.installedAt}`);
          log('cyan', `    Skipping download to save time and bandwidth`);
        }
      } catch (err) {
        log('yellow', `    Warning: Could not read GPU package metadata: ${err.message}`);
      }
    }

    if (!skipDownload) {
      // Download tarball
      log('cyan', `  Downloading ${variant} package...`);
      log('cyan', `  URL: ${releaseUrl}`);
      await downloadFile(releaseUrl, tarPath);
      log('green', `  ✓ Downloaded ${variant} package (~1.5GB)`);

      // Verify cryptographic checksum before extraction
      const filename = `cuda-libs-${variant}.tar.gz`;
      try {
        await verifyChecksum(tarPath, filename, checksums);
      } catch (err) {
        // Delete potentially malicious file
        fs.unlinkSync(tarPath);
        throw err;
      }

      // Extract tarball to gpu-libs directory
      log('cyan', '  Extracting libraries...');
      execSync(`tar -xzf "${tarPath}" -C "${tmpDir}"`, { stdio: 'inherit' });

      // Move libraries from extracted ${variant}/libs/ to gpu-libs directory
      const extractedLibsDir = path.join(tmpDir, variant, 'libs');
      if (fs.existsSync(extractedLibsDir)) {
        const libFiles = fs.readdirSync(extractedLibsDir);
        for (const file of libFiles) {
          const srcPath = path.join(extractedLibsDir, file);
          const destPath = path.join(gpuLibsDir, file);
          fs.copyFileSync(srcPath, destPath);
        }
        log('green', `  ✓ Extracted ${libFiles.length} libraries to ${gpuLibsDir}`);
      } else {
        throw new Error(`Expected directory not found: ${extractedLibsDir}`);
      }

      // Cleanup
      fs.unlinkSync(tarPath);
      execSync(`rm -rf "${path.join(tmpDir, variant)}"`, { stdio: 'ignore' });

      // Save GPU package info for systemd service generation
      const packageMetadata = {
        variant: packageInfo.variant,
        architectures: packageInfo.architectures,
        smVersion: gpuInfo.smVersion,
        computeCap: gpuInfo.computeCap,
        gpuName: gpuInfo.gpuName,
        version: GPU_LIBS_VERSION,
        libsPath: gpuLibsDir,
        installedAt: new Date().toISOString()
      };

      try {
        fs.writeFileSync(gpuPackageInfoPath, JSON.stringify(packageMetadata, null, 2));
        log('green', `   ✓ Saved package metadata to ${gpuPackageInfoPath}`);
      } catch (err) {
        log('yellow', `   ⚠️  Could not save package metadata: ${err.message}`);
      }
    }

    log('green', '\n✅ GPU acceleration enabled!');
    log('cyan', `   Architecture: ${packageInfo.architectures}`);
    log('cyan', `   Libraries: ${gpuLibsDir}`);
    log('cyan', '   Your system will use CUDA for faster transcription\n');

  } catch (err) {
    log('yellow', `\n⚠️  Failed to download GPU libraries: ${err.message}`);
    log('cyan', '   Continuing with CPU-only mode');
    log('cyan', '   You can manually download from:');
    log('cyan', `   ${releaseUrl}`);
    log('cyan', '\n   Manual installation:');
    log('cyan', `   1. Download: curl -L -o /tmp/cuda-libs-${variant}.tar.gz ${releaseUrl}`);
    log('cyan', `   2. Extract: tar -xzf /tmp/cuda-libs-${variant}.tar.gz -C /tmp`);
    log('cyan', `   3. Install: cp /tmp/${variant}/libs/*.so ${gpuLibsDir}/`);
  }
}

function detectOrtLibrary() {
  log('cyan', '\n🔍 Detecting ONNX Runtime library path...');

  // CRITICAL: Check npm package library FIRST (GPU-enabled, bundled with package)
  // This is the bundled library with CUDA support
  const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
  if (fs.existsSync(npmOrtLib)) {
    log('green', `✓ Found ONNX Runtime (bundled): ${npmOrtLib}`);
    log('cyan', '  Using bundled GPU-enabled library with CUDA provider support');
    return npmOrtLib;
  }

  log('yellow', `⚠️  Warning: GPU-enabled ONNX Runtime not found at ${npmOrtLib}`);
  log('yellow', '   Falling back to system Python installation (may be CPU-only)');


  try {
    // Try to find ONNX Runtime through Python (fallback, usually CPU-only)
    const ortPath = execSync(
      'python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), \'capi\'))"',
      { encoding: 'utf-8' }
    ).trim();

    if (!fs.existsSync(ortPath)) {
      log('yellow', '⚠️  Warning: ONNX Runtime capi directory not found at ' + ortPath);
      log('yellow', '   Daemon may not work correctly without onnxruntime-gpu');
      log('cyan', '   Install with: pip3 install onnxruntime-gpu');
      return null;
    }

    // Find the actual .so file
    const ortFiles = fs.readdirSync(ortPath).filter(f => f.startsWith('libonnxruntime.so'));

    if (ortFiles.length === 0) {
      log('yellow', '⚠️  Warning: Could not find libonnxruntime.so in ' + ortPath);
      log('yellow', '   Daemon may not work correctly');
      log('cyan', '   Install with: pip3 install onnxruntime-gpu');
      return null;
    }

    // Use the first (or only) .so file found
    const ortLibPath = path.join(ortPath, ortFiles[0]);
    log('yellow', `⚠️  Using Python ONNX Runtime: ${ortLibPath}`);
    log('yellow', '   Note: This may be CPU-only and lack CUDA support');

    // Store in a config file for systemd service generation
    const configDir = path.join(__dirname, 'config');
    const envFilePath = path.join(configDir, 'detected-environment.json');

    const envConfig = {
      ORT_DYLIB_PATH: ortLibPath,
      detected_at: new Date().toISOString(),
      onnxruntime_version: execSync('python3 -c "import onnxruntime; print(onnxruntime.__version__)"', { encoding: 'utf-8' }).trim(),
      warning: 'Using Python pip installation - may be CPU-only'
    };

    try {
      fs.writeFileSync(envFilePath, JSON.stringify(envConfig, null, 2));
      log('green', `✓ Saved environment config to ${envFilePath}`);
    } catch (err) {
      log('yellow', `Warning: Could not save environment config: ${err.message}`);
    }

    return ortLibPath;

  } catch (err) {
    log('yellow', '\n⚠️  Could not detect ONNX Runtime:');
    log('yellow', `   ${err.message}`);
    log('cyan', '\n📦 Please install onnxruntime-gpu for optimal performance:');
    log('cyan', '   pip3 install onnxruntime-gpu');
    log('cyan', '\n   The daemon will not work correctly without this library!');
    return null;
  }
}

function generateSystemdService(ortLibPath) {
  log('cyan', '\n⚙️  Generating systemd service files...');

  try {
    const systemdDir = path.join(os.homedir(), '.config', 'systemd', 'user');

    // Create systemd directory if it doesn't exist
    if (!fs.existsSync(systemdDir)) {
      fs.mkdirSync(systemdDir, { recursive: true });
      log('green', `✓ Created ${systemdDir}`);
    }

    // Detect display environment variables (used by both daemon and UI services)
    const runtimeDir = process.env.XDG_RUNTIME_DIR || `/run/user/${process.getuid()}`;
    let waylandDisplay = null;
    let xDisplay = process.env.DISPLAY || null;

    try {
      const sockets = fs.readdirSync(runtimeDir).filter(f => f.startsWith('wayland-'));
      if (sockets.length > 0) {
        // Use the first wayland socket (usually wayland-0 or wayland-1)
        waylandDisplay = sockets[0];
        log('cyan', `  Detected Wayland display: ${waylandDisplay}`);
      }
    } catch (err) {
      // Wayland socket not found, may be X11-only system
    }

    // 1. Generate daemon service from template
    const templatePath = path.join(__dirname, 'templates', 'swictation-daemon.service.template');
    if (!fs.existsSync(templatePath)) {
      log('yellow', `⚠️  Warning: Template not found at ${templatePath}`);
      log('yellow', '   Skipping daemon service generation');
    } else {
      let template = fs.readFileSync(templatePath, 'utf8');

      // Replace placeholders
      const installDir = __dirname; // npm package installation directory
      template = template.replace(/__INSTALL_DIR__/g, installDir);

      // CRITICAL: Detect GPU variant to determine which ONNX Runtime to use
      const configDir = path.join(os.homedir(), '.config', 'swictation');
      const gpuPackageInfoPath = path.join(configDir, 'gpu-package-info.json');

      let finalOrtLibPath, finalLdLibraryPath;
      let variant = 'latest'; // default

      // Try to read GPU package info to get variant
      if (fs.existsSync(gpuPackageInfoPath)) {
        try {
          const metadata = JSON.parse(fs.readFileSync(gpuPackageInfoPath, 'utf8'));
          variant = metadata.variant || 'latest';
          log('cyan', `  Detected GPU package variant: ${variant}`);
        } catch (err) {
          log('yellow', `  Warning: Could not read GPU package metadata: ${err.message}`);
        }
      }

      // Detect CUDA paths upfront (needed for logging)
      const detectedCudaPaths = detectCudaLibraryPaths();

      if (variant === 'legacy') {
        // LEGACY (Maxwell, Pascal, Volta): Use ONNX Runtime 1.23.2 from gpu-libs (CUDA 11.8)
        const gpuLibsDir = path.join(os.homedir(), '.local', 'share', 'swictation', 'gpu-libs');
        const legacyOrtPath = path.join(gpuLibsDir, 'libonnxruntime.so');

        if (fs.existsSync(legacyOrtPath)) {
          finalOrtLibPath = legacyOrtPath;
          // LD_LIBRARY_PATH only needs gpu-libs (includes all CUDA 11.8 libs)
          finalLdLibraryPath = gpuLibsDir;
          log('cyan', `  Using LEGACY ONNX Runtime 1.23.2: ${finalOrtLibPath}`);
          log('cyan', `  Using LEGACY CUDA 11.8 libraries: ${finalLdLibraryPath}`);
        } else {
          log('yellow', `  ⚠️  Legacy ONNX Runtime not found at ${legacyOrtPath}`);
          log('yellow', '     Falling back to npm bundled ONNX Runtime');
          finalOrtLibPath = ortLibPath;
          const nativeLibPath = detectNpmNativeLibPath();
          finalLdLibraryPath = [...detectedCudaPaths, nativeLibPath].join(':');
        }
      } else {
        // LATEST/MODERN: Use npm bundled ONNX Runtime (CUDA 12)
        finalOrtLibPath = ortLibPath;
        const nativeLibPath = detectNpmNativeLibPath();
        finalLdLibraryPath = [...detectedCudaPaths, nativeLibPath].join(':');
        log('cyan', `  Using npm bundled ONNX Runtime: ${finalOrtLibPath}`);
      }

      // CRITICAL: Trim all whitespace and newlines from paths to prevent malformed service file
      if (finalOrtLibPath) {
        const cleanPath = finalOrtLibPath.trim().replace(/[\r\n]/g, '');
        template = template.replace(/__ORT_DYLIB_PATH__/g, cleanPath);
        log('cyan', `  ORT_DYLIB_PATH set to: ${cleanPath}`);
      } else {
        log('yellow', '⚠️  Warning: ORT_DYLIB_PATH not detected');
        log('yellow', '   Service file will contain placeholder - you must set it manually');
      }

      // CRITICAL: Trim and clean LD_LIBRARY_PATH
      const cleanLdPath = finalLdLibraryPath.trim().replace(/[\r\n]/g, '');
      template = template.replace(/__LD_LIBRARY_PATH__/g, cleanLdPath);
      log('cyan', `  LD_LIBRARY_PATH set to: ${cleanLdPath}`);

      if (detectedCudaPaths.length > 0) {
        log('cyan', `  Detected ${detectedCudaPaths.length} CUDA library path(s):`);
        detectedCudaPaths.forEach(p => log('cyan', `    ${p}`));
      } else {
        log('yellow', '  ⚠️  No CUDA libraries detected (CPU-only mode)');
      }

      // Add display environment variables before ImportEnvironment
      if (waylandDisplay || xDisplay) {
        const envVars = [];
        if (waylandDisplay) envVars.push(`Environment="WAYLAND_DISPLAY=${waylandDisplay}"`);
        if (xDisplay) envVars.push(`Environment="DISPLAY=${xDisplay}"`);

        // Insert before ImportEnvironment= line
        template = template.replace(
          /ImportEnvironment=/,
          `${envVars.join('\n')}\n\n# Import full user environment for PulseAudio/PipeWire session\n# This ensures all audio devices are detected properly (4 devices instead of 1)\n# Required for microphone access in user session\nImportEnvironment=`
        );
      }

      // VALIDATION: Check for malformed Environment variables
      const validateServiceFile = (content) => {
        const lines = content.split('\n');
        for (let i = 0; i < lines.length; i++) {
          if (lines[i].includes('Environment=')) {
            const quoteCnt = (lines[i].match(/"/g) || []).length;
            if (quoteCnt % 2 !== 0) {
              throw new Error(`Malformed Environment variable at line ${i+1}: ${lines[i]}`);
            }
          }
        }
      };

      try {
        validateServiceFile(template);
        log('green', '  ✓ Service file validation passed');
      } catch (err) {
        log('red', `  ✗ Service file validation failed: ${err.message}`);
        throw err;
      }

      // Write daemon service file
      const daemonServicePath = path.join(systemdDir, 'swictation-daemon.service');
      fs.writeFileSync(daemonServicePath, template);
      log('green', `✓ Generated daemon service: ${daemonServicePath}`);

      if (ortLibPath) {
        log('cyan', '  Service configured with detected ONNX Runtime path');
      } else {
        log('yellow', '  ⚠️  Please edit the service file to set ORT_DYLIB_PATH manually');
      }
    }

    // 2. Install UI service (template-based, like daemon service)
    const uiServiceTemplate = path.join(__dirname, 'templates', 'swictation-ui.service.template');
    if (fs.existsSync(uiServiceTemplate)) {
      let uiTemplate = fs.readFileSync(uiServiceTemplate, 'utf8');

      // Replace placeholders
      uiTemplate = uiTemplate.replace(/__INSTALL_DIR__/g, __dirname);
      uiTemplate = uiTemplate.replace(/__DISPLAY__/g, xDisplay || ':0');
      uiTemplate = uiTemplate.replace(/__WAYLAND_DISPLAY__/g, waylandDisplay || 'wayland-0');

      const uiServiceDest = path.join(systemdDir, 'swictation-ui.service');
      fs.writeFileSync(uiServiceDest, uiTemplate);
      log('green', `✓ Installed UI service: ${uiServiceDest}`);
    } else {
      log('yellow', `⚠️  Warning: UI service template not found at ${uiServiceTemplate}`);
      log('yellow', '   You can manually create it later');
    }

    // CRITICAL: Reload systemd to pick up service file changes
    // Without this, systemd uses cached versions and services fail to start with old paths
    log('cyan', '\n🔄 Reloading systemd daemon...');
    try {
      execSync('systemctl --user daemon-reload', { stdio: 'ignore' });
      log('green', '✓ Systemd daemon reloaded - service files updated');
    } catch (err) {
      log('yellow', `⚠️  Could not reload systemd: ${err.message}`);
      log('yellow', '   You may need to run manually: systemctl --user daemon-reload');
    }

  } catch (err) {
    log('yellow', `⚠️  Failed to generate systemd services: ${err.message}`);
    log('cyan', '  You can manually create them later using: swictation setup');
  }
}

/**
 * Phase 2: Interactive config migration with pacman/apt-style prompts
 * Handles conflicts between old and new config files
 */
async function interactiveConfigMigration() {
  log('cyan', '\n📝 Checking configuration files...');

  const configDir = path.join(os.homedir(), '.config', 'swictation');
  const configPath = path.join(configDir, 'config.toml');
  const newConfigTemplate = path.join(__dirname, 'config', 'config.toml');

  // If no existing config, just copy the template
  if (!fs.existsSync(configPath)) {
    if (fs.existsSync(newConfigTemplate)) {
      try {
        fs.copyFileSync(newConfigTemplate, configPath);
        log('green', `✓ Created config file: ${configPath}`);
      } catch (err) {
        log('yellow', `⚠️  Could not create config: ${err.message}`);
      }
    }
    return;
  }

  // Check if new template exists
  if (!fs.existsSync(newConfigTemplate)) {
    // No template in package - daemon will generate default config on first run
    return;
  }

  // Read both configs
  let oldConfig, newConfig;
  try {
    oldConfig = fs.readFileSync(configPath, 'utf8');
    newConfig = fs.readFileSync(newConfigTemplate, 'utf8');
  } catch (err) {
    log('yellow', `⚠️  Error reading config files: ${err.message}`);
    return;
  }

  // If configs are identical, no action needed
  if (oldConfig === newConfig) {
    log('green', '✓ Config file is up to date');
    return;
  }

  // Configs differ - offer migration options
  log('yellow', '\n⚠️  Config file exists and differs from new template');
  log('cyan', '\nOptions:');
  log('cyan', '  [K] Keep    - Keep your current config (default)');
  log('cyan', '  [N] New     - Replace with new config (backup old)');
  log('cyan', '  [M] Merge   - Keep old, add new required fields');
  log('cyan', '  [D] Diff    - Show differences');
  log('cyan', '  [S] Skip    - Continue without changes');

  // For non-interactive installs, default to Keep
  if (!process.stdin.isTTY) {
    log('green', '\n✓ Non-interactive mode: Keeping existing config');
    log('cyan', '  Tip: Run "swictation setup" to review config changes');
    return;
  }

  // Interactive prompt (simplified for postinstall)
  log('yellow', '\n⚠️  Interactive mode not available during postinstall');
  log('cyan', '   Defaulting to: Keep existing config');
  log('cyan', '   New config template available at:');
  log('cyan', `   ${newConfigTemplate}`);
  log('cyan', '\n   To update config manually:');
  log('cyan', `   diff ${configPath} ${newConfigTemplate}`);
  log('green', '\n✓ Kept existing config');
}

/**
 * Phase 3: Detect GPU VRAM for intelligent model selection
 * Prevents loading models that are too large for available VRAM
 */
function detectGPUVRAM() {
  log('cyan', '\n🎮 Detecting GPU capabilities...');

  const gpuInfo = {
    hasGPU: false,
    gpuName: null,
    vramMB: 0,
    vramGB: 0,
    cudaVersion: null,
    driverVersion: null,
    recommendedModel: null
  };

  if (!detectNvidiaGPU()) {
    log('cyan', '  No NVIDIA GPU detected - CPU mode will be used');
    return gpuInfo;
  }

  gpuInfo.hasGPU = true;

  try {
    // Get comprehensive GPU information
    const gpuData = execSync(
      'nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader,nounits',
      { encoding: 'utf8' }
    ).trim();

    const [name, vramMB, driver] = gpuData.split(',').map(s => s.trim());

    gpuInfo.gpuName = name;
    gpuInfo.vramMB = parseInt(vramMB);
    gpuInfo.vramGB = Math.round(gpuInfo.vramMB / 1024);
    gpuInfo.driverVersion = driver;

    // Try to get CUDA version
    try {
      const cudaVersion = execSync('nvidia-smi | grep "CUDA Version" | awk \'{print $9}\'', { encoding: 'utf8' }).trim();
      if (cudaVersion) {
        gpuInfo.cudaVersion = cudaVersion;
      }
    } catch {
      // CUDA version detection optional
    }

    log('green', `✓ GPU Detected: ${gpuInfo.gpuName}`);
    log('cyan', `  VRAM: ${gpuInfo.vramGB}GB (${gpuInfo.vramMB}MB)`);
    log('cyan', `  Driver: ${gpuInfo.driverVersion}`);
    if (gpuInfo.cudaVersion) {
      log('cyan', `  CUDA: ${gpuInfo.cudaVersion}`);
    }

    // Intelligent model recommendation based on VRAM
    // Based on empirical data from real-world testing:
    // 0.6B model: ~3.5GB VRAM (fits in 4GB with headroom) - VERIFIED ON RTX A1000
    // 1.1B model: ~6GB VRAM (needs at least 6GB for safety)
    if (gpuInfo.vramMB >= 6000) {
      // 6GB+ VRAM: Can safely run 1.1B model
      gpuInfo.recommendedModel = '1.1b-gpu';
      log('green', `  ✓ Sufficient VRAM for 1.1B model (best quality)`);
    } else if (gpuInfo.vramMB >= 3500) {
      // 3.5-6GB VRAM: Run 0.6B model (proven safe on 4GB)
      // This includes exactly 4GB GPUs like RTX A1000
      gpuInfo.recommendedModel = '0.6b-gpu';
      if (gpuInfo.vramMB >= 4000) {
        log('green', `  ✓ VRAM sufficient for 0.6B GPU model`);
      } else {
        log('yellow', `  ⚠️  Limited VRAM - Recommending 0.6B model`);
      }
      log('cyan', `     (1.1B model requires ~6GB VRAM)`);
    } else {
      // <3.5GB VRAM: Too little for GPU acceleration
      gpuInfo.recommendedModel = 'cpu-only';
      log('yellow', `  ⚠️  Insufficient VRAM for GPU models`);
      log('cyan', `     GPU models require minimum 3.5GB VRAM`);
      log('cyan', `     Falling back to CPU-only mode`);
    }

    // Save GPU info for later use by daemon
    const configDir = path.join(os.homedir(), '.config', 'swictation');
    const gpuInfoPath = path.join(configDir, 'gpu-info.json');

    try {
      fs.writeFileSync(gpuInfoPath, JSON.stringify(gpuInfo, null, 2));
      log('green', `  ✓ Saved GPU info to ${gpuInfoPath}`);
    } catch (err) {
      log('yellow', `  ⚠️  Could not save GPU info: ${err.message}`);
    }

  } catch (err) {
    log('yellow', `⚠️  Error detecting GPU details: ${err.message}`);
    log('cyan', '   GPU detected but could not read specifications');
  }

  return gpuInfo;
}

/**
 * Test-load a specific model to verify it actually works
 * @param {string} modelName - Model name (e.g., '1.1b-gpu', '0.6b-gpu')
 * @param {string} daemonBin - Path to daemon binary
 * @param {string} ortLibPath - Path to ONNX Runtime library
 * @returns {Promise<{success: boolean, model: string, reason?: string}>}
 */
async function testLoadModel(modelName, daemonBin, ortLibPath) {
  log('cyan', `  🔄 Test-loading ${modelName} model (max 30s)...`);

  // Create a minimal temporary config for testing
  const configDir = path.join(os.homedir(), '.config', 'swictation');
  const configPath = path.join(configDir, 'config.toml');
  const needsTempConfig = !fs.existsSync(configPath);

  if (needsTempConfig) {
    try {
      fs.mkdirSync(configDir, { recursive: true });

      // Create a proper working config
      const modelDir = path.join(os.homedir(), '.local', 'share', 'swictation', 'models');
      const properConfig = `# Swictation Configuration
# Generated by postinstall script

# Unix socket for CLI control
socket_path = "/tmp/swictation.sock"

# VAD (Voice Activity Detection) settings
vad_model_path = "${modelDir}/silero-vad/silero_vad.onnx"
vad_threshold = 0.25
vad_min_silence = 0.8
vad_min_speech = 0.25
vad_max_speech = 30.0

# STT (Speech-to-Text) settings
# Auto-selects model based on available GPU VRAM
stt_model_override = "auto"
stt_0_6b_model_path = "${modelDir}/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx"
stt_1_1b_model_path = "${modelDir}/parakeet-tdt-1.1b-onnx"
num_threads = 4

# Corrections Engine - Phonetic matching threshold
# Controls how fuzzy the phonetic matching is for learned corrections
# - 0.0 = Exact match only (very strict)
# - 0.3 = Default (balanced fuzzy matching)
# - 1.0 = Very fuzzy (may over-correct)
phonetic_threshold = 0.3

[hotkeys]
# Main toggle hotkey for start/stop recording
toggle = "Super+Shift+D"
# Push-to-talk (hold to record)
push_to_talk = "Super+Space"
`;
      fs.writeFileSync(configPath, properConfig);
      log('green', '✓ Created default configuration');
    } catch (err) {
      log('yellow', `    ⚠️  Could not create config: ${err.message}`);
    }
  }

  // Detect CUDA library paths dynamically (same as systemd service)
  const cudaPaths = detectCudaLibraryPaths();
  const nativeLibPath = path.join(__dirname, 'lib', 'native');
  const ldLibraryPath = [...cudaPaths, nativeLibPath].join(':');

  const modelFlag = `--test-model=${modelName}`;
  const env = {
    ...process.env,
    ORT_DYLIB_PATH: ortLibPath,
    LD_LIBRARY_PATH: ldLibraryPath,
    CUDA_HOME: '/usr/local/cuda',
    RUST_LOG: 'info'
  };

  try {
    const output = execSync(
      `timeout 30s "${daemonBin}" ${modelFlag} --dry-run 2>&1`,
      { encoding: 'utf8', env, stdio: 'pipe' }
    );

    // Check for success indicators in dry-run output
    // Daemon outputs "Dry-run complete" and "Would load: MODEL_NAME" in dry-run mode
    if (output.includes('Dry-run complete') || output.includes('Would load:')) {
      log('green', `    ✓ ${modelName} verified (dry-run passed)`);
      return { success: true, model: modelName };
    } else {
      log('yellow', `    ⚠️  ${modelName} verification uncertain (no success indicator)`);
      return { success: false, model: modelName, reason: 'No success indicator in dry-run output' };
    }
  } catch (err) {
    // execSync throws even on success if exit code is non-zero
    // But check if the output still contains success indicators
    const output = err.stdout || '';
    if (output.includes('Dry-run complete') || output.includes('Would load:')) {
      log('green', `    ✓ ${modelName} verified (dry-run passed)`);
      return { success: true, model: modelName };
    }

    log('yellow', `    ✗ ${modelName} failed to load`);
    log('cyan', `      Error: ${err.message.split('\n')[0]}`);
    return { success: false, model: modelName, reason: err.message };
  }
}

/**
 * Test models in order from best to worst, returning the first that works
 * @param {object} gpuInfo - GPU information from detectGPUVRAM()
 * @param {string} daemonBin - Path to daemon binary
 * @param {string} ortLibPath - Path to ONNX Runtime library
 * @returns {Promise<{recommendedModel: string, tested: boolean, vramVerified?: boolean, fallbackToCpu?: boolean}>}
 */
async function testModelsInOrder(gpuInfo, daemonBin, ortLibPath) {
  log('cyan', '\n🧪 Testing models on your GPU...');
  log('cyan', `   GPU: ${gpuInfo.gpuName || 'Unknown'} with ${gpuInfo.vramGB}GB VRAM`);

  if (!gpuInfo.hasGPU) {
    log('cyan', '  No GPU - skipping model tests');
    return { recommendedModel: 'cpu-only', tested: false };
  }

  if (!fs.existsSync(daemonBin)) {
    log('yellow', '  ⚠️  Daemon binary not found, skipping model tests');
    return { recommendedModel: gpuInfo.recommendedModel, tested: false };
  }

  // Test models in order from best to worst
  const modelsToTest = [];

  if (gpuInfo.vramMB >= 5500) {
    modelsToTest.push('1.1b-gpu');
  }
  if (gpuInfo.vramMB >= 3500) {
    modelsToTest.push('0.6b-gpu');
  }

  if (modelsToTest.length === 0) {
    log('cyan', '  Insufficient VRAM for GPU models');
    return { recommendedModel: 'cpu-only', tested: false };
  }

  log('cyan', `  Testing ${modelsToTest.length} model(s)...\n`);

  for (const model of modelsToTest) {
    const result = await testLoadModel(model, daemonBin, ortLibPath);
    if (result.success) {
      log('green', `\n  ✓ Selected: ${model} (verified working)`);

      // Update config with tested model (if config exists and is on "auto")
      updateConfigWithTestedModel(model);

      return {
        recommendedModel: model,
        tested: true,
        vramVerified: true
      };
    }
  }

  // All tests failed - fall back to CPU
  log('yellow', '\n  ⚠️  All GPU models failed to load');
  log('cyan', '     Falling back to CPU-only mode');
  return {
    recommendedModel: 'cpu-only',
    tested: true,
    fallbackToCpu: true
  };
}

function detectSystemCapabilities() {
  const capabilities = {
    hasGPU: false,
    gpuName: null,
    gpuMemoryMB: 0,
    cpuCores: os.cpus().length,
    totalRAMGB: Math.round(os.totalmem() / (1024 * 1024 * 1024))
  };

  // Detect NVIDIA GPU and get details
  if (detectNvidiaGPU()) {
    capabilities.hasGPU = true;

    try {
      // Get GPU memory
      const gpuMemory = execSync('nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits', { encoding: 'utf8' });
      capabilities.gpuMemoryMB = parseInt(gpuMemory.trim());

      // Get GPU name
      const gpuName = execSync('nvidia-smi --query-gpu=name --format=csv,noheader', { encoding: 'utf8' });
      capabilities.gpuName = gpuName.trim();
    } catch (err) {
      // GPU detected but couldn't get details
      capabilities.gpuMemoryMB = 0;
    }
  }

  return capabilities;
}

function recommendOptimalModel(capabilities) {
  // Recommendation logic based on hardware
  if (capabilities.hasGPU) {
    if (capabilities.gpuMemoryMB >= 4000) {
      // High-end GPU: Recommend 1.1B model (best quality, full GPU acceleration)
      return {
        model: '1.1b',
        reason: `GPU detected: ${capabilities.gpuName} (${Math.round(capabilities.gpuMemoryMB/1024)}GB VRAM)`,
        description: 'Best quality - Full GPU acceleration with FP32 precision',
        size: '~75MB download (FP32 + INT8 versions)',
        performance: '62x realtime speed on GPU'
      };
    } else {
      // Lower VRAM: Recommend 0.6B model
      return {
        model: '0.6b',
        reason: `GPU detected but limited VRAM (${Math.round(capabilities.gpuMemoryMB/1024)}GB)`,
        description: 'Lighter model for lower VRAM systems',
        size: '~111MB',
        performance: 'Fast on GPU'
      };
    }
  } else {
    // CPU-only systems
    if (capabilities.cpuCores >= 8 && capabilities.totalRAMGB >= 16) {
      // Powerful CPU: Can handle 1.1B INT8
      return {
        model: '1.1b',
        reason: `Powerful CPU (${capabilities.cpuCores} cores, ${capabilities.totalRAMGB}GB RAM)`,
        description: 'INT8 quantized for fast CPU inference',
        size: '~1.1GB download',
        performance: 'Good CPU performance with INT8'
      };
    } else {
      // Weaker CPU: Recommend 0.6B
      return {
        model: '0.6b',
        reason: `CPU-only system (${capabilities.cpuCores} cores, ${capabilities.totalRAMGB}GB RAM)`,
        description: 'Lighter model for CPU-only systems',
        size: '~111MB',
        performance: 'Optimized for CPU'
      };
    }
  }
}

/**
 * Phase 5: Wayland-specific setup (ydotool + GNOME shortcuts)
 * Detects Wayland and offers automated setup for text injection and hotkeys
 */
/**
 * Phase 5: Auto-install system dependencies based on detected environment
 * Detects display server (X11/Wayland), desktop environment, and installs required packages
 */
async function setupWaylandIntegration() {
  log('cyan', '\n🔍 Detecting system environment...');

  const isWayland = process.env.WAYLAND_DISPLAY || process.env.XDG_SESSION_TYPE === 'wayland';
  const isGnome = (process.env.XDG_CURRENT_DESKTOP === 'ubuntu:GNOME' ||
                   process.env.XDG_CURRENT_DESKTOP === 'GNOME');
  // SWAYSOCK is the definitive Sway indicator (XDG_CURRENT_DESKTOP often not set in Sway)
  const isSway = !!process.env.SWAYSOCK || process.env.XDG_CURRENT_DESKTOP?.toLowerCase().includes('sway');
  const isKDE = process.env.XDG_CURRENT_DESKTOP?.toLowerCase().includes('kde');

  const results = {
    displayServer: isWayland ? 'wayland' : 'x11',
    desktopEnvironment: isGnome ? 'gnome' : (isSway ? 'sway' : (isKDE ? 'kde' : 'unknown')),
    textInjectionTool: null,
    pipewireInstalled: false,
    gnomeShortcuts: false
  };

  // Display detection results
  log('green', `✓ Display Server: ${results.displayServer.toUpperCase()}`);
  log('green', `✓ Desktop Environment: ${results.desktopEnvironment.toUpperCase()}`);

  // 1. Install text injection tool based on environment
  if (isWayland) {
    log('cyan', '\n📱 Wayland detected - setting up text injection...');

    if (isGnome) {
      // GNOME Wayland needs ydotool with full setup (udev, input group, etc.)
      try {
        execSync('which ydotool', { stdio: 'ignore' });
        log('green', '✓ ydotool already installed');
        results.textInjectionTool = 'ydotool';
      } catch {
        log('yellow', '⚠️  ydotool not installed (required for GNOME Wayland)');
        log('cyan', '\n📦 Installing ydotool with full setup...');

        try {
          const setupScript = path.join(__dirname, 'scripts', 'setup-ydotool.sh');
          if (fs.existsSync(setupScript)) {
            execSync(`bash "${setupScript}"`, { stdio: 'inherit' });
            results.textInjectionTool = 'ydotool';
            log('green', '✓ ydotool setup completed');
          } else {
            log('yellow', '  Setup script not found, falling back to direct install');
            if (installPackage('ydotool', 'ydotool')) {
              results.textInjectionTool = 'ydotool';
              log('yellow', '  ⚠️  Note: May need manual udev/group configuration');
              log('cyan', '  Run: ./scripts/setup-ydotool.sh for complete setup');
            }
          }
        } catch (err) {
          log('yellow', `  ⚠️  Automated setup failed: ${err.message}`);
          log('cyan', '  Install manually: sudo apt install ydotool');
          log('cyan', '  Or run: ./scripts/setup-ydotool.sh');
        }
      }

      // Setup GNOME keyboard shortcuts
      log('cyan', '\n⌨️  Configuring GNOME keyboard shortcuts...');
      try {
        const setupScript = path.join(__dirname, 'scripts', 'setup-gnome-shortcuts.sh');
        if (fs.existsSync(setupScript)) {
          execSync(`bash "${setupScript}"`, { stdio: 'inherit' });
          results.gnomeShortcuts = true;
          log('green', '✓ GNOME keyboard shortcuts configured');
          log('cyan', '  Hotkey: Super+Shift+D to toggle recording');
        } else {
          log('yellow', '  Setup script not found, skipping');
        }
      } catch (err) {
        log('yellow', `  ⚠️  Could not auto-configure shortcuts: ${err.message}`);
        log('cyan', '  Run manually: ./scripts/setup-gnome-shortcuts.sh');
      }

    } else if (isSway || isKDE) {
      // Sway and KDE Wayland need wtype
      try {
        execSync('which wtype', { stdio: 'ignore' });
        log('green', '✓ wtype already installed');
        results.textInjectionTool = 'wtype';
      } catch {
        log('yellow', `⚠️  wtype not installed (required for ${results.desktopEnvironment.toUpperCase()} Wayland)`);
        log('cyan', '\n📦 Installing wtype...');

        if (installPackage('wtype', 'wtype')) {
          results.textInjectionTool = 'wtype';
        }
      }

      if (isSway) {
        log('cyan', '\n⌨️  Sway detected - hotkeys require manual configuration');
        log('cyan', '  Add to ~/.config/sway/config:');
        log('cyan', '  bindsym $mod+Shift+d exec sh -c \'echo "{\\"action\\": \\"toggle\\"}" | nc -U /tmp/swictation.sock\'');
      }
    } else {
      // Unknown Wayland compositor - suggest wtype as generic option
      log('yellow', '\n⚠️  Unknown Wayland compositor detected');
      log('cyan', '  Attempting to install wtype (generic Wayland text injection)...');

      try {
        execSync('which wtype', { stdio: 'ignore' });
        log('green', '✓ wtype already installed');
        results.textInjectionTool = 'wtype';
      } catch {
        if (installPackage('wtype', 'wtype')) {
          results.textInjectionTool = 'wtype';
        }
      }
    }

  } else {
    // X11 environment needs xdotool
    log('cyan', '\n🖥️  X11 detected - setting up text injection...');

    try {
      execSync('which xdotool', { stdio: 'ignore' });
      log('green', '✓ xdotool already installed');
      results.textInjectionTool = 'xdotool';
    } catch {
      log('yellow', '⚠️  xdotool not installed (required for X11)');
      log('cyan', '\n📦 Installing xdotool...');

      if (installPackage('xdotool', 'xdotool')) {
        results.textInjectionTool = 'xdotool';
      }
    }
  }

  // 2. Check and install pipewire (needed for all environments)
  log('cyan', '\n🎵 Checking for pipewire (audio system)...');

  try {
    execSync('which pipewire', { stdio: 'ignore' });
    log('green', '✓ pipewire already installed');
    results.pipewireInstalled = true;
  } catch {
    log('yellow', '⚠️  pipewire not installed (recommended for audio capture)');
    log('cyan', '\n📦 Installing pipewire...');

    // Different package names for different distros
    const pkgManager = detectPackageManager();
    let pipewirePkg = 'pipewire';

    if (pkgManager?.name === 'apt') {
      pipewirePkg = 'pipewire pipewire-pulse';
    } else if (pkgManager?.name === 'dnf') {
      pipewirePkg = 'pipewire pipewire-pulseaudio';
    } else if (pkgManager?.name === 'pacman') {
      pipewirePkg = 'pipewire pipewire-pulse';
    }

    if (installPackage(pipewirePkg, 'pipewire')) {
      results.pipewireInstalled = true;
      log('cyan', '  Note: You may need to restart your session for pipewire to take effect');
    }
  }

  // 3. Check for netcat (needed for socket communication)
  try {
    execSync('which nc', { stdio: 'ignore' });
  } catch {
    log('cyan', '\n🔌 Installing netcat (for CLI communication)...');
    const pkgManager = detectPackageManager();
    let netcatPkg = 'netcat';

    if (pkgManager?.name === 'apt') {
      netcatPkg = 'netcat-openbsd';
    } else if (pkgManager?.name === 'dnf') {
      netcatPkg = 'nmap-ncat';
    } else if (pkgManager?.name === 'pacman') {
      netcatPkg = 'openbsd-netcat';
    }

    installPackage(netcatPkg, 'netcat');
  }

  return results;
}

/**
 * Phase 6: Auto-enable and start systemd service
 * Enables service to start on login and attempts to start it now
 */
async function enableAndStartService() {
  log('cyan', '\n🚀 Enabling systemd service...');

  const serviceName = 'swictation-daemon.service';
  const results = {
    enabled: false,
    started: false
  };

  try {
    // Enable service (start on login)
    try {
      execSync(`systemctl --user enable ${serviceName}`, { stdio: 'ignore' });
      log('green', '✓ Service enabled (will start on login)');
      results.enabled = true;
    } catch (err) {
      log('yellow', `  ⚠️  Could not enable service: ${err.message}`);
      log('cyan', '  You can enable manually: systemctl --user enable swictation-daemon.service');
    }

    // Try to start service now
    log('cyan', '\n▶️  Starting daemon service...');
    try {
      execSync(`systemctl --user start ${serviceName}`, { stdio: 'pipe' });

      // Wait a moment for service to initialize
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Check if service started successfully
      try {
        const status = execSync(`systemctl --user is-active ${serviceName}`, { encoding: 'utf8' }).trim();
        if (status === 'active') {
          log('green', '✓ Daemon started successfully');
          results.started = true;
        } else {
          log('yellow', `  Service status: ${status}`);
        }
      } catch {
        log('yellow', '  ⚠️  Service may not be running');
        log('cyan', '  Check status: systemctl --user status swictation-daemon.service');
      }
    } catch (err) {
      // Service start may fail if models aren't downloaded yet - this is expected
      if (err.message.includes('models') || err.message.includes('No such file')) {
        log('cyan', '  ℹ Service will start after downloading models');
        log('cyan', '  Download models: swictation download-model 1.1b-gpu');
      } else {
        log('yellow', `  ⚠️  Could not start service: ${err.message}`);
        log('cyan', '  Start manually: swictation start');
      }
    }
  } catch (err) {
    log('yellow', `⚠️  Service setup failed: ${err.message}`);
    log('cyan', 'Service can be started manually after model download');
  }

  return results;
}

/**
 * Phase 7: Check if NVIDIA hibernation configuration is needed
 * Detects laptop + NVIDIA GPU and checks if hibernation support is configured
 */
async function checkNvidiaHibernation() {
  try {
    const status = checkNvidiaHibernationStatus();

    if (!status.isLaptop) {
      log('green', '✓ Not a laptop - NVIDIA hibernation check skipped');
      return;
    }

    if (!status.hasNvidiaGpu) {
      log('cyan', 'ℹ No NVIDIA GPU detected');
      return;
    }

    log('green', `✓ Detected: Laptop with NVIDIA GPU`);
    log('cyan', `  Distribution: ${status.distribution}`);

    if (status.isConfigured) {
      log('green', '✓ NVIDIA hibernation support already configured');
      return;
    }

    // NVIDIA GPU on laptop without hibernation configuration
    log('yellow', '\n⚠️  NVIDIA Hibernation Support Not Configured');
    log('yellow', '   Without this, your GPU may enter a defunct state after hibernation,');
    log('yellow', '   causing CUDA errors (719/999) and requiring a reboot.\n');
    log('cyan', '   To configure hibernation support, run:');
    log('green', '   sudo swictation setup\n');
    log('cyan', '   This will:');
    log('cyan', '   - Set NVreg_PreserveVideoMemoryAllocations=1');
    log('cyan', '   - Create /etc/modprobe.d/nvidia-power-management.conf');
    log('cyan', '   - Update initramfs');
    log('cyan', '   - Require a reboot\n');

  } catch (err) {
    log('yellow', `⚠️  Error checking NVIDIA hibernation status: ${err.message}`);
  }
}

/**
 * Check if a specific model is already downloaded
 * @param {string} modelName - Model name (e.g., '0.6b-gpu', '1.1b-gpu', 'cpu-only')
 * @returns {boolean}
 */
function isModelDownloaded(modelName) {
  const modelDir = path.join(os.homedir(), '.local', 'share', 'swictation', 'models');

  // Map model names to directory names
  const modelDirs = {
    '0.6b': 'sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx',
    '0.6b-gpu': 'sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx',
    '1.1b': 'sherpa-onnx-nemo-parakeet-tdt-1.1b-v3-onnx',
    '1.1b-gpu': 'sherpa-onnx-nemo-parakeet-tdt-1.1b-v3-onnx',
    'cpu-only': 'sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx-int8'
  };

  const targetDir = modelDirs[modelName];
  if (!targetDir) return false;

  const modelPath = path.join(modelDir, targetDir);
  if (!fs.existsSync(modelPath)) return false;

  // Verify required model files exist
  const requiredFiles = ['encoder.onnx', 'decoder.onnx', 'tokens.txt'];
  for (const file of requiredFiles) {
    const filePath = path.join(modelPath, file);
    // For INT8 models, check for .int8.onnx variants
    const int8FilePath = path.join(modelPath, file.replace('.onnx', '.int8.onnx'));

    if (!fs.existsSync(filePath) && !fs.existsSync(int8FilePath)) {
      return false;
    }
  }

  return true;
}

/**
 * Auto-download recommended model if not present
 * @param {string} recommendedModel - Model to download (e.g., '0.6b-gpu')
 */
async function autoDownloadModel(recommendedModel) {
  // Check if model already exists
  if (isModelDownloaded(recommendedModel)) {
    log('green', `✓ Recommended model (${recommendedModel}) already downloaded`);
    return true;
  }

  log('cyan', `\n📥 Auto-downloading recommended model: ${recommendedModel}`);
  log('cyan', '   This is a one-time download (may take a few minutes)...\n');

  try {
    const ModelDownloader = require(path.join(__dirname, 'lib', 'model-downloader.js'));

    // Map model names to downloader keys
    const modelMap = {
      '0.6b-gpu': '0.6b',
      '0.6b': '0.6b',
      '1.1b-gpu': '1.1b',
      '1.1b': '1.1b',
      'cpu-only': '0.6b'  // CPU-only uses same 0.6b model
    };

    const modelKey = modelMap[recommendedModel];
    if (!modelKey) {
      log('yellow', `⚠️  Unknown model type: ${recommendedModel}`);
      return false;
    }

    const downloader = new ModelDownloader({ force: false });

    // Download VAD + recommended model
    await downloader.downloadModels(['vad', modelKey]);

    log('green', '\n✓ Model downloaded successfully!');
    return true;
  } catch (err) {
    log('yellow', `⚠️  Auto-download failed: ${err.message}`);
    log('cyan', '   You can download manually later with:');
    log('green', `   swictation download-model ${recommendedModel}`);
    return false;
  }
}

async function showNextSteps() {
  log('green', '\n✨ Swictation installed successfully!');

  // Try to read GPU info from detection
  const gpuInfoPath = path.join(os.homedir(), '.config', 'swictation', 'gpu-info.json');
  let gpuInfo = null;
  let recommendation;

  try {
    if (fs.existsSync(gpuInfoPath)) {
      gpuInfo = JSON.parse(fs.readFileSync(gpuInfoPath, 'utf8'));
    }
  } catch {
    // Fall back to old detection method
  }

  // Use GPU info if available, otherwise fall back to old method
  if (gpuInfo && gpuInfo.recommendedModel) {
    log('cyan', '\n📊 System Detection:');
    if (gpuInfo.hasGPU) {
      console.log(`   GPU: ${gpuInfo.gpuName} (${gpuInfo.vramGB}GB VRAM)`);
      console.log(`   Driver: ${gpuInfo.driverVersion}`);
      if (gpuInfo.cudaVersion) {
        console.log(`   CUDA: ${gpuInfo.cudaVersion}`);
      }
    } else {
      const capabilities = detectSystemCapabilities();
      console.log(`   CPU: ${capabilities.cpuCores} cores, ${capabilities.totalRAMGB}GB RAM`);
    }
    console.log('');

    log('cyan', '🎯 Recommended Model:');
    if (gpuInfo.recommendedModel === '1.1b-gpu' || gpuInfo.recommendedModel === '1.1b') {
      log('green', '   1.1B GPU - Best quality with full CUDA acceleration');
      console.log('   Size: ~75MB download (FP32 + INT8 versions)');
      console.log('   Performance: 62x realtime speed on GPU');
      console.log('   VRAM: ~6GB required');
    } else if (gpuInfo.recommendedModel === '0.6b-gpu' || gpuInfo.recommendedModel === '0.6b') {
      log('green', '   0.6B GPU - Optimized for 4GB VRAM systems');
      console.log('   Size: ~111MB download');
      console.log('   Performance: Fast GPU acceleration');
      console.log('   VRAM: ~4GB required');
    } else {
      log('cyan', '   CPU-optimized models');
      console.log('   Multiple sizes available (0.6B - 1.1B)');
      console.log('   Note: Consider GPU models for better performance');
    }
    recommendation = { model: gpuInfo.recommendedModel };
  } else {
    // Fallback to old detection
    const capabilities = detectSystemCapabilities();
    recommendation = recommendOptimalModel(capabilities);

    log('cyan', '\n📊 System Detection:');
    console.log(`   ${recommendation.reason}`);
    console.log('');

    log('cyan', '🎯 Recommended Model:');
    log('green', `   ${recommendation.model.toUpperCase()} - ${recommendation.description}`);
    console.log(`   Size: ${recommendation.size}`);
    console.log(`   Performance: ${recommendation.performance}`);
  }
  console.log('');

  // Auto-download model if not present
  const modelDownloaded = await autoDownloadModel(recommendation.model);

  // Only show manual download instructions if auto-download failed
  if (!modelDownloaded) {
    log('cyan', 'Next steps:');
    console.log('  1. Download recommended AI model:');
    log('cyan', '     pip install "huggingface_hub[cli]"  # Required for downloads');
    if (recommendation.model !== 'cpu-only') {
      log('green', `     swictation download-model ${recommendation.model}`);
    } else {
      log('green', '     swictation download-model 0.6b  # Recommended for CPU');
    }
    console.log('');
    console.log('     Or download all models (9.43 GB):');
    log('cyan', '     swictation download-models');
    console.log('');
  }

  // Show setup steps (only if model download succeeded or skipped)
  if (modelDownloaded || isModelDownloaded(recommendation.model)) {
    log('cyan', 'Next steps:');
    console.log('  1. Run initial setup:');
    log('cyan', '     swictation setup');
    console.log('');
    console.log('  2. Start the service:');
    log('cyan', '     swictation start');
    console.log('');
    console.log('  3. Toggle recording with:');
    log('cyan', '     swictation toggle');
    console.log('');
  }

  console.log('For more information:');
  log('cyan', '  swictation help');
  console.log('');
}

/**
 * Update config.toml with tested/recommended model while preserving user settings
 * @param {string} testedModel - The model that was successfully tested (e.g., '0.6b-gpu', '1.1b-gpu')
 */
function updateConfigWithTestedModel(testedModel) {
  const configDir = path.join(os.homedir(), '.config', 'swictation');
  const configPath = path.join(configDir, 'config.toml');

  if (!fs.existsSync(configPath)) {
    // No config exists, will be created by main()
    return;
  }

  try {
    let configContent = fs.readFileSync(configPath, 'utf8');

    // Parse current config to check if user has customized it
    const currentOverride = configContent.match(/stt_model_override\s*=\s*"([^"]+)"/);

    if (currentOverride && currentOverride[1] === 'auto') {
      // Only update if it's still on "auto" - user hasn't customized it
      configContent = configContent.replace(
        /stt_model_override\s*=\s*"auto"/,
        `stt_model_override = "${testedModel}"`
      );

      fs.writeFileSync(configPath, configContent);
      log('green', `  ✓ Updated config: stt_model_override = "${testedModel}"`);
      log('cyan', `    Config file: ${configPath}`);
    } else if (currentOverride) {
      log('cyan', `  ℹ Config already customized: stt_model_override = "${currentOverride[1]}"`);
      log('cyan', `    Recommended model: ${testedModel}`);
      log('cyan', `    To use tested model, edit: ${configPath}`);
    }
  } catch (err) {
    log('yellow', `  ⚠️  Could not update config: ${err.message}`);
  }
}

// Main postinstall process
async function main() {
  log('cyan', '🚀 Setting up Swictation...\n');

  try {
    // Platform and basic checks
    checkPlatform();
    ensureBinaryPermissions();
    createDirectories();

    // Phase 0: Stop running services BEFORE any modifications
    // This prevents CUDA state corruption and file conflicts
    log('cyan', '\n═══ Phase 0: Stop Running Services ═══');
    await stopExistingServices();

    // Phase 1: Clean up old/conflicting service files
    log('cyan', '\n═══ Phase 1: Service Cleanup ═══');
    await cleanOldServices();

    // Phase 1.5: Clean up conflicting installations
    log('cyan', '\n═══ Phase 1.5: Cleanup Old Installations ═══');
    cleanupOldOnnxRuntime();
    cleanupOldNpmInstallations();

    // Phase 2: Handle config file migration
    log('cyan', '\n═══ Phase 2: Configuration ═══');
    await interactiveConfigMigration();

    // Phase 3: Detect GPU capabilities
    log('cyan', '\n═══ Phase 3: GPU Detection ═══');
    let gpuInfo = detectGPUVRAM();

    // Download GPU libraries if needed
    if (gpuInfo.hasGPU && gpuInfo.recommendedModel !== 'cpu-only') {
      await downloadGPULibraries();
    } else if (!gpuInfo.hasGPU) {
      log('cyan', '\nℹ No NVIDIA GPU detected - skipping GPU library download');
      log('cyan', '  CPU-only mode will be used');
    }

    // Detect ONNX Runtime library (needed for test-loading)
    const ortLibPath = detectOrtLibrary();

    // Phase 3.5: Model test-loading (actual verification)
    if (!SKIP_MODEL_TEST && gpuInfo.hasGPU && gpuInfo.recommendedModel !== 'cpu-only') {
      log('cyan', '\n═══ Phase 3.5: Model Verification ═══');
      const daemonBin = path.join(__dirname, 'lib', 'native', 'swictation-daemon.bin');

      const testResult = await testModelsInOrder(gpuInfo, daemonBin, ortLibPath);

      // Update gpuInfo with test results
      gpuInfo.recommendedModel = testResult.recommendedModel;
      gpuInfo.tested = testResult.tested;
      gpuInfo.vramVerified = testResult.vramVerified || false;
      gpuInfo.fallbackToCpu = testResult.fallbackToCpu || false;

      // Save updated GPU info with test results
      const configDir = path.join(os.homedir(), '.config', 'swictation');
      const gpuInfoPath = path.join(configDir, 'gpu-info.json');

      try {
        fs.writeFileSync(gpuInfoPath, JSON.stringify(gpuInfo, null, 2));
        log('green', `  ✓ Saved verified GPU info to ${gpuInfoPath}`);
      } catch (err) {
        log('yellow', `  ⚠️  Could not save GPU info: ${err.message}`);
      }
    } else if (SKIP_MODEL_TEST) {
      log('cyan', '\n═══ Phase 3.5: Model Verification ═══');
      log('yellow', '  ⚠️  Model test-loading skipped (SKIP_MODEL_TEST=1)');
      log('cyan', '     Using VRAM-based heuristics only');
    }

    // Phase 4: Generate systemd services
    log('cyan', '\n═══ Phase 4: Service Installation ═══');
    generateSystemdService(ortLibPath);

    // Phase 5: Wayland-specific setup (ydotool + GNOME shortcuts)
    log('cyan', '\n═══ Phase 5: Wayland Integration ═══');
    const waylandResults = await setupWaylandIntegration();

    // Phase 6: Auto-enable and start systemd service
    log('cyan', '\n═══ Phase 6: Service Activation ═══');
    const serviceResults = await enableAndStartService();

    // Phase 7: Check NVIDIA hibernation configuration (laptops only)
    log('cyan', '\n═══ Phase 7: NVIDIA Hibernation Check ═══');
    await checkNvidiaHibernation();

    // Final checks and next steps
    checkDependencies();
    showNextSteps();

    log('green', '\n✅ Postinstall completed successfully!');

  } catch (err) {
    log('red', `\n❌ Postinstall error: ${err.message}`);
    log('yellow', '\nSome steps may have failed, but installation can continue.');
    log('cyan', 'Run "swictation setup" to complete configuration manually.');
    // Don't exit with error - npm install should succeed even if postinstall has issues
  }
}

// Run postinstall
main().catch(err => {
  log('red', `Postinstall error: ${err.message}`);
  // Don't exit with error code - npm install should still succeed
  process.exit(0);
});