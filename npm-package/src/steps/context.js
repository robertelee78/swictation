/**
 * The one immutable context every step reads (ADR-037, amendment 4).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * Steps must never rediscover facts from globals. `sudo npm install -g`
 * and a plain `swictation setup` resolve DIFFERENT users, different homes
 * and different platform packages, so a step that calls os.homedir() or
 * resolveBinaryPaths() on its own can write a unit into /root while the
 * check that "verified" it read the real user's home — the install then
 * reports healthy and nothing works. Resolving once, up front, and freezing
 * the result makes that class of bug unrepresentable.
 *
 * The context is frozen. Phases that learn something new (postinstall
 * detects the GPU at slot 4) do not mutate it — they derive a new frozen
 * context with deriveContext() and hand that to later slots.
 */

const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');

const MODES = ['postinstall', 'setup', 'doctor'];

/**
 * Who we are versus whose install this is.
 *
 * Under `sudo`, the effective user is root but the install belongs to
 * SUDO_USER — and $HOME may be either, depending on whether sudo was given
 * -H. Steps that write into a user home declare `forbidRoot` and are
 * BLOCKED in that case rather than guessing.
 */
function resolveUsers(env = process.env, platform = process.platform) {
  const uid = typeof process.getuid === 'function' ? process.getuid() : null;
  let effectiveUser = null;
  try {
    effectiveUser = os.userInfo().username;
  } catch {
    effectiveUser = env.USER || env.LOGNAME || null;
  }

  const sudoUser = env.SUDO_USER || null;
  const elevatedForAnother = uid === 0 && !!sudoUser && sudoUser !== effectiveUser;

  const targetUser = elevatedForAnother ? sudoUser : effectiveUser;
  const targetHome = elevatedForAnother
    ? lookupHome(sudoUser, platform)
    : os.homedir();

  return { uid, isRoot: uid === 0, effectiveUser, targetUser, targetHome, elevatedForAnother };
}

/**
 * Another user's home directory, or null when it cannot be determined.
 * Best-effort and never throws: this feeds the doctor header and the
 * blocked-step explanation, never a write path.
 */
function lookupHome(user, platform) {
  try {
    if (platform === 'darwin') {
      const out = execFileSync('dscl', ['.', '-read', `/Users/${user}`, 'NFSHomeDirectory'], {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      const match = out.match(/NFSHomeDirectory:\s*(.+)/);
      return match ? match[1].trim() : null;
    }
    const out = execFileSync('getent', ['passwd', user], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const fields = out.trim().split(':');
    return fields.length >= 6 ? fields[5] : null;
  } catch {
    return null;
  }
}

/**
 * Capabilities, as facts about this process rather than probes.
 *
 * `network` is declared, not probed: reachability tests cost seconds and
 * produce false negatives behind captive portals, and every download path
 * already retries and reports. Users opt out explicitly.
 */
function resolveCapabilities(env = process.env, users = resolveUsers(env)) {
  return {
    network: env.SWICTATION_OFFLINE !== '1' && env.npm_config_offline !== 'true',
    // A session exists unless we are root acting on someone else's behalf,
    // where the user bus, launchd domain and home all belong to another uid.
    session: !users.elevatedForAnother,
    root: users.isRoot,
  };
}

/** Silent NVIDIA presence probe — a resolved fact, never re-run inside steps. */
function resolveNvidia(platform) {
  if (platform !== 'linux') return false;
  try {
    return require('../../postinstall').detectNvidiaGPU();
  } catch {
    return false;
  }
}

/**
 * The GPU library variant this machine wants, resolved ONCE.
 *
 * `undefined` means "not probed" (not a GPU machine); `null` means "probed
 * and could not tell", which a step must report as unknown rather than
 * silently accepting whatever variant happens to be installed. The probe is
 * run with a no-op logger because contexts are built inside doctor too.
 */
function resolveGpuVariant(platform, hasNvidiaGpu) {
  if (platform !== 'linux' || hasNvidiaGpu !== true) return undefined;
  try {
    const postinstall = require('../../postinstall');
    const gpu = postinstall.detectGPUComputeCapability(() => {});
    if (!gpu || !gpu.smVersion) return null;
    const selected = postinstall.selectGPUPackageVariant(gpu.smVersion);
    return (selected && selected.variant) || null;
  } catch {
    return null;
  }
}

/** The recommendation a previous install recorded, or null. */
function readSavedGpuInfo() {
  try {
    const { getConfigDir } = require('../paths');
    const fs = require('fs');
    const parsed = JSON.parse(fs.readFileSync(path.join(getConfigDir(), 'gpu-info.json'), 'utf8'));
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

/** Platform binaries, or null when the platform package is not installed. */
function resolveBinaries() {
  try {
    return require('../resolve-binary').resolveBinaryPaths();
  } catch {
    return null;
  }
}

/**
 * Build the frozen context.
 *
 * @param {object} options
 * @param {'postinstall'|'setup'|'doctor'} options.mode
 * @param {(color: string, message: string) => void} options.log
 * @param {object} [options.gpuInfo] - caller already has live detection
 * @param {string} [options.ortLibPath]
 * @param {function} [options.generateDefaultConfig]
 * @param {object} [options.overrides] - test seams (platform, caps, users…)
 */
function createContext(options = {}) {
  const {
    mode = 'setup',
    log = (_color, message) => console.log(message),
    generateDefaultConfig,
    ...facts
  } = options;

  if (!MODES.includes(mode)) throw new Error(`Unknown context mode: ${mode}`);

  const env = facts.env || process.env;
  const platform = facts.platform || process.platform;
  const users = facts.users || resolveUsers(env, platform);
  const gpuInfo = facts.gpuInfo !== undefined ? facts.gpuInfo : readSavedGpuInfo();

  const hasNvidiaGpu = facts.hasNvidiaGpu !== undefined ? facts.hasNvidiaGpu : resolveNvidia(platform);
  const home = users.targetHome;

  const ctx = {
    mode,
    platform,
    arch: facts.arch || process.arch,
    env,
    log,
    clock: facts.clock || (() => new Date()),

    uid: users.uid,
    isRoot: users.isRoot,
    effectiveUser: users.effectiveUser,
    targetUser: users.targetUser,
    targetHome: users.targetHome,
    elevatedForAnother: users.elevatedForAnother,

    caps: facts.caps || resolveCapabilities(env, users),

    binaryPaths: facts.binaryPaths !== undefined ? facts.binaryPaths : resolveBinaries(),
    gpuInfo,
    // The single answer to "which model is this install about", resolved
    // once so check() never re-derives it (and never re-runs detection).
    selectedModel: facts.selectedModel !== undefined
      ? facts.selectedModel
      : (gpuInfo && typeof gpuInfo.recommendedModel === 'string' ? gpuInfo.recommendedModel : null),
    hasNvidiaGpu,
    // undefined = not a GPU machine; null = probed and undeterminable.
    gpuVariant: facts.gpuVariant !== undefined
      ? facts.gpuVariant
      : resolveGpuVariant(platform, hasNvidiaGpu),
    ortLibPath: facts.ortLibPath !== undefined ? facts.ortLibPath : null,

    // Artifact locations, derived from the TARGET home rather than
    // os.homedir(): under sudo those differ, and a step that reads one while
    // writing the other reports healthy over an install that does not work.
    systemdUnitPath: facts.systemdUnitPath !== undefined
      ? facts.systemdUnitPath
      : (home ? path.join(home, '.config', 'systemd', 'user', 'swictation-daemon.service') : null),
    launchAgentsDir: facts.launchAgentsDir !== undefined
      ? facts.launchAgentsDir
      : (home ? path.join(home, 'Library', 'LaunchAgents') : null),
    daemonPlistPath: facts.daemonPlistPath !== undefined
      ? facts.daemonPlistPath
      : (home ? path.join(home, 'Library', 'LaunchAgents', 'com.swictation.daemon.plist') : null),

    generateDefaultConfig: generateDefaultConfig
      || (() => require('../../postinstall').generateDefaultConfig()),
  };

  return Object.freeze(ctx);
}

/** A new frozen context carrying newly resolved facts. Never mutates. */
function deriveContext(ctx, patch = {}) {
  return Object.freeze({ ...ctx, ...patch });
}

module.exports = {
  MODES,
  createContext,
  deriveContext,
  resolveUsers,
  resolveCapabilities,
  resolveNvidia,
  resolveGpuVariant,
  readSavedGpuInfo,
  lookupHome,
};
