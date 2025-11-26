#!/usr/bin/env node

/**
 * Version Synchronization Script
 *
 * Reads versions.json and updates all package.json files to have matching
 * distribution versions. Ensures swictation, @swictation/linux-x64, and
 * @swictation/darwin-arm64 always have the same version.
 *
 * Usage:
 *   node scripts/sync-versions.js [--dry-run] [--verbose]
 *
 * Exit codes:
 *   0 - Success
 *   1 - Validation error or file not found
 *   2 - JSON parse error
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
  dim: '\x1b[2m'
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function logDim(message) {
  console.log(`${colors.dim}${message}${colors.reset}`);
}

// Parse command line arguments
const args = process.argv.slice(2);
const isDryRun = args.includes('--dry-run');
const isVerbose = args.includes('--verbose');

if (isDryRun) {
  log('cyan', '\n🔍 DRY RUN MODE - No files will be modified\n');
}

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
  log('cyan', '📖 Reading versions.json...');

  if (!fs.existsSync(versionsFile)) {
    log('red', `❌ ERROR: versions.json not found at ${versionsFile}`);
    log('yellow', '   Run this script from npm-package/ directory');
    process.exit(1);
  }

  try {
    const content = fs.readFileSync(versionsFile, 'utf8');
    const versions = JSON.parse(content);

    // Validate required fields
    if (!versions.distribution) {
      log('red', '❌ ERROR: versions.json missing "distribution" field');
      process.exit(1);
    }

    // Validate semver format
    const semverRegex = /^\d+\.\d+\.\d+$/;
    if (!semverRegex.test(versions.distribution)) {
      log('red', `❌ ERROR: Invalid version format "${versions.distribution}"`);
      log('yellow', '   Expected semver format: X.Y.Z');
      process.exit(1);
    }

    log('green', `✓ Distribution version: ${versions.distribution}`);

    if (isVerbose && versions.components) {
      logDim(`  Component versions:`);
      for (const [name, info] of Object.entries(versions.components)) {
        if (info.version) {
          logDim(`    - ${name}: ${info.version}`);
        }
      }
    }

    return versions;
  } catch (err) {
    log('red', `❌ ERROR: Failed to parse versions.json`);
    log('yellow', `   ${err.message}`);
    process.exit(2);
  }
}

/**
 * Update a package.json file
 */
function updatePackageJson(filePath, distributionVersion, isMainPackage = false) {
  const packageName = path.basename(path.dirname(filePath));
  const relativePath = path.relative(rootDir, filePath);

  log('cyan', `\n📦 Processing ${relativePath}...`);

  // Check if file exists
  if (!fs.existsSync(filePath)) {
    log('yellow', `⚠️  File not found (will be created in later tasks)`);
    log('dim', `   Skipping: ${filePath}`);
    return { skipped: true, reason: 'not found' };
  }

  // Read and parse package.json
  let pkg;
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    pkg = JSON.parse(content);
  } catch (err) {
    log('red', `❌ ERROR: Failed to parse ${relativePath}`);
    log('yellow', `   ${err.message}`);
    return { error: true, reason: err.message };
  }

  const currentVersion = pkg.version;
  let modified = false;
  const changes = [];

  // Update version
  if (currentVersion !== distributionVersion) {
    changes.push(`version: ${currentVersion} → ${distributionVersion}`);
    pkg.version = distributionVersion;
    modified = true;
  }

  // Update optionalDependencies in main package
  if (isMainPackage && pkg.optionalDependencies) {
    if (pkg.optionalDependencies['@swictation/linux-x64']) {
      const currentLinux = pkg.optionalDependencies['@swictation/linux-x64'];
      if (currentLinux !== distributionVersion) {
        changes.push(`optionalDependencies.@swictation/linux-x64: ${currentLinux} → ${distributionVersion}`);
        pkg.optionalDependencies['@swictation/linux-x64'] = distributionVersion;
        modified = true;
      }
    }

    if (pkg.optionalDependencies['@swictation/darwin-arm64']) {
      const currentMacos = pkg.optionalDependencies['@swictation/darwin-arm64'];
      if (currentMacos !== distributionVersion) {
        changes.push(`optionalDependencies.@swictation/darwin-arm64: ${currentMacos} → ${distributionVersion}`);
        pkg.optionalDependencies['@swictation/darwin-arm64'] = distributionVersion;
        modified = true;
      }
    }
  }

  // Report changes
  if (changes.length > 0) {
    log('yellow', '📝 Changes:');
    changes.forEach(change => logDim(`   - ${change}`));

    if (!isDryRun) {
      try {
        const newContent = JSON.stringify(pkg, null, 2) + '\n';
        fs.writeFileSync(filePath, newContent, 'utf8');
        log('green', '✓ Updated successfully');
      } catch (err) {
        log('red', `❌ ERROR: Failed to write ${relativePath}`);
        log('yellow', `   ${err.message}`);
        return { error: true, reason: err.message };
      }
    } else {
      log('cyan', '✓ Would update (dry run)');
    }
  } else {
    log('green', '✓ Already synchronized');
  }

  return { modified, changes, currentVersion, newVersion: distributionVersion };
}

/**
 * Main execution
 */
function main() {
  log('cyan', '═══════════════════════════════════════════');
  log('cyan', '  Swictation Version Synchronization');
  log('cyan', '═══════════════════════════════════════════');

  // Read versions
  const versions = readVersions();
  const distVersion = versions.distribution;

  // Track results
  const results = {
    main: null,
    linux: null,
    macos: null
  };

  // Update main package
  results.main = updatePackageJson(mainPackageFile, distVersion, true);

  // Update platform packages
  results.linux = updatePackageJson(linuxPackageFile, distVersion, false);
  results.macos = updatePackageJson(macosPackageFile, distVersion, false);

  // Summary
  log('cyan', '\n═══════════════════════════════════════════');
  log('cyan', '  Summary');
  log('cyan', '═══════════════════════════════════════════\n');

  const processed = [
    { name: 'swictation (main)', result: results.main },
    { name: '@swictation/linux-x64', result: results.linux },
    { name: '@swictation/darwin-arm64', result: results.macos }
  ];

  let modifiedCount = 0;
  let skippedCount = 0;
  let errorCount = 0;
  let unchangedCount = 0;

  processed.forEach(({ name, result }) => {
    if (result.error) {
      log('red', `❌ ${name}: ERROR`);
      errorCount++;
    } else if (result.skipped) {
      log('yellow', `⚠️  ${name}: SKIPPED (${result.reason})`);
      skippedCount++;
    } else if (result.modified) {
      log('green', `✓ ${name}: UPDATED (${result.changes.length} changes)`);
      modifiedCount++;
    } else {
      log('green', `✓ ${name}: UP TO DATE`);
      unchangedCount++;
    }
  });

  console.log('');
  log('cyan', `Distribution version: ${distVersion}`);
  log('cyan', `Modified: ${modifiedCount}`);
  log('cyan', `Unchanged: ${unchangedCount}`);
  if (skippedCount > 0) {
    log('yellow', `Skipped: ${skippedCount}`);
  }
  if (errorCount > 0) {
    log('red', `Errors: ${errorCount}`);
  }

  if (isDryRun) {
    log('cyan', '\n🔍 DRY RUN COMPLETE - No files were modified');
  }

  // Exit with appropriate code
  if (errorCount > 0) {
    log('red', '\n❌ Synchronization failed');
    process.exit(1);
  } else if (modifiedCount > 0 && !isDryRun) {
    log('green', '\n✅ Synchronization complete');
    process.exit(0);
  } else if (skippedCount === processed.length) {
    log('yellow', '\n⚠️  All packages skipped (platform packages not created yet)');
    log('yellow', '   This is normal during initial setup');
    process.exit(0);
  } else {
    log('green', '\n✅ All packages already synchronized');
    process.exit(0);
  }
}

// Run
main();
