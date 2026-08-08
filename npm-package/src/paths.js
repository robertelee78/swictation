/**
 * Platform-appropriate directory paths — single source of truth for the JS layer.
 *
 * These MUST mirror the Rust `swictation-paths` crate (ADR-034):
 *   config dir  — macOS: ~/Library/Application Support/swictation  Linux: ~/.config/swictation
 *   data dir    — macOS: ~/Library/Application Support/swictation  Linux: ~/.local/share/swictation
 *   cache dir   — macOS: ~/Library/Caches/swictation               Linux: ~/.cache/swictation
 *
 * Never hardcode `.config` / `.local/share` literals elsewhere; require this module.
 */

const path = require('path');
const os = require('os');

function getConfigDir() {
  if (process.platform === 'darwin') {
    return path.join(os.homedir(), 'Library', 'Application Support', 'swictation');
  }
  return path.join(os.homedir(), '.config', 'swictation');
}

function getDataDir() {
  if (process.platform === 'darwin') {
    return path.join(os.homedir(), 'Library', 'Application Support', 'swictation');
  }
  return path.join(os.homedir(), '.local', 'share', 'swictation');
}

function getCacheDir() {
  if (process.platform === 'darwin') {
    return path.join(os.homedir(), 'Library', 'Caches', 'swictation');
  }
  return path.join(os.homedir(), '.cache', 'swictation');
}

/** Directory the daemon reads models from (config.rs get_default_model_dir). */
function getModelsDir() {
  return path.join(getDataDir(), 'models');
}

/** Directory for downloaded GPU libraries (outside npm-owned trees). */
function getGpuLibsDir() {
  return path.join(getDataDir(), 'gpu-libs');
}

module.exports = { getConfigDir, getDataDir, getCacheDir, getModelsDir, getGpuLibsDir };
