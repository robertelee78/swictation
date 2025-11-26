#!/usr/bin/env node

/**
 * Publishing Automation Script
 *
 * Publishes all three Swictation packages in the correct order:
 * 1. Verify versions are synchronized
 * 2. Publish @swictation/linux-x64 (with --access public)
 * 3. Publish @swictation/darwin-arm64 (with --access public)
 * 4. Wait for platform packages to be available on npm registry
 * 5. Publish main swictation package
 *
 * Usage:
 *   node scripts/publish-all.js [--dry-run] [--skip-verify] [--tag <tag>] [--verbose]
 *
 * Options:
 *   --dry-run       Test publishing without actually uploading (npm publish --dry-run)
 *   --skip-verify   Skip version verification (not recommended)
 *   --tag <tag>     Publish with a specific npm dist-tag (default: latest)
 *   --verbose       Show detailed output
 *   --help          Show this help message
 *
 * Environment:
 *   NPM_TOKEN       npm authentication token (required for CI/CD)
 *
 * Exit codes:
 *   0 - All packages published successfully
 *   1 - Publishing failed
 *   2 - Pre-publish validation failed
 *   3 - npm authentication failed
 */

const { execSync, spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const https = require('https');

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
const isDryRun = args.includes('--dry-run');
const skipVerify = args.includes('--skip-verify');
const isVerbose = args.includes('--verbose');
const showHelp = args.includes('--help') || args.includes('-h');

// Parse --tag option
let distTag = 'latest';
const tagIndex = args.indexOf('--tag');
if (tagIndex !== -1 && args[tagIndex + 1]) {
  distTag = args[tagIndex + 1];
}

if (showHelp) {
  console.log(`
Swictation Publishing Automation

Usage: node scripts/publish-all.js [options]

Options:
  --dry-run       Test publishing without actually uploading
  --skip-verify   Skip version verification (not recommended)
  --tag <tag>     Publish with a specific npm dist-tag (default: latest)
  --verbose       Show detailed output
  --help          Show this help message

Environment Variables:
  NPM_TOKEN       npm authentication token (required for CI/CD)

Example:
  node scripts/publish-all.js --dry-run
  node scripts/publish-all.js --tag beta
  node scripts/publish-all.js --verbose
`);
  process.exit(0);
}

// Paths
const rootDir = path.join(__dirname, '..');
const versionsFile = path.join(rootDir, 'versions.json');
const linuxPackageDir = path.join(rootDir, 'packages', 'linux-x64');
const macosPackageDir = path.join(rootDir, 'packages', 'darwin-arm64');
const mainPackageDir = rootDir;

/**
 * Execute a shell command and return output
 */
function exec(command, options = {}) {
  if (isVerbose) {
    logDim(`   $ ${command}`);
  }
  try {
    const output = execSync(command, {
      encoding: 'utf8',
      stdio: isVerbose ? 'inherit' : 'pipe',
      ...options
    });
    return output;
  } catch (err) {
    if (!isVerbose) {
      console.error(err.stdout);
      console.error(err.stderr);
    }
    throw err;
  }
}

/**
 * Sleep for specified milliseconds
 */
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Check if package is available on npm registry
 */
function checkPackageOnRegistry(packageName, version) {
  return new Promise((resolve) => {
    const url = `https://registry.npmjs.org/${packageName}/${version}`;

    https.get(url, (res) => {
      if (res.statusCode === 200) {
        resolve(true);
      } else {
        resolve(false);
      }
    }).on('error', () => {
      resolve(false);
    });
  });
}

/**
 * Wait for package to become available on npm registry
 */
async function waitForPackage(packageName, version, maxWaitSeconds = 300) {
  const startTime = Date.now();
  const maxWaitMs = maxWaitSeconds * 1000;

  log('cyan', `⏳ Waiting for ${packageName}@${version} to be available on npm...`);

  while (Date.now() - startTime < maxWaitMs) {
    const available = await checkPackageOnRegistry(packageName, version);

    if (available) {
      const waitTime = ((Date.now() - startTime) / 1000).toFixed(1);
      log('green', `✓ ${packageName}@${version} is available (waited ${waitTime}s)`);
      return true;
    }

    // Wait 5 seconds before next check
    await sleep(5000);

    if (isVerbose) {
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(0);
      logDim(`   Still waiting... (${elapsed}s elapsed)`);
    }
  }

  log('yellow', `⚠️  Timeout waiting for ${packageName}@${version}`);
  log('yellow', `   Package may still be propagating through npm CDN`);
  return false;
}

/**
 * Verify npm authentication
 */
function verifyNpmAuth() {
  log('cyan', '\n🔑 Verifying npm authentication...');

  try {
    const whoami = exec('npm whoami').trim();
    log('green', `✓ Authenticated as: ${whoami}`);
    return true;
  } catch (err) {
    log('red', '❌ npm authentication failed');
    log('yellow', '\n   Please run: npm login');
    log('yellow', '   Or set NPM_TOKEN environment variable for CI/CD');

    if (process.env.CI) {
      log('yellow', '\n   In GitHub Actions, add NPM_TOKEN to repository secrets:');
      logDim('     1. Go to Settings → Secrets and variables → Actions');
      logDim('     2. Add new secret: NPM_TOKEN');
      logDim('     3. Value: your npm automation token from npmjs.com');
    }

    return false;
  }
}

/**
 * Verify versions are synchronized
 */
function verifyVersions() {
  if (skipVerify) {
    log('yellow', '\n⚠️  Skipping version verification (--skip-verify flag)');
    return true;
  }

  log('cyan', '\n📋 Verifying versions are synchronized...');

  try {
    exec('node scripts/verify-versions.js', { cwd: rootDir });
    log('green', '✓ All versions synchronized');
    return true;
  } catch (err) {
    log('red', '❌ Version verification failed');
    log('yellow', '\n   Fix versions by running: npm run version:sync');
    return false;
  }
}

/**
 * Publish a package
 */
function publishPackage(packageDir, packageName, options = {}) {
  const { access = 'public', tag = 'latest', dryRun = false } = options;

  log('cyan', `\n📦 Publishing ${packageName}...`);
  logDim(`   Directory: ${packageDir}`);
  logDim(`   Access: ${access}`);
  logDim(`   Tag: ${tag}`);

  if (dryRun) {
    log('yellow', '   Mode: DRY RUN (not actually publishing)');
  }

  // Build npm publish command
  const publishArgs = [
    'publish',
    '--access', access,
    '--tag', tag
  ];

  if (dryRun) {
    publishArgs.push('--dry-run');
  }

  try {
    // Check that package.json exists
    const packageJsonPath = path.join(packageDir, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      throw new Error(`package.json not found in ${packageDir}`);
    }

    // Read package version
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    const version = packageJson.version;

    if (isVerbose) {
      logDim(`   Version: ${version}`);
    }

    // Execute npm publish
    exec(`npm ${publishArgs.join(' ')}`, { cwd: packageDir });

    if (dryRun) {
      log('green', `✓ ${packageName}@${version} - dry run successful`);
    } else {
      log('green', `✓ ${packageName}@${version} published`);
    }

    return { success: true, version, packageName };

  } catch (err) {
    log('red', `❌ Failed to publish ${packageName}`);

    if (err.message.includes('EPUBLISHCONFLICT')) {
      log('yellow', '   Package version already published');
      log('yellow', '   Bump version with: npm run version:bump');
    } else if (err.message.includes('ENEEDAUTH')) {
      log('yellow', '   Authentication required - run: npm login');
    } else if (err.message.includes('E403')) {
      log('yellow', '   Permission denied - check npm account has publish rights');
    } else {
      logDim(`   ${err.message}`);
    }

    return { success: false, error: err.message, packageName };
  }
}

/**
 * Main publishing flow
 */
async function main() {
  logBold('\n═══════════════════════════════════════════');
  logBold('  Swictation Publishing Automation');
  logBold('═══════════════════════════════════════════');

  if (isDryRun) {
    log('yellow', '\n🧪 DRY RUN MODE - No packages will actually be published\n');
  }

  // Read versions for display
  let distributionVersion = 'unknown';
  try {
    const versions = JSON.parse(fs.readFileSync(versionsFile, 'utf8'));
    distributionVersion = versions.distribution;
    log('cyan', `Distribution version: ${distributionVersion}`);
    log('cyan', `Dist tag: ${distTag}\n`);
  } catch (err) {
    log('red', '❌ ERROR: Could not read versions.json');
    process.exit(2);
  }

  // Step 1: Verify npm authentication (skip for dry-run)
  if (!isDryRun) {
    if (!verifyNpmAuth()) {
      process.exit(3);
    }
  } else {
    log('yellow', '\n⏭️  Skipping auth check (dry-run mode)');
  }

  // Step 2: Verify versions are synchronized
  if (!verifyVersions()) {
    process.exit(2);
  }

  // Step 3: Publish platform packages
  logBold('\n═══════════════════════════════════════════');
  logBold('  Publishing Platform Packages');
  logBold('═══════════════════════════════════════════');

  const linuxResult = publishPackage(linuxPackageDir, '@swictation/linux-x64', {
    access: 'public',
    tag: distTag,
    dryRun: isDryRun
  });

  if (!linuxResult.success) {
    log('red', '\n❌ Publishing failed');
    process.exit(1);
  }

  const macosResult = publishPackage(macosPackageDir, '@swictation/darwin-arm64', {
    access: 'public',
    tag: distTag,
    dryRun: isDryRun
  });

  if (!macosResult.success) {
    log('red', '\n❌ Publishing failed');
    process.exit(1);
  }

  // Step 4: Wait for platform packages to be available (skip for dry-run)
  if (!isDryRun) {
    logBold('\n═══════════════════════════════════════════');
    logBold('  Waiting for npm Registry Propagation');
    logBold('═══════════════════════════════════════════\n');

    const linuxAvailable = await waitForPackage('@swictation/linux-x64', linuxResult.version);
    const macosAvailable = await waitForPackage('@swictation/darwin-arm64', macosResult.version);

    if (!linuxAvailable || !macosAvailable) {
      log('yellow', '\n⚠️  Platform packages may not be fully propagated yet');
      log('yellow', '   Main package publication may fail if npm cannot find dependencies');
      log('yellow', '\n   Options:');
      logDim('     1. Wait a few more minutes and try again');
      logDim('     2. Continue anyway (main package install may fail initially)');

      // Still continue - npm will eventually propagate
    }
  } else {
    log('yellow', '\n⏭️  Skipping registry wait check (dry-run mode)');
  }

  // Step 5: Publish main package
  logBold('\n═══════════════════════════════════════════');
  logBold('  Publishing Main Package');
  logBold('═══════════════════════════════════════════');

  const mainResult = publishPackage(mainPackageDir, 'swictation', {
    access: 'public',
    tag: distTag,
    dryRun: isDryRun
  });

  if (!mainResult.success) {
    log('red', '\n❌ Publishing failed');
    process.exit(1);
  }

  // Success summary
  logBold('\n═══════════════════════════════════════════');
  logBold('  Publishing Complete');
  logBold('═══════════════════════════════════════════\n');

  log('green', '✅ All packages published successfully!\n');

  if (isDryRun) {
    log('yellow', '🧪 This was a dry run - no packages were actually published\n');
    log('cyan', '   To publish for real, run without --dry-run flag:');
    logDim('   $ node scripts/publish-all.js\n');
  } else {
    log('cyan', 'Published packages:');
    log('green', `  ✓ @swictation/linux-x64@${linuxResult.version}`);
    log('green', `  ✓ @swictation/darwin-arm64@${macosResult.version}`);
    log('green', `  ✓ swictation@${mainResult.version}\n`);

    log('cyan', 'Installation:');
    logDim(`  $ npm install -g swictation@${distTag}\n`);

    log('cyan', 'Registry:');
    logDim(`  https://www.npmjs.com/package/swictation\n`);
  }

  process.exit(0);
}

// Handle errors
process.on('unhandledRejection', (err) => {
  log('red', '\n❌ Unhandled error:');
  console.error(err);
  process.exit(1);
});

// Run
main().catch((err) => {
  log('red', '\n❌ Publishing failed:');
  console.error(err);
  process.exit(1);
});
