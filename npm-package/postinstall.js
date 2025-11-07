#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');

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
}

function ensureBinaryPermissions() {
  const binDir = path.join(__dirname, 'bin');
  const daemonBinary = path.join(binDir, 'swictation-daemon');
  const uiBinary = path.join(binDir, 'swictation-ui');
  const cliBinary = path.join(binDir, 'swictation');

  // Make sure all binaries are executable
  const binaries = [daemonBinary, uiBinary, cliBinary];

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
    { name: 'xdotool', type: 'optional', package: 'xdotool (for X11)' }
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

function showNextSteps() {
  log('green', '\n✨ Swictation installed successfully!');
  log('cyan', '\nNext steps:');
  console.log('  1. Run initial setup:');
  log('cyan', '     swictation setup');
  console.log('');
  console.log('  2. Start the service:');
  log('cyan', '     swictation start');
  console.log('');
  console.log('  3. Toggle recording with:');
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
  checkDependencies();
  showNextSteps();
}

// Run postinstall
main().catch(err => {
  log('red', `Postinstall error: ${err.message}`);
  // Don't exit with error code - npm install should still succeed
  process.exit(0);
});