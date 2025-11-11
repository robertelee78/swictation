#!/usr/bin/env node

/**
 * Model Downloader for Swictation
 * Downloads AI models from HuggingFace using huggingface-cli
 */

const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

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
    name: 'Parakeet-TDT 0.6B',
    size: '2.47 GB',
    repo: 'csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3',
    targetDir: 'sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx',
    files: ['encoder.onnx', 'decoder.onnx', 'joiner.onnx', 'tokens.txt']
  },
  '1.1b': {
    name: 'Parakeet-TDT 1.1B INT8',
    size: '6.96 GB',
    repo: 'jenerallee78/parakeet-tdt-1.1b-onnx',
    targetDir: 'parakeet-tdt-1.1b-onnx',
    files: ['encoder.int8.onnx', 'decoder.int8.onnx', 'joiner.int8.onnx', 'tokens.txt']
  }
};

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
      execSync('hf version', { stdio: 'ignore' });
      return true;
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

    // Handle HuggingFace CLI downloads
    this.log(`\n📦 Downloading ${model.name} (${model.size})...`);
    this.log(`   Repository: ${model.repo}`);
    this.log(`   Destination: ${targetPath}\n`);

    return new Promise((resolve, reject) => {
      // Build hf CLI command
      const args = [
        'download',
        model.repo,
        '--local-dir', targetPath
      ];

      // Note: We don't use --include because it's unreliable with hf CLI
      // Just download all files from the repository

      const proc = spawn('hf', args, {
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
   * Download multiple models
   */
  async downloadModels(modelKeys) {
    // Check for hf CLI
    if (!this.checkHuggingFaceCli()) {
      this.error('hf CLI not found!');
      this.log('');
      this.log('The model downloader requires the hf CLI from the huggingface_hub package.');
      this.log('');
      this.log('Install with:');
      this.log('  pip install --upgrade huggingface_hub[cli]');
      this.log('');
      this.log('Or using pipx (recommended):');
      this.log('  pipx install huggingface_hub[cli]');
      this.log('');
      throw new Error('hf CLI is required');
    }

    this.ensureModelDir();

    const totalSize = modelKeys.reduce((sum, key) => {
      const model = MODELS[key];
      return sum + (model ? model.size : '');
    }, '');

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
