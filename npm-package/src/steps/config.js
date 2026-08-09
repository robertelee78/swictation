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

const OVERRIDE_KEY = 'stt_model_override';
const AUTO_OVERRIDE = 'auto';

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

/**
 * Sidecar recording values postinstall wrote into config.toml on the user's
 * behalf (ADR-035). Without it, an installer-written `stt_model_override` is
 * indistinguishable from a user-authored one, so the config step preserved it
 * forever and no later install ever re-tested the hardware.
 */
function statePath() {
  return path.join(getConfigDir(), 'postinstall-state.json');
}

function readPostinstallState() {
  try {
    const parsed = JSON.parse(fs.readFileSync(statePath(), 'utf8'));
    // `null`, arrays and scalars all survive JSON.parse; callers index this as
    // a plain record, so anything else is treated as no state at all.
    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
    return parsed;
  } catch {
    return {};
  }
}

/** Record the override postinstall just wrote. Best-effort; never throws. */
function recordManagedOverride(value) {
  try {
    const p = statePath();
    fs.mkdirSync(path.dirname(p), { recursive: true });
    const state = readPostinstallState();
    state.managedOverride = value;
    state.managedOverrideAt = new Date().toISOString();
    fs.writeFileSync(p, JSON.stringify(state, null, 2));
    return true;
  } catch {
    return false;
  }
}

/**
 * Forget the recorded override. The marker is a one-shot claim of ownership:
 * once the reset has consumed it, or once the configured value no longer
 * matches it, it must go. Leaving it behind is an ABA bug — the user later
 * re-selecting that same value would look installer-authored and get reset.
 * Best-effort; never throws.
 */
function clearManagedOverride() {
  try {
    const state = readPostinstallState();
    if (!('managedOverride' in state) && !('managedOverrideAt' in state)) return true;
    delete state.managedOverride;
    delete state.managedOverrideAt;
    fs.writeFileSync(statePath(), JSON.stringify(state, null, 2));
    return true;
  } catch {
    return false;
  }
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

/**
 * True when `key` appears as a plain single-line `key = "value"` carrying
 * exactly `expectedValue`. Quoted keys, multiline strings and the like parse
 * fine but a line-regex rewrite would corrupt them, so callers skip those.
 */
function isSimpleKeyLine(content, key, expectedValue) {
  const lineMatch = content.match(new RegExp(`^\\s*${key}\\s*=\\s*(.*)$`, 'm'));
  if (!lineMatch) return false;
  try {
    return TOML.parse(`probe = ${lineMatch[1]}`).probe === expectedValue;
  } catch {
    return false;
  }
}

/** True when the config file exists and parses. */
function check() {
  const p = configPath();
  if (!fs.existsSync(p)) return false;
  return parseConfig(fs.readFileSync(p, 'utf8')).error === null;
}

/**
 * @param {object} ctx - { log(color,msg), generateDefaultConfig(),
 *   resetManagedOverride? } — set `resetManagedOverride` on the pre-download
 *   pass only. The post-download pass runs after postinstall has written a
 *   freshly tested model, which the reset rule would otherwise undo.
 * @returns {{ action: 'created'|'replaced-invalid'|'healed'|'kept',
 *   healedKeys: string[], resetOverride?: boolean }}
 */
function run(ctx) {
  const { log, generateDefaultConfig, resetManagedOverride = false } = ctx;
  const p = configPath();
  fs.mkdirSync(path.dirname(p), { recursive: true });

  if (!fs.existsSync(p)) {
    fs.writeFileSync(p, generateDefaultConfig());
    log('green', `  Config created at ${p}`);
    return { action: 'created', healedKeys: [], resetOverride: false };
  }

  const raw = fs.readFileSync(p, 'utf8');
  const { parsed, error } = parseConfig(raw);

  if (error) {
    // Broken config (e.g. the pre-ADR-034 examples containing `null`).
    timestampedBackup(p, log);
    fs.writeFileSync(p, generateDefaultConfig());
    log('yellow', `  ⚠️  Existing config did not parse (${error.message}); replaced with defaults (backup kept)`);
    return { action: 'replaced-invalid', healedKeys: [], resetOverride: false };
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
      // Let the daemon report a stale path rather than write invalid TOML.
      if (!isSimpleKeyLine(content, key, configured)) continue;
      const updated = replaceKeyLine(content, key, defaultPath);
      if (updated !== null) {
        content = updated;
        healedKeys.push(key);
      }
    }
  }

  // Installer-authored override (ADR-035): postinstall writes the model it
  // verified into `stt_model_override` and records it in the sidecar. When the
  // config still holds exactly that value, no human chose it — reset to "auto"
  // so this install re-tests the hardware (a GPU that disappeared would
  // otherwise keep forcing a branch that now errors). Any other value is the
  // user's and is never touched.
  let resetOverride = false;
  let consumeMarker = false;
  if (resetManagedOverride) {
    const managed = readPostinstallState().managedOverride;
    const configured = parsed[OVERRIDE_KEY];
    if (typeof managed === 'string' && managed !== AUTO_OVERRIDE) {
      if (configured === managed) {
        if (isSimpleKeyLine(content, OVERRIDE_KEY, configured)) {
          const updated = replaceKeyLine(content, OVERRIDE_KEY, AUTO_OVERRIDE);
          if (updated !== null) {
            content = updated;
            resetOverride = true;
            consumeMarker = true;
          }
        }
      } else {
        // The configured value is no longer the one postinstall wrote, so the
        // user owns this key now. Drop the marker: if they later select that
        // same model deliberately, no install may reset it back to "auto".
        consumeMarker = true;
      }
    }
  }

  const modified = healedKeys.length > 0 || resetOverride;
  if (modified) {
    timestampedBackup(p, log);
    fs.writeFileSync(p, content);
  }
  // Only after the reset is durably on disk — a failed write leaves the marker
  // in place so the next install retries.
  if (consumeMarker) clearManagedOverride();

  if (modified) {
    if (healedKeys.length > 0) {
      log('green', `  Healed stale model paths: ${healedKeys.join(', ')} (everything else preserved)`);
    }
    if (resetOverride) {
      log('green', `  Reset installer-written ${OVERRIDE_KEY} to "${AUTO_OVERRIDE}" — hardware will be re-tested`);
    }
    return { action: 'healed', healedKeys, resetOverride };
  }

  log('green', '  Existing config preserved (user settings kept)');
  return { action: 'kept', healedKeys: [], resetOverride: false };
}

module.exports = {
  check,
  run,
  configPath,
  statePath,
  readPostinstallState,
  recordManagedOverride,
  clearManagedOverride,
  replaceKeyLine,
  HEALABLE_PATH_KEYS,
};
