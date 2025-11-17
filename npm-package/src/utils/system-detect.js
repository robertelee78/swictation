const fs = require('fs');
const { execSync } = require('child_process');

/**
 * Detect if the system is a laptop by checking for battery presence
 * @returns {boolean} True if battery detected (laptop), false otherwise
 */
function isLaptop() {
  try {
    const powerSupplyDir = '/sys/class/power_supply';

    if (!fs.existsSync(powerSupplyDir)) {
      return false;
    }

    const supplies = fs.readdirSync(powerSupplyDir);

    // Check each power supply for battery type
    for (const supply of supplies) {
      try {
        const typePath = `${powerSupplyDir}/${supply}/type`;
        if (fs.existsSync(typePath)) {
          const type = fs.readFileSync(typePath, 'utf8').trim();
          if (type === 'Battery') {
            return true;
          }
        }
      } catch {
        // Skip this supply if we can't read it
        continue;
      }
    }

    return false;
  } catch {
    return false;
  }
}

/**
 * Detect if NVIDIA GPU is present and drivers are loaded
 * @returns {boolean} True if NVIDIA GPU detected, false otherwise
 */
function hasNvidiaGpu() {
  try {
    execSync('nvidia-smi', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Check if NVIDIA hibernation support is already configured
 * @returns {boolean} True if configured (parameter = 1), false otherwise
 */
function isNvidiaConfigured() {
  try {
    const paramPath = '/sys/module/nvidia/parameters/PreserveVideoMemoryAllocations';

    if (!fs.existsSync(paramPath)) {
      // Module not loaded or parameter doesn't exist
      return false;
    }

    const value = fs.readFileSync(paramPath, 'utf8').trim();
    return value === '1';
  } catch {
    return false;
  }
}

/**
 * Detect the Linux distribution to determine correct initramfs update command
 * @returns {string} Distribution name: 'ubuntu', 'debian', 'fedora', 'arch', or 'unknown'
 */
function detectDistribution() {
  try {
    const osRelease = fs.readFileSync('/etc/os-release', 'utf8').toLowerCase();

    if (osRelease.includes('ubuntu')) {
      return 'ubuntu';
    } else if (osRelease.includes('debian')) {
      return 'debian';
    } else if (osRelease.includes('fedora')) {
      return 'fedora';
    } else if (osRelease.includes('arch')) {
      return 'arch';
    }

    return 'unknown';
  } catch {
    return 'unknown';
  }
}

/**
 * Check if the NVIDIA modprobe config file already exists
 * @returns {boolean} True if config file exists, false otherwise
 */
function nvidiaModprobeConfigExists() {
  return fs.existsSync('/etc/modprobe.d/nvidia-power-management.conf');
}

module.exports = {
  isLaptop,
  hasNvidiaGpu,
  isNvidiaConfigured,
  detectDistribution,
  nvidiaModprobeConfigExists
};
