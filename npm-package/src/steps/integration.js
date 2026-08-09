/**
 * Desktop-integration step (ADR-037 Phase B) — text injection, audio, and
 * the NVIDIA hibernation advisory.
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * This step is the reason `needsSession` exists in the contract. Everything
 * it decides is read out of the SESSION: WAYLAND_DISPLAY, XDG_SESSION_TYPE,
 * XDG_CURRENT_DESKTOP, SWAYSOCK. Under `sudo npm install -g`, those are
 * root's — which usually means empty — so the legacy phase concluded "X11,
 * unknown desktop" on a GNOME Wayland laptop, installed xdotool instead of
 * ydotool, and left the user with an install that could not type a single
 * character. It then wrote the GNOME shortcut into root's dconf, where
 * nothing would ever read it. The spike found this by inspection; it is the
 * case `needsSession` turns into an honest `blocked` with a repair command
 * instead of a confidently wrong install.
 *
 * The verdict covers exactly what run() can repair: the text-injection tool
 * this session needs, and pipewire. Two things deliberately do NOT drive it:
 *
 *   NVIDIA hibernation — real, checkable, and NOT repairable without root.
 *     A verdict of `unhealthy` would mark the step permanently failed on
 *     every affected laptop, because run() would return and the check would
 *     come back red forever. `unknown` is the honest state for a condition
 *     the step can see but cannot establish: it prints the evidence and the
 *     `sudo swictation setup` repair, and it does not turn doctor red over an
 *     advisory that the installer has always treated as advisory.
 *
 *   GNOME shortcut registration — run() configures it, but reading it back
 *     needs a session dbus that may not exist where check() runs (a doctor
 *     over ssh). A check that shells into gsettings and treats every failure
 *     as "not configured" would report a broken hotkey on a machine whose
 *     hotkey works. It stays a typed run() component instead.
 */

const { execFileSync } = require('child_process');
const { healthy, unknown, unhealthy, componentOk, componentFailed, componentSkipped } = require('./health');

/** Is `tool` on PATH? The one probe check() is allowed — it is read-only. */
function onPath(tool) {
  try {
    execFileSync('which', [tool], { stdio: ['ignore', 'ignore', 'ignore'] });
    return true;
  } catch {
    return false;
  }
}

/**
 * The session's display stack, read from the context's env rather than
 * process.env so a caller can describe a session it is not itself inside.
 */
function describeSession(env) {
  const desktop = (env.XDG_CURRENT_DESKTOP || '').toLowerCase();
  const wayland = !!env.WAYLAND_DISPLAY || env.XDG_SESSION_TYPE === 'wayland';
  const sway = !!env.SWAYSOCK || desktop.includes('sway');
  const gnome = desktop.includes('gnome');
  const kde = desktop.includes('kde');

  // Whether a graphical session was DESCRIBED at all — distinct from which
  // one it is. With none of these set there is no session here: ssh without
  // X forwarding, a cron job, a serial console, doctor from a tmux over a
  // remote shell. The `x11` default below is a fallback for "not Wayland",
  // and reading it as a positive finding is how an empty environment came to
  // look like a bare X11 desktop that needed xdotool.
  const described = !!(env.WAYLAND_DISPLAY || env.DISPLAY || env.XDG_SESSION_TYPE
    || env.XDG_CURRENT_DESKTOP || env.SWAYSOCK);

  return {
    described,
    displayServer: wayland ? 'wayland' : 'x11',
    desktop: gnome ? 'gnome' : sway ? 'sway' : kde ? 'kde' : (desktop || 'unknown'),
    wayland,
    gnome,
    sway,
    kde,
  };
}

/**
 * The injection tool this session needs, mirroring setupWaylandIntegration()'s
 * own branching so the check never asks for something the repair will not
 * install.
 */
function requiredInjector(session) {
  if (!session.wayland) return 'xdotool';
  if (session.gnome) return 'ydotool';
  return 'wtype';
}

function hibernationStatus() {
  try {
    return require('../nvidia-hibernation-setup').checkNvidiaHibernationStatus();
  } catch {
    return null;
  }
}

/**
 * @param {object} ctx
 * @param {object} [probe] - seam over the two things that read the machine
 *   rather than the context, so tests can drive every branch on any host.
 */
function inspectLinux(ctx, probe = { onPath, hibernation: hibernationStatus }) {
  const session = describeSession(ctx.env || process.env);
  const onPathFn = probe.onPath || onPath;

  // No session described: every verdict below would be about a desktop that
  // was never observed. `unknown` is the only honest answer — the absence of
  // evidence is not evidence of X11.
  if (!session.described) {
    return unknown('INTEGRATION_NO_SESSION',
      'no graphical session is described here, so the right text-injection tool is unknown', {
        evidence: [
          'none of WAYLAND_DISPLAY, DISPLAY, XDG_SESSION_TYPE, XDG_CURRENT_DESKTOP, SWAYSOCK is set',
          'expected over ssh, from cron, or on a console — run this inside the desktop session',
        ],
        repair: 'run "swictation doctor" from a terminal inside your desktop session',
      });
  }

  const injector = requiredInjector(session);
  const evidence = [
    `display server: ${session.displayServer}`,
    `desktop: ${session.desktop}`,
  ];

  if (!onPathFn(injector)) {
    return unhealthy('INTEGRATION_NO_INJECTOR',
      `${injector} is not installed — this session cannot inject text`, {
        evidence: [...evidence, `${session.displayServer} + ${session.desktop} needs ${injector}`],
      });
  }
  evidence.push(`text injection: ${injector}`);

  if (!onPathFn('pipewire')) {
    return unhealthy('INTEGRATION_NO_AUDIO',
      'pipewire is not installed — audio capture will not work', { evidence });
  }
  evidence.push('audio: pipewire');

  const hibernation = (probe.hibernation || hibernationStatus)();
  if (hibernation && hibernation.needsConfiguration) {
    // See the fence: visible, unrepairable without root, and not a reason to
    // mark the step failed forever.
    return unknown('INTEGRATION_HIBERNATION',
      'NVIDIA hibernation support is not configured on this laptop', {
        evidence: [
          ...evidence,
          `distribution: ${hibernation.distribution}`,
          'without NVreg_PreserveVideoMemoryAllocations=1 the GPU can enter a',
          'defunct state after hibernation (CUDA 719/999) until the next reboot',
        ],
        repair: 'sudo swictation setup',
      });
  }
  if (hibernation && hibernation.isLaptop && hibernation.hasNvidiaGpu) {
    evidence.push('nvidia hibernation: configured');
  }

  // The claim is exactly what was proven and no more. `which ydotool` shows a
  // file exists; whether a keystroke ever lands also depends on ydotoold
  // running and /dev/uinput being writable by this user, neither of which is
  // observable from here. Saying "installed for this session" invited the
  // reader to conclude injection works, and the commonest ydotool failure is
  // precisely an installed binary with no daemon behind it.
  evidence.push('runtime liveness unverified: ydotool needs ydotoold running and');
  evidence.push('  /dev/uinput writable; a present binary does not prove a keystroke lands');

  return healthy('INTEGRATION_TOOLS_PRESENT',
    `${injector} and pipewire are on PATH for this ${session.displayServer} session`, { evidence });
}

function inspectDarwin() {
  // Accessibility is a TCC grant. The database is SIP-protected and the only
  // supported query (AXIsProcessTrusted) answers for the CALLING process —
  // node, not the daemon — so nothing readable from here describes the
  // daemon's grant. `unknown` says that; `healthy` would be a guess.
  return unknown('INTEGRATION_TCC_UNKNOWN',
    'macOS Accessibility permission cannot be verified from outside the daemon', {
      evidence: [
        'System Settings → Privacy & Security → Accessibility must list swictation-daemon',
        'macOS prompts for it the first time the daemon injects text',
      ],
      repair: 'open System Settings → Privacy & Security → Accessibility',
    });
}

module.exports = {
  id: 'integration',
  title: 'Platform integration...',
  entrypoints: ['postinstall', 'setup'],
  after: ['services'],
  // The fence in one field: without the user's session this step reads
  // root's environment and configures the wrong desktop.
  needsSession: true,
  forbidRoot: true,

  applies(ctx) {
    return ctx.platform === 'linux' || ctx.platform === 'darwin';
  },

  check(ctx) {
    return ctx.platform === 'darwin' ? inspectDarwin(ctx) : inspectLinux(ctx);
  },

  async run(ctx) {
    const postinstall = require('../../postinstall');

    if (ctx.platform === 'darwin') {
      // No prose block here: the check()'s health summary + evidence + repair
      // already state exactly this (grant Accessibility in System Settings),
      // and the framework renders them. Printing it again from run() was the
      // double-print users saw under `setup --repair` (ADR-037 health round).
      return {
        changed: false,
        components: [componentSkipped('accessibility', 'granted by the user, not by the installer')],
        warnings: [],
      };
    }

    const components = [];
    const warnings = [];
    let results;
    try {
      results = await postinstall.setupWaylandIntegration();
    } catch (err) {
      return {
        changed: false,
        components: [componentFailed('desktop-integration', err.message, err)],
        warnings: [],
      };
    }

    const session = describeSession(ctx.env || process.env);
    const injector = requiredInjector(session);
    components.push(results.textInjectionTool
      ? componentOk('text-injection', `${results.textInjectionTool} available`)
      : componentFailed('text-injection', `${injector} could not be installed`));
    components.push(results.pipewireInstalled
      ? componentOk('audio', 'pipewire available')
      : componentFailed('audio', 'pipewire could not be installed'));
    if (session.gnome) {
      components.push(results.gnomeShortcuts
        ? componentOk('gnome-shortcuts', 'Super+Shift+D registered')
        : componentSkipped('gnome-shortcuts', 'not configured — set the hotkey manually'));
    }

    // The legacy phase's guidance block, not a paraphrase of it: it explains
    // what a defunct GPU after hibernation actually looks like (CUDA 719/999,
    // reboot required) and names the exact command. A laptop user losing CUDA
    // after every suspend needs that, not a one-line warning — and dropping it
    // was a silent regression of the migration, invisible because nothing
    // references a function that is merely no longer called.
    try {
      await postinstall.checkNvidiaHibernation();
    } catch (err) {
      components.push(componentFailed('nvidia-hibernation', err.message, err));
    }

    const hibernation = hibernationStatus();
    if (hibernation && hibernation.needsConfiguration) {
      // Reported, never performed: this needs root, and silently acquiring it
      // during an unattended `npm install` is not something an installer does.
      warnings.push('NVIDIA hibernation support is not configured — run: sudo swictation setup');
    }

    return { changed: true, components, warnings };
  },

  _internals: {
    onPath,
    describeSession,
    requiredInjector,
    hibernationStatus,
    inspectLinux,
    inspectDarwin,
  },
};
