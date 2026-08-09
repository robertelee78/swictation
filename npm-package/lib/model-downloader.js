#!/usr/bin/env node

/**
 * Model Downloader for Swictation
 * Downloads AI models from HuggingFace using huggingface-cli
 */

const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { downloadWithRetry, checksumFile } = require('../src/download');
const { InstallError } = require('../src/install-error');

// Every .mlmodelc bundle has exactly this internal structure
const MLMODELC_INTERNAL_FILES = [
  'weights/weight.bin',
  'metadata.json',
  'model.mil',
  'coremldata.bin',
  'analytics/coremldata.bin'
];

// Integrity manifest: per-model pinned revision + per-file sha256/size.
// Regenerate with scripts/generate-model-manifest.js (see ADR-036).
const DEFAULT_MANIFEST_PATH = path.join(__dirname, '..', 'models.manifest.json');

// Model definitions
const MODELS = {
  vad: {
    name: 'Silero VAD v6',
    size: '629 KB',
    // Use k2-fsa/sherpa-onnx pre-converted model with correct tensor format
    // Tensor names: x, h, c → prob, new_h, new_c
    directUrl: 'https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx',
    targetDir: 'silero-vad',
    files: ['silero_vad.onnx']
  },
  '0.6b': {
    name: 'Parakeet-TDT 0.6B v3',
    size: '2.55 GB',
    // Latest v3: multilingual (25 EU languages), 6.34% WER avg, 1.93% LibriSpeech clean
    // FP32 with external weights (encoder.onnx + encoder.weights)
    repo: 'csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3',
    targetDir: 'parakeet-tdt-0.6b-v3-onnx',
    files: ['encoder.onnx', 'encoder.weights', 'decoder.onnx', 'joiner.onnx', 'tokens.txt']
  },
  '1.1b': {
    name: 'Parakeet-TDT 1.1B',
    size: '6.96 GB',
    // v1 (only ONNX version available); includes both INT8 and FP32 variants
    // Linux/NVIDIA: uses INT8 (encoder.int8.onnx) — fast on CPU, adequate on CUDA
    // macOS/CoreML: uses FP32 (encoder.onnx) — convert to FP16 with scripts/convert-to-fp16.py
    repo: 'jenerallee78/parakeet-tdt-1.1b-onnx',
    targetDir: 'parakeet-tdt-1.1b-onnx',
    files: [
      'encoder.onnx', 'encoder.weights',           // FP32 (for macOS FP16 conversion)
      'encoder.int8.onnx', 'encoder.int8.weights',  // INT8 (for Linux)
      'decoder.onnx', 'decoder.int8.onnx',
      'joiner.onnx', 'joiner.int8.onnx',
      'tokens.txt'
    ]
  },
  '0.6b-coreml': {
    name: 'Parakeet-TDT 0.6B v3 CoreML (Native)',
    size: '2.67 GB',
    // Native CoreML models for macOS — full ANE acceleration
    // These are pre-compiled .mlmodelc bundles from FluidInference
    repo: 'FluidInference/parakeet-tdt-0.6b-v3-coreml',
    targetDir: 'parakeet-tdt-0.6b-coreml',
    // Download all mlmodelc directories and vocab files
    files: [
      'Encoder.mlmodelc',
      'Decoder.mlmodelc',
      'RNNTJoint.mlmodelc',
      'MelEncoder.mlmodelc',
      'Preprocessor.mlmodelc',
      'parakeet_v3_vocab.json',
      'parakeet_vocab.json',
      'config.json'
    ]
  },
  '1.1b-coreml': {
    name: 'Parakeet-TDT 1.1B CoreML (Native)',
    size: '1.9 GB',
    // Native CoreML models for macOS Apple Silicon — full ANE acceleration
    // Pre-compiled .mlmodelc bundles for encoder/decoder/joiner architecture
    repo: 'jenerallee78/parakeet-tdt-1.1b-coreml',
    targetDir: 'parakeet-tdt-1.1b-coreml',
    files: [
      'encoder.mlmodelc',
      'decoder.mlmodelc',
      'joiner.mlmodelc',
      'tokens.txt'
    ]
  }
};

// macOS model strategy:
// - Preferred: Native CoreML models (parakeet-tdt-0.6b-coreml) — full ANE acceleration
// - Fallback: FP32 ONNX models with ORT CoreML EP — partial acceleration (~32% of nodes)
// The Rust daemon auto-detects CoreML models and uses them when available.
//
// FP16 model variants for macOS Apple Silicon optimization.
// After downloading FP32 models, run: python3 scripts/convert-to-fp16.py
// This generates *.fp16.onnx files alongside the originals.
// The Rust model loader (recognizer_ort.rs) automatically prefers .fp16.onnx
// files on macOS (darwin), falling back to FP32 if FP16 variants are absent.

/**
 * Expand a model's declared file list into the concrete files on the wire:
 * a .mlmodelc bundle is a DIRECTORY, so it becomes its five internal leaves.
 * @returns {string[]} repo-relative paths
 */
function expandModelFiles(model) {
  const out = [];
  for (const file of model.files) {
    if (file.endsWith('.mlmodelc')) {
      for (const internal of MLMODELC_INTERNAL_FILES) out.push(`${file}/${internal}`);
    } else {
      out.push(file);
    }
  }
  return out;
}

/**
 * Read the integrity manifest. Returns null when it is absent or unreadable —
 * callers degrade to unverified downloads rather than refusing to install.
 * @param {string} [manifestPath]
 * @returns {object|null}
 */
function loadManifest(manifestPath = DEFAULT_MANIFEST_PATH) {
  try {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    if (!manifest || typeof manifest.models !== 'object' || manifest.models === null) return null;
    return manifest;
  } catch {
    return null;
  }
}

// A file that failed verification is renamed out of the way with this suffix.
const QUARANTINE_SUFFIX_RE = /\.corrupt\.\d+(?:\.\d+)?$/;

/**
 * Move a file that failed verification out of the way.
 *
 * The bytes are kept, not deleted: a corrupt 4 GB download is still the user's
 * disk state, and having it on hand is what makes a support report possible.
 * What matters is that it stops occupying the path the daemon loads from — and
 * the path every existence/size check looks at, which is how a corrupt file
 * used to survive into the next install as "already downloaded".
 *
 * @param {string} filePath
 * @returns {string|null} the quarantine path, or null if there was nothing to move
 */
function quarantineFile(filePath) {
  const stamp = Date.now();
  let target = `${filePath}.corrupt.${stamp}`;
  for (let n = 1; fs.existsSync(target); n++) target = `${filePath}.corrupt.${stamp}.${n}`;
  try {
    fs.renameSync(filePath, target);
    return target;
  } catch {
    return null;
  }
}

/**
 * Verify one downloaded file against its manifest entry.
 *
 * Size is checked first because it is a stat() and rules out the common
 * failures (truncated transfer, HTML error page written as a model) without
 * reading gigabytes. The hash is streamed — these files reach 4 GB and must
 * never be buffered.
 *
 * @param {string} filePath
 * @param {{sha256: string, size: number}} expected
 * @returns {Promise<{ok: boolean, reason?: string}>}
 */
async function verifyFile(filePath, expected) {
  let stat;
  try {
    stat = fs.statSync(filePath);
  } catch (err) {
    return { ok: false, reason: `missing (${err.code || err.message})` };
  }
  if (!stat.isFile()) return { ok: false, reason: 'not a regular file' };
  if (stat.size !== expected.size) {
    return { ok: false, reason: `size mismatch: expected ${expected.size} bytes, got ${stat.size}` };
  }

  let actual;
  try {
    actual = await checksumFile(filePath);
  } catch (err) {
    return { ok: false, reason: `unreadable: ${err.message}` };
  }
  if (actual !== expected.sha256) {
    return { ok: false, reason: `sha256 mismatch: expected ${expected.sha256}, got ${actual}` };
  }
  return { ok: true };
}

class ModelDownloader {
  constructor(options = {}) {
    // Platform-aware default (ADR-034): the previous hardcoded XDG path sent
    // macOS downloads to a directory the daemon never reads.
    this.modelDir = options.modelDir || require('../src/paths').getModelsDir();
    this.force = options.force || false;
    this.verbose = options.verbose || false;
    this.manifestPath = options.manifestPath || DEFAULT_MANIFEST_PATH;
    this._manifest = undefined;   // undefined = not loaded yet, null = absent
    this._manifestWarned = false;
    this._partialWarned = new Set();
  }

  /** Seam over child_process.spawn so the spawn-driven tiers stay testable. */
  _spawn(command, args, options) {
    return spawn(command, args, options);
  }

  /** Seam over downloadWithRetry, for the same reason. */
  _download(url, dest, options) {
    return downloadWithRetry(url, dest, options);
  }

  /**
   * The parsed manifest, or null. Warns ONCE per downloader when it is absent:
   * that is a shipping defect worth shouting about, but repeating it per file
   * across ~50 files would bury everything else.
   */
  manifest() {
    if (this._manifest === undefined) {
      this._manifest = loadManifest(this.manifestPath);
      if (this._manifest === null && !this._manifestWarned) {
        this._manifestWarned = true;
        this.log('\n⚠️  WARNING: models.manifest.json is missing or unreadable.');
        this.log('   Model downloads will NOT be integrity-verified and will use the');
        this.log('   mutable "main" branch. Reinstall the package to restore verification.\n');
      }
    }
    return this._manifest;
  }

  /** The manifest entry for a model key, or null. */
  manifestEntry(modelKey) {
    const manifest = this.manifest();
    return (manifest && manifest.models[modelKey]) || null;
  }

  /** The expected {sha256, size} for one repo-relative file, or null. */
  _expectedFile(modelKey, filePath) {
    const entry = this.manifestEntry(modelKey);
    if (!entry) return null;
    return entry.files.find(f => f.path === filePath) || null;
  }

  /**
   * Download URL for one repo-relative file, pinned to the manifest revision.
   * Falls back to `main` only when the manifest cannot supply a revision.
   */
  fileUrl(modelKey, filePath) {
    const model = MODELS[modelKey];
    const entry = this.manifestEntry(modelKey);
    const source = (entry && entry.source) || {};
    const repo = source.repo || (model && model.repo);
    if (!repo) throw new Error(`No repository known for model: ${modelKey}`);
    return `https://huggingface.co/${repo}/resolve/${source.revision || 'main'}/${encodeURI(filePath)}`;
  }

  /**
   * Verify a downloaded model tree against the manifest.
   *
   * Files the manifest does not know about are reported as `unverified` and
   * warned about, never failed: a manifest that lags a MODELS edit must not
   * block an install.
   *
   * @returns {Promise<{ok: boolean, verified: string[], failures: {path: string, reason: string}[], unverified: string[]}>}
   */
  async verifyModelFiles(modelKey, targetPath, filePaths) {
    const report = { ok: true, verified: [], failures: [], unverified: [] };
    const entry = this.manifestEntry(modelKey);
    if (!entry) {
      report.unverified = [...filePaths];
      return report;
    }

    const expectedByPath = new Map(entry.files.map(f => [f.path, f]));
    for (const rel of filePaths) {
      const expected = expectedByPath.get(rel);
      if (!expected) {
        report.unverified.push(rel);
        this.log(`   ⚠️  ${rel} is not in models.manifest.json — kept WITHOUT verification`);
        continue;
      }
      const result = await verifyFile(path.join(targetPath, rel), expected);
      if (result.ok) {
        report.verified.push(rel);
      } else {
        report.ok = false;
        report.failures.push({ path: rel, reason: result.reason });
      }
    }
    return report;
  }

  /** Build the InstallError raised when a downloaded file fails verification. */
  _integrityError(modelKey, rel, reason, targetPath) {
    return new InstallError('SW-E004', `Integrity check failed for ${modelKey}`, {
      cause: `${rel}: ${reason}`,
      fix: 'The downloaded file does not match the signed manifest — it was corrupted in\n' +
           '  transit or served by something other than the pinned upstream revision.\n' +
           `  Remove the partial download and retry:\n    rm -rf "${targetPath}"\n` +
           `    swictation download-model ${modelKey}`,
      context: { model: modelKey, file: rel, reason },
    });
  }

  /**
   * Ensure model directory exists
   */
  ensureModelDir() {
    if (!fs.existsSync(this.modelDir)) {
      fs.mkdirSync(this.modelDir, { recursive: true });
      this.log(`Created model directory: ${this.modelDir}`);
    }
  }

  /**
   * Check if hf CLI is installed
   */
  checkHuggingFaceCli() {
    try {
      // Try common locations for hf CLI
      const paths = [
        'hf',
        `${os.homedir()}/.local/bin/hf`,
        '/usr/local/bin/hf',
        '/usr/bin/hf'
      ];

      for (const hfPath of paths) {
        try {
          execSync(`${hfPath} version`, { stdio: 'ignore' });
          // Store the working path for later use
          this.hfPath = hfPath;
          return true;
        } catch {
          // Try next path
        }
      }
      return false;
    } catch {
      return false;
    }
  }

  /**
   * Check if a model is already downloaded.
   *
   * With a manifest this compares every file's SIZE — cheap enough to run on
   * every install, and it catches the half-written tree that existence-only
   * checks happily reported as installed. Content verification is the
   * download path's job (hashing 7 GB on every startup is not a check anyone
   * would keep). Without a manifest — or with one that does not describe the
   * whole model — this degrades to the old existence test.
   */
  isModelDownloaded(modelKey) {
    const model = MODELS[modelKey];
    const entry = this.manifestEntry(modelKey);

    const targetDir = (entry && entry.targetDir) || (model && model.targetDir);
    if (!targetDir) return false;

    const modelPath = path.join(this.modelDir, targetDir);
    if (!fs.existsSync(modelPath)) return false;

    if (entry && this._manifestCoversModel(modelKey, entry)) {
      return entry.files.every(file => {
        // A quarantined name is by definition a file that failed verification;
        // it can never be what makes a model count as installed.
        if (QUARANTINE_SUFFIX_RE.test(file.path)) return false;
        try {
          const stat = fs.statSync(path.join(modelPath, file.path));
          return stat.isFile() && stat.size === file.size;
        } catch {
          return false;
        }
      });
    }

    if (entry) this._warnPartialEntry(modelKey);

    if (!model) return false;
    return model.files.every(file => fs.existsSync(path.join(modelPath, file)));
  }

  /**
   * Whether the manifest entry describes EVERY file MODELS declares.
   *
   * A `--model=` regeneration that never merged, or a hand-edit, produces an
   * entry covering a subset — and `[].every()` is vacuously true, so an empty
   * one covers nothing at all. Either would let a fraction of a model vouch for
   * the whole of it. A key MODELS does not know (a fixture, a manifest that
   * outlived a removed model) has nothing to cross-check against, so the
   * manifest stays authoritative there.
   */
  _manifestCoversModel(modelKey, entry) {
    const model = MODELS[modelKey];
    if (!model) return true;
    const covered = new Set(entry.files.map(f => f.path));
    return expandModelFiles(model).every(f => covered.has(f));
  }

  /** Warn once per model that its manifest entry is too incomplete to trust. */
  _warnPartialEntry(modelKey) {
    if (this._partialWarned.has(modelKey)) return;
    this._partialWarned.add(modelKey);
    this.log(`\n⚠️  WARNING: models.manifest.json does not cover every file declared`);
    this.log(`   for "${modelKey}". Treating it as unverifiable and falling back to an`);
    this.log('   existence check. Regenerate with scripts/generate-model-manifest.js.\n');
  }

  /**
   * Download a specific model using hf CLI or direct URL
   */
  async downloadModel(modelKey) {
    const model = MODELS[modelKey];
    if (!model) {
      throw new Error(`Unknown model: ${modelKey}`);
    }

    // Check if already downloaded
    if (!this.force && this.isModelDownloaded(modelKey)) {
      this.log(`✓ ${model.name} already downloaded (use --force to re-download)`);
      return;
    }

    const targetPath = path.join(this.modelDir, model.targetDir);

    // Handle direct URL downloads (e.g., k2-fsa VAD)
    if (model.directUrl) {
      this.log(`\n📦 Downloading ${model.name} (${model.size})...`);
      this.log(`   URL: ${model.directUrl}`);
      this.log(`   Destination: ${targetPath}\n`);

      // Create target directory
      if (!fs.existsSync(targetPath)) {
        fs.mkdirSync(targetPath, { recursive: true });
      }

      const fileName = model.files[0];
      const filePath = path.join(targetPath, fileName);
      // curl writes to a staging name so an unverified (or half-written) file
      // can never occupy the path the daemon loads from.
      const stagingPath = `${filePath}.partial`;

      await new Promise((resolve, reject) => {
        const proc = this._spawn('curl', [
          '-L',  // Follow redirects
          '--fail',  // a 404 must be an error, not an HTML file named silero_vad.onnx
          model.directUrl,
          '-o', stagingPath,
          '--progress-bar'
        ], {
          stdio: this.verbose ? 'inherit' : 'pipe'
        });

        let stderr = '';

        if (!this.verbose) {
          proc.stdout.on('data', (data) => {
            process.stdout.write(data);
          });

          proc.stderr.on('data', (data) => {
            // curl progress goes to stderr
            process.stderr.write(data);
            stderr += data.toString();
          });
        }

        proc.on('close', (code) => {
          if (code !== 0) {
            fs.rmSync(stagingPath, { force: true });
            reject(new Error(`Download failed with code ${code}\n${stderr}`));
            return;
          }
          resolve();
        });

        proc.on('error', (err) => {
          fs.rmSync(stagingPath, { force: true });
          reject(new Error(`Failed to spawn curl: ${err.message}`));
        });
      });

      const expected = this._expectedFile(modelKey, fileName);
      if (expected) {
        const result = await verifyFile(stagingPath, expected);
        if (!result.ok) {
          fs.rmSync(stagingPath, { force: true });
          throw this._integrityError(modelKey, fileName, result.reason, targetPath);
        }
        this.log(`   ✓ verified ${fileName}`);
      } else {
        this.log(`   ⚠️  ${fileName} is not in models.manifest.json — kept WITHOUT verification`);
      }

      fs.renameSync(stagingPath, filePath);
      this.log(`✓ ${model.name} downloaded successfully\n`);
      return;
    }

    // Handle HuggingFace CLI downloads (with fallback chain)
    this.log(`\n📦 Downloading ${model.name} (${model.size})...`);
    this.log(`   Repository: ${model.repo}`);
    this.log(`   Destination: ${targetPath}\n`);

    // Tier 1: Try hf CLI if available
    if (this.hfAvailable || this.checkHuggingFaceCli()) {
      try {
        await this._downloadWithHfCli(modelKey, model, targetPath);
        return;
      } catch (err) {
        this.log(`   hf CLI download failed: ${err.message}`);
        this.log('   Falling back to direct HTTP download...\n');
      }
    }

    // Tier 2: Try to auto-install hf CLI, then retry
    if (!this.hfAvailable) {
      this.log('   Attempting to auto-install hf CLI...');
      const installed = this.tryInstallHfCli();
      if (installed) {
        this.hfAvailable = true;
        try {
          await this._downloadWithHfCli(modelKey, model, targetPath);
          return;
        } catch (err) {
          this.log(`   hf CLI download failed after install: ${err.message}`);
          this.log('   Falling back to direct HTTP download...\n');
        }
      } else {
        this.log('   Auto-install of hf CLI failed. Using direct HTTP download.\n');
      }
    }

    // Tier 3 verifies each file as it lands, so a Tier 1/2 integrity failure
    // above is recoverable: we re-fetch at the pinned revision rather than
    // aborting the install.

    // Tier 3: Direct HTTP download from HuggingFace
    await this.downloadModelDirect(modelKey);
  }

  /**
   * Download a model using the hf CLI (extracted from original downloadModel)
   */
  _downloadWithHfCli(modelKey, model, targetPath) {
    return new Promise((resolve, reject) => {
      const args = [
        'download',
        model.repo,
        '--local-dir', targetPath
      ];

      // Pin to the manifest revision so the CLI tier resolves the same
      // immutable commit the hashes were generated from.
      const entry = this.manifestEntry(modelKey);
      const revision = entry && entry.source && entry.source.revision;
      if (revision) {
        args.push('--revision', revision);
      } else {
        this.log('   ⚠️  No pinned revision in the manifest — hf CLI will use "main".');
      }

      // Note: We don't use --include because it's unreliable with hf CLI
      // Just download all files from the repository

      const proc = this._spawn(this.hfPath || 'hf', args, {
        stdio: this.verbose ? 'inherit' : 'pipe'
      });

      let stderr = '';

      if (!this.verbose) {
        proc.stdout.on('data', (data) => {
          // Show progress
          process.stdout.write(data);
        });

        proc.stderr.on('data', (data) => {
          stderr += data.toString();
        });
      }

      proc.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Download failed with code ${code}\n${stderr}`));
          return;
        }

        // The CLI tier writes files under their FINAL names, so verification
        // happens after the fact. A failure here rejects, which drops us to the
        // direct-HTTP tier — that one re-fetches and verifies file by file.
        this.log('   Verifying downloaded files...');
        this.verifyModelFiles(modelKey, targetPath, expandModelFiles(model)).then((report) => {
          if (!report.ok) {
            // Quarantine before falling through: leaving a rejected file under
            // its real name lets the next run's size-only isModelDownloaded
            // bless it, and a same-size corruption would never be re-hashed.
            for (const failure of report.failures) {
              const moved = quarantineFile(path.join(targetPath, failure.path));
              if (moved) this.log(`   ⚠️  quarantined ${failure.path} → ${path.basename(moved)}`);
            }
            const detail = report.failures.map(f => `${f.path} (${f.reason})`).join('; ');
            reject(new Error(`integrity check failed for ${report.failures.length} file(s): ${detail}`));
            return;
          }
          this.log(`   ✓ verified ${report.verified.length} file(s)`);

          // Run post-download processing if defined
          if (model.postDownload) {
            model.postDownload(targetPath);
          }

          this.log(`✓ ${model.name} downloaded successfully\n`);
          resolve();
        }, reject);
      });

      proc.on('error', (err) => {
        reject(new Error(`Failed to spawn hf CLI: ${err.message}`));
      });
    });
  }

  /**
   * Attempt to auto-install the hf CLI.
   * On macOS: try brew first, then pip3.
   * On Linux: try pip3.
   * @returns {boolean} true if hf CLI is now available
   */
  tryInstallHfCli() {
    const platform = os.platform();

    // On macOS, try brew first
    if (platform === 'darwin') {
      try {
        this.log('   Trying: brew install huggingface-cli');
        execSync('brew install huggingface-cli', { stdio: 'pipe', timeout: 120000 });
        if (this.checkHuggingFaceCli()) {
          this.log('   Successfully installed hf CLI via brew.');
          return true;
        }
      } catch {
        this.log('   brew install failed or not available.');
      }
    }

    // Try pip3 on both macOS and Linux
    try {
      this.log('   Trying: pip3 install "huggingface_hub[cli]"');
      execSync('pip3 install "huggingface_hub[cli]"', { stdio: 'pipe', timeout: 120000 });
      if (this.checkHuggingFaceCli()) {
        this.log('   Successfully installed hf CLI via pip3.');
        return true;
      }
    } catch {
      this.log('   pip3 install failed or not available.');
    }

    return false;
  }

  /**
   * Download a model directly from HuggingFace via HTTP.
   * Handles both flat files and .mlmodelc directory bundles.
   * Uses downloadWithRetry for robust downloading with retry/resume.
   */
  async downloadModelDirect(modelKey) {
    const model = MODELS[modelKey];
    if (!model) {
      throw new Error(`Unknown model: ${modelKey}`);
    }
    if (!model.repo) {
      throw new Error(`Model ${modelKey} has no repo defined for direct download`);
    }

    const targetPath = path.join(this.modelDir, model.targetDir);

    // Expand the file list: .mlmodelc bundles become multiple internal files
    const filesToDownload = expandModelFiles(model);

    this.log(`   Downloading ${filesToDownload.length} file(s) via direct HTTP...`);

    for (let i = 0; i < filesToDownload.length; i++) {
      const file = filesToDownload[i];
      const url = this.fileUrl(modelKey, file);
      const dest = path.join(targetPath, file);
      const expected = this._expectedFile(modelKey, file);

      // Already correct on disk (an interrupted install, or a re-run): don't
      // re-pull gigabytes we can prove we already have.
      if (!this.force && expected && fs.existsSync(dest)) {
        const existing = await verifyFile(dest, expected);
        if (existing.ok) {
          this.log(`   [${i + 1}/${filesToDownload.length}] ${file} — already verified, skipping`);
          continue;
        }
      }

      this.log(`   [${i + 1}/${filesToDownload.length}] ${file}`);

      // Download to a staging name; the file only takes its real name once it
      // has been proven to match the manifest.
      const staging = `${dest}.download`;

      // downloadWithRetry resumes from `<its dest>.partial`, so introducing the
      // staging name moved the resume file from <dest>.partial to
      // <dest>.download.partial. Adopt a pre-upgrade partial instead of
      // orphaning it — otherwise an interrupted install restarts from zero and
      // leaves the old bytes on disk forever.
      const legacyPartial = `${dest}.partial`;
      const stagingPartial = `${staging}.partial`;
      if (fs.existsSync(legacyPartial) && !fs.existsSync(stagingPartial)) {
        try {
          fs.renameSync(legacyPartial, stagingPartial);
          this.log(`   ↻ resuming ${file} from a pre-upgrade partial download`);
        } catch {
          // Not worth failing an install over: we just download from zero.
        }
      }

      // A complete staging file means the bytes arrived and we were interrupted
      // during hashing. Verification below still has to pass, so re-fetching
      // gigabytes to reach the same check buys nothing.
      const stagedComplete = !this.force && expected && fs.existsSync(staging) &&
        fs.statSync(staging).size === expected.size;

      if (stagedComplete) {
        this.log('         already staged in full — verifying without re-downloading');
        // The staged file supersedes any resume partial (including one just
        // migrated from the legacy name) — remove it so it can't linger.
        fs.rmSync(stagingPartial, { force: true });
      } else {
        await this._download(url, staging, {
          maxRetries: 3,
          timeout: 60000,
          checkDiskSpace: i === 0 // only check disk space on first file
        });
      }

      if (expected) {
        const result = await verifyFile(staging, expected);
        if (!result.ok) {
          // Never leave a bad file as a resume base for the next attempt.
          fs.rmSync(staging, { force: true });
          throw this._integrityError(modelKey, file, result.reason, targetPath);
        }
      } else {
        this.log(`   ⚠️  ${file} is not in models.manifest.json — kept WITHOUT verification`);
      }

      fs.mkdirSync(path.dirname(dest), { recursive: true });
      fs.renameSync(staging, dest);
    }

    const verifiedCount = filesToDownload.filter(f => this._expectedFile(modelKey, f)).length;
    if (verifiedCount > 0) this.log(`   ✓ verified ${verifiedCount} file(s) against models.manifest.json`);

    // Run post-download processing if defined
    if (model.postDownload) {
      model.postDownload(targetPath);
    }

    this.log(`✓ ${model.name} downloaded successfully (via direct HTTP)\n`);
  }

  /**
   * Download multiple models
   */
  async downloadModels(modelKeys) {
    this.hfAvailable = this.checkHuggingFaceCli();

    if (!this.hfAvailable) {
      this.log('Note: hf CLI not found. Will attempt auto-install or use direct HTTP download.');
    }

    this.ensureModelDir();

    this.log(`\n🚀 Swictation Model Downloader`);
    this.log(`   Downloading ${modelKeys.length} model(s)`);
    this.log(`   Destination: ${this.modelDir}\n`);

    for (const modelKey of modelKeys) {
      await this.downloadModel(modelKey);
    }

    this.log('✨ All models downloaded successfully!\n');
    this.log('Next steps:');
    this.log('  1. Run: swictation setup');
    this.log('  2. Run: swictation start\n');
  }

  /**
   * Log message
   */
  log(message) {
    console.log(message);
  }

  /**
   * Log error
   */
  error(message) {
    console.error(`❌ ${message}`);
  }
}

// Expose the model table so callers derive valid names from one place (ADR-034).
ModelDownloader.MODELS = MODELS;
// Integrity surface (ADR-036), exposed for scripts/generate-model-manifest.js
// and tests/test-model-manifest.js.
ModelDownloader.MLMODELC_INTERNAL_FILES = MLMODELC_INTERNAL_FILES;
ModelDownloader.DEFAULT_MANIFEST_PATH = DEFAULT_MANIFEST_PATH;
ModelDownloader.expandModelFiles = expandModelFiles;
ModelDownloader.loadManifest = loadManifest;
ModelDownloader.verifyFile = verifyFile;
ModelDownloader.quarantineFile = quarantineFile;
ModelDownloader.QUARANTINE_SUFFIX_RE = QUARANTINE_SUFFIX_RE;

module.exports = ModelDownloader;

// CLI support
if (require.main === module) {
  const args = process.argv.slice(2);
  const options = {
    force: args.includes('--force'),
    verbose: args.includes('--verbose')
  };

  const modelArg = args.find(arg => arg.startsWith('--model='));
  const modelValue = modelArg ? modelArg.split('=')[1] : 'both';

  let modelKeys = [];
  switch (modelValue) {
    case '0.6b':
      modelKeys = ['vad', '0.6b'];
      break;
    case '1.1b':
      modelKeys = ['vad', '1.1b'];
      break;
    case '0.6b-coreml':
      modelKeys = ['vad', '0.6b-coreml'];
      break;
    case 'both':
    default:
      modelKeys = ['vad', '0.6b', '1.1b'];
  }

  const downloader = new ModelDownloader(options);

  downloader.downloadModels(modelKeys)
    .then(() => process.exit(0))
    .catch(err => {
      console.error(`\n❌ Download failed: ${err.message}`);
      process.exit(1);
    });
}
