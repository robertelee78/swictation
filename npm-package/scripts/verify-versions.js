#!/usr/bin/env node

/**
 * Version Verification Script
 *
 * Validates that all package.json files have synchronized versions before publishing.
 * Ensures swictation, @swictation/linux-x64, and @swictation/darwin-arm64 all have
 * the same version as specified in versions.json.
 *
 * Usage:
 *   node scripts/verify-versions.js [--verbose] [--allow-missing]
 *
 * Exit codes:
 *   0 - All versions are synchronized
 *   1 - Versions are out of sync or validation failed
 *   2 - File read/parse error
 */

const fs = require('fs');
const path = require('path');

// Colors for terminal output
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  red: '\x1b[31m',
  dim: '\x1b[2m',
  bold: '\x1b[1m'
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function logDim(message) {
  console.log(`${colors.dim}${message}${colors.reset}`);
}

function logBold(message) {
  console.log(`${colors.bold}${message}${colors.reset}`);
}

// Parse command line arguments
const args = process.argv.slice(2);
const isVerbose = args.includes('--verbose');
const allowMissing = args.includes('--allow-missing');

// Paths
const rootDir = path.join(__dirname, '..');
const versionsFile = path.join(rootDir, 'versions.json');
const mainPackageFile = path.join(rootDir, 'package.json');
const linuxPackageFile = path.join(rootDir, 'packages', 'linux-x64', 'package.json');
const macosPackageFile = path.join(rootDir, 'packages', 'darwin-arm64', 'package.json');

/**
 * Read and parse versions.json
 */
function readVersions() {
  if (isVerbose) {
    log('cyan', '📖 Reading versions.json...');
  }

  if (!fs.existsSync(versionsFile)) {
    log('red', '❌ ERROR: versions.json not found');
    logDim(`   Expected at: ${versionsFile}`);
    process.exit(2);
  }

  try {
    const content = fs.readFileSync(versionsFile, 'utf8');
    const versions = JSON.parse(content);

    if (!versions.distribution) {
      log('red', '❌ ERROR: versions.json missing "distribution" field');
      process.exit(2);
    }

    const semverRegex = /^\d+\.\d+\.\d+$/;
    if (!semverRegex.test(versions.distribution)) {
      log('red', `❌ ERROR: Invalid distribution version format: "${versions.distribution}"`);
      log('yellow', '   Expected semver format: X.Y.Z');
      process.exit(2);
    }

    if (isVerbose) {
      log('green', `✓ Distribution version: ${versions.distribution}`);
    }

    return versions;
  } catch (err) {
    log('red', '❌ ERROR: Failed to parse versions.json');
    logDim(`   ${err.message}`);
    process.exit(2);
  }
}

/**
 * Verify a package.json file has correct version
 */
function verifyPackageJson(filePath, expectedVersion, packageName, isMainPackage = false) {
  const relativePath = path.relative(rootDir, filePath);

  if (isVerbose) {
    log('cyan', `\n📦 Verifying ${relativePath}...`);
  }

  // Check if file exists
  if (!fs.existsSync(filePath)) {
    if (allowMissing) {
      if (isVerbose) {
        log('yellow', `⚠️  File not found (allowed with --allow-missing)`);
      }
      return { missing: true, valid: true };
    } else {
      log('red', `❌ ${packageName}: package.json not found`);
      logDim(`   Expected at: ${filePath}`);
      logDim(`   Hint: Use --allow-missing if platform packages not created yet`);
      return { missing: true, valid: false };
    }
  }

  // Read and parse package.json
  let pkg;
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    pkg = JSON.parse(content);
  } catch (err) {
    log('red', `❌ ${packageName}: Failed to parse package.json`);
    logDim(`   ${err.message}`);
    return { error: true, valid: false };
  }

  const errors = [];

  // Verify version field
  if (!pkg.version) {
    errors.push('version field missing');
  } else if (pkg.version !== expectedVersion) {
    errors.push(`version mismatch: ${pkg.version} (expected ${expectedVersion})`);
  }

  // For main package, verify optionalDependencies
  if (isMainPackage && pkg.optionalDependencies) {
    if (pkg.optionalDependencies['@swictation/linux-x64']) {
      const linuxVersion = pkg.optionalDependencies['@swictation/linux-x64'];
      if (linuxVersion !== expectedVersion) {
        errors.push(`optionalDependencies.@swictation/linux-x64 mismatch: ${linuxVersion} (expected ${expectedVersion})`);
      }
    }

    if (pkg.optionalDependencies['@swictation/darwin-arm64']) {
      const macosVersion = pkg.optionalDependencies['@swictation/darwin-arm64'];
      if (macosVersion !== expectedVersion) {
        errors.push(`optionalDependencies.@swictation/darwin-arm64 mismatch: ${macosVersion} (expected ${expectedVersion})`);
      }
    }
  }

  // Report results
  if (errors.length > 0) {
    log('red', `❌ ${packageName}: FAILED`);
    errors.forEach(err => logDim(`   - ${err}`));
    return { errors, valid: false };
  } else {
    if (isVerbose) {
      log('green', `✓ ${packageName}: OK (version ${pkg.version})`);
    }
    return { valid: true, version: pkg.version };
  }
}

/**
 * Main execution
 */
function main() {
  logBold('\n═══════════════════════════════════════════');
  logBold('  Swictation Version Verification');
  logBold('═══════════════════════════════════════════');

  // Read versions.json
  const versions = readVersions();
  const expectedVersion = versions.distribution;

  log('cyan', `\n🎯 Expected version: ${expectedVersion}\n`);

  // Track results
  const results = {
    main: verifyPackageJson(mainPackageFile, expectedVersion, 'swictation (main)', true),
    linux: verifyPackageJson(linuxPackageFile, expectedVersion, '@swictation/linux-x64', false),
    macos: verifyPackageJson(macosPackageFile, expectedVersion, '@swictation/darwin-arm64', false)
  };

  // Summary
  logBold('\n═══════════════════════════════════════════');
  logBold('  Summary');
  logBold('═══════════════════════════════════════════\n');

  const packages = [
    { name: 'swictation (main)', result: results.main },
    { name: '@swictation/linux-x64', result: results.linux },
    { name: '@swictation/darwin-arm64', result: results.macos }
  ];

  let validCount = 0;
  let missingCount = 0;
  let errorCount = 0;

  packages.forEach(({ name, result }) => {
    if (result.error) {
      log('red', `❌ ${name}: ERROR`);
      errorCount++;
    } else if (result.missing) {
      if (allowMissing) {
        log('yellow', `⚠️  ${name}: MISSING (allowed)`);
        validCount++; // Count as valid since we're allowing missing
      } else {
        log('red', `❌ ${name}: MISSING`);
        errorCount++;
      }
      missingCount++;
    } else if (!result.valid) {
      log('red', `❌ ${name}: OUT OF SYNC`);
      errorCount++;
    } else {
      log('green', `✓ ${name}: OK`);
      validCount++;
    }
  });

  console.log('');
  log('cyan', `Distribution version: ${expectedVersion}`);
  log('green', `Valid: ${validCount}`);
  if (missingCount > 0) {
    log('yellow', `Missing: ${missingCount}`);
  }
  if (errorCount > 0) {
    log('red', `Errors: ${errorCount}`);
  }

  // Exit with appropriate code
  if (errorCount > 0) {
    log('red', '\n❌ Version verification FAILED');
    log('yellow', '\n   Fix versions by running: npm run version:sync');
    process.exit(1);
  } else if (missingCount > 0 && !allowMissing) {
    log('red', '\n❌ Version verification FAILED - platform packages missing');
    log('yellow', '\n   Options:');
    logDim('     1. Create platform packages first (recommended)');
    logDim('     2. Run with --allow-missing flag to skip validation');
    process.exit(1);
  } else {
    log('green', '\n✅ All versions are synchronized');
    log('cyan', '\n   Safe to publish packages');
    process.exit(0);
  }
}

// Run
main();
