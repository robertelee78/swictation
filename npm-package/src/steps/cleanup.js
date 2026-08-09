/**
 * Legacy-artifact cleanup step (ADR-037 Phase B) — npm lifecycle only.
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * Every artifact this step removes is a SHADOW: an older copy that still
 * resolves ahead of, or alongside, the one this package ships. A Python-era
 * `swictation.service` that systemd still starts. An onnxruntime 1.20 in
 * ~/.local/lib that the dynamic linker finds before ours. A `swictation`
 * under /usr/local/lib/node_modules that `which` picks over the new global.
 * None of them break anything on their own; each one makes the install
 * behave like the version it came from, which is the hardest class of bug to
 * report and the easiest to prevent.
 *
 * `entrypoints: ['postinstall']` is the point of the step, not a detail.
 * Removing another installation's files is a thing an INSTALL may do — the
 * user just asked for this version and npm is mid-lifecycle. It is not a
 * thing `swictation setup` may do: that is a diagnostic command a user runs
 * when something is broken, sometimes on a machine deliberately carrying two
 * installs, and having it delete /usr/local/lib/node_modules/swictation
 * (with sudo, at that) would be a genuinely destructive surprise.
 *
 * run() also stops running services before anything else touches the disk —
 * upgrading underneath a live CUDA process corrupts its state. That is a
 * precondition rather than an artifact, so check() cannot see it, and it is
 * why this step is reasserted on every install rather than skipped when the
 * check is already green.
 */

const fs = require('fs');
const os = require('os');
const path = require('path');
const { healthy, unhealthy, componentOk, componentFailed } = require('./health');

/** Old system-wide and Python-era unit files, by absolute path. */
function legacyServicePaths(home, platform) {
  if (platform === 'darwin') return [];
  return [
    '/usr/lib/systemd/user/swictation.service',
    '/usr/lib/systemd/system/swictation.service',
    path.join(home, '.config', 'systemd', 'user', 'swictation.service'),
  ];
}

/** Global npm trees an older install may still occupy. */
const LEGACY_NPM_PATHS = [
  '/usr/local/lib/node_modules/swictation',
  '/usr/local/nodejs/lib/node_modules/swictation',
  '/usr/lib/node_modules/swictation',
  '/opt/homebrew/lib/node_modules/swictation',
];

/** Directory entries under `parent`, or [] when it cannot be listed. */
function safeReaddir(parent) {
  try {
    return fs.readdirSync(parent);
  } catch {
    return [];
  }
}

/**
 * pip-installed onnxruntime trees that shadow ours.
 *
 * The interpreter versions are DISCOVERED, not listed. A hardcoded
 * 3.10–3.13 stopped seeing conflicts the day Python 3.14 shipped, and an
 * onnxruntime 1.20 under 3.14 shadows ours exactly as hard as one under 3.12
 * — the check simply went blind to it, silently, on a schedule set by
 * python.org rather than by anything in this repo.
 */
function pythonOrtDirs(home, platform) {
  const dirs = [];

  const userLib = path.join(home, '.local', 'lib');
  for (const entry of safeReaddir(userLib)) {
    if (!/^python\d/.test(entry)) continue;
    dirs.push(path.join(userLib, entry, 'site-packages', 'onnxruntime'));
  }

  if (platform === 'darwin') {
    const frameworkLib = path.join(home, 'Library', 'Python');
    for (const entry of safeReaddir(frameworkLib)) {
      if (!/^\d/.test(entry)) continue;
      dirs.push(path.join(frameworkLib, entry, 'lib', 'python', 'site-packages', 'onnxruntime'));
    }
  }

  return dirs;
}

/**
 * Whether a pip onnxruntime tree conflicts with ours.
 *
 * Conflicting means <1.22 — OR unreadable. A `libonnxruntime.so` whose
 * version cannot be parsed is not evidence of a modern install; it is a
 * library of unknown vintage sitting ahead of ours on the linker path, which
 * is the entire failure this cleanup exists for. Returning false there made
 * it invisible to the check AND to the repair, so nobody would ever find it.
 *
 * postinstall's cleanupOldOnnxRuntime() calls THIS function rather than
 * carrying its own copy: two versions of this rule is how a check starts
 * naming artifacts the repair refuses to remove, and a step that can never go
 * green is worse than one that never noticed.
 */
function hasConflictingOrt(ortDir, platform) {
  const capiDir = path.join(ortDir, 'capi');
  const ortName = platform === 'darwin' ? 'libonnxruntime' : 'libonnxruntime.so';
  const candidates = safeReaddir(capiDir).filter(name => name.includes(ortName));
  if (candidates.length === 0) return false;

  return candidates.some(name => {
    const match = name.match(/(\d+)\.(\d+)/);
    if (!match) return true; // unknown vintage — fail closed
    return parseInt(match[1], 10) * 100 + parseInt(match[2], 10) < 122;
  });
}

/** Stale `com.swictation.*` plists left by an earlier layout (macOS). */
function legacyPlists(home, platform) {
  if (platform !== 'darwin') return [];
  const dir = path.join(home, 'Library', 'LaunchAgents');
  try {
    return fs.readdirSync(dir)
      .filter(name => name.startsWith('com.swictation.') && name.endsWith('.plist'))
      // The two this version generates are current, not legacy; cleanOldServices()
      // removes and regenerates them, so listing them here would make the check
      // permanently red immediately after a successful install.
      .filter(name => name !== 'com.swictation.daemon.plist' && name !== 'com.swictation.ui.plist')
      .map(name => path.join(dir, name));
  } catch {
    return [];
  }
}

/** Everything still on disk that this step would remove. */
function survivingArtifacts(ctx) {
  const home = ctx.targetHome || os.homedir();
  const platform = ctx.platform || process.platform;
  const found = [];

  for (const unit of legacyServicePaths(home, platform)) {
    if (fs.existsSync(unit)) found.push(`legacy service unit: ${unit}`);
  }
  for (const plist of legacyPlists(home, platform)) {
    found.push(`stale LaunchAgent: ${plist}`);
  }
  const ownPackage = path.resolve(__dirname, '..', '..');
  for (const dir of LEGACY_NPM_PATHS) {
    // A global install legitimately sits at one of these paths and must never
    // delete itself.
    if (fs.existsSync(dir) && path.resolve(dir) !== ownPackage) {
      found.push(`old npm installation: ${dir}`);
    }
  }
  for (const dir of pythonOrtDirs(home, platform)) {
    if (hasConflictingOrt(dir, platform)) found.push(`conflicting ONNX Runtime (<1.22): ${dir}`);
  }
  return found;
}

module.exports = {
  id: 'cleanup',
  title: 'Cleaning up previous installations...',
  // See the fence: deleting another installation's files belongs to an
  // install, never to a diagnostic the user ran to fix something else.
  entrypoints: ['postinstall'],
  after: ['platform'],
  forbidRoot: true,

  applies(ctx) {
    return ctx.platform === 'linux' || ctx.platform === 'darwin';
  },

  check(ctx) {
    const found = survivingArtifacts(ctx);
    if (found.length > 0) {
      return unhealthy('CLEANUP_LEGACY_ARTIFACTS',
        `${found.length} artifact(s) from a previous installation are still on disk`, {
          evidence: found.slice(0, 8),
          repair: 'npm install -g swictation --force',
        });
    }
    return healthy('CLEANUP_OK', 'no conflicting artifacts from previous installations');
  },

  async run(ctx) {
    const postinstall = require('../../postinstall');
    const before = survivingArtifacts(ctx);
    const components = [];

    // FIRST, before any file is touched: an upgrade that rewrites libraries
    // underneath a live daemon corrupts its CUDA context and the next start
    // fails for reasons that have nothing to do with what changed.
    try {
      await postinstall.stopExistingServices();
      components.push(componentOk('stop-services', 'running services stopped'));
    } catch (err) {
      components.push(componentFailed('stop-services', err.message, err));
    }

    for (const [id, fn] of [
      ['legacy-services', () => postinstall.cleanOldServices()],
      ['python-onnxruntime', () => postinstall.cleanupOldOnnxRuntime()],
      ['old-npm-installs', () => postinstall.cleanupOldNpmInstallations()],
    ]) {
      try {
        await fn();
        components.push(componentOk(id, 'checked'));
      } catch (err) {
        components.push(componentFailed(id, err.message, err));
      }
    }

    const after = survivingArtifacts(ctx);
    const warnings = after.length > 0
      ? [`${after.length} legacy artifact(s) could not be removed — they may need sudo`]
      : [];

    return { changed: before.length > after.length, components, warnings };
  },

  _internals: {
    LEGACY_NPM_PATHS,
    legacyServicePaths,
    legacyPlists,
    pythonOrtDirs,
    hasConflictingOrt,
    survivingArtifacts,
  },
};
