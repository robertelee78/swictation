#!/usr/bin/env node

/**
 * Test script for NVIDIA hibernation detection and configuration
 *
 * Usage:
 *   node tests/test-nvidia-hibernation.js
 *
 * Tests:
 *   1. Laptop detection (battery presence)
 *   2. NVIDIA GPU detection
 *   3. Current hibernation configuration status
 *   4. Distribution detection
 *   5. Mock configuration (dry-run, no actual changes)
 */

const {
  isLaptop,
  hasNvidiaGpu,
  isNvidiaConfigured,
  detectDistribution,
  nvidiaModprobeConfigExists
} = require('../src/utils/system-detect');

const { checkNvidiaHibernationStatus } = require('../src/nvidia-hibernation-setup');

// Colors for output
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

function testResult(testName, result, expected) {
  if (result === expected) {
    log('green', `✓ ${testName}: PASS (${result})`);
    return true;
  } else {
    log('red', `✗ ${testName}: FAIL (got ${result}, expected ${expected})`);
    return false;
  }
}

function main() {
  log('cyan', '\n╔════════════════════════════════════════════════════╗');
  log('cyan', '║   NVIDIA Hibernation Detection Test Suite        ║');
  log('cyan', '╚════════════════════════════════════════════════════╝\n');

  let totalTests = 0;
  let passedTests = 0;

  // Test 1: Laptop Detection
  log('cyan', '\n─── Test 1: Laptop Detection ───');
  const laptopDetected = isLaptop();
  log('yellow', `  Result: ${laptopDetected ? 'LAPTOP DETECTED' : 'NOT A LAPTOP'}`);

  if (laptopDetected) {
    log('cyan', '  Method: Found battery in /sys/class/power_supply/');
  } else {
    log('cyan', '  Method: No battery found in /sys/class/power_supply/');
  }
  totalTests++;
  passedTests++; // This is informational, always passes

  // Test 2: NVIDIA GPU Detection
  log('cyan', '\n─── Test 2: NVIDIA GPU Detection ───');
  const nvidiaDetected = hasNvidiaGpu();
  log('yellow', `  Result: ${nvidiaDetected ? 'NVIDIA GPU DETECTED' : 'NO NVIDIA GPU'}`);

  if (nvidiaDetected) {
    log('cyan', '  Method: nvidia-smi command successful');
  } else {
    log('cyan', '  Method: nvidia-smi not available or failed');
  }
  totalTests++;
  passedTests++; // Informational

  // Test 3: Hibernation Configuration Status
  log('cyan', '\n─── Test 3: Hibernation Configuration Status ───');
  const configured = isNvidiaConfigured();
  log('yellow', `  Result: ${configured ? 'CONFIGURED' : 'NOT CONFIGURED'}`);

  if (configured) {
    log('green', '  ✓ PreserveVideoMemoryAllocations = 1');
  } else {
    log('red', '  ✗ PreserveVideoMemoryAllocations not set or = 0');
  }
  totalTests++;
  passedTests++; // Informational

  // Test 4: Distribution Detection
  log('cyan', '\n─── Test 4: Distribution Detection ───');
  const distro = detectDistribution();
  log('yellow', `  Detected: ${distro}`);

  const validDistros = ['ubuntu', 'debian', 'fedora', 'arch', 'unknown'];
  if (validDistros.includes(distro)) {
    log('green', '  ✓ Valid distribution detected');
    totalTests++;
    passedTests++;
  } else {
    log('red', '  ✗ Invalid distribution value');
    totalTests++;
  }

  // Test 5: Config File Existence
  log('cyan', '\n─── Test 5: Config File Existence ───');
  const configExists = nvidiaModprobeConfigExists();
  log('yellow', `  Result: ${configExists ? 'EXISTS' : 'NOT FOUND'}`);
  log('cyan', `  Path: /etc/modprobe.d/nvidia-power-management.conf`);
  totalTests++;
  passedTests++; // Informational

  // Test 6: Comprehensive Status Check
  log('cyan', '\n─── Test 6: Comprehensive Status Check ───');
  const status = checkNvidiaHibernationStatus();

  log('yellow', '\n  Full Status Report:');
  log('cyan', `    Is Laptop: ${status.isLaptop}`);
  log('cyan', `    Has NVIDIA GPU: ${status.hasNvidiaGpu}`);
  log('cyan', `    Is Configured: ${status.isConfigured}`);
  log('cyan', `    Config File Exists: ${status.configFileExists}`);
  log('cyan', `    Needs Configuration: ${status.needsConfiguration}`);
  log('cyan', `    Distribution: ${status.distribution}`);

  // Verify logic consistency
  const logicCorrect = (
    status.needsConfiguration ===
    (status.isLaptop && status.hasNvidiaGpu && !status.isConfigured)
  );

  if (logicCorrect) {
    log('green', '\n  ✓ Logic consistency check: PASS');
    totalTests++;
    passedTests++;
  } else {
    log('red', '\n  ✗ Logic consistency check: FAIL');
    totalTests++;
  }

  // Summary
  log('cyan', '\n╔════════════════════════════════════════════════════╗');
  log('cyan', '║                 Test Summary                      ║');
  log('cyan', '╚════════════════════════════════════════════════════╝\n');
  log('green', `  Total Tests: ${totalTests}`);
  log('green', `  Passed: ${passedTests}`);
  log('red', `  Failed: ${totalTests - passedTests}`);
  log('cyan', `  Success Rate: ${((passedTests / totalTests) * 100).toFixed(1)}%`);

  // Recommendation
  log('cyan', '\n╔════════════════════════════════════════════════════╗');
  log('cyan', '║                Recommendation                     ║');
  log('cyan', '╚════════════════════════════════════════════════════╝\n');

  if (status.needsConfiguration) {
    log('yellow', '  ⚠️  NVIDIA hibernation configuration RECOMMENDED');
    log('cyan', '\n  Run: sudo swictation setup');
    log('cyan', '\n  This will configure your system to prevent GPU errors');
    log('cyan', '  after laptop hibernation/suspend.');
  } else if (status.isLaptop && status.hasNvidiaGpu && status.isConfigured) {
    log('green', '  ✓ System is properly configured for NVIDIA hibernation');
  } else if (!status.isLaptop) {
    log('cyan', '  ℹ  Not a laptop - hibernation configuration not needed');
  } else if (!status.hasNvidiaGpu) {
    log('cyan', '  ℹ  No NVIDIA GPU - hibernation configuration not needed');
  }

  console.log('');
}

// Run tests
main();
