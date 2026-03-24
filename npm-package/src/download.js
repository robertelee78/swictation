'use strict';

const fs = require('fs');
const path = require('path');
const https = require('https');
const http = require('http');
const crypto = require('crypto');
const os = require('os');
const { InstallError } = require('./install-error');

// ── Progress Reporter ────────────────────────────────────────────────

class ProgressReporter {
  constructor() {
    this.isTTY = process.stdout.isTTY;
    this.startTime = Date.now();
    this.lastUpdate = 0;
    this.lastBytes = 0;
    this.lastPrintedPercent = 0; // for non-TTY 10% intervals
  }

  /**
   * Render progress. Called at most once per second by the download loop.
   * @param {number} downloaded - Bytes downloaded so far
   * @param {number|null} total - Total bytes (null if unknown)
   * @param {number} retryNum - Current retry (0 = first attempt)
   * @param {number} resumedFrom - Bytes resumed from
   */
  update(downloaded, total, retryNum, resumedFrom) {
    const now = Date.now();
    if (now - this.lastUpdate < 1000 && downloaded !== total) return;
    this.lastUpdate = now;

    const elapsed = (now - this.startTime) / 1000 || 1;
    const speed = downloaded / elapsed;
    const speedStr = this._formatBytes(speed) + '/s';

    if (total && total > 0) {
      const pct = Math.min(100, Math.round((downloaded / total) * 100));
      const eta = speed > 0 ? Math.round((total - downloaded) / speed) : 0;
      const etaStr = this._formatEta(eta);

      if (this.isTTY) {
        const barWidth = 20;
        const filled = Math.round((pct / 100) * barWidth);
        const bar = '='.repeat(filled) + (filled < barWidth ? '>' : '') + ' '.repeat(Math.max(0, barWidth - filled - 1));
        const line = `  [${bar}] ${this._formatBytes(downloaded)} / ${this._formatBytes(total)}  ${pct}%  ${speedStr}  ETA ${etaStr}`;
        process.stdout.write(`\r${line}`);
        if (downloaded >= total) process.stdout.write('\n');
      } else {
        // Non-TTY: print at 10% intervals
        const bucket = Math.floor(pct / 10) * 10;
        if (bucket > this.lastPrintedPercent || downloaded >= total) {
          this.lastPrintedPercent = bucket;
          console.log(`  Progress: ${pct}% (${this._formatBytes(downloaded)} / ${this._formatBytes(total)})`);
        }
      }
    } else {
      // Unknown total
      if (this.isTTY) {
        process.stdout.write(`\r  ${this._formatBytes(downloaded)} downloaded  ${speedStr}`);
      } else {
        const mb = Math.floor(downloaded / (100 * 1024 * 1024));
        if (mb > this.lastPrintedPercent) {
          this.lastPrintedPercent = mb;
          console.log(`  ${this._formatBytes(downloaded)} downloaded  ${speedStr}`);
        }
      }
    }
  }

  /** Clear the line (TTY only) */
  clear() {
    if (this.isTTY) process.stdout.write('\r' + ' '.repeat(80) + '\r');
  }

  _formatBytes(bytes) {
    if (bytes >= 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(1) + 'GB';
    if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + 'MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(0) + 'KB';
    return bytes + 'B';
  }

  _formatEta(secs) {
    if (secs <= 0) return '0s';
    if (secs < 60) return secs + 's';
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m + 'm ' + s + 's';
  }
}

// ── Disk Space Check ─────────────────────────────────────────────────

/**
 * Check available disk space at the given path.
 * @param {string} dirPath - Directory to check
 * @returns {number|null} - Available bytes, or null if can't determine
 */
function getAvailableDiskSpace(dirPath) {
  // Node 18.15+ has fs.statfsSync
  if (typeof fs.statfsSync === 'function') {
    try {
      const stats = fs.statfsSync(dirPath);
      return stats.bavail * stats.bsize;
    } catch {
      // fall through to df
    }
  }

  // Fallback: parse df output
  try {
    const { execSync } = require('child_process');
    const output = execSync(`df -k "${dirPath}" 2>/dev/null | tail -1`, { encoding: 'utf8' });
    const cols = output.trim().split(/\s+/);
    // df -k: columns are Filesystem 1K-blocks Used Available ...
    const availKB = parseInt(cols[3]);
    if (!isNaN(availKB)) return availKB * 1024;
  } catch {
    // ignore
  }

  return null;
}

/**
 * Validate that sufficient disk space exists before downloading.
 * @param {string} destDir - Destination directory
 * @param {number} contentLength - Expected download size in bytes
 * @throws {InstallError} if insufficient space
 */
function validateDiskSpace(destDir, contentLength) {
  if (!contentLength || contentLength <= 0) return;

  const available = getAvailableDiskSpace(destDir);
  if (available === null) return; // can't determine, proceed optimistically

  const needed = Math.ceil(contentLength * 1.1); // 10% buffer
  if (available < needed) {
    const fmt = (b) => (b / (1024 * 1024 * 1024)).toFixed(1) + 'GB';
    throw new InstallError('SW-E002', 'Insufficient disk space for download', {
      cause: `Need ${fmt(needed)} but only ${fmt(available)} available at ${destDir}`,
      fix: `Free up disk space at ${destDir} and re-run: npm install -g swictation`,
      context: { path: destDir, needed: fmt(needed), available: fmt(available) },
    });
  }
}

// ── SHA-256 Checksum (streaming) ─────────────────────────────────────

/**
 * Calculate SHA-256 checksum of a file using streaming.
 * @param {string} filePath
 * @returns {Promise<string>} hex digest
 */
function checksumFile(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
    stream.on('error', reject);
  });
}

// ── Core Download ────────────────────────────────────────────────────

/**
 * Perform a single HTTP(S) GET with redirect following, Range resume, and progress.
 * Returns a promise that resolves with metadata or rejects on error.
 */
function _singleDownload(url, dest, { resumeFrom = 0, onProgress, timeout = 30000, maxRedirects = 5 }) {
  return new Promise((resolve, reject) => {
    const partialPath = dest + '.partial';
    let redirectCount = 0;
    let totalBytes = null;
    let downloadedBytes = resumeFrom;
    let lastProgressTime = 0;

    function doRequest(requestUrl) {
      if (redirectCount > maxRedirects) {
        return reject(new Error(`Too many redirects (>${maxRedirects})`));
      }

      const proto = requestUrl.startsWith('https') ? https : http;
      const headers = {};
      if (resumeFrom > 0) {
        headers['Range'] = `bytes=${resumeFrom}-`;
      }

      const req = proto.get(requestUrl, { headers, timeout }, (response) => {
        const { statusCode } = response;

        // Handle redirects
        if (statusCode === 301 || statusCode === 302 || statusCode === 303 || statusCode === 307 || statusCode === 308) {
          redirectCount++;
          const location = response.headers.location;
          if (!location) return reject(new Error('Redirect with no Location header'));
          // Resolve relative redirects
          const redirectUrl = location.startsWith('http') ? location : new URL(location, requestUrl).href;
          response.resume(); // drain
          return doRequest(redirectUrl);
        }

        // Handle 416 Range Not Satisfiable (file already complete or server doesn't support range)
        if (statusCode === 416) {
          response.resume();
          return resolve({ bytesDownloaded: resumeFrom, resumed: resumeFrom > 0, retries: 0, skipped: false });
        }

        if (statusCode === 206) {
          // Partial content — resume is working
          const contentRange = response.headers['content-range'];
          if (contentRange) {
            const match = contentRange.match(/bytes \d+-\d+\/(\d+)/);
            if (match) totalBytes = parseInt(match[1]);
          }
        } else if (statusCode >= 200 && statusCode < 300) {
          // Full content
          const cl = response.headers['content-length'];
          if (cl) totalBytes = parseInt(cl) + resumeFrom;
          // If we were trying to resume but got 200, server doesn't support Range — restart
          if (resumeFrom > 0) {
            downloadedBytes = 0;
          }
        } else {
          response.resume();
          return reject(new Error(`HTTP ${statusCode} from ${requestUrl}`));
        }

        // Open write stream (append if resuming with 206, truncate otherwise)
        const writeMode = (statusCode === 206) ? 'a' : 'w';
        const file = fs.createWriteStream(partialPath, { flags: writeMode });

        response.on('data', (chunk) => {
          downloadedBytes += chunk.length;
          if (onProgress) {
            const now = Date.now();
            if (now - lastProgressTime >= 1000 || downloadedBytes === totalBytes) {
              lastProgressTime = now;
              onProgress(downloadedBytes, totalBytes);
            }
          }
        });

        response.pipe(file);

        file.on('finish', () => {
          file.close(() => {
            // Rename partial to final
            try {
              fs.renameSync(partialPath, dest);
            } catch (err) {
              return reject(err);
            }
            resolve({
              bytesDownloaded: downloadedBytes,
              resumed: resumeFrom > 0 && statusCode === 206,
              retries: 0,
              skipped: false,
            });
          });
        });

        file.on('error', (err) => {
          // Don't delete partial — we'll resume from it
          reject(err);
        });

        response.on('error', (err) => {
          file.close();
          reject(err);
        });
      });

      req.on('timeout', () => {
        req.destroy();
        reject(Object.assign(new Error('Socket idle timeout'), { code: 'ETIMEDOUT' }));
      });

      req.on('error', (err) => {
        reject(err);
      });
    }

    doRequest(url);
  });
}

// ── Public API ───────────────────────────────────────────────────────

/**
 * Download a file with retry, resume, progress, disk space check, and checksum skip.
 *
 * @param {string} url - URL to download
 * @param {string} dest - Destination file path
 * @param {object} [options]
 * @param {number}   [options.maxRetries=3]
 * @param {number[]} [options.backoff=[2000,8000,32000]]
 * @param {number}   [options.timeout=30000] - Socket idle timeout in ms
 * @param {string}   [options.expectedChecksum] - SHA-256 hex; skip download if dest matches
 * @param {boolean}  [options.checkDiskSpace=true]
 * @param {Function} [options.onProgress] - (downloaded, total) => void
 * @returns {Promise<{bytesDownloaded:number, resumed:boolean, retries:number, skipped:boolean}>}
 */
async function downloadWithRetry(url, dest, options = {}) {
  const {
    maxRetries = 3,
    backoff = [2000, 8000, 32000],
    timeout = 30000,
    expectedChecksum = null,
    checkDiskSpace = true,
    onProgress = null,
  } = options;

  // ── 1. Skip if dest exists and checksum matches ──
  if (expectedChecksum && fs.existsSync(dest)) {
    try {
      const existing = await checksumFile(dest);
      if (existing === expectedChecksum.toLowerCase()) {
        return { bytesDownloaded: 0, resumed: false, retries: 0, skipped: true };
      }
    } catch {
      // File corrupt or unreadable — re-download
    }
  }

  // ── 2. Disk space pre-check via HEAD request ──
  if (checkDiskSpace) {
    try {
      const contentLength = await _headContentLength(url);
      if (contentLength > 0) {
        const destDir = path.dirname(dest);
        if (!fs.existsSync(destDir)) fs.mkdirSync(destDir, { recursive: true });
        validateDiskSpace(destDir, contentLength);
      }
    } catch (err) {
      if (err instanceof InstallError) throw err;
      // HEAD request failed — proceed anyway, will fail on write if no space
    }
  }

  // ── 3. Ensure dest directory exists ──
  const destDir = path.dirname(dest);
  if (!fs.existsSync(destDir)) fs.mkdirSync(destDir, { recursive: true });

  // ── 4. Set up progress reporter ──
  const progress = onProgress || new ProgressReporter();
  const progressCb = typeof progress === 'function' ? progress : progress.update.bind(progress);

  // ── 5. Retry loop ──
  let lastError = null;
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    if (attempt > 0) {
      const delay = backoff[Math.min(attempt - 1, backoff.length - 1)];
      const partialPath = dest + '.partial';
      const resumeBytes = fs.existsSync(partialPath) ? fs.statSync(partialPath).size : 0;
      const resumeStr = resumeBytes > 0 ? ` — resuming from ${_fmtBytes(resumeBytes)}` : '';
      console.log(`\x1b[33m  Retry ${attempt}/${maxRetries}${resumeStr}...\x1b[0m`);
      await _sleep(delay);
    }

    try {
      // Check for existing partial file to resume from
      const partialPath = dest + '.partial';
      const resumeFrom = fs.existsSync(partialPath) ? fs.statSync(partialPath).size : 0;

      const result = await _singleDownload(url, dest, {
        resumeFrom,
        onProgress: progressCb,
        timeout,
      });

      // Clear progress line if using built-in reporter
      if (typeof progress !== 'function' && progress.clear) progress.clear();

      result.retries = attempt;
      return result;
    } catch (err) {
      lastError = err;
      // If it's a disk space error, don't retry
      if (err instanceof InstallError && err.code === 'SW-E002') throw err;
      // If it's an HTTP error (4xx), don't retry
      if (err.message && err.message.match(/^HTTP [45]\d\d/)) {
        // 5xx are retriable, 4xx are not
        if (err.message.match(/^HTTP 4\d\d/)) throw _wrapDownloadError(err, url, dest);
      }
    }
  }

  // All retries exhausted
  const partialPath = dest + '.partial';
  const partialSize = fs.existsSync(partialPath) ? fs.statSync(partialPath).size : 0;

  throw new InstallError('SW-E003', 'Download failed after all retries', {
    cause: lastError ? (lastError.code ? `${lastError.code}: ${lastError.message}` : lastError.message) : 'Unknown error',
    fix: partialSize > 0
      ? `Re-run installation (download will resume from ${_fmtBytes(partialSize)}):\n         npm install -g swictation\n\n  Manual alternative:\n    curl -L -C - -o "${dest}" "${url}"\n    Then re-run: npm install -g swictation`
      : `Re-run installation: npm install -g swictation\n\n  Manual alternative:\n    curl -L -o "${dest}" "${url}"\n    Then re-run: npm install -g swictation`,
    original: lastError,
    context: { url, dest, partialSize },
  });
}

// ── Helpers ──────────────────────────────────────────────────────────

/** HEAD request to get Content-Length (follows redirects) */
function _headContentLength(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 5) return resolve(0);
    const proto = url.startsWith('https') ? https : http;

    const req = proto.request(url, { method: 'HEAD', timeout: 10000 }, (res) => {
      if ([301, 302, 303, 307, 308].includes(res.statusCode) && res.headers.location) {
        const loc = res.headers.location.startsWith('http') ? res.headers.location : new URL(res.headers.location, url).href;
        res.resume();
        return _headContentLength(loc, redirects + 1).then(resolve, reject);
      }
      res.resume();
      const cl = parseInt(res.headers['content-length'] || '0');
      resolve(isNaN(cl) ? 0 : cl);
    });
    req.on('timeout', () => { req.destroy(); resolve(0); });
    req.on('error', () => resolve(0));
    req.end();
  });
}

function _wrapDownloadError(err, url, dest) {
  return InstallError.fromSystemError(err, 'SW-E003', 'Download failed', { url, dest });
}

function _sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function _fmtBytes(bytes) {
  if (bytes >= 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(1) + 'GB';
  if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(0) + 'MB';
  if (bytes >= 1024) return (bytes / 1024).toFixed(0) + 'KB';
  return bytes + 'B';
}

module.exports = {
  downloadWithRetry,
  ProgressReporter,
  validateDiskSpace,
  getAvailableDiskSpace,
  checksumFile,
};
