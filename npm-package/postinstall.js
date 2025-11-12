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

function detectOrtLibrary() {
  log('cyan', '\n🔍 Detecting ONNX Runtime library path...');

  try {
    // Try to find ONNX Runtime through Python
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
    log('green', `✓ Found ONNX Runtime: ${ortLibPath}`);

    // Store in a config file for systemd service generation
    const configDir = path.join(__dirname, 'config');
    const envFilePath = path.join(configDir, 'detected-environment.json');

    const envConfig = {
      ORT_DYLIB_PATH: ortLibPath,
      detected_at: new Date().toISOString(),
      onnxruntime_version: execSync('python3 -c "import onnxruntime; print(onnxruntime.__version__)"', { encoding: 'utf-8' }).trim()
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

      if (ortLibPath) {
        template = template.replace(/__ORT_DYLIB_PATH__/g, ortLibPath);
      } else {
        log('yellow', '⚠️  Warning: ORT_DYLIB_PATH not detected');
        log('yellow', '   Service file will contain placeholder - you must set it manually');
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

    // 2. Install UI service (copy directly, no template)
    const uiServiceSource = path.join(__dirname, 'config', 'swictation-ui.service');
    if (fs.existsSync(uiServiceSource)) {
      const uiServiceDest = path.join(systemdDir, 'swictation-ui.service');
      fs.copyFileSync(uiServiceSource, uiServiceDest);
      log('green', `✓ Installed UI service: ${uiServiceDest}`);
    } else {
      log('yellow', `⚠️  Warning: UI service not found at ${uiServiceSource}`);
      log('yellow', '   You can manually install it later');
    }

  } catch (err) {
    log('yellow', `⚠️  Failed to generate systemd services: ${err.message}`);
    log('cyan', '  You can manually create them later using: swictation setup');
  }
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

function showNextSteps() {
  log('green', '\n✨ Swictation installed successfully!');

  // Detect system and recommend optimal model
  const capabilities = detectSystemCapabilities();
  const recommendation = recommendOptimalModel(capabilities);

  log('cyan', '\n📊 System Detection:');
  console.log(`   ${recommendation.reason}`);
  console.log('');

  log('cyan', '🎯 Recommended Model:');
  log('green', `   ${recommendation.model.toUpperCase()} - ${recommendation.description}`);
  console.log(`   Size: ${recommendation.size}`);
  console.log(`   Performance: ${recommendation.performance}`);
  console.log('');

  log('cyan', 'Next steps:');
  console.log('  1. Download recommended AI model:');
  log('cyan', '     pip install "huggingface_hub[cli]"  # Required for downloads');
  log('green', `     swictation download-model ${recommendation.model}`);
  console.log('');
  console.log('     Or download all models (9.43 GB):');
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
  const ortLibPath = detectOrtLibrary();
  generateSystemdService(ortLibPath);
  checkDependencies();
  showNextSteps();
}

// Run postinstall
main().catch(err => {
  log('red', `Postinstall error: ${err.message}`);
  // Don't exit with error code - npm install should still succeed
  process.exit(0);
});