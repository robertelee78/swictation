#!/usr/bin/env node

/**
 * Generate SHA-512 checksums for GPU library releases
 * Usage: node scripts/generate-gpu-checksums.js
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const os = require('os');

const GPU_LIBS_VERSION = '1.2.0';
const VARIANTS = ['latest', 'legacy', 'modern'];
const BASE_URL = 'https://github.com/robertelee78/swictation/releases/download';

/**
 * Download a file from URL
 */
function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (redirectResponse) => {
          if (redirectResponse.statusCode !== 200) {
            reject(new Error(`Failed to download: HTTP ${redirectResponse.statusCode}`));
            return;
          }

          redirectResponse.pipe(file);
          file.on('finish', () => {
            file.close();
            resolve();
          });
        }).on('error', (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
      } else if (response.statusCode === 200) {
        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      } else {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
      }
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
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
 * Format bytes for display
 */
function formatBytes(bytes) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

/**
 * Main function
 */
async function main() {
  console.log('GPU Library Checksum Generator');
  console.log('================================\n');

  const tmpDir = path.join(os.tmpdir(), 'swictation-checksum-gen');

  // Create temp directory
  if (!fs.existsSync(tmpDir)) {
    fs.mkdirSync(tmpDir, { recursive: true });
  }

  const checksums = [];

  for (const variant of VARIANTS) {
    const filename = `cuda-libs-${variant}.tar.gz`;
    const url = `${BASE_URL}/gpu-libs-v${GPU_LIBS_VERSION}/${filename}`;
    const destPath = path.join(tmpDir, filename);

    console.log(`Processing ${variant} variant...`);
    console.log(`  URL: ${url}`);

    // Download file
    console.log(`  Downloading...`);
    try {
      await downloadFile(url, destPath);
      const stats = fs.statSync(destPath);
      console.log(`  ✓ Downloaded ${formatBytes(stats.size)}`);
    } catch (err) {
      console.error(`  ✗ Download failed: ${err.message}`);
      process.exit(1);
    }

    // Calculate checksum
    console.log(`  Calculating SHA-512 checksum...`);
    try {
      const checksum = await calculateChecksum(destPath);
      console.log(`  ✓ Checksum: ${checksum.substring(0, 16)}...`);

      checksums.push({
        variant,
        filename,
        checksum
      });
    } catch (err) {
      console.error(`  ✗ Checksum calculation failed: ${err.message}`);
      process.exit(1);
    }

    // Clean up downloaded file
    fs.unlinkSync(destPath);
    console.log(`  ✓ Cleaned up temporary file\n`);
  }

  // Generate checksums.txt
  const checksumsPath = path.join(__dirname, '..', 'checksums.txt');
  let checksumsContent = '# SHA-512 checksums for swictation GPU libraries\n';
  checksumsContent += `# Generated: ${new Date().toISOString()}\n`;
  checksumsContent += `# Version: gpu-libs-v${GPU_LIBS_VERSION}\n\n`;

  for (const { filename, checksum } of checksums) {
    checksumsContent += `${checksum}  ${filename}\n`;
  }

  fs.writeFileSync(checksumsPath, checksumsContent);
  console.log('✓ Generated checksums.txt');
  console.log(`  Location: ${checksumsPath}\n`);

  // Display checksums
  console.log('Checksums:');
  console.log('==========');
  for (const { variant, checksum } of checksums) {
    console.log(`${variant.padEnd(10)} ${checksum}`);
  }

  // Clean up temp directory
  fs.rmdirSync(tmpDir);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
