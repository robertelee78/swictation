/**
 * Systemd daemon-unit generation — single source of truth (ADR-034).
 *
 * templates/swictation-daemon.service.template is rendered here and ONLY here.
 * Both postinstall.js and `swictation setup` (bin/swictation) call
 * buildDaemonServiceUnit() so the two paths can never diverge again.
 * The previous inline generator in bin/swictation omitted ORT_DYLIB_PATH,
 * which the template marks CRITICAL (blank-output failure mode).
 */

const fs = require('fs');
const path = require('path');
const { getDataDir, getGpuLibsDir } = require('./paths');

const TEMPLATE_PATH = path.join(__dirname, '..', 'templates', 'swictation-daemon.service.template');

/**
 * Detect CUDA library paths in priority order:
 * 1. User's gpu-libs directory (downloaded multi-architecture packages)
 * 2. Common system CUDA installations containing cuDNN/CUDA runtime
 */
function detectCudaLibraryPaths() {
  const paths = [];

  const gpuLibsDir = getGpuLibsDir();
  if (fs.existsSync(gpuLibsDir)) {
    paths.push(gpuLibsDir);
  }

  const cudaDirs = [
    '/usr/local/cuda/lib64',
    '/usr/local/cuda/lib',
    '/usr/local/cuda-13/lib64',
    '/usr/local/cuda-13/lib',
    '/usr/local/cuda-12.9/lib64',
    '/usr/local/cuda-12.9/lib',
    '/usr/local/cuda-12/lib64',
    '/usr/local/cuda-12/lib',
  ];

  for (const dir of cudaDirs) {
    try {
      if (fs.existsSync(dir)) {
        const files = fs.readdirSync(dir);
        if (files.some(f => f.startsWith('libcudnn.so') || f.startsWith('libcudart.so'))) {
          if (!paths.includes(dir)) {
            paths.push(dir);
          }
        }
      }
    } catch (err) {
      // Ignore errors from directories we can't read
    }
  }

  return paths;
}

/** Every Environment= line must have balanced quotes or systemd rejects the unit. */
function validateServiceFile(content) {
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes('Environment=')) {
      const quoteCnt = (lines[i].match(/"/g) || []).length;
      if (quoteCnt % 2 !== 0) {
        throw new Error(`Malformed Environment variable at line ${i + 1}: ${lines[i]}`);
      }
    }
  }
}

/**
 * Render the daemon service unit from the template.
 *
 * @param {object} binaryPaths - result of resolveBinaryPaths(): { binDir, libDir, ... }
 * @param {function} [log] - optional (color, message) logger for progress output
 * @returns {{ content: string, ortLibPath: string, ldLibraryPath: string, ortFound: boolean }}
 */
function buildDaemonServiceUnit(binaryPaths, log = () => {}) {
  if (!fs.existsSync(TEMPLATE_PATH)) {
    throw new Error(`Service template not found at ${TEMPLATE_PATH}`);
  }
  let template = fs.readFileSync(TEMPLATE_PATH, 'utf8');

  template = template.replace(/__INSTALL_DIR__/g, binaryPaths.binDir);

  const detectedCudaPaths = detectCudaLibraryPaths();

  // ONNX Runtime location priority: downloaded gpu-libs, then platform package lib.
  const gpuLibsDir = getGpuLibsDir();
  const gpuLibsOrtPath = path.join(gpuLibsDir, 'libonnxruntime.so');
  const platformOrtPath = path.join(binaryPaths.libDir, 'libonnxruntime.so');

  let ortLibPath;
  let ldLibraryPath;
  let ortFound = true;
  if (fs.existsSync(gpuLibsOrtPath)) {
    ortLibPath = gpuLibsOrtPath;
    ldLibraryPath = [gpuLibsDir, binaryPaths.libDir, ...detectedCudaPaths].join(':');
    log('cyan', `  Using downloaded ONNX Runtime: ${ortLibPath}`);
  } else if (fs.existsSync(platformOrtPath)) {
    ortLibPath = platformOrtPath;
    ldLibraryPath = [...detectedCudaPaths, binaryPaths.libDir].join(':');
    log('cyan', `  Using platform package ONNX Runtime: ${ortLibPath}`);
  } else {
    ortLibPath = platformOrtPath;
    ldLibraryPath = [...detectedCudaPaths, binaryPaths.libDir].join(':');
    ortFound = false;
    log('yellow', `  ⚠️  ONNX Runtime not found in gpu-libs: ${gpuLibsOrtPath}`);
    log('yellow', `  ⚠️  ONNX Runtime not found in platform: ${platformOrtPath}`);
    log('yellow', '  ⚠️  Service may fail - run GPU library download manually');
  }

  // Trim whitespace/newlines so a stray capture can't malform the unit.
  const cleanOrtPath = ortLibPath.trim().replace(/[\r\n]/g, '');
  template = template.replace(/__ORT_DYLIB_PATH__/g, cleanOrtPath);
  const cleanLdPath = ldLibraryPath.trim().replace(/[\r\n]/g, '');
  template = template.replace(/__LD_LIBRARY_PATH__/g, cleanLdPath);
  log('cyan', `  ORT_DYLIB_PATH set to: ${cleanOrtPath}`);
  log('cyan', `  LD_LIBRARY_PATH set to: ${cleanLdPath}`);

  // Display environment (Wayland socket detection + X11 DISPLAY).
  const runtimeDir = process.env.XDG_RUNTIME_DIR || `/run/user/${process.getuid()}`;
  let waylandDisplay = null;
  const xDisplay = process.env.DISPLAY || null;
  try {
    const sockets = fs.readdirSync(runtimeDir).filter(f => f.startsWith('wayland-'));
    if (sockets.length > 0) {
      waylandDisplay = sockets[0];
    }
  } catch (err) {
    // Wayland socket not found, may be X11-only system
  }

  if (waylandDisplay || xDisplay) {
    const envVars = [];
    if (waylandDisplay) envVars.push(`Environment="WAYLAND_DISPLAY=${waylandDisplay}"`);
    if (xDisplay) envVars.push(`Environment="DISPLAY=${xDisplay}"`);
    template = template.replace(
      /ImportEnvironment=/,
      `${envVars.join('\n')}\n\n# Import full user environment for PulseAudio/PipeWire session\n# This ensures all audio devices are detected properly (4 devices instead of 1)\n# Required for microphone access in user session\nImportEnvironment=`
    );
  }

  validateServiceFile(template);

  return { content: template, ortLibPath: cleanOrtPath, ldLibraryPath: cleanLdPath, ortFound };
}

module.exports = { buildDaemonServiceUnit, detectCudaLibraryPaths, validateServiceFile };
