#!/usr/bin/env node

/**
 * Get secure socket paths matching Rust implementation
 *
 * Platform-specific paths (matching rust-crates/swictation-daemon/src/socket_utils.rs):
 * - macOS: ~/Library/Application Support/swictation
 * - Linux: XDG_RUNTIME_DIR > ~/.local/share/swictation
 */

const os = require('os');
const path = require('path');
const fs = require('fs');

/**
 * Get socket directory with same logic as Rust socket_utils.rs
 * @returns {string} Socket directory path
 */
function getSocketDir() {
  // macOS: Use ~/Library/Application Support/swictation (matches dirs::data_local_dir())
  if (process.platform === 'darwin') {
    const macDir = path.join(os.homedir(), 'Library', 'Application Support', 'swictation');

    // Create directory if it doesn't exist
    if (!fs.existsSync(macDir)) {
      fs.mkdirSync(macDir, { recursive: true, mode: 0o700 });
    }

    return macDir;
  }

  // Linux: Try XDG_RUNTIME_DIR first (best practice for sockets)
  if (process.env.XDG_RUNTIME_DIR && fs.existsSync(process.env.XDG_RUNTIME_DIR)) {
    return process.env.XDG_RUNTIME_DIR;
  }

  // Linux fallback: ~/.local/share/swictation
  const fallbackDir = path.join(os.homedir(), '.local', 'share', 'swictation');

  // Create directory if it doesn't exist
  if (!fs.existsSync(fallbackDir)) {
    fs.mkdirSync(fallbackDir, { recursive: true, mode: 0o700 });
  }

  return fallbackDir;
}

/**
 * Get IPC socket path (main toggle commands)
 * @returns {string} Socket path
 */
function getIpcSocketPath() {
  return path.join(getSocketDir(), 'swictation.sock');
}

/**
 * Get metrics broadcast socket path (UI clients)
 * @returns {string} Socket path
 */
function getMetricsSocketPath() {
  return path.join(getSocketDir(), 'swictation_metrics.sock');
}

module.exports = {
  getSocketDir,
  getIpcSocketPath,
  getMetricsSocketPath
};
