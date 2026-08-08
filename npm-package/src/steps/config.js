/**
 * Config step — upgrade-safe configuration handling (ADR-035).
 *
 * Step-contract shape (check/run) so this lifts directly into the hybrid
 * postinstall/setup step registry. Replaces interactiveConfigMigration(),
 * which unconditionally clobbered config.toml with defaults on every
 * install/upgrade (single-slot backup, so the second upgrade destroyed the
 * original). Since ADR-034 every daemon config key has a compiled default,
 * so a partial config loads fine — postinstall must never wholesale-rewrite
 * the user's file. Strategy:
 *   - absent           → write generated defaults
 *   - unparseable      → timestamped backup, write defaults, loud warning
 *   - parseable        → leave the file byte-for-byte intact except stale
 *                        MODEL PATH keys, healed only when the configured
 *                        path no longer exists but the platform default does
 *                        (the ADR-033 crash-loop case). Timestamped backup
 *                        only when actually modifying.
 */

const fs = require('fs');
const path = require('path');
const TOML = require('smol-toml');
const { getConfigDir, getModelsDir } = require('../paths');

/** Keys postinstall may heal, mapped to their platform-default subdirectory. */
const HEALABLE_PATH_KEYS = {
  vad_model_path: () => path.join(getModelsDir(), 'silero-vad', 'silero_vad.onnx'),
  stt_0_6b_model_path: () => path.join(getModelsDir(), 'parakeet-tdt-0.6b-v3-onnx'),
  stt_1_1b_model_path: () => path.join(getModelsDir(), 'parakeet-tdt-1.1b-onnx'),
  stt_coreml_model_path: () => path.join(getModelsDir(), 'parakeet-tdt-1.1b-coreml'),
};

function configPath() {
  return path.join(getConfigDir(), 'config.toml');
}

function timestampedBackup(filePath, log) {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const backupPath = `${filePath}.bak.${stamp}`;
  fs.copyFileSync(filePath, backupPath);
  log('cyan', `  Backed up config to ${backupPath}`);
  return backupPath;
}

function parseConfig(raw) {
  try {
    return { parsed: TOML.parse(raw), error: null };
  } catch (err) {
    return { parsed: null, error: err };
  }
}

/**
 * Replace `key = "<value>"` in place, preserving all other lines verbatim
 * (comments and formatting included). Returns the new content, or null if the
 * key line was not found (absent keys are fine — the daemon defaults them).
 */
function replaceKeyLine(content, key, newValue) {
  const re = new RegExp(`^(\\s*${key}\\s*=\\s*).*$`, 'm');
  if (!re.test(content)) return null;
  return content.replace(re, `$1${JSON.stringify(newValue)}`);
}

/** True when the config file exists and parses. */
function check() {
  const p = configPath();
  if (!fs.existsSync(p)) return false;
  return parseConfig(fs.readFileSync(p, 'utf8')).error === null;
}

/**
 * @param {object} ctx - { log(color,msg), generateDefaultConfig() }
 * @returns {{ action: 'created'|'replaced-invalid'|'healed'|'kept', healedKeys: string[] }}
 */
function run(ctx) {
  const { log, generateDefaultConfig } = ctx;
  const p = configPath();
  fs.mkdirSync(path.dirname(p), { recursive: true });

  if (!fs.existsSync(p)) {
    fs.writeFileSync(p, generateDefaultConfig());
    log('green', `  Config created at ${p}`);
    return { action: 'created', healedKeys: [] };
  }

  const raw = fs.readFileSync(p, 'utf8');
  const { parsed, error } = parseConfig(raw);

  if (error) {
    // Broken config (e.g. the pre-ADR-034 examples containing `null`).
    timestampedBackup(p, log);
    fs.writeFileSync(p, generateDefaultConfig());
    log('yellow', `  ⚠️  Existing config did not parse (${error.message}); replaced with defaults (backup kept)`);
    return { action: 'replaced-invalid', healedKeys: [] };
  }

  // Parseable: preserve the user's file. Heal only model-path keys whose
  // configured target is missing while the platform default exists — the
  // stale-path-after-migration case (ADR-033). A path the user customized to
  // a real location is never touched.
  let content = raw;
  const healedKeys = [];
  for (const [key, defaultPathFn] of Object.entries(HEALABLE_PATH_KEYS)) {
    const configured = parsed[key];
    if (typeof configured !== 'string' || configured.length === 0) continue;
    const defaultPath = defaultPathFn();
    if (configured === defaultPath) continue;
    if (!fs.existsSync(configured) && fs.existsSync(defaultPath)) {
      // Guard: only heal simple single-line `key = "value"` forms. Quoted
      // keys, multiline strings etc. parse fine but a line-regex rewrite
      // would corrupt them — skip those and let the daemon report the stale
      // path rather than write invalid TOML.
      const lineMatch = content.match(new RegExp(`^\\s*${key}\\s*=\\s*(.*)$`, 'm'));
      let simpleLine = false;
      if (lineMatch) {
        try {
          const rhs = TOML.parse(`probe = ${lineMatch[1]}`);
          simpleLine = rhs.probe === configured;
        } catch {
          simpleLine = false;
        }
      }
      if (!simpleLine) continue;
      const updated = replaceKeyLine(content, key, defaultPath);
      if (updated !== null) {
        content = updated;
        healedKeys.push(key);
      }
    }
  }

  if (healedKeys.length > 0) {
    timestampedBackup(p, log);
    fs.writeFileSync(p, content);
    log('green', `  Healed stale model paths: ${healedKeys.join(', ')} (everything else preserved)`);
    return { action: 'healed', healedKeys };
  }

  log('green', '  Existing config preserved (user settings kept)');
  return { action: 'kept', healedKeys: [] };
}

module.exports = { check, run, configPath, replaceKeyLine, HEALABLE_PATH_KEYS };
