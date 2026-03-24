'use strict';

const HELP_BASE_URL = 'https://github.com/robertelee78/swictation/wiki/errors';

// Error code registry — every user-facing install error has a code
const ERROR_CODES = {
  'SW-E001': { summary: 'Unsupported platform or architecture' },
  'SW-E002': { summary: 'Insufficient disk space' },
  'SW-E003': { summary: 'Download failed (network)' },
  'SW-E004': { summary: 'Checksum verification failed' },
  'SW-E005': { summary: 'GPU detection failed' },
  'SW-E006': { summary: 'Service setup failed (permission)' },
  'SW-E007': { summary: 'Python/hf CLI not found' },
  'SW-E008': { summary: 'Model download failed' },
  'SW-E009': { summary: 'Binary not found in platform package' },
  'SW-E010': { summary: 'ONNX Runtime load failed' },
};

// Map Node.js/system errno codes to human-readable messages
const ERRNO_MAP = {
  'ECONNRESET':   'Network connection was interrupted',
  'ETIMEDOUT':    'Server did not respond within 30 seconds',
  'ECONNREFUSED': 'Could not reach download server — check your internet connection',
  'ENOTFOUND':    'DNS lookup failed — check your internet connection',
  'EHOSTUNREACH': 'Server is unreachable — check your network or try again later',
  'EPIPE':        'Connection was closed unexpectedly',
  'EAI_AGAIN':    'DNS lookup timed out — check your internet connection',
  'ENOSPC':       'Not enough disk space',
  'EACCES':       'Permission denied',
  'ENOENT':       'Required file not found',
  'EROFS':        'Read-only file system — cannot write to destination',
};

// Platform-specific fix suggestions for common errors
function getPlatformFix(errnoCode, context) {
  const platform = process.platform;

  switch (errnoCode) {
    case 'EACCES':
      if (context && context.path) {
        if (platform === 'darwin') {
          return `Check permissions on ${context.path}\n         Or try: sudo chown -R $(whoami) ${context.path}`;
        }
        if (context.path.includes('systemd')) {
          return `Run: sudo loginctl enable-linger $USER\n         Or check ~/.config/systemd/user/ permissions`;
        }
        if (context.path.includes('LaunchAgents')) {
          return `Check ~/Library/LaunchAgents/ permissions`;
        }
        return `Check permissions on ${context.path}`;
      }
      return platform === 'darwin'
        ? 'Check directory permissions or try: sudo chown -R $(whoami) <path>'
        : 'Check directory permissions or ownership';

    case 'ENOSPC':
      if (context && context.needed && context.available) {
        return `Free up disk space at ${context.path || 'the destination'}\n         Need: ${context.needed}, Available: ${context.available}`;
      }
      return 'Free up disk space and try again';

    case 'ECONNRESET':
    case 'ETIMEDOUT':
    case 'ECONNREFUSED':
    case 'ENOTFOUND':
    case 'EHOSTUNREACH':
      return 'Check your internet connection and try again: npm install -g swictation';

    default:
      return null;
  }
}

class InstallError extends Error {
  /**
   * @param {string} code - Error code (e.g., 'SW-E003')
   * @param {string} message - Short error description
   * @param {object} opts
   * @param {string} opts.cause - Human-readable cause
   * @param {string} opts.fix - Actionable fix instruction
   * @param {Error}  [opts.original] - Original error (for logging)
   * @param {object} [opts.context] - Additional context (path, url, etc.)
   */
  constructor(code, message, { cause, fix, original, context } = {}) {
    super(message);
    this.name = 'InstallError';
    this.code = code;
    this.installCause = cause || message;
    this.fix = fix || 'Re-run installation: npm install -g swictation';
    this.helpUrl = `${HELP_BASE_URL}#${code}`;
    this.original = original || null;
    this.context = context || {};
  }

  /**
   * Format error for user-facing console output (no stack traces)
   */
  format() {
    const lines = [
      `\x1b[31m[FAIL] ${this.message} (${this.code})\x1b[0m`,
      `  \x1b[33mCause:\x1b[0m ${this.installCause}`,
      `  \x1b[32mFix:\x1b[0m   ${this.fix}`,
      `  \x1b[36mHelp:\x1b[0m  ${this.helpUrl}`,
    ];
    return lines.join('\n');
  }

  /**
   * Format error for log file (includes stack trace)
   */
  formatForLog() {
    const lines = [
      `[FAIL] ${this.message} (${this.code})`,
      `  Cause: ${this.installCause}`,
      `  Fix:   ${this.fix}`,
      `  Help:  ${this.helpUrl}`,
    ];
    if (this.original) {
      lines.push(`  Original: ${this.original.message}`);
      if (this.original.stack) {
        lines.push(`  Stack: ${this.original.stack}`);
      }
    }
    if (Object.keys(this.context).length > 0) {
      lines.push(`  Context: ${JSON.stringify(this.context)}`);
    }
    return lines.join('\n');
  }

  /**
   * Create an InstallError from a system/Node.js error with smart mapping
   */
  static fromSystemError(err, code, message, context) {
    const errno = err.code || '';
    const humanCause = ERRNO_MAP[errno] || err.message;
    const platformFix = getPlatformFix(errno, context);

    return new InstallError(code, message, {
      cause: humanCause,
      fix: platformFix || 'Re-run installation: npm install -g swictation',
      original: err,
      context,
    });
  }
}

module.exports = { InstallError, ERROR_CODES, ERRNO_MAP, getPlatformFix };
