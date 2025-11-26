#!/usr/bin/env node

/**
 * Version Bumping Script
 *
 * Bumps the distribution version (major/minor/patch) in versions.json
 * and optionally extracts component versions from Cargo.toml files.
 * Automatically runs sync-versions.js after successful bump.
 *
 * Usage:
 *   node scripts/bump-version.js <major|minor|patch> [--dry-run] [--force-extract] [--verbose]
 *
 * Examples:
 *   node scripts/bump-version.js patch              # 0.7.9 → 0.7.10
 *   node scripts/bump-version.js minor              # 0.7.9 → 0.8.0
 *   node scripts/bump-version.js major              # 0.7.9 → 1.0.0
 *   node scripts/bump-version.js patch --dry-run    # Preview changes
 *   node scripts/bump-version.js patch --force-extract  # Re-extract all component versions
 *
 * Exit codes:
 *   0 - Success
 *   1 - Invalid arguments or validation error
 *   2 - File read/write error
 *   3 - Sync script execution failed
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

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
const bumpType = args.find(arg => !arg.startsWith('--'));
const isDryRun = args.includes('--dry-run');
const forceExtract = args.includes('--force-extract');
const isVerbose = args.includes('--verbose');

// Validate bump type
if (!bumpType || !['major', 'minor', 'patch'].includes(bumpType)) {
  log('red', '\n❌ ERROR: Invalid or missing bump type');
  log('yellow', '   Usage: node scripts/bump-version.js <major|minor|patch> [--dry-run] [--force-extract] [--verbose]');
  log('cyan', '\n   Examples:');
  logDim('     npm run version:bump patch              # 0.7.9 → 0.7.10');
  logDim('     npm run version:bump minor              # 0.7.9 → 0.8.0');
  logDim('     npm run version:bump major              # 0.7.9 → 1.0.0');
  logDim('     npm run version:bump patch --dry-run    # Preview changes');
  process.exit(1);
}

if (isDryRun) {
  log('cyan', '\n🔍 DRY RUN MODE - No files will be modified\n');
}

// Paths
const rootDir = path.join(__dirname, '..');
const versionsFile = path.join(rootDir, 'versions.json');
const syncScript = path.join(rootDir, 'scripts', 'sync-versions.js');

// Component Cargo.toml paths
const componentPaths = {
  daemon: {
    cargo: path.join(rootDir, '..', 'rust-crates', 'swictation-daemon', 'Cargo.toml'),
    description: 'Main daemon binary'
  },
  ui: {
    cargo: path.join(rootDir, '..', 'tauri-ui', 'src-tauri', 'Cargo.toml'),
    description: 'Tauri UI application'
  },
  'context-learning': {
    cargo: path.join(rootDir, '..', 'rust-crates', 'swictation-context-learning', 'Cargo.toml'),
    description: 'Context-aware meta-learning'
  }
};

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

    log('green', `✓ Current distribution version: ${versions.distribution}`);
    return versions;
  } catch (err) {
    log('red', `❌ ERROR: Failed to parse versions.json`);
    log('yellow', `   ${err.message}`);
    process.exit(2);
  }
}

/**
 * Bump a semantic version string
 */
function bumpVersion(currentVersion, type) {
  const parts = currentVersion.split('.').map(Number);

  switch (type) {
    case 'major':
      parts[0]++;
      parts[1] = 0;
      parts[2] = 0;
      break;
    case 'minor':
      parts[1]++;
      parts[2] = 0;
      break;
    case 'patch':
      parts[2]++;
      break;
  }

  return parts.join('.');
}

/**
 * Extract version from Cargo.toml file
 */
function extractCargoVersion(cargoPath) {
  if (!fs.existsSync(cargoPath)) {
    return null;
  }

  try {
    const content = fs.readFileSync(cargoPath, 'utf8');
    const match = content.match(/^version\s*=\s*"([^"]+)"/m);

    if (match && match[1]) {
      const version = match[1];
      const semverRegex = /^\d+\.\d+\.\d+$/;

      if (semverRegex.test(version)) {
        return version;
      } else {
        log('yellow', `⚠️  Warning: Invalid semver format in ${path.basename(cargoPath)}: ${version}`);
        return null;
      }
    }

    log('yellow', `⚠️  Warning: Could not find version field in ${path.basename(cargoPath)}`);
    return null;
  } catch (err) {
    log('yellow', `⚠️  Warning: Failed to read ${cargoPath}: ${err.message}`);
    return null;
  }
}

/**
 * Extract component versions from all Cargo.toml files
 */
function extractComponentVersions() {
  log('cyan', '\n📦 Extracting component versions from Cargo.toml files...');

  const extracted = {};
  let successCount = 0;
  let failCount = 0;

  for (const [component, info] of Object.entries(componentPaths)) {
    const version = extractCargoVersion(info.cargo);

    if (version) {
      extracted[component] = version;
      log('green', `✓ ${component}: ${version}`);
      if (isVerbose) {
        logDim(`  Source: ${path.relative(rootDir, info.cargo)}`);
      }
      successCount++;
    } else {
      log('red', `✗ ${component}: extraction failed`);
      failCount++;
    }
  }

  log('cyan', `\nExtracted ${successCount} component versions, ${failCount} failed`);

  return extracted;
}

/**
 * Update versions.json with new distribution version and component versions
 */
function updateVersionsFile(versions, newDistVersion, componentVersions) {
  log('cyan', '\n📝 Updating versions.json...');

  const changes = [];

  // Update distribution version
  const oldDistVersion = versions.distribution;
  if (oldDistVersion !== newDistVersion) {
    versions.distribution = newDistVersion;
    changes.push(`distribution: ${oldDistVersion} → ${newDistVersion}`);
  }

  // Update component versions if extracted
  if (componentVersions && Object.keys(componentVersions).length > 0) {
    if (!versions.components) {
      versions.components = {};
    }

    for (const [component, newVersion] of Object.entries(componentVersions)) {
      const oldVersion = versions.components[component]?.version;

      if (oldVersion !== newVersion) {
        if (!versions.components[component]) {
          versions.components[component] = {
            version: newVersion,
            source: componentPaths[component]?.cargo || '',
            description: componentPaths[component]?.description || ''
          };
        } else {
          versions.components[component].version = newVersion;
        }

        changes.push(`components.${component}.version: ${oldVersion || 'undefined'} → ${newVersion}`);
      }
    }
  }

  // Update metadata timestamp
  const oldTimestamp = versions.metadata?.last_updated;
  const newTimestamp = new Date().toISOString();

  if (!versions.metadata) {
    versions.metadata = {};
  }

  versions.metadata.last_updated = newTimestamp;
  changes.push(`metadata.last_updated: ${oldTimestamp || 'undefined'} → ${newTimestamp}`);

  // Report changes
  if (changes.length > 0) {
    log('yellow', '\nChanges to be made:');
    changes.forEach(change => logDim(`  - ${change}`));
  } else {
    log('green', '\nNo changes needed (already at target version)');
    return false;
  }

  // Write file (unless dry-run)
  if (!isDryRun) {
    try {
      const newContent = JSON.stringify(versions, null, 2) + '\n';
      fs.writeFileSync(versionsFile, newContent, 'utf8');
      log('green', '\n✓ versions.json updated successfully');
      return true;
    } catch (err) {
      log('red', `\n❌ ERROR: Failed to write versions.json`);
      log('yellow', `   ${err.message}`);
      process.exit(2);
    }
  } else {
    log('cyan', '\n✓ Would update versions.json (dry run)');
    return true;
  }
}

/**
 * Run sync-versions.js to propagate changes
 */
function runSyncScript() {
  log('cyan', '\n🔄 Running sync-versions.js to propagate changes...');

  if (isDryRun) {
    log('cyan', '✓ Would run sync-versions.js (dry run)');
    return;
  }

  try {
    const syncCommand = `node "${syncScript}"`;
    execSync(syncCommand, {
      cwd: rootDir,
      stdio: 'inherit'
    });
    log('green', '\n✓ Synchronization completed successfully');
  } catch (err) {
    log('red', '\n❌ ERROR: Synchronization failed');
    log('yellow', `   ${err.message}`);
    log('yellow', '\n   versions.json was updated, but package.json files may be out of sync');
    log('yellow', '   Run manually: npm run version:sync');
    process.exit(3);
  }
}

/**
 * Main execution
 */
function main() {
  logBold('\n═══════════════════════════════════════════');
  logBold('  Swictation Version Bumping');
  logBold('═══════════════════════════════════════════');

  // Read current versions
  const versions = readVersions();
  const currentVersion = versions.distribution;

  // Calculate new version
  const newVersion = bumpVersion(currentVersion, bumpType);

  log('cyan', `\n🎯 Bump type: ${bumpType}`);
  log('yellow', `📊 Version change: ${currentVersion} → ${newVersion}`);

  // Extract component versions if requested or if forcing
  let componentVersions = {};
  if (forceExtract) {
    componentVersions = extractComponentVersions();
  } else if (isVerbose) {
    log('cyan', '\n💡 Tip: Use --force-extract to update component versions from Cargo.toml');
  }

  // Update versions.json
  const wasModified = updateVersionsFile(versions, newVersion, componentVersions);

  // Run sync script if changes were made
  if (wasModified && !isDryRun) {
    runSyncScript();
  }

  // Summary
  logBold('\n═══════════════════════════════════════════');
  logBold('  Summary');
  logBold('═══════════════════════════════════════════\n');

  log('green', `✓ Distribution version: ${currentVersion} → ${newVersion}`);

  if (Object.keys(componentVersions).length > 0) {
    log('green', `✓ Component versions extracted: ${Object.keys(componentVersions).length}`);
  }

  if (isDryRun) {
    log('cyan', '\n🔍 DRY RUN COMPLETE - No files were modified');
    log('cyan', '   Run without --dry-run to apply changes');
  } else if (wasModified) {
    log('green', '\n✅ Version bump complete');
    log('cyan', '\nNext steps:');
    logDim('  1. Review the changes with: git diff versions.json package.json');
    logDim('  2. Test the build process');
    logDim('  3. Commit the version bump');
    logDim('  4. Create a git tag: git tag v' + newVersion);
  } else {
    log('yellow', '\n⚠️  No changes were made');
  }

  process.exit(0);
}

// Run
main();
