#!/usr/bin/env node

/**
 * Get secure socket paths matching Rust implementation
 * Priority: XDG_RUNTIME_DIR > ~/.local/share/swictation
 */

const os = require('os');
const path = require('path');
const fs = require('fs');

/**
 * Get socket directory with same logic as Rust socket_utils.rs
 * @returns {string} Socket directory path
 */
function getSocketDir() {
  // Try XDG_RUNTIME_DIR first (best practice for sockets)
  if (process.env.XDG_RUNTIME_DIR && fs.existsSync(process.env.XDG_RUNTIME_DIR)) {
    return process.env.XDG_RUNTIME_DIR;
  }

  // Fallback to ~/.local/share/swictation
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
