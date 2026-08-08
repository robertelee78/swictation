#!/usr/bin/env node

// Preuninstall cleanup — USER-SCOPE ONLY (ADR-034).
//
// This script must never escalate to sudo or touch system paths: an npm
// lifecycle hook doing distro-package cleanup deleted more than it owned
// (the old list included /opt/swictation — destroying source checkouts).
// System-level leftovers from pre-npm installs are reported, not removed.

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

function log(color, msg) {
  const colors = {
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    red: '\x1b[31m',
    cyan: '\x1b[36m',
    reset: '\x1b[0m'
  };
  console.log(`${colors[color] || ''}[Swictation Cleanup] ${msg}${colors.reset}`);
}

function safeExec(cmd, options = {}) {
  try {
    execSync(cmd, { stdio: 'ignore', ...options });
    return true;
  } catch {
    return false;
  }
}

function cleanupLinux() {
  log('cyan', 'Stopping systemd services...');
  const services = [
    'swictation-daemon.service',
    'swictation-ui.service',
    'swictation-tray.service',
    'swictation.service' // Old Python service
  ];

  for (const service of services) {
    if (safeExec(`systemctl --user is-active --quiet ${service}`)) {
      safeExec(`systemctl --user stop ${service}`);
      log('green', `  ✓ Stopped ${service}`);
    }
    if (safeExec(`systemctl --user is-enabled --quiet ${service}`)) {
      safeExec(`systemctl --user disable ${service}`);
      log('green', `  ✓ Disabled ${service}`);
    }
    const unitPath = path.join(os.homedir(), '.config', 'systemd', 'user', service);
    if (fs.existsSync(unitPath)) {
      try {
        fs.rmSync(unitPath);
        log('green', `  ✓ Removed ${unitPath}`);
      } catch {
        log('yellow', `  ⚠️  Could not remove ${unitPath}`);
      }
    }
  }
  safeExec('systemctl --user daemon-reload');
}

function cleanupMacOS() {
  log('cyan', 'Unloading launchd services...');
  const launchAgents = ['com.swictation.daemon', 'com.swictation.ui'];
  const launchAgentsDir = path.join(os.homedir(), 'Library', 'LaunchAgents');

  for (const label of launchAgents) {
    const plist = path.join(launchAgentsDir, `${label}.plist`);
    // bootout is idempotent; ignore failures for agents that aren't loaded.
    safeExec(`launchctl bootout gui/$(id -u)/${label}`, { shell: '/bin/bash' });
    if (fs.existsSync(plist)) {
      try {
        fs.rmSync(plist);
        log('green', `  ✓ Removed ${plist}`);
      } catch {
        log('yellow', `  ⚠️  Could not remove ${plist}`);
      }
    }
  }
}

function cleanupLegacyUserPython() {
  // Legacy pre-npm installs put a Python package in user site-packages.
  const removed = [];
  const userPythonBase = path.join(os.homedir(), '.local', 'lib');
  if (fs.existsSync(userPythonBase)) {
    try {
      const pythonDirs = fs.readdirSync(userPythonBase);
      for (const pyDir of pythonDirs) {
        if (pyDir.startsWith('python3')) {
          const swictPath = path.join(userPythonBase, pyDir, 'site-packages', 'swictation');
          if (fs.existsSync(swictPath)) {
            try {
              fs.rmSync(swictPath, { recursive: true, force: true });
              removed.push(swictPath);
              log('green', `  ✓ Removed legacy user install: ${swictPath}`);
            } catch {
              log('yellow', `  ⚠️  Could not remove: ${swictPath}`);
            }
          }
        }
      }
    } catch {}
  }
  return removed;
}

function reportSystemLeftovers() {
  // Old system-wide installs are NOT removed by npm (no sudo here, by design).
  const systemPaths = [
    '/usr/local/lib/python3/dist-packages/swictation',
    '/etc/swictation',
    '/usr/share/doc/swictation'
  ];
  const found = systemPaths.filter(p => {
    try {
      return fs.existsSync(p);
    } catch {
      return false;
    }
  });
  if (found.length > 0) {
    log('yellow', 'Legacy system-wide files detected (left in place; remove manually if desired):');
    for (const p of found) {
      log('yellow', `  sudo rm -rf ${p}`);
    }
  }

  const hasPipPackage = safeExec('pip3 list 2>/dev/null | grep -i swictation', { shell: true });
  if (hasPipPackage) {
    log('yellow', '  ⚠️  Found pip3 swictation package — run: pip3 uninstall swictation');
  }
}

function cleanup() {
  log('cyan', '========================================');
  log('cyan', 'Swictation Preuninstall Cleanup');
  log('cyan', '========================================');

  if (process.platform === 'darwin') {
    cleanupMacOS();
  } else {
    cleanupLinux();
  }
  cleanupLegacyUserPython();
  reportSystemLeftovers();

  const configDir = process.platform === 'darwin'
    ? path.join(os.homedir(), 'Library', 'Application Support', 'swictation')
    : path.join(os.homedir(), '.config', 'swictation');
  const dataDir = process.platform === 'darwin'
    ? configDir
    : path.join(os.homedir(), '.local', 'share', 'swictation');

  log('cyan', '========================================');
  log('green', '✓ Cleanup Complete');
  log('cyan', '========================================');
  log('yellow', 'Preserved (user data, including downloaded models):');
  log('yellow', `  - ${configDir}`);
  if (dataDir !== configDir) {
    log('yellow', `  - ${dataDir}`);
  }
  log('yellow', 'To remove all user data and models:');
  log('cyan', `  rm -rf "${configDir}"${dataDir !== configDir ? ` "${dataDir}"` : ''}`);
}

// npm sets npm_command=uninstall for a true uninstall; during an upgrade the
// replaced version's preuninstall runs under npm_command=install. The previous
// guard checked process.argv for 'uninstall', which npm never passes — so
// cleanup NEVER ran and services were left pointing at deleted binaries.
const npmCommand = process.env.npm_command || '';
const isUninstall = npmCommand === 'uninstall';

if (isUninstall || process.argv.includes('--force')) {
  cleanup();
} else {
  log('cyan', `Skipping cleanup (npm_command=${npmCommand || 'unknown'} — not an uninstall)`);
  log('cyan', 'Use --force to run cleanup manually');
}
