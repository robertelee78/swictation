/**
 * Final verification step (ADR-037 Phase B) — autostart state and the
 * optional-dependency inventory.
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * The split with [[services]] is deliberate and worth stating, because both
 * steps look at the same unit file: `services` owns whether the unit is
 * CORRECT (ORT_DYLIB_PATH present and pointing at a real library, ExecStart
 * naming the binary this install resolved). `verify` owns whether the unit is
 * ENABLED — whether anything will actually start it. Those fail
 * independently: ADR-034's bug was a perfectly enabled unit producing blank
 * output, and the reverse — a flawless unit nothing ever loads — looks
 * identical to the user and to every check that only reads file contents.
 *
 * A unit that exists and is not enabled is `unknown`, never `unhealthy`.
 * Declining auto-start is a legitimate configuration, and `swictation setup`
 * asks about it explicitly; painting that red would train users to ignore a
 * red doctor. `unknown` states the fact, prints how to enable it, and leaves
 * the exit code at 0.
 *
 * run() enables and starts the daemon under the npm lifecycle ONLY. Under
 * `swictation setup` the autostart prompt at the end of that command owns the
 * decision, and enabling here would make answering "No" meaningless — the
 * same failure mode amendment P1 fixed for the macOS LaunchAgents.
 *
 * Optional dependencies (wtype, xdotool, nc, hf) are inventory, never a
 * verdict: they are optional, the install has always merely listed them, and
 * a doctor that goes red because `hf` is absent is a doctor nobody reads.
 */

const fs = require('fs');
const { execFileSync } = require('child_process');
const { healthy, unknown, unhealthy, notApplicable, componentOk, componentFailed, componentSkipped } = require('./health');

/** Linux-only: macOS has no optional CLI dependencies to report. */
const OPTIONAL_TOOLS = [
  ['systemctl', 'systemd'],
  ['nc', 'netcat'],
  ['wtype', 'wtype (Wayland text injection)'],
  ['xdotool', 'xdotool (X11 text injection)'],
  ['hf', 'huggingface_hub[cli]'],
];

function onPath(tool) {
  try {
    execFileSync('which', [tool], { stdio: ['ignore', 'ignore', 'ignore'] });
    return true;
  } catch {
    return false;
  }
}

/** `systemctl --user is-enabled swictation-daemon`, or null when unaskable. */
function systemdEnabled() {
  try {
    const out = execFileSync('systemctl', ['--user', 'is-enabled', 'swictation-daemon'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    return out.trim() === 'enabled';
  } catch (err) {
    // `is-enabled` exits nonzero BOTH when the unit is disabled and when
    // systemd cannot be reached at all. "disabled" and "no user bus" are
    // different answers, and only stdout distinguishes them.
    const out = ((err.stdout || '') + '').trim();
    if (out === 'disabled' || out === 'static' || out === 'linked') return false;
    return null;
  }
}

/**
 * The RAW `systemctl --user is-active` state, or null when unaskable.
 *
 * A boolean collapsed `failed` into `inactive`, and those are different
 * reports: a daemon that has never been started is waiting, a daemon systemd
 * marks `failed` has crashed and is the single most useful thing a doctor can
 * say about a broken install.
 */
function systemdActiveState() {
  try {
    return execFileSync('systemctl', ['--user', 'is-active', 'swictation-daemon'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim() || null;
  } catch (err) {
    // is-active exits nonzero for every non-active state and prints the state.
    return ((err.stdout || '') + '').trim() || null;
  }
}

function optionalInventory(platform) {
  if (platform === 'darwin') return [];
  return OPTIONAL_TOOLS.map(([tool, label]) => `${onPath(tool) ? '✓' : '–'} ${tool} (${label})`);
}

/**
 * @param {function} [probeEnabled] - seam over `systemctl --user is-enabled`.
 * @param {function} [probeActive]  - seam over `systemctl --user is-active`.
 */
function inspectLinux(ctx, probeEnabled = systemdEnabled, probeActive = systemdActiveState) {
  const unitPath = ctx.systemdUnitPath;
  if (!unitPath || !fs.existsSync(unitPath)) {
    // services owns this failure and names it precisely. Reporting it here
    // too would print the same problem twice with two different repairs.
    return unknown('VERIFY_NO_UNIT', 'there is no daemon unit to enable yet', {
      evidence: [unitPath || 'target home could not be resolved'],
      repair: 'swictation setup --services',
    });
  }

  const enabled = probeEnabled();
  const evidence = [unitPath, ...optionalInventory(ctx.platform)];

  if (enabled === null) {
    return unknown('VERIFY_NO_SYSTEMD',
      'the systemd user bus did not answer, so autostart state is unknown', { evidence });
  }
  if (!enabled) {
    return unknown('VERIFY_NOT_ENABLED',
      'the daemon unit is installed but not enabled — it will not start on login', {
        evidence,
        repair: 'systemctl --user enable --now swictation-daemon',
      });
  }

  // "Will start on login" and "is running right now" are different claims,
  // and only the second one is what the user came to find out. Reporting
  // VERIFY_OK for an enabled-but-dead daemon put the one fact that mattered
  // into the evidence list, under a green row nobody reads — and an enabled
  // unit whose daemon exits at startup (a missing model, a bad ORT path) is
  // the single most common shape of "installed but nothing happens".
  const active = probeActive();
  if (active === null) {
    return unknown('VERIFY_ACTIVE_UNKNOWN',
      'the daemon unit is enabled, but systemd did not report whether it is running', { evidence });
  }
  if (active === 'failed') {
    return unhealthy('VERIFY_DAEMON_FAILED',
      'the daemon unit is enabled and systemd reports it as failed', {
        evidence: [...evidence, 'journalctl --user -u swictation-daemon -n 50 shows why'],
        repair: 'swictation doctor --deep',
      });
  }
  if (active !== 'active') {
    return unknown('VERIFY_NOT_RUNNING',
      `the daemon unit is enabled but the daemon is not running (${active})`, {
        evidence: [...evidence, 'it will start on login; starting it now shows any startup error'],
        repair: 'systemctl --user start swictation-daemon',
      });
  }

  evidence.push('daemon: running');
  return healthy('VERIFY_OK', 'the daemon unit is enabled and the daemon is running', { evidence });
}

/** @param {function} [probeState] - seam over the live launchctl query. */
function inspectDarwin(ctx, probeState) {
  const plistPath = ctx.daemonPlistPath;
  if (!plistPath || !fs.existsSync(plistPath)) {
    return unknown('VERIFY_NO_UNIT', 'there is no daemon LaunchAgent to load yet', {
      evidence: [plistPath || 'target home could not be resolved'],
      repair: 'swictation setup --services',
    });
  }

  const state = (probeState || require('../../postinstall').launchdServiceState)();
  const evidence = [plistPath, ...optionalInventory(ctx.platform)];

  if (!state.daemon.loaded) {
    return unknown('VERIFY_NOT_LOADED',
      'the daemon LaunchAgent is installed but not loaded — it will not start on login', {
        evidence,
        repair: `launchctl bootstrap gui/$(id -u) "${plistPath}"`,
      });
  }
  evidence.push(state.ui.loaded ? 'ui agent: loaded' : 'ui agent: not loaded (optional)');

  // The same distinction the systemd branch makes: bootstrapped is not
  // running. `launchctl print` answers 0 for a job that exists and has no
  // process, and a daemon that exits at startup looks exactly like this.
  if (!state.daemon.running) {
    return unknown('VERIFY_NOT_RUNNING',
      'the daemon LaunchAgent is loaded but the daemon is not running', {
        evidence: [...evidence, 'it will start on login; starting it now shows any startup error'],
        repair: 'swictation start',
      });
  }

  return healthy('VERIFY_OK', 'the daemon LaunchAgent is loaded and the daemon is running', { evidence });
}

module.exports = {
  id: 'verify',
  title: 'Verifying installation...',
  entrypoints: ['postinstall', 'setup'],
  after: ['services'],
  needsSession: true,
  forbidRoot: true,

  applies(ctx) {
    return ctx.platform === 'linux' || ctx.platform === 'darwin';
  },

  check(ctx) {
    if (ctx.platform === 'darwin') return inspectDarwin(ctx);
    if (ctx.platform === 'linux') return inspectLinux(ctx);
    return notApplicable('VERIFY_NOT_APPLICABLE', `no service model for ${ctx.platform}`);
  },

  async run(ctx) {
    const postinstall = require('../../postinstall');
    const components = [];

    if (ctx.platform === 'linux') {
      if (ctx.mode === 'setup') {
        // The autostart prompt at the end of `swictation setup` owns this
        // choice. Enabling here would answer it before it is asked.
        components.push(componentSkipped('autostart',
          'left to the auto-start prompt at the end of setup'));
      } else {
        try {
          const result = await postinstall.enableAndStartService();
          components.push(result.enabled
            ? componentOk('autostart', result.started ? 'enabled and started' : 'enabled')
            : componentFailed('autostart', 'systemctl --user enable did not succeed'));
        } catch (err) {
          components.push(componentFailed('autostart', err.message, err));
        }
      }
    } else {
      const state = postinstall.launchdServiceState();
      components.push(state.daemon.loaded
        ? componentOk('autostart',
          state.daemon.running ? 'daemon LaunchAgent is loaded and running' : 'daemon LaunchAgent is loaded')
        : componentSkipped('autostart', 'daemon LaunchAgent is not loaded'));
    }

    // Inventory, printed for the user, never a verdict — see the fence.
    try {
      postinstall.checkDependencies();
      components.push(componentOk('optional-dependencies', 'inventory reported'));
    } catch (err) {
      components.push(componentFailed('optional-dependencies', err.message, err));
    }

    return { changed: ctx.mode !== 'setup' && ctx.platform === 'linux', components, warnings: [] };
  },

  _internals: {
    OPTIONAL_TOOLS,
    onPath,
    systemdEnabled,
    systemdActiveState,
    optionalInventory,
    inspectLinux,
    inspectDarwin,
  },
};
