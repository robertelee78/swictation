const fs = require('fs');
const { execSync } = require('child_process');
const { isLaptop, hasNvidiaGpu, isNvidiaConfigured, detectDistribution, nvidiaModprobeConfigExists } = require('./utils/system-detect');

/**
 * Configure NVIDIA GPU for hibernation compatibility
 * This sets up the PreserveVideoMemoryAllocations kernel parameter
 *
 * @param {object} options - Configuration options
 * @param {boolean} options.interactive - Whether to prompt user for confirmation
 * @param {function} options.log - Logging function (defaults to console.log)
 * @returns {Promise<object>} Result object with success status and message
 */
async function configureNvidiaHibernation(options = {}) {
  const { interactive = false, log = console.log } = options;

  const result = {
    success: false,
    message: '',
    needsReboot: false
  };

  // Check if this is a laptop with NVIDIA GPU
  if (!isLaptop()) {
    result.message = 'Not a laptop - NVIDIA hibernation configuration not needed';
    result.success = true;
    return result;
  }

  if (!hasNvidiaGpu()) {
    result.message = 'No NVIDIA GPU detected - configuration not needed';
    result.success = true;
    return result;
  }

  // Check if already configured
  if (isNvidiaConfigured()) {
    result.message = 'NVIDIA hibernation support already configured';
    result.success = true;
    return result;
  }

  // If interactive, ask for confirmation
  if (interactive) {
    log('\n⚠️  NVIDIA GPU detected on laptop without hibernation support configured');
    log('   This can cause GPU errors after hibernation/suspend.');
    log('\n   Configuration required:');
    log('   - Create /etc/modprobe.d/nvidia-power-management.conf');
    log('   - Set NVreg_PreserveVideoMemoryAllocations=1');
    log('   - Update initramfs');
    log('   - Reboot required after configuration\n');

    // For now, return instruction message (full interactive prompt would require inquirer)
    result.message = 'Run "sudo swictation setup" to configure NVIDIA hibernation support';
    result.success = true;
    return result;
  }

  // Perform automatic configuration
  try {
    // Create modprobe configuration
    const configContent = `# NVIDIA Power Management for Laptop Hibernation
# Preserves GPU memory allocations during hibernation/suspend
# Reference: https://download.nvidia.com/XFree86/Linux-x86_64/latest/README/powermanagement.html

options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp
`;

    // Write config to temporary location first
    const tempConfigPath = '/tmp/nvidia-power-management.conf';
    fs.writeFileSync(tempConfigPath, configContent);
    log('  ✓ Created configuration file');

    // Copy to /etc/modprobe.d/ (requires sudo)
    execSync(`sudo cp ${tempConfigPath} /etc/modprobe.d/nvidia-power-management.conf`, { stdio: 'inherit' });
    log('  ✓ Installed configuration to /etc/modprobe.d/');

    // Clean up temp file
    fs.unlinkSync(tempConfigPath);

    // Update initramfs based on distribution
    const distro = detectDistribution();
    log(`  Detected distribution: ${distro}`);

    switch (distro) {
      case 'ubuntu':
      case 'debian':
        execSync('sudo update-initramfs -u', { stdio: 'inherit' });
        log('  ✓ Updated initramfs (update-initramfs)');
        break;

      case 'fedora':
        execSync('sudo dracut -f', { stdio: 'inherit' });
        log('  ✓ Updated initramfs (dracut)');
        break;

      case 'arch':
        execSync('sudo mkinitcpio -P', { stdio: 'inherit' });
        log('  ✓ Updated initramfs (mkinitcpio)');
        break;

      default:
        log('  ⚠️  Unknown distribution - please update initramfs manually');
        log('     Ubuntu/Debian: sudo update-initramfs -u');
        log('     Fedora: sudo dracut -f');
        log('     Arch: sudo mkinitcpio -P');
        break;
    }

    result.success = true;
    result.needsReboot = true;
    result.message = 'NVIDIA hibernation support configured successfully. Reboot required.';

    log('\n  ✅ Configuration complete!');
    log('  ⚠️  REBOOT REQUIRED for changes to take effect\n');

  } catch (err) {
    result.success = false;
    result.message = `Configuration failed: ${err.message}`;
    log(`\n  ❌ Configuration failed: ${err.message}`);
    log('  You can configure manually by running: sudo swictation setup\n');
  }

  return result;
}

/**
 * Check if NVIDIA hibernation configuration is needed
 * Returns diagnostic information without making changes
 *
 * @returns {object} Diagnostic information
 */
function checkNvidiaHibernationStatus() {
  return {
    isLaptop: isLaptop(),
    hasNvidiaGpu: hasNvidiaGpu(),
    isConfigured: isNvidiaConfigured(),
    configFileExists: nvidiaModprobeConfigExists(),
    needsConfiguration: isLaptop() && hasNvidiaGpu() && !isNvidiaConfigured(),
    distribution: detectDistribution()
  };
}

module.exports = {
  configureNvidiaHibernation,
  checkNvidiaHibernationStatus
};
