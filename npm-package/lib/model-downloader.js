#!/usr/bin/env node

/**
 * Model Downloader for Swictation
 * Downloads AI models from HuggingFace using huggingface-cli
 */

const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { downloadWithRetry } = require('../src/download');

// Every .mlmodelc bundle has exactly this internal structure
const MLMODELC_INTERNAL_FILES = [
  'weights/weight.bin',
  'metadata.json',
  'model.mil',
  'coremldata.bin',
  'analytics/coremldata.bin'
];

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

class ModelDownloader {
  constructor(options = {}) {
    this.modelDir = options.modelDir || path.join(
      os.homedir(),
      '.local',
      'share',
      'swictation',
      'models'
    );
    this.force = options.force || false;
    this.verbose = options.verbose || false;
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
   * Check if a model is already downloaded
   */
  isModelDownloaded(modelKey) {
    const model = MODELS[modelKey];
    if (!model) return false;

    const modelPath = path.join(this.modelDir, model.targetDir);
    if (!fs.existsSync(modelPath)) return false;

    // Check if all required files exist
    return model.files.every(file => {
      const filePath = path.join(modelPath, file);
      return fs.existsSync(filePath);
    });
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

      return new Promise((resolve, reject) => {
        // Create target directory
        if (!fs.existsSync(targetPath)) {
          fs.mkdirSync(targetPath, { recursive: true });
        }

        const filePath = path.join(targetPath, model.files[0]);
        const proc = spawn('curl', [
          '-L',  // Follow redirects
          model.directUrl,
          '-o', filePath,
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
            reject(new Error(`Download failed with code ${code}\n${stderr}`));
            return;
          }

          this.log(`✓ ${model.name} downloaded successfully\n`);
          resolve();
        });

        proc.on('error', (err) => {
          reject(new Error(`Failed to spawn curl: ${err.message}`));
        });
      });
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

      // Note: We don't use --include because it's unreliable with hf CLI
      // Just download all files from the repository

      const proc = spawn(this.hfPath || 'hf', args, {
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

        // Run post-download processing if defined
        if (model.postDownload) {
          model.postDownload(targetPath);
        }

        this.log(`✓ ${model.name} downloaded successfully\n`);
        resolve();
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
    const filesToDownload = [];
    for (const file of model.files) {
      if (file.endsWith('.mlmodelc')) {
        // Expand .mlmodelc bundle to its internal files
        for (const internalFile of MLMODELC_INTERNAL_FILES) {
          filesToDownload.push(path.join(file, internalFile));
        }
      } else {
        filesToDownload.push(file);
      }
    }

    this.log(`   Downloading ${filesToDownload.length} file(s) via direct HTTP...`);

    for (let i = 0; i < filesToDownload.length; i++) {
      const file = filesToDownload[i];
      const url = `https://huggingface.co/${model.repo}/resolve/main/${file}`;
      const dest = path.join(targetPath, file);

      this.log(`   [${i + 1}/${filesToDownload.length}] ${file}`);

      await downloadWithRetry(url, dest, {
        maxRetries: 3,
        timeout: 60000,
        checkDiskSpace: i === 0 // only check disk space on first file
      });
    }

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
