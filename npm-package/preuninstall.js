#!/usr/bin/env node

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

function cleanup() {
  log('cyan', '========================================');
  log('cyan', 'Swictation Preuninstall Cleanup');
  log('cyan', '========================================');
  log('yellow', '');
  log('yellow', 'Cleaning up old installations and legacy files...');
  log('yellow', '');

  // 1. Stop and disable systemd services
  log('cyan', 'Step 1: Stopping systemd services...');
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
  }

  // 2. Remove old Python installation paths
  log('cyan', '');
  log('cyan', 'Step 2: Removing old Python installations...');

  const pythonPaths = [
    '/opt/swictation',
    '/usr/local/lib/python3/dist-packages/swictation',
    '/usr/local/lib/python3.10/dist-packages/swictation',
    '/usr/local/lib/python3.11/dist-packages/swictation',
    '/usr/local/lib/python3.12/dist-packages/swictation'
  ];

  // Also check user Python paths
  const userPythonBase = path.join(os.homedir(), '.local', 'lib');
  if (fs.existsSync(userPythonBase)) {
    try {
      const pythonDirs = fs.readdirSync(userPythonBase);
      for (const pyDir of pythonDirs) {
        if (pyDir.startsWith('python3')) {
          const swictPath = path.join(userPythonBase, pyDir, 'site-packages', 'swictation');
          if (fs.existsSync(swictPath)) {
            pythonPaths.push(swictPath);
          }
        }
      }
    } catch {}
  }

  for (const pythonPath of pythonPaths) {
    if (fs.existsSync(pythonPath)) {
      try {
        if (pythonPath.startsWith('/opt') || pythonPath.includes('/usr/local')) {
          // Need sudo for system paths
          if (safeExec(`sudo rm -rf "${pythonPath}"`)) {
            log('green', `  ✓ Removed: ${pythonPath}`);
          }
        } else {
          // User paths don't need sudo
          fs.rmSync(pythonPath, { recursive: true, force: true });
          log('green', `  ✓ Removed: ${pythonPath}`);
        }
      } catch (err) {
        log('yellow', `  ⚠️  Could not remove: ${pythonPath}`);
      }
    }
  }

  // 3. Remove old symlinks
  log('cyan', '');
  log('cyan', 'Step 3: Removing old symlinks...');

  const binPaths = [
    '/usr/local/bin/swictation',
    '/usr/local/bin/swictation-cli',
    '/usr/local/bin/swictation-legacy',
    '/usr/bin/swictation'
  ];

  for (const binPath of binPaths) {
    if (fs.existsSync(binPath)) {
      try {
        const stats = fs.lstatSync(binPath);
        const realPath = stats.isSymbolicLink() ? fs.readlinkSync(binPath) : fs.realpathSync(binPath);

        // Only remove if it points to Python version or old /opt/swictation
        if (realPath.includes('python') ||
            realPath.includes('/opt/swictation') ||
            realPath.includes('swictationd.py')) {
          if (safeExec(`sudo rm -f "${binPath}"`)) {
            log('green', `  ✓ Removed old symlink: ${binPath} -> ${realPath}`);
          }
        }
      } catch (err) {
        log('yellow', `  ⚠️  Could not check: ${binPath}`);
      }
    }
  }

  // 4. Remove old .deb package files
  log('cyan', '');
  log('cyan', 'Step 4: Cleaning old .deb packages...');

  const debInfoPaths = [
    '/var/lib/dpkg/info/swictation.list',
    '/var/lib/dpkg/info/swictation.md5sums',
    '/var/lib/dpkg/info/swictation.postinst',
    '/var/lib/dpkg/info/swictation.preinst',
    '/var/lib/dpkg/info/swictation.prerm'
  ];

  for (const debPath of debInfoPaths) {
    if (fs.existsSync(debPath)) {
      if (safeExec(`sudo rm -f "${debPath}"`)) {
        log('green', `  ✓ Removed: ${debPath}`);
      }
    }
  }

  // 5. Remove old system config directories
  log('cyan', '');
  log('cyan', 'Step 5: Removing old system directories...');

  const systemDirs = [
    '/etc/swictation',
    '/usr/share/doc/swictation'
  ];

  for (const sysDir of systemDirs) {
    if (fs.existsSync(sysDir)) {
      if (safeExec(`sudo rm -rf "${sysDir}"`)) {
        log('green', `  ✓ Removed: ${sysDir}`);
      }
    }
  }

  // 6. Check for pip packages
  log('cyan', '');
  log('cyan', 'Step 6: Checking for pip packages...');

  const hasPipPackage = safeExec('pip3 list 2>/dev/null | grep -i swictation', { shell: true });
  if (hasPipPackage) {
    log('yellow', '  ⚠️  Found pip3 swictation package');
    log('yellow', '     Run: pip3 uninstall swictation');
  } else {
    log('green', '  ✓ No pip3 packages found');
  }

  // 7. Summary
  log('cyan', '');
  log('cyan', '========================================');
  log('green', '✓ Cleanup Complete');
  log('cyan', '========================================');
  log('yellow', '');
  log('yellow', 'The following directories are preserved (user data):');
  log('yellow', `  - ${path.join(os.homedir(), '.config', 'swictation')}`);
  log('yellow', `  - ${path.join(os.homedir(), '.local', 'share', 'swictation')}`);
  log('yellow', '');
  log('yellow', 'To manually remove all user data and models:');
  log('cyan', `  rm -rf ${path.join(os.homedir(), '.config', 'swictation')}`);
  log('cyan', `  rm -rf ${path.join(os.homedir(), '.local', 'share', 'swictation')}`);
  log('yellow', '');
}

// Only run cleanup if this is actually an uninstall
// (npm runs preuninstall even for updates, so check if we're truly uninstalling)
const isUninstall = process.env.npm_config_global === 'true' &&
                    process.argv.includes('uninstall');

if (isUninstall || process.argv.includes('--force')) {
  cleanup();
} else {
  log('cyan', 'Skipping cleanup (update detected, not uninstall)');
  log('cyan', 'Use --force to run cleanup during updates');
}
