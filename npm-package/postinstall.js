#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');
const https = require('https');

// Environment variable support for model test-loading
const SKIP_MODEL_TEST = process.env.SKIP_MODEL_TEST === '1';
const ENABLE_MODEL_TEST = process.env.TEST_MODEL_LOADING === '1';

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

  // CRITICAL: Check npm package library FIRST (GPU-enabled)
  // This is the bundled library with CUDA support
  const npmOrtLib = path.join(__dirname, 'lib', 'native', 'libonnxruntime.so');
  if (fs.existsSync(npmOrtLib)) {
    log('green', `✓ Found ONNX Runtime (GPU-enabled): ${npmOrtLib}`);
    log('cyan', '  Using bundled GPU-enabled library with CUDA provider support');
    return npmOrtLib;
  } else {
    log('yellow', `⚠️  Warning: GPU-enabled ONNX Runtime not found at ${npmOrtLib}`);
    log('yellow', '   Falling back to system Python installation (may be CPU-only)');
  }

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
    log('yellow', '⚠️  No config template found in package');
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
    // Based on empirical data from the analysis:
    // 0.6B model: ~3.5GB VRAM (fits in 4GB with headroom)
    // 1.1B model: ~6GB VRAM (needs at least 8GB for safety)
    if (gpuInfo.vramMB >= 6000) {
      // 6GB+ VRAM: Can safely run 1.1B model
      gpuInfo.recommendedModel = '1.1b';
      log('green', `  ✓ Sufficient VRAM for 1.1B model (best quality)`);
    } else if (gpuInfo.vramMB >= 4000) {
      // 4-6GB VRAM: Run 0.6B model (proven safe on 4GB)
      gpuInfo.recommendedModel = '0.6b';
      log('yellow', `  ⚠️  Limited VRAM - Recommending 0.6B model`);
      log('cyan', `     (1.1B model requires ~6GB VRAM)`);
    } else {
      // <4GB VRAM: Too little for GPU acceleration
      gpuInfo.recommendedModel = 'cpu-only';
      log('yellow', `  ⚠️  Insufficient VRAM for GPU models`);
      log('cyan', `     GPU models require minimum 4GB VRAM`);
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

  const modelFlag = `--test-model=${modelName}`;
  const env = {
    ...process.env,
    ORT_DYLIB_PATH: ortLibPath,
    LD_LIBRARY_PATH: `/usr/local/cuda/lib64:/usr/local/cuda-12.9/lib64:${path.join(__dirname, 'lib', 'native')}`,
    CUDA_HOME: '/usr/local/cuda',
    RUST_LOG: 'info'
  };

  try {
    const output = execSync(
      `timeout 30s "${daemonBin}" ${modelFlag} --dry-run 2>&1`,
      { encoding: 'utf8', env, stdio: 'pipe' }
    );

    // Check for success indicators in output
    if (output.includes('Model loaded successfully') || output.includes('Selected model')) {
      log('green', `    ✓ ${modelName} loaded successfully`);
      return { success: true, model: modelName };
    } else {
      log('yellow', `    ⚠️  ${modelName} load uncertain (no success indicator)`);
      return { success: false, model: modelName, reason: 'No success indicator' };
    }
  } catch (err) {
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

function showNextSteps() {
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
    if (gpuInfo.recommendedModel === '1.1b') {
      log('green', '   1.1B - Best quality - Full GPU acceleration with FP32 precision');
      console.log('   Size: ~75MB download (FP32 + INT8 versions)');
      console.log('   Performance: 62x realtime speed on GPU');
    } else if (gpuInfo.recommendedModel === '0.6b') {
      log('yellow', '   0.6B - Lighter model for limited VRAM systems');
      console.log('   Size: ~111MB');
      console.log('   Performance: Fast on GPU');
    } else {
      log('cyan', '   CPU-optimized models');
      console.log('   Multiple sizes available (0.6B - 1.1B)');
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

  try {
    // Platform and basic checks
    checkPlatform();
    ensureBinaryPermissions();
    createDirectories();

    // Phase 1: Clean up old/conflicting service files
    log('cyan', '\n═══ Phase 1: Service Cleanup ═══');
    await cleanOldServices();

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