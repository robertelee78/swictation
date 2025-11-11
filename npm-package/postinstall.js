#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');
const https = require('https');

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

async function downloadGPULibraries() {
  const hasGPU = detectNvidiaGPU();

  if (!hasGPU) {
    log('cyan', '\nℹ No NVIDIA GPU detected - skipping GPU library download');
    log('cyan', '  CPU-only mode will be used');
    return;
  }

  log('green', '\n✓ NVIDIA GPU detected!');
  log('cyan', '📦 Downloading GPU acceleration libraries...');

  const version = require('./package.json').version;
  const releaseUrl = `https://github.com/robertelee78/swictation/releases/download/v${version}/swictation-gpu-libs.tar.gz`;
  const tmpDir = path.join(os.tmpdir(), 'swictation-gpu-install');
  const tarPath = path.join(tmpDir, 'gpu-libs.tar.gz');
  const nativeDir = path.join(__dirname, 'lib', 'native');

  try {
    // Create temp directory
    if (!fs.existsSync(tmpDir)) {
      fs.mkdirSync(tmpDir, { recursive: true });
    }

    // Download tarball
    log('cyan', `  Downloading from: ${releaseUrl}`);
    await downloadFile(releaseUrl, tarPath);
    log('green', '  ✓ Downloaded GPU libraries');

    // Extract tarball
    log('cyan', '  Extracting...');
    execSync(`tar -xzf "${tarPath}" -C "${nativeDir}"`, { stdio: 'inherit' });
    log('green', '  ✓ Extracted GPU libraries');

    // Cleanup
    fs.unlinkSync(tarPath);
    fs.rmdirSync(tmpDir);

    log('green', '✓ GPU acceleration enabled!');
    log('cyan', '  Your system will use CUDA for faster transcription');
  } catch (err) {
    log('yellow', `\n⚠ Failed to download GPU libraries: ${err.message}`);
    log('cyan', '  Continuing with CPU-only mode');
    log('cyan', '  You can manually download from:');
    log('cyan', `  ${releaseUrl}`);
  }
}

function showNextSteps() {
  log('green', '\n✨ Swictation installed successfully!');
  log('cyan', '\nNext steps:');
  console.log('  1. Download AI models (9.43 GB):');
  log('cyan', '     pip install "huggingface_hub[cli]"  # Required for model downloads');
  log('cyan', '     swictation download-models');
  console.log('');
  console.log('  2. Run initial setup:');
  log('cyan', '     swictation setup');
  console.log('');
  console.log('  3. Start the service:');
  log('cyan', '     swictation start');
  console.log('');
  console.log('  4. Toggle recording with:');
  log('cyan', '     swictation toggle');
  console.log('');
  console.log('For more information:');
  log('cyan', '  swictation help');
  console.log('');
}

// Main postinstall process
async function main() {
  log('cyan', '🚀 Setting up Swictation...\n');

  checkPlatform();
  ensureBinaryPermissions();
  createDirectories();
  await downloadGPULibraries();
  checkDependencies();
  showNextSteps();
}

// Run postinstall
main().catch(err => {
  log('red', `Postinstall error: ${err.message}`);
  // Don't exit with error code - npm install should still succeed
  process.exit(0);
});