#!/usr/bin/env node

/**
 * Test checksum verification functionality
 * Tests both valid and invalid checksums to ensure security
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const os = require('os');

// Colors for console output
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

/**
 * Load expected checksums from checksums.txt
 */
function loadChecksums() {
  const checksumsPath = path.join(__dirname, '..', 'checksums.txt');

  if (!fs.existsSync(checksumsPath)) {
    throw new Error('checksums.txt not found - package may be corrupted');
  }

  const content = fs.readFileSync(checksumsPath, 'utf8');
  const checksums = new Map();

  for (const line of content.split('\n')) {
    // Skip comments and empty lines
    if (line.trim().startsWith('#') || line.trim() === '') {
      continue;
    }

    // Parse "hash  filename" format
    const match = line.match(/^([a-f0-9]{128})\s+(.+)$/);
    if (match) {
      const [, hash, filename] = match;
      checksums.set(filename, hash);
    }
  }

  return checksums;
}

/**
 * Calculate SHA-512 checksum of a file
 */
function calculateChecksum(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha512');
    const stream = fs.createReadStream(filePath);

    stream.on('data', (chunk) => {
      hash.update(chunk);
    });

    stream.on('end', () => {
      resolve(hash.digest('hex'));
    });

    stream.on('error', (err) => {
      reject(err);
    });
  });
}

/**
 * Verify downloaded file checksum matches expected value
 */
async function verifyChecksum(filePath, filename, checksums) {
  const expectedChecksum = checksums.get(filename);

  if (!expectedChecksum) {
    throw new Error(`No checksum found for ${filename} - package may be corrupted`);
  }

  const actualChecksum = await calculateChecksum(filePath);

  if (actualChecksum !== expectedChecksum) {
    throw new Error(
      `SECURITY: Checksum mismatch for ${filename}!\n` +
      `  Expected: ${expectedChecksum}\n` +
      `  Actual:   ${actualChecksum}\n` +
      `  This could indicate a corrupted download or supply chain attack.\n` +
      `  DO NOT extract this file. Please report this issue.`
    );
  }
}

/**
 * Test valid checksum verification
 */
async function testValidChecksum() {
  log('cyan', '\nTest 1: Valid Checksum Verification');
  log('cyan', '====================================');

  const tmpDir = path.join(os.tmpdir(), 'swictation-checksum-test');
  if (!fs.existsSync(tmpDir)) {
    fs.mkdirSync(tmpDir, { recursive: true });
  }

  // Create a test file with known content
  const testFile = path.join(tmpDir, 'test-valid.txt');
  const testContent = 'This is a test file for checksum verification.';
  fs.writeFileSync(testFile, testContent);

  // Calculate checksum
  const actualChecksum = await calculateChecksum(testFile);
  log('green', `  ✓ Calculated checksum: ${actualChecksum.substring(0, 32)}...`);

  // Create checksums map with the actual checksum
  const checksums = new Map();
  checksums.set('test-valid.txt', actualChecksum);

  // Verify - should succeed
  try {
    await verifyChecksum(testFile, 'test-valid.txt', checksums);
    log('green', '  ✓ Valid checksum verification PASSED');

    // Cleanup
    fs.unlinkSync(testFile);
    return true;
  } catch (err) {
    log('red', `  ✗ Valid checksum verification FAILED: ${err.message}`);
    fs.unlinkSync(testFile);
    return false;
  }
}

/**
 * Test invalid checksum detection
 */
async function testInvalidChecksum() {
  log('cyan', '\nTest 2: Invalid Checksum Detection');
  log('cyan', '===================================');

  const tmpDir = path.join(os.tmpdir(), 'swictation-checksum-test');
  if (!fs.existsSync(tmpDir)) {
    fs.mkdirSync(tmpDir, { recursive: true });
  }

  // Create a test file
  const testFile = path.join(tmpDir, 'test-invalid.txt');
  const testContent = 'This file has been tampered with!';
  fs.writeFileSync(testFile, testContent);

  // Create checksums map with a WRONG checksum
  const checksums = new Map();
  const wrongChecksum = '0'.repeat(128); // Obviously wrong checksum
  checksums.set('test-invalid.txt', wrongChecksum);

  // Verify - should FAIL
  try {
    await verifyChecksum(testFile, 'test-invalid.txt', checksums);
    log('red', '  ✗ Invalid checksum detection FAILED - should have thrown error!');
    fs.unlinkSync(testFile);
    return false;
  } catch (err) {
    if (err.message.includes('SECURITY: Checksum mismatch')) {
      log('green', '  ✓ Invalid checksum detected correctly');
      log('cyan', `  ✓ Error message: ${err.message.split('\n')[0]}`);
      fs.unlinkSync(testFile);
      return true;
    } else {
      log('red', `  ✗ Wrong error: ${err.message}`);
      fs.unlinkSync(testFile);
      return false;
    }
  }
}

/**
 * Test missing checksum handling
 */
async function testMissingChecksum() {
  log('cyan', '\nTest 3: Missing Checksum Handling');
  log('cyan', '==================================');

  const tmpDir = path.join(os.tmpdir(), 'swictation-checksum-test');
  if (!fs.existsSync(tmpDir)) {
    fs.mkdirSync(tmpDir, { recursive: true });
  }

  // Create a test file
  const testFile = path.join(tmpDir, 'test-missing.txt');
  fs.writeFileSync(testFile, 'Test content');

  // Create empty checksums map (no entry for our file)
  const checksums = new Map();

  // Verify - should FAIL
  try {
    await verifyChecksum(testFile, 'test-missing.txt', checksums);
    log('red', '  ✗ Missing checksum handling FAILED - should have thrown error!');
    fs.unlinkSync(testFile);
    return false;
  } catch (err) {
    if (err.message.includes('No checksum found')) {
      log('green', '  ✓ Missing checksum detected correctly');
      log('cyan', `  ✓ Error message: ${err.message}`);
      fs.unlinkSync(testFile);
      return true;
    } else {
      log('red', `  ✗ Wrong error: ${err.message}`);
      fs.unlinkSync(testFile);
      return false;
    }
  }
}

/**
 * Verify actual checksums.txt integrity
 */
async function testActualChecksums() {
  log('cyan', '\nTest 4: Verify checksums.txt Integrity');
  log('cyan', '=======================================');

  try {
    const checksums = loadChecksums();

    const expectedFiles = [
      'cuda-libs-latest.tar.gz',
      'cuda-libs-legacy.tar.gz',
      'cuda-libs-modern.tar.gz'
    ];

    log('cyan', `  Loaded ${checksums.size} checksums`);

    let allPresent = true;
    for (const file of expectedFiles) {
      if (checksums.has(file)) {
        const hash = checksums.get(file);
        log('green', `  ✓ ${file}: ${hash.substring(0, 32)}...`);
      } else {
        log('red', `  ✗ Missing checksum for: ${file}`);
        allPresent = false;
      }
    }

    if (allPresent) {
      log('green', '  ✓ All GPU library checksums present');
      return true;
    } else {
      log('red', '  ✗ Some checksums are missing');
      return false;
    }
  } catch (err) {
    log('red', `  ✗ Failed to load checksums: ${err.message}`);
    return false;
  }
}

/**
 * Main test runner
 */
async function main() {
  log('cyan', '\n╔════════════════════════════════════════════════╗');
  log('cyan', '║  Checksum Verification Test Suite             ║');
  log('cyan', '╚════════════════════════════════════════════════╝');

  const results = [];

  results.push(await testValidChecksum());
  results.push(await testInvalidChecksum());
  results.push(await testMissingChecksum());
  results.push(await testActualChecksums());

  // Cleanup temp directory
  const tmpDir = path.join(os.tmpdir(), 'swictation-checksum-test');
  if (fs.existsSync(tmpDir)) {
    fs.rmdirSync(tmpDir, { recursive: true });
  }

  // Summary
  log('cyan', '\n╔════════════════════════════════════════════════╗');
  log('cyan', '║  Test Summary                                  ║');
  log('cyan', '╚════════════════════════════════════════════════╝\n');

  const passed = results.filter(r => r).length;
  const total = results.length;

  if (passed === total) {
    log('green', `✓ All ${total} tests PASSED`);
    log('green', '\n✅ Checksum verification is working correctly!');
    log('cyan', '   GPU library downloads are now protected against:');
    log('cyan', '   • Corrupted downloads');
    log('cyan', '   • Man-in-the-middle attacks');
    log('cyan', '   • Compromised GitHub releases');
    log('cyan', '   • Supply chain attacks\n');
    process.exit(0);
  } else {
    log('red', `✗ ${total - passed}/${total} tests FAILED`);
    log('red', '\n❌ Checksum verification has issues!');
    process.exit(1);
  }
}

main().catch((err) => {
  log('red', `\nFatal error: ${err.message}`);
  console.error(err);
  process.exit(1);
});
