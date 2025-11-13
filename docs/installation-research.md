# Installation Best Practices Research
## npm Postinstall Scripts for System-Level Packages with Daemon Management

**Research Date**: 2025-11-13
**Project**: Swictation v0.3.0
**Focus Areas**: Service cleanup, config migration, GPU detection, systemd management

---

## Executive Summary

This research document compiles best practices from Linux package managers (apt, dnf, pacman), npm ecosystem tools (PM2, node-windows), and systemd-managed daemons to improve swictation's installation and upgrade process.

### Key Findings

1. **Old Installation Cleanup**: Requires multi-phase approach with graceful service stopping before file removal
2. **Config Migration**: Follow Debian conffile patterns with MD5 tracking and user prompts
3. **GPU Detection**: Use nvidia-smi with fallback mechanisms and clear user communication
4. **Service Management**: Prefer drop-in overrides over direct service file replacement

---

## 1. Old Installation Cleanup Best Practices

### 1.1 Multi-Phase Cleanup Strategy

**Pattern from Debian/dpkg**:
- **Phase 1**: Detect old installations before unpacking new files
- **Phase 2**: Stop and disable old services gracefully
- **Phase 3**: Remove old service files
- **Phase 4**: Reload systemd daemon
- **Phase 5**: Continue with new installation

**Current swictation Implementation Analysis**:
```javascript
// postinstall.js lines 63-139
async function cleanOldServices() {
  // ✅ GOOD: Checks multiple locations
  // ✅ GOOD: Stops services before removal
  // ✅ GOOD: Handles both system and user services
  // ⚠️  MISSING: No backup of user-modified configs
  // ⚠️  MISSING: No version detection to skip if already upgraded
}
```

### 1.2 Recommended Improvements

**1. Add Version Detection**:
```javascript
function detectInstalledVersion() {
  const serviceFiles = [
    '/usr/lib/systemd/user/swictation.service',
    '~/.config/systemd/user/swictation-daemon.service'
  ];

  for (const file of serviceFiles) {
    if (fs.existsSync(file)) {
      const content = fs.readFileSync(file, 'utf8');
      // Check for version marker or Python vs Node.js detection
      if (content.includes('python') || !content.includes('node_modules')) {
        return 'legacy-python';
      } else if (content.includes('/usr/local/lib/node_modules/swictation')) {
        return 'current-nodejs';
      }
    }
  }
  return 'none';
}
```

**2. Add Interactive Prompts for Config Backup** (using inquirer dependency):
```javascript
const inquirer = require('inquirer');

async function handleConfigMigration() {
  const configFile = path.join(os.homedir(), '.config', 'swictation', 'config.json');

  if (fs.existsSync(configFile)) {
    const stats = fs.statSync(configFile);
    const currentMD5 = computeMD5(configFile);
    const packagedMD5 = getPackagedConfigMD5();

    if (currentMD5 !== packagedMD5) {
      const answer = await inquirer.prompt([{
        type: 'list',
        name: 'action',
        message: 'Configuration file has been modified. What would you like to do?',
        choices: [
          { name: 'Keep your current config', value: 'keep' },
          { name: 'Use new default config (backup old)', value: 'replace' },
          { name: 'Show differences', value: 'diff' }
        ]
      }]);

      if (answer.action === 'replace') {
        fs.copyFileSync(configFile, configFile + '.backup');
        log('green', `✓ Backed up config to ${configFile}.backup`);
      }
    }
  }
}
```

**3. Improve Service Cleanup with Error Recovery**:
```javascript
async function cleanOldServices() {
  const cleanup = {
    stopped: [],
    disabled: [],
    removed: [],
    failed: []
  };

  try {
    // Stop all services first (critical for clean upgrade)
    for (const service of oldServices) {
      try {
        await stopService(service);
        cleanup.stopped.push(service);
      } catch (err) {
        cleanup.failed.push({ service, error: err.message, phase: 'stop' });
      }
    }

    // Then disable
    for (const service of cleanup.stopped) {
      try {
        await disableService(service);
        cleanup.disabled.push(service);
      } catch (err) {
        cleanup.failed.push({ service, error: err.message, phase: 'disable' });
      }
    }

    // Finally remove files
    for (const service of cleanup.disabled) {
      try {
        await removeServiceFile(service);
        cleanup.removed.push(service);
      } catch (err) {
        cleanup.failed.push({ service, error: err.message, phase: 'remove' });
      }
    }

    // Report results
    if (cleanup.failed.length > 0) {
      log('yellow', '\n⚠️  Some cleanup operations failed:');
      for (const failure of cleanup.failed) {
        log('yellow', `  ${failure.service} (${failure.phase}): ${failure.error}`);
      }
      log('cyan', '\nYou may need to manually clean up these services');
    }

    return cleanup;

  } finally {
    execSync('systemctl --user daemon-reload 2>/dev/null || true');
    execSync('sudo systemctl daemon-reload 2>/dev/null || true');
  }
}
```

---

## 2. Config File Migration Patterns

### 2.1 Debian Conffile Approach

**How Debian Handles Config Files** (from dpkg documentation):

1. **MD5 Tracking**: dpkg stores MD5 hash of original config file
2. **Change Detection**: Compares current file MD5 to original + new package MD5
3. **User Prompts**: If both changed, prompts user with options:
   - Keep current version
   - Install new version (saves old as `.dpkg-old`)
   - Show differences
   - View side-by-side comparison

**Debian's Conffile Migration Recipe**:
```bash
# From DpkgConffileHandling wiki
if [ -f "$oldconffile" ]; then
    md5sum="$(md5sum "$oldconffile" | sed -e 's/ .*//')"
    old_md5sum="$(dpkg-query -W -f='${Conffiles}' $PKGNAME | \
                  grep -F "$oldconffile" | md5sum | sed -e 's/ .*//')"
    if [ "$md5sum" != "$old_md5sum" ]; then
        # User modified - preserve changes
        mv "$oldconffile" "$newconffile"
    fi
fi
```

### 2.2 Key Principles for Config Migration

**DO**:
- ✅ Track original config file checksums
- ✅ Detect user modifications before overwriting
- ✅ Provide interactive choices when conflicts occur
- ✅ Always backup user-modified configs
- ✅ Document migration in changelog/upgrade notes

**DON'T**:
- ❌ Silently overwrite modified configs
- ❌ Remove user data without warning
- ❌ Fail installation if config conflicts exist
- ❌ Modify dpkg-handled conffiles in postinst (Debian rule)

### 2.3 Implementation for Swictation

```javascript
class ConfigMigrationManager {
  constructor() {
    this.configDir = path.join(os.homedir(), '.config', 'swictation');
    this.trackingFile = path.join(this.configDir, '.installed-checksums.json');
  }

  async migrateConfig(configName) {
    const configPath = path.join(this.configDir, configName);
    const newConfigPath = path.join(__dirname, 'config', configName);

    // Load tracking data
    const tracking = this.loadTracking();

    // Check if config exists and was modified
    if (fs.existsSync(configPath)) {
      const currentMD5 = this.computeMD5(configPath);
      const originalMD5 = tracking[configName]?.md5;
      const newMD5 = this.computeMD5(newConfigPath);

      if (originalMD5 && currentMD5 !== originalMD5) {
        // User modified the config
        if (currentMD5 === newMD5) {
          // User manually updated to new version - no action
          return { action: 'none', reason: 'already-updated' };
        } else {
          // Conflict: both user and package changed
          return await this.handleConflict(configPath, newConfigPath);
        }
      }
    }

    // No conflict - safe to install new config
    fs.copyFileSync(newConfigPath, configPath);
    this.updateTracking(configName, this.computeMD5(newConfigPath));
    return { action: 'installed', reason: 'no-conflict' };
  }

  async handleConflict(currentPath, newPath) {
    const answer = await inquirer.prompt([{
      type: 'list',
      name: 'action',
      message: `Config file ${path.basename(currentPath)} has local changes. How to proceed?`,
      choices: [
        { name: 'Keep my version (save new as .new)', value: 'keep' },
        { name: 'Use new version (backup mine as .backup)', value: 'replace' },
        { name: 'Show diff', value: 'diff' },
        { name: 'Manual merge (opens editor)', value: 'merge' }
      ]
    }]);

    switch (answer.action) {
      case 'keep':
        fs.copyFileSync(newPath, currentPath + '.new');
        log('cyan', `New config saved as ${currentPath}.new`);
        return { action: 'kept', newPath: currentPath + '.new' };

      case 'replace':
        fs.copyFileSync(currentPath, currentPath + '.backup');
        fs.copyFileSync(newPath, currentPath);
        log('cyan', `Old config backed up to ${currentPath}.backup`);
        return { action: 'replaced', backupPath: currentPath + '.backup' };

      case 'diff':
        // Show diff and re-prompt
        this.showDiff(currentPath, newPath);
        return this.handleConflict(currentPath, newPath);

      case 'merge':
        // Open merge tool (if available)
        return this.openMergeTool(currentPath, newPath);
    }
  }

  computeMD5(filePath) {
    const crypto = require('crypto');
    const content = fs.readFileSync(filePath);
    return crypto.createHash('md5').update(content).digest('hex');
  }

  loadTracking() {
    if (fs.existsSync(this.trackingFile)) {
      return JSON.parse(fs.readFileSync(this.trackingFile, 'utf8'));
    }
    return {};
  }

  updateTracking(configName, md5) {
    const tracking = this.loadTracking();
    tracking[configName] = {
      md5,
      updated: new Date().toISOString()
    };
    fs.writeFileSync(this.trackingFile, JSON.stringify(tracking, null, 2));
  }
}
```

---

## 3. GPU Detection and VRAM Checking

### 3.1 Current Implementation Analysis

```javascript
// postinstall.js lines 225-232
function detectNvidiaGPU() {
  try {
    execSync('nvidia-smi', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}
```

**Issues**:
- ❌ Only checks if nvidia-smi exists (not if GPU is functional)
- ❌ Doesn't check VRAM amount
- ❌ Doesn't verify CUDA version compatibility
- ❌ No fallback if nvidia-smi missing from PATH

### 3.2 Enhanced GPU Detection

```javascript
class GPUDetector {
  constructor() {
    this.requirements = {
      minVRAM: 2048,      // MB
      minCUDAVersion: 12.0,
      requiredDriverVersion: 525.0
    };
  }

  async detectGPU() {
    const result = {
      hasGPU: false,
      name: null,
      vramMB: 0,
      cudaVersion: null,
      driverVersion: null,
      compatible: false,
      warnings: [],
      errors: []
    };

    // Step 1: Check nvidia-smi availability
    const nvidiaSmiPath = await this.findNvidiaSmi();
    if (!nvidiaSmiPath) {
      result.errors.push('nvidia-smi not found - NVIDIA drivers not installed');
      return result;
    }

    try {
      // Step 2: Get GPU info
      const gpuInfo = execSync(
        `${nvidiaSmiPath} --query-gpu=name,memory.total,driver_version --format=csv,noheader,nounits`,
        { encoding: 'utf8' }
      ).trim().split(',').map(s => s.trim());

      result.name = gpuInfo[0];
      result.vramMB = parseInt(gpuInfo[1]);
      result.driverVersion = parseFloat(gpuInfo[2]);
      result.hasGPU = true;

      // Step 3: Check CUDA version
      try {
        const cudaVersion = execSync('nvcc --version 2>/dev/null | grep release | awk \'{print $5}\' | cut -d, -f1',
                                     { encoding: 'utf8' }).trim();
        result.cudaVersion = parseFloat(cudaVersion);
      } catch (err) {
        result.warnings.push('CUDA toolkit not installed - using runtime from driver');
        // Driver includes CUDA runtime, toolkit is optional
      }

      // Step 4: Validate compatibility
      if (result.vramMB < this.requirements.minVRAM) {
        result.warnings.push(
          `Low VRAM: ${result.vramMB}MB (recommended: ${this.requirements.minVRAM}MB+)`
        );
      }

      if (result.driverVersion < this.requirements.requiredDriverVersion) {
        result.errors.push(
          `Driver version ${result.driverVersion} is too old (need ${this.requirements.requiredDriverVersion}+)`
        );
      } else {
        result.compatible = true;
      }

      // Step 5: Test actual CUDA functionality
      const cudaWorks = await this.testCUDA();
      if (!cudaWorks) {
        result.warnings.push('CUDA detected but test execution failed');
        result.compatible = false;
      }

    } catch (err) {
      result.errors.push(`GPU detection failed: ${err.message}`);
    }

    return result;
  }

  async findNvidiaSmi() {
    const possiblePaths = [
      '/usr/bin/nvidia-smi',
      '/usr/local/cuda/bin/nvidia-smi',
      '/opt/cuda/bin/nvidia-smi'
    ];

    // Check PATH first
    try {
      const pathResult = execSync('which nvidia-smi 2>/dev/null', { encoding: 'utf8' }).trim();
      if (pathResult) return pathResult;
    } catch {}

    // Check common locations
    for (const p of possiblePaths) {
      if (fs.existsSync(p)) return p;
    }

    // Try find command as last resort
    try {
      const findResult = execSync(
        'find /usr /opt -name nvidia-smi 2>/dev/null | head -1',
        { encoding: 'utf8', timeout: 5000 }
      ).trim();
      if (findResult) return findResult;
    } catch {}

    return null;
  }

  async testCUDA() {
    try {
      // Quick test using nvidia-smi to query compute processes
      execSync('nvidia-smi -L', { stdio: 'ignore', timeout: 3000 });
      return true;
    } catch {
      return false;
    }
  }

  getRecommendedModel(gpuInfo) {
    if (!gpuInfo.compatible) {
      return {
        model: 'cpu-only',
        reason: 'GPU not compatible or not detected',
        performance: 'Slower CPU inference'
      };
    }

    if (gpuInfo.vramMB >= 8000) {
      return {
        model: '1.1b-fp32',
        reason: `High VRAM (${Math.round(gpuInfo.vramMB/1024)}GB)`,
        performance: '62x realtime on GPU'
      };
    } else if (gpuInfo.vramMB >= 4000) {
      return {
        model: '1.1b-int8',
        reason: `Medium VRAM (${Math.round(gpuInfo.vramMB/1024)}GB)`,
        performance: '40-50x realtime on GPU'
      };
    } else {
      return {
        model: '0.6b',
        reason: `Limited VRAM (${Math.round(gpuInfo.vramMB/1024)}GB)`,
        performance: 'Fast on GPU'
      };
    }
  }

  displayResults(gpuInfo) {
    if (!gpuInfo.hasGPU) {
      log('yellow', '\n⚠️  No NVIDIA GPU detected');
      for (const error of gpuInfo.errors) {
        log('red', `   ✗ ${error}`);
      }
      log('cyan', '\n   Continuing with CPU-only mode');
      return;
    }

    log('green', `\n✓ GPU Detected: ${gpuInfo.name}`);
    log('cyan', `  VRAM: ${Math.round(gpuInfo.vramMB/1024)}GB`);
    log('cyan', `  Driver: ${gpuInfo.driverVersion}`);
    if (gpuInfo.cudaVersion) {
      log('cyan', `  CUDA: ${gpuInfo.cudaVersion}`);
    }

    if (gpuInfo.warnings.length > 0) {
      log('yellow', '\n⚠️  Warnings:');
      for (const warning of gpuInfo.warnings) {
        log('yellow', `  • ${warning}`);
      }
    }

    if (gpuInfo.errors.length > 0) {
      log('red', '\n✗ Errors:');
      for (const error of gpuInfo.errors) {
        log('red', `  • ${error}`);
      }
    }

    if (gpuInfo.compatible) {
      log('green', '\n✓ GPU acceleration will be enabled');
      const recommendation = this.getRecommendedModel(gpuInfo);
      log('cyan', `  Recommended model: ${recommendation.model}`);
      log('cyan', `  Expected performance: ${recommendation.performance}`);
    } else {
      log('yellow', '\n⚠️  GPU detected but not compatible - using CPU mode');
    }
  }
}
```

### 3.3 VRAM Usage Estimation

```javascript
// Model VRAM requirements (approximate)
const MODEL_VRAM_REQUIREMENTS = {
  '1.1b-fp32': {
    vram: 3500,      // MB
    systemRam: 1024, // MB
    description: 'Full precision - best quality'
  },
  '1.1b-int8': {
    vram: 1200,
    systemRam: 512,
    description: 'Quantized - good quality, less VRAM'
  },
  '0.6b': {
    vram: 800,
    systemRam: 512,
    description: 'Lighter model'
  }
};

function estimateVRAMUsage(modelName, batchSize = 1) {
  const baseReq = MODEL_VRAM_REQUIREMENTS[modelName];
  if (!baseReq) return null;

  // VRAM scales with batch size
  const estimated = {
    model: baseReq.vram,
    batch: baseReq.vram * 0.2 * (batchSize - 1),  // ~20% per additional batch
    overhead: 500,  // CUDA/cuDNN overhead
    total: 0
  };

  estimated.total = estimated.model + estimated.batch + estimated.overhead;

  return estimated;
}
```

---

## 4. Systemd Service Management Best Practices

### 4.1 Service File Locations and Precedence

**systemd Load Order** (highest to lowest priority):
1. `/etc/systemd/system/` - Administrator customizations (HIGHEST)
2. `/etc/systemd/system/*.d/` - Drop-in overrides
3. `/run/systemd/system/` - Runtime units
4. `/usr/lib/systemd/system/` - Package-installed units (LOWEST)

**Key Rules**:
- ✅ Packages install to `/usr/lib/systemd/system/`
- ✅ User customizations go to `/etc/systemd/system/`
- ✅ Drop-in overrides use `*.d/override.conf` pattern
- ❌ Never edit files in `/usr/lib/systemd/system/` directly

### 4.2 Service File Migration Strategy

**Current swictation approach** (postinstall.js line 390-443):
```javascript
// ⚠️  PROBLEM: Installs to ~/.config/systemd/user/
// This is correct for USER services, but:
// - Gets overwritten on each npm install
// - User customizations are lost
// - No migration of old customizations
```

**Recommended Approach**:

```javascript
class SystemdServiceManager {
  constructor() {
    this.packageServiceDir = path.join(__dirname, 'config');
    this.systemUserDir = '/usr/lib/systemd/user';
    this.userConfigDir = path.join(os.homedir(), '.config', 'systemd', 'user');
  }

  async installService(serviceName) {
    const packageService = path.join(this.packageServiceDir, serviceName);
    const userService = path.join(this.userConfigDir, serviceName);
    const dropinDir = path.join(this.userConfigDir, `${serviceName}.d`);

    // Check if user has customized the service
    if (fs.existsSync(userService)) {
      const hasCustomizations = await this.detectCustomizations(
        packageService,
        userService
      );

      if (hasCustomizations) {
        // Convert direct edits to drop-in overrides
        await this.migrateToDropin(serviceName, userService, dropinDir);
      } else {
        // No customizations - safe to update
        fs.copyFileSync(packageService, userService);
        log('green', `✓ Updated ${serviceName}`);
      }
    } else {
      // First install - copy directly
      fs.copyFileSync(packageService, userService);
      log('green', `✓ Installed ${serviceName}`);
    }

    // Reload daemon
    execSync('systemctl --user daemon-reload');
  }

  async detectCustomizations(packageFile, userFile) {
    const packageContent = fs.readFileSync(packageFile, 'utf8');
    const userContent = fs.readFileSync(userFile, 'utf8');

    // Remove comments and whitespace for comparison
    const normalize = (content) => {
      return content
        .split('\n')
        .filter(line => !line.trim().startsWith('#'))
        .filter(line => line.trim().length > 0)
        .join('\n');
    };

    return normalize(packageContent) !== normalize(userContent);
  }

  async migrateToDropin(serviceName, userFile, dropinDir) {
    log('cyan', `\n⚠️  Detected customizations in ${serviceName}`);

    const answer = await inquirer.prompt([{
      type: 'list',
      name: 'action',
      message: 'How would you like to handle service customizations?',
      choices: [
        {
          name: 'Migrate to drop-in override (recommended)',
          value: 'migrate',
          short: 'Preserves customizations, gets upstream updates'
        },
        {
          name: 'Keep custom service file (lose upstream updates)',
          value: 'keep',
          short: 'Your customizations, but no automatic updates'
        },
        {
          name: 'Reset to default (backup old)',
          value: 'reset',
          short: 'Fresh start, old version backed up'
        },
        {
          name: 'Show differences',
          value: 'diff'
        }
      ]
    }]);

    switch (answer.action) {
      case 'migrate':
        await this.createDropinOverride(serviceName, userFile, dropinDir);
        break;

      case 'keep':
        log('yellow', `  Keeping custom ${serviceName}`);
        log('yellow', '  ⚠️  You will not receive automatic updates to this service');
        break;

      case 'reset':
        fs.copyFileSync(userFile, userFile + '.backup');
        fs.copyFileSync(
          path.join(this.packageServiceDir, serviceName),
          userFile
        );
        log('green', `  ✓ Backed up to ${serviceName}.backup`);
        log('green', `  ✓ Reset to default ${serviceName}`);
        break;

      case 'diff':
        this.showServiceDiff(
          path.join(this.packageServiceDir, serviceName),
          userFile
        );
        // Re-prompt
        return this.migrateToDropin(serviceName, userFile, dropinDir);
    }
  }

  async createDropinOverride(serviceName, userFile, dropinDir) {
    // Extract customizations by diffing against package version
    const packageService = path.join(this.packageServiceDir, serviceName);
    const diff = await this.computeServiceDiff(packageService, userFile);

    // Create drop-in directory
    if (!fs.existsSync(dropinDir)) {
      fs.mkdirSync(dropinDir, { recursive: true });
    }

    // Write override file
    const overrideFile = path.join(dropinDir, 'override.conf');
    fs.writeFileSync(overrideFile, diff);

    // Replace user service with package default
    fs.copyFileSync(packageService, userFile);

    log('green', `  ✓ Created drop-in override: ${overrideFile}`);
    log('cyan', '  Your customizations preserved in drop-in file');
    log('cyan', '  Base service will receive upstream updates automatically');
  }

  async computeServiceDiff(baseFile, customFile) {
    // Parse both files
    const base = this.parseServiceFile(baseFile);
    const custom = this.parseServiceFile(customFile);

    // Find differences
    const overrides = [];

    for (const [section, directives] of Object.entries(custom)) {
      if (!base[section]) {
        // Entire section is new
        overrides.push(`[${section}]`);
        for (const [key, value] of Object.entries(directives)) {
          overrides.push(`${key}=${value}`);
        }
        overrides.push('');
      } else {
        // Check for directive differences
        const sectionOverrides = [];
        for (const [key, value] of Object.entries(directives)) {
          if (base[section][key] !== value) {
            sectionOverrides.push(`${key}=${value}`);
          }
        }

        if (sectionOverrides.length > 0) {
          overrides.push(`[${section}]`);
          overrides.push(...sectionOverrides);
          overrides.push('');
        }
      }
    }

    return overrides.join('\n');
  }

  parseServiceFile(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    const sections = {};
    let currentSection = null;

    for (const line of content.split('\n')) {
      const trimmed = line.trim();

      if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
        currentSection = trimmed.slice(1, -1);
        sections[currentSection] = {};
      } else if (currentSection && trimmed.includes('=')) {
        const [key, ...valueParts] = trimmed.split('=');
        const value = valueParts.join('=');
        sections[currentSection][key] = value;
      }
    }

    return sections;
  }
}
```

### 4.3 Service Enable/Disable Best Practices

**From PM2 Research**:
- PM2 uses `pm2 startup` to generate systemd service
- Runs without sudo initially, then displays sudo command for user
- Saves process list with `pm2 save`
- Updates on upgrade with `pm2 update`

**For Swictation**:

```javascript
async function setupSystemdService() {
  const manager = new SystemdServiceManager();

  // Phase 1: Install service files
  await manager.installService('swictation-daemon.service');
  await manager.installService('swictation-ui.service');

  // Phase 2: Ask user about auto-start
  const answer = await inquirer.prompt([{
    type: 'confirm',
    name: 'enable',
    message: 'Enable swictation daemon to start automatically at login?',
    default: true
  }]);

  if (answer.enable) {
    try {
      execSync('systemctl --user enable swictation-daemon.service');
      log('green', '✓ Enabled swictation-daemon for automatic startup');

      const startNow = await inquirer.prompt([{
        type: 'confirm',
        name: 'start',
        message: 'Start the daemon now?',
        default: false
      }]);

      if (startNow.start) {
        execSync('systemctl --user start swictation-daemon.service');
        log('green', '✓ Started swictation-daemon');
      }
    } catch (err) {
      log('yellow', `⚠️  Could not enable service: ${err.message}`);
      log('cyan', '  You can enable it later with:');
      log('cyan', '  systemctl --user enable swictation-daemon.service');
    }
  }
}
```

---

## 5. Example Patterns from Other Packages

### 5.1 PM2 - Process Manager

**Key Patterns**:
- ✅ Separate startup script generation from service management
- ✅ Asks user permission before enabling auto-start
- ✅ Provides clear upgrade path (`pm2 update`)
- ✅ Saves process state for restore after reboot

**Code Pattern**:
```bash
# PM2 startup workflow
pm2 startup systemd  # Generates script, shows sudo command
# User runs: sudo env PATH=$PATH:... pm2 startup systemd -u user --hp /home/user
pm2 save            # Saves current process list
# After upgrade:
pm2 update          # Restarts PM2 daemon with new version
```

### 5.2 service-systemd npm Package

**Features**:
- Programmatic service installation/removal
- Handles user vs system services
- Template-based service generation
- Daemon reload after changes

**Usage Example**:
```javascript
const service = require('service-systemd');

service.add({
  name: 'my-app',
  cwd: '/path/to/app',
  app: 'index.js',
  args: ['--config', 'prod.json'],
  env: {
    NODE_ENV: 'production'
  }
}, (err) => {
  if (err) return console.error(err);
  console.log('Service installed');
});
```

### 5.3 node-windows (Comparison)

**Key Differences**:
- Windows: Uses NSSM or WinSW for service management
- Linux equivalent: systemd service files
- Migration path: node-windows → systemd requires rewrite

---

## 6. Security Considerations

### 6.1 Postinstall Script Security

**From npm Security Research**:

**Threats**:
- ❌ Arbitrary code execution during install
- ❌ Privilege escalation via sudo in postinstall
- ❌ Supply chain attacks via compromised dependencies

**Best Practices**:
- ✅ Use `npm install --ignore-scripts` by default in CI
- ✅ Never run postinstall as root
- ✅ Prompt for sudo only when absolutely necessary
- ✅ Use `inquirer` for user confirmation before system changes
- ✅ Validate all external downloads (checksums)

### 6.2 Service Security Hardening

**Current swictation service** (daemon.service):
```ini
# ⚠️  Security hardening COMMENTED OUT (lines 39-46)
#PrivateTmp=true
#ProtectSystem=strict
#ProtectHome=read-only
#ReadWritePaths=%h/.local/share/swictation
```

**Recommended**:
```ini
[Service]
# Process isolation
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.local/share/swictation
ReadWritePaths=%h/.cache/swictation
ReadWritePaths=%h/.config/swictation

# Prevent privilege escalation
NoNewPrivileges=true

# Restrict system calls
SystemCallFilter=@system-service
SystemCallFilter=~@privileged @resources

# Limit capabilities
CapabilityBoundingSet=
AmbientCapabilities=

# Network access (restrict if not needed)
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6

# Device access (USB audio devices)
DeviceAllow=/dev/snd rw
DevicePolicy=closed

# Read-only runtime
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Restrict realtime
RestrictRealtime=true
```

---

## 7. Interactive Installation Flow

### 7.1 Recommended User Experience

```javascript
async function interactivePostinstall() {
  console.clear();
  log('cyan', '╔═══════════════════════════════════════╗');
  log('cyan', '║     Swictation Installation Setup    ║');
  log('cyan', '╚═══════════════════════════════════════╝\n');

  // Phase 1: Platform checks (silent, auto-exit if incompatible)
  const platformOK = checkPlatform();
  if (!platformOK) process.exit(0);

  // Phase 2: Old installation cleanup (auto-detect, confirm before remove)
  const hasOldInstall = await detectOldInstallation();
  if (hasOldInstall) {
    const removeOld = await inquirer.prompt([{
      type: 'confirm',
      name: 'remove',
      message: 'Old installation detected. Remove old services and configs?',
      default: true
    }]);

    if (removeOld.remove) {
      await cleanOldServices();
    }
  }

  // Phase 3: GPU detection and library download
  const gpuDetector = new GPUDetector();
  const gpuInfo = await gpuDetector.detectGPU();
  gpuDetector.displayResults(gpuInfo);

  if (gpuInfo.compatible) {
    const downloadGPU = await inquirer.prompt([{
      type: 'confirm',
      name: 'download',
      message: 'Download GPU acceleration libraries? (~75MB)',
      default: true
    }]);

    if (downloadGPU.download) {
      await downloadGPULibraries();
    }
  }

  // Phase 4: Service installation
  await setupSystemdService();

  // Phase 5: Next steps
  showNextSteps(gpuInfo);
}
```

### 7.2 Non-Interactive Mode

```javascript
// Support CI/automated installations
const isNonInteractive = process.env.CI ||
                        process.env.npm_config_yes ||
                        !process.stdin.isTTY;

if (isNonInteractive) {
  // Use sensible defaults
  await nonInteractivePostinstall({
    cleanOld: true,
    downloadGPU: true,
    enableService: false,  // Don't auto-enable in CI
    skipPrompts: true
  });
} else {
  await interactivePostinstall();
}
```

---

## 8. Testing Strategy

### 8.1 Installation Scenarios to Test

**Fresh Installation**:
- [ ] Ubuntu 24.04 with NVIDIA GPU
- [ ] Ubuntu 24.04 without GPU (CPU-only)
- [ ] Ubuntu 22.04 (should warn about GLIBC)
- [ ] Debian 13 (Trixie)
- [ ] Fedora 39+

**Upgrade Scenarios**:
- [ ] Python version → v0.3.0
- [ ] v0.2.x → v0.3.0
- [ ] v0.3.0 → v0.3.0 (reinstall)
- [ ] With modified configs
- [ ] With running services

**Edge Cases**:
- [ ] nvidia-smi not in PATH
- [ ] CUDA installed but no GPU
- [ ] Service files exist but corrupted
- [ ] Insufficient VRAM
- [ ] No systemd (should gracefully degrade)

### 8.2 Test Automation

```javascript
// test-postinstall.js
const { describe, it, before, after } = require('mocha');
const { expect } = require('chai');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { execSync } = require('child_process');

describe('Postinstall Script', () => {
  let testDir;

  before(() => {
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-test-'));
  });

  after(() => {
    fs.rmSync(testDir, { recursive: true, force: true });
  });

  describe('Old Installation Cleanup', () => {
    it('should detect old Python-based services', async () => {
      // Create mock old service
      const oldService = path.join(testDir, 'swictation.service');
      fs.writeFileSync(oldService, '[Service]\nExecStart=/usr/bin/python3 ...');

      const version = detectServiceVersion(oldService);
      expect(version).to.equal('legacy-python');
    });

    it('should not remove Node.js services', async () => {
      // Create mock current service
      const currentService = path.join(testDir, 'swictation-daemon.service');
      fs.writeFileSync(currentService,
        '[Service]\nExecStart=/usr/local/lib/node_modules/swictation/...');

      const version = detectServiceVersion(currentService);
      expect(version).to.equal('current-nodejs');
    });
  });

  describe('GPU Detection', () => {
    it('should detect NVIDIA GPU via nvidia-smi', async () => {
      const detector = new GPUDetector();
      const result = await detector.detectGPU();

      // Will pass/fail based on test machine
      if (result.hasGPU) {
        expect(result.name).to.be.a('string');
        expect(result.vramMB).to.be.above(0);
      }
    });

    it('should handle missing nvidia-smi gracefully', async () => {
      // Mock PATH without nvidia-smi
      const oldPath = process.env.PATH;
      process.env.PATH = '/usr/bin';

      try {
        const detector = new GPUDetector();
        const result = await detector.detectGPU();

        expect(result.hasGPU).to.be.false;
        expect(result.errors).to.have.lengthOf.at.least(1);
      } finally {
        process.env.PATH = oldPath;
      }
    });
  });

  describe('Service Installation', () => {
    it('should create systemd user directory', () => {
      const systemdDir = path.join(testDir, '.config', 'systemd', 'user');
      createDirectories(); // from postinstall.js

      expect(fs.existsSync(systemdDir)).to.be.true;
    });

    it('should generate service files from templates', () => {
      const manager = new SystemdServiceManager();
      // Test with mock template
    });
  });
});
```

---

## 9. Specific Recommendations for Swictation

### 9.1 Immediate Improvements (Priority 1)

1. **Add Version Detection**
   - Detect if upgrading from Python version vs Node.js version
   - Skip cleanup if already on v0.3.0+
   - Store version marker in `~/.config/swictation/installed-version`

2. **Improve GPU Detection**
   - Add VRAM checking with warnings
   - Verify CUDA functionality, not just presence
   - Better error messages for missing drivers

3. **Add Config Migration**
   - Track installed config checksums
   - Prompt user before overwriting modified configs
   - Support drop-in overrides for services

4. **Non-Interactive Mode**
   - Detect CI environment
   - Support `--yes` flag for scripted installations
   - Skip all prompts with sensible defaults

### 9.2 Medium Priority (Priority 2)

1. **Service Management**
   - Use drop-in overrides instead of replacing service files
   - Migrate existing customizations automatically
   - Provide `swictation service` command for manual management

2. **Better Cleanup**
   - Create uninstall script
   - Track all installed files for removal
   - Backup user data before removal

3. **Validation**
   - Test ONNX Runtime library before finishing install
   - Verify systemd service can start
   - Check for common misconfigurations

### 9.3 Nice-to-Have (Priority 3)

1. **Installation Report**
   - Generate `/var/log/swictation-install.log`
   - Include system info, GPU detection results
   - List all installed files and services

2. **Health Check Command**
   - `swictation doctor` - check installation health
   - Verify all dependencies present
   - Test GPU acceleration
   - Report service status

3. **Migration Tool**
   - `swictation migrate-from-python` - helper for Python→Node.js
   - Import old configs automatically
   - Preserve user settings

---

## 10. Code Examples for Implementation

### 10.1 Version Detection and Skipping Redundant Cleanup

```javascript
function getInstalledVersion() {
  const versionFile = path.join(os.homedir(), '.config', 'swictation', '.version');

  if (fs.existsSync(versionFile)) {
    return fs.readFileSync(versionFile, 'utf8').trim();
  }

  // Check for old Python installation markers
  const oldMarkers = [
    '/usr/lib/systemd/user/swictation.service',
    path.join(os.homedir(), '.local', 'share', 'swictation', 'python-marker')
  ];

  for (const marker of oldMarkers) {
    if (fs.existsSync(marker)) {
      return 'legacy-python';
    }
  }

  return null;
}

function saveInstalledVersion() {
  const versionFile = path.join(os.homedir(), '.config', 'swictation', '.version');
  const currentVersion = require('./package.json').version;

  fs.writeFileSync(versionFile, currentVersion);
}

async function smartCleanup() {
  const installedVersion = getInstalledVersion();
  const currentVersion = require('./package.json').version;

  if (installedVersion === currentVersion) {
    log('green', '✓ Already running latest version - skipping cleanup');
    return { skipped: true, reason: 'same-version' };
  }

  if (installedVersion === 'legacy-python') {
    log('cyan', '🔄 Upgrading from Python version to Node.js version');
    return await cleanOldServices(); // Full cleanup needed
  }

  if (installedVersion && installedVersion !== currentVersion) {
    log('cyan', `🔄 Upgrading from v${installedVersion} to v${currentVersion}`);
    // Selective cleanup - only changed files
    return await incrementalUpgrade(installedVersion, currentVersion);
  }

  // Fresh installation
  log('cyan', '✨ Fresh installation detected');
  return { fresh: true };
}
```

### 10.2 Incremental Upgrade System

```javascript
class UpgradeManager {
  constructor() {
    this.migrations = {
      '0.2.0': this.upgradeTo_0_2_0.bind(this),
      '0.3.0': this.upgradeTo_0_3_0.bind(this),
      // Add future migrations here
    };
  }

  async upgrade(fromVersion, toVersion) {
    log('cyan', `\n🔄 Upgrading from ${fromVersion} to ${toVersion}`);

    // Find migrations between versions
    const migrationsToRun = this.getMigrationPath(fromVersion, toVersion);

    for (const version of migrationsToRun) {
      log('cyan', `  Running migration to ${version}...`);

      try {
        await this.migrations[version]();
        log('green', `  ✓ Migrated to ${version}`);
      } catch (err) {
        log('red', `  ✗ Migration failed: ${err.message}`);
        throw err;
      }
    }

    saveInstalledVersion();
    log('green', `\n✓ Upgrade complete: ${toVersion}`);
  }

  getMigrationPath(from, to) {
    const versions = Object.keys(this.migrations).sort();
    const fromIndex = versions.indexOf(from);
    const toIndex = versions.indexOf(to);

    if (fromIndex === -1 || toIndex === -1) {
      throw new Error(`Unknown version: ${from} → ${to}`);
    }

    return versions.slice(fromIndex + 1, toIndex + 1);
  }

  async upgradeTo_0_2_0() {
    // v0.2.0 specific migrations
    log('cyan', '    • Updating service file paths...');
    // ... migration code ...
  }

  async upgradeTo_0_3_0() {
    // v0.3.0 specific migrations
    log('cyan', '    • Cleaning up old Python services...');
    await cleanOldServices();

    log('cyan', '    • Migrating to new daemon architecture...');
    const manager = new SystemdServiceManager();
    await manager.installService('swictation-daemon.service');

    log('cyan', '    • Updating environment variables...');
    // Update CUDA paths, etc.
  }
}
```

---

## 11. Summary and Action Items

### Key Takeaways

1. **Multi-Phase Cleanup**: Stop services → Disable → Remove files → Reload daemon
2. **Config Tracking**: Use MD5 checksums to detect user modifications
3. **Interactive Prompts**: Use `inquirer` for user-friendly conflict resolution
4. **GPU Validation**: Test functionality, not just presence
5. **Drop-in Overrides**: Preserve user customizations while allowing updates
6. **Version Markers**: Track installed version to enable smart upgrades
7. **Non-Interactive Support**: Support CI/automated installations

### Recommended Implementation Order

**Phase 1 - Critical Fixes** (Implement Now):
- [ ] Add version detection to skip redundant cleanup
- [ ] Enhance GPU detection with VRAM checking
- [ ] Add non-interactive mode for CI environments
- [ ] Store version marker after successful install

**Phase 2 - User Experience** (Next Sprint):
- [ ] Add config file migration with user prompts
- [ ] Implement drop-in override support for services
- [ ] Create incremental upgrade system
- [ ] Add installation logging

**Phase 3 - Polish** (Future):
- [ ] Create `swictation doctor` health check
- [ ] Add comprehensive test suite
- [ ] Create migration tool from Python version
- [ ] Add installation report generation

---

## 12. References and Resources

### Documentation Sources

- **Debian Policy Manual**: https://www.debian.org/doc/debian-policy/
- **DpkgConffileHandling**: https://wiki.debian.org/DpkgConffileHandling
- **systemd.unit man page**: https://www.freedesktop.org/software/systemd/man/systemd.unit.html
- **systemd.service man page**: https://www.freedesktop.org/software/systemd/man/systemd.service.html
- **npm scripts documentation**: https://docs.npmjs.com/cli/v6/using-npm/scripts/
- **PM2 Documentation**: https://pm2.keymetrics.io/docs/usage/startup/
- **NVIDIA-SMI Documentation**: https://developer.nvidia.com/system-management-interface

### npm Packages Referenced

- **inquirer**: https://www.npmjs.com/package/inquirer (v8.2.5 - already a dependency)
- **service-systemd**: https://www.npmjs.com/package/service-systemd
- **PM2**: https://www.npmjs.com/package/pm2
- **which**: https://www.npmjs.com/package/which (v2.0.2 - already a dependency)

### Example Repositories

- **PM2**: https://github.com/Unitech/pm2
- **node-windows**: https://github.com/coreybutler/node-windows
- **service-systemd**: https://github.com/srvsh/service-systemd

---

## Appendix A: Current swictation Implementation Review

### Strengths ✅

1. **Multi-location service cleanup** (postinstall.js:66-72)
   - Checks `/usr/lib/systemd/user/`, `/usr/lib/systemd/system/`, `~/.config/systemd/user/`
   - Handles both system and user services

2. **GPU detection and library download** (postinstall.js:264-310)
   - Downloads GPU-accelerated libraries only when NVIDIA GPU detected
   - Falls back gracefully to CPU-only mode

3. **System requirements checking** (postinstall.js:22-57)
   - Validates Linux x64 platform
   - Checks GLIBC version for Ubuntu 24.04+ compatibility

4. **Binary permissions** (postinstall.js:141-161)
   - Ensures all executables are chmod 755

### Weaknesses ❌

1. **No version detection**
   - Runs full cleanup every time, even on reinstall
   - Can't distinguish Python version from Node.js version

2. **Service files overwritten**
   - User customizations lost on upgrade
   - No migration to drop-in overrides

3. **Limited GPU validation**
   - Only checks if nvidia-smi exists
   - Doesn't verify VRAM or CUDA functionality

4. **No config migration**
   - No tracking of modified configs
   - No prompts before overwriting

5. **Silent overwrites**
   - No user confirmation for destructive operations
   - Non-interactive mode not supported

6. **No rollback mechanism**
   - If installation fails, system left in inconsistent state
   - No backup of old installation

### Security Concerns ⚠️

1. **sudo in postinstall** (postinstall.js:88, 96, 105, 129)
   - Uses sudo for system service operations
   - Could be exploited in supply chain attack

2. **Service hardening disabled** (config/swictation-daemon.service:39-46)
   - All security directives commented out
   - Running with full user permissions

3. **External downloads** (postinstall.js:289)
   - Downloads from GitHub releases without checksum verification
   - Could be MITM attacked

---

## Appendix B: Sample Drop-in Override

### Example: Custom Resource Limits

**Base Service** (`~/.config/systemd/user/swictation-daemon.service`):
```ini
[Unit]
Description=Swictation Voice-to-Text Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/lib/node_modules/swictation/bin/swictation-daemon
Restart=on-failure
RestartSec=5
Environment="RUST_LOG=info"
Environment="ORT_DYLIB_PATH=/usr/local/lib/node_modules/swictation/lib/native/libonnxruntime.so"

[Install]
WantedBy=default.target
```

**User's Drop-in Override** (`~/.config/systemd/user/swictation-daemon.service.d/override.conf`):
```ini
[Service]
# User's customization: Limit memory for low-RAM system
MemoryMax=4G

# User's customization: Higher logging for debugging
Environment="RUST_LOG=debug"

# User's customization: Use different CUDA path
Environment="LD_LIBRARY_PATH=/opt/cuda/lib64:/usr/local/lib/node_modules/swictation/lib/native"
```

**Effective Service** (merged by systemd):
- Base service provides defaults
- Drop-in overrides only specific directives
- User customizations preserved across package updates
- Package can update base service without losing overrides

---

## Appendix C: Installation Test Checklist

### Pre-Installation Checks
- [ ] Verify platform (Linux x64)
- [ ] Check GLIBC version (2.39+)
- [ ] Test internet connectivity (for downloads)
- [ ] Check available disk space (10GB recommended)

### Fresh Installation Tests
- [ ] Create all required directories
- [ ] Set binary permissions correctly
- [ ] Download GPU libraries if NVIDIA detected
- [ ] Generate service files from templates
- [ ] Install service files to correct location
- [ ] Create version marker file

### Upgrade Tests
- [ ] Detect existing version correctly
- [ ] Stop running services gracefully
- [ ] Backup modified configs
- [ ] Migrate service customizations to drop-ins
- [ ] Update version marker
- [ ] Restart services with new version

### GPU Detection Tests
- [ ] nvidia-smi in PATH
- [ ] nvidia-smi not in PATH but available
- [ ] NVIDIA GPU present and functional
- [ ] NVIDIA drivers installed but no GPU
- [ ] No NVIDIA hardware (CPU-only)
- [ ] Insufficient VRAM warning

### Service Management Tests
- [ ] Enable service without starting
- [ ] Enable and start service immediately
- [ ] Disable service without stopping
- [ ] Stop service before disable
- [ ] Reload daemon after file changes
- [ ] Preserve drop-in overrides on upgrade

### Error Handling Tests
- [ ] Network timeout during download
- [ ] Insufficient permissions for systemd
- [ ] Corrupted service file
- [ ] Missing ONNX Runtime library
- [ ] systemd not available

### Cleanup Tests
- [ ] Remove all installed files
- [ ] Stop and disable services
- [ ] Remove systemd service files
- [ ] Preserve user data (optional)
- [ ] Leave system in clean state

---

*End of Installation Research Document*
