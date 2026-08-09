/**
 * Per-file integrity manifest for the extracted CUDA/ONNX-Runtime set
 * (ADR-037 Phase B; the amendment-7 debt that kept gpu-libs at `unknown`).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * gpu-package-info.json is a RECEIPT: it records what was asked for. It
 * lives in the config dir, survives every npm upgrade, and knows nothing
 * about the ~40 files that actually landed — which is why ADR-035's bug
 * (receipt intact, 1.5 GB of libraries deleted) was invisible to it, and why
 * the Phase-A check refused to say `healthy` on the strength of a receipt
 * plus one sentinel .so.
 *
 * This manifest is the opposite: it is generated FROM the bytes that were
 * written, at the moment they were written, by hashing each file as it
 * streams out of the extracted tarball into getGpuLibsDir(). Nothing in it
 * is copied from the tarball's own metadata, so it cannot inherit a lie from
 * an archive that unpacked partially — a truncated copy hashes to whatever
 * actually reached the disk, and the size recorded is the size on disk.
 *
 * It lives BESIDE the libraries, not in the config dir, deliberately: the
 * failure mode being defended against is exactly a manifest outliving the
 * files it describes. If an upgrade removes the directory, the manifest goes
 * with it and `check()` falls back to `unknown` instead of vouching for
 * files that are gone.
 *
 * Two verification depths, because hashing 1.5 GB is not something a check
 * that runs on every install may do:
 *   verifyInventory()  stat() per file — presence, regular-file, exact size.
 *                      What `check()` runs, and what promotes gpu-libs to
 *                      `healthy` at last.
 *   verifyHashes()     streamed sha256 per file. `doctor --deep` only.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const MANIFEST_NAME = 'gpu-libs.manifest.json';
const SCHEMA_VERSION = 1;

/** Where the manifest lives: beside the libraries it describes. */
function manifestPath(gpuLibsDir = require('./paths').getGpuLibsDir()) {
  return path.join(gpuLibsDir, MANIFEST_NAME);
}

/**
 * The parsed manifest, or null when absent/unreadable/not a recognizable
 * manifest. Null is the honest answer for a pre-Phase-B install: those
 * directories are real but undescribed, and the caller degrades to `unknown`.
 */
function readManifest(gpuLibsDir) {
  try {
    const parsed = JSON.parse(fs.readFileSync(manifestPath(gpuLibsDir), 'utf8'));
    if (!parsed || typeof parsed !== 'object' || !Array.isArray(parsed.files)) return null;
    if (parsed.files.length === 0) return null;
    return parsed;
  } catch {
    return null;
  }
}

/**
 * Copy one file and hash the bytes on the way through.
 *
 * A single pass: the same stream feeds the destination and the digest, so
 * the recorded hash is provably of what was written rather than of a later
 * re-read that a concurrent writer could have changed underneath.
 *
 * @returns {Promise<{path: string, size: number, sha256: string}>}
 */
function copyAndHash(srcPath, destPath, relName) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    let size = 0;

    const source = fs.createReadStream(srcPath);
    const sink = fs.createWriteStream(destPath);

    const fail = (err) => {
      source.destroy();
      sink.destroy();
      reject(err);
    };

    source.on('error', fail);
    sink.on('error', fail);
    source.on('data', (chunk) => {
      hash.update(chunk);
      size += chunk.length;
    });
    sink.on('finish', () => resolve({
      path: relName || path.basename(destPath),
      size,
      sha256: hash.digest('hex'),
    }));
    source.pipe(sink);
  });
}

/**
 * Write the manifest for a freshly extracted set.
 * Best-effort by contract: a manifest that cannot be written leaves the step
 * at `unknown`, which is exactly where it was before this file existed.
 *
 * @returns {boolean} whether it landed
 */
function writeManifest(gpuLibsDir, { variant, version, files, clock = () => new Date() }) {
  try {
    fs.writeFileSync(manifestPath(gpuLibsDir), JSON.stringify({
      schemaVersion: SCHEMA_VERSION,
      variant,
      version,
      generatedAt: clock().toISOString(),
      fileCount: files.length,
      files,
    }, null, 2));
    return true;
  } catch {
    return false;
  }
}

/**
 * Presence + size of every file the manifest declares.
 *
 * `missing` covers absent files AND anything that is not a regular file: a
 * directory named libcudnn.so satisfies existsSync and satisfies nothing
 * else. Extra files in the directory are NOT a failure — the platform
 * package legitimately ships its own CPU libonnxruntime.so alongside.
 *
 * @returns {{ok: boolean, checked: number, missing: string[], mismatched: string[]}}
 */
function verifyInventory(manifest, gpuLibsDir) {
  const missing = [];
  const mismatched = [];

  for (const file of manifest.files) {
    const target = path.join(gpuLibsDir, file.path);
    let stat;
    try {
      stat = fs.statSync(target);
    } catch {
      missing.push(file.path);
      continue;
    }
    if (!stat.isFile()) {
      missing.push(file.path);
      continue;
    }
    if (stat.size !== file.size) {
      mismatched.push(`${file.path}: expected ${file.size} bytes, found ${stat.size}`);
    }
  }

  return {
    ok: missing.length === 0 && mismatched.length === 0,
    checked: manifest.files.length,
    missing,
    mismatched,
  };
}

/** Streamed sha256 of one file, or null when it cannot be read. */
function hashFile(filePath) {
  return new Promise((resolve) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('error', () => resolve(null));
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

/**
 * Full content verification — `doctor --deep` only.
 * Streamed, never buffered: these files reach 500 MB individually.
 *
 * @returns {Promise<{ok: boolean, checked: number, failures: string[]}>}
 */
async function verifyHashes(manifest, gpuLibsDir) {
  const failures = [];
  for (const file of manifest.files) {
    const actual = await hashFile(path.join(gpuLibsDir, file.path));
    if (actual === null) {
      failures.push(`${file.path}: unreadable`);
    } else if (actual !== file.sha256) {
      failures.push(`${file.path}: sha256 mismatch`);
    }
  }
  return { ok: failures.length === 0, checked: manifest.files.length, failures };
}

module.exports = {
  MANIFEST_NAME,
  SCHEMA_VERSION,
  manifestPath,
  readManifest,
  writeManifest,
  copyAndHash,
  hashFile,
  verifyInventory,
  verifyHashes,
};
