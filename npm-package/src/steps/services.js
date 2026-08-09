/**
 * Service-unit step — systemd (Linux) / launchd (macOS). ADR-037.
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * The predicate is NOT "a unit file exists". ADR-034's failure mode was a
 * unit that existed and loaded perfectly while producing blank
 * transcriptions, because `swictation setup` had its own inline generator
 * that omitted ORT_DYLIB_PATH. The template marks that variable CRITICAL for
 * exactly this reason. So the check reads the installed unit and demands
 * that ORT_DYLIB_PATH be present AND point at a library that is on disk —
 * buildDaemonServiceUnit() writes the key even when it could not find the
 * runtime, so "key present" alone proves nothing.
 *
 * It also demands that the unit's ExecStart point at the binary THIS
 * context resolved. An npm upgrade moves the platform package, and a unit
 * left pointing at the previous version's path is a service that no longer
 * starts — invisible to any check that only greps for the ORT variable.
 *
 * macOS gets the same predicate through its own plumbing: the plist carries
 * ORT_DYLIB_PATH, and ProgramArguments points at a wrapper script rather
 * than the daemon, because SIP strips DYLD_* from launchd processes. A plist
 * whose wrapper an upgrade deleted is a service that cannot start at all.
 *
 * `after: ['gpu-libs']` is SOFT on purpose: the unit is written against
 * freshly installed CUDA libraries when they arrived, but a failed gpu-libs
 * step must still leave a CPU-capable unit behind rather than no unit at all.
 *
 * Generation is delegated to postinstall's generateSystemdService() /
 * generateLaunchdServices(), which own compositor detection, UI unit
 * selection, log rotation, launchctl bootstrapping and systemd reload.
 */

const fs = require('fs');
const { execFileSync } = require('child_process');
const path = require('path');
const { healthy, unhealthy, unknown, componentOk, componentFailed } = require('./health');

const PLIST_PLACEHOLDER_RE = /\{\{[A-Z_]+\}\}/;

/**
 * A library path only counts when it names a non-empty regular file.
 * An interrupted download leaves a zero-byte file that existsSync happily
 * accepts, and a zero-byte libonnxruntime is the blank-output failure with
 * the check agreeing that everything is fine.
 */
function isUsableLibrary(filePath) {
  try {
    const stat = fs.statSync(filePath);
    return stat.isFile() && stat.size > 0;
  } catch {
    return false;
  }
}

/** `Environment="ORT_DYLIB_PATH=/path/to/lib"` → the path, or null. */
function ortPathFromUnit(content) {
  const match = content.match(/^\s*Environment="ORT_DYLIB_PATH=([^"]*)"\s*$/m);
  return match ? match[1].trim() : null;
}

/** `ExecStart=/path/to/swictation-daemon` → the path, or null. */
function execStartFromUnit(content) {
  const match = content.match(/^\s*ExecStart=(.+)$/m);
  return match ? match[1].trim().split(/\s+/)[0] : null;
}

/** `<key>ORT_DYLIB_PATH</key><string>/path</string>` → the path, or null. */
function ortPathFromPlist(content) {
  const match = content.match(/<key>ORT_DYLIB_PATH<\/key>\s*<string>([^<]*)<\/string>/);
  return match ? match[1].trim() : null;
}

/** First `<string>` inside ProgramArguments → the executable, or null. */
function programFromPlist(content) {
  const block = content.match(/<key>ProgramArguments<\/key>\s*<array>([\s\S]*?)<\/array>/);
  if (!block) return null;
  const first = block[1].match(/<string>([^<]*)<\/string>/);
  return first ? first[1].trim() : null;
}

/** `systemctl --user is-enabled` — read-only, so check() may ask. */
function unitIsEnabled() {
  try {
    const out = execFileSync('systemctl', ['--user', 'is-enabled', 'swictation-daemon'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    return out.trim() === 'enabled';
  } catch {
    return false;
  }
}

function checkLinux(ctx) {
  const unitPath = ctx.systemdUnitPath;
  if (!unitPath) {
    return unknown('SERVICES_NO_TARGET_HOME',
      'cannot locate the target user home, so the unit path is unknown', {
        evidence: [`effective user: ${ctx.effectiveUser}`, `target user: ${ctx.targetUser}`],
      });
  }
  if (!fs.existsSync(unitPath)) {
    return unhealthy('SERVICES_UNIT_MISSING', 'no systemd unit installed', { evidence: [unitPath] });
  }

  let content;
  try {
    content = fs.readFileSync(unitPath, 'utf8');
  } catch (err) {
    return unhealthy('SERVICES_UNIT_UNREADABLE', `unit unreadable: ${err.message}`, {
      evidence: [unitPath],
    });
  }

  const ortPath = ortPathFromUnit(content);
  if (!ortPath) {
    return unhealthy('SERVICES_NO_ORT',
      'unit has no ORT_DYLIB_PATH — the ADR-034 blank-output failure mode', {
        evidence: [unitPath],
      });
  }
  if (!isUsableLibrary(ortPath)) {
    return unhealthy('SERVICES_ORT_MISSING',
      'ORT_DYLIB_PATH does not name a usable library file', {
        evidence: [unitPath, `ORT_DYLIB_PATH=${ortPath}`],
      });
  }

  const execStart = execStartFromUnit(content);
  if (!execStart || !fs.existsSync(execStart)) {
    return unhealthy('SERVICES_EXEC_MISSING', 'unit ExecStart points at a missing binary', {
      evidence: [unitPath, `ExecStart=${execStart || '(none)'}`],
    });
  }
  const expectedExec = ctx.binaryPaths && ctx.binaryPaths.daemon;
  if (expectedExec && path.resolve(execStart) !== path.resolve(expectedExec)) {
    return unhealthy('SERVICES_EXEC_STALE',
      'unit points at a different daemon binary than this install resolves', {
        evidence: [`unit: ${execStart}`, `resolved: ${expectedExec}`],
      });
  }

  // Enablement is reported as evidence, not as a failure: a correct unit
  // that the user chose not to auto-start is a legitimate configuration,
  // and `setup`'s autostart prompt is where that choice belongs.
  const evidence = [unitPath, `ORT_DYLIB_PATH=${ortPath}`];
  if (!unitIsEnabled()) {
    evidence.push('note: unit is not enabled — it will not start on login (systemctl --user enable swictation-daemon)');
  }
  return healthy('SERVICES_OK', 'systemd unit present with a working ONNX Runtime path', { evidence });
}

function checkDarwin(ctx) {
  const plistPath = ctx.daemonPlistPath;
  if (!plistPath) {
    return unknown('SERVICES_NO_TARGET_HOME',
      'cannot locate the target user home, so the plist path is unknown', {
        evidence: [`effective user: ${ctx.effectiveUser}`, `target user: ${ctx.targetUser}`],
      });
  }
  if (!fs.existsSync(plistPath)) {
    return unhealthy('SERVICES_PLIST_MISSING', 'no LaunchAgent plist installed', {
      evidence: [plistPath],
    });
  }

  let content;
  try {
    content = fs.readFileSync(plistPath, 'utf8');
  } catch (err) {
    return unhealthy('SERVICES_PLIST_UNREADABLE', `plist unreadable: ${err.message}`, {
      evidence: [plistPath],
    });
  }

  if (PLIST_PLACEHOLDER_RE.test(content)) {
    return unhealthy('SERVICES_PLIST_TEMPLATE',
      'plist still contains unsubstituted {{PLACEHOLDER}} values', { evidence: [plistPath] });
  }

  const program = programFromPlist(content);
  if (!program) {
    return unhealthy('SERVICES_NO_PROGRAM', 'plist has no ProgramArguments entry', {
      evidence: [plistPath],
    });
  }
  if (!fs.existsSync(program)) {
    // The wrapper, not the daemon: SIP strips DYLD_* from launchd, so the
    // wrapper is what sets the library environment. Without it nothing runs.
    return unhealthy('SERVICES_LAUNCHER_MISSING', 'the daemon launcher wrapper is missing', {
      evidence: [plistPath, `ProgramArguments[0]=${program}`],
    });
  }

  const ortPath = ortPathFromPlist(content);
  if (!ortPath) {
    return unhealthy('SERVICES_NO_ORT',
      'plist has no ORT_DYLIB_PATH — the ADR-034 blank-output failure mode', {
        evidence: [plistPath],
      });
  }
  if (!isUsableLibrary(ortPath)) {
    return unhealthy('SERVICES_ORT_MISSING',
      'ORT_DYLIB_PATH does not name a usable library file', {
        evidence: [plistPath, `ORT_DYLIB_PATH=${ortPath}`],
      });
  }

  return healthy('SERVICES_OK', 'LaunchAgent plist present with a working ONNX Runtime path', {
    evidence: [plistPath, `ORT_DYLIB_PATH=${ortPath}`],
  });
}

module.exports = {
  id: 'services',
  // Reused verbatim as postinstall's phase banner — keep in sync with the plan.
  title: 'Configuring system services...',
  entrypoints: ['postinstall', 'setup'],
  after: ['gpu-libs'],
  needsSession: true,
  forbidRoot: true,

  applies(ctx) {
    return ctx.platform === 'linux' || ctx.platform === 'darwin';
  },

  check(ctx) {
    return ctx.platform === 'darwin' ? checkDarwin(ctx) : checkLinux(ctx);
  },

  run(ctx) {
    const postinstall = require('../../postinstall');
    const generator = ctx.platform === 'darwin' ? 'launchd' : 'systemd';
    try {
      if (ctx.platform === 'darwin') {
        // Interactive `swictation setup` asks "enable auto-start?" AFTER this
        // step. Bootstrapping the agents here would make answering No
        // meaningless, so generation and loading are separated and setup
        // loads only on consent. postinstall keeps loading inline.
        postinstall.generateLaunchdServices(ctx.ortLibPath || null, {
          load: ctx.mode !== 'setup',
        });
      } else {
        postinstall.generateSystemdService(ctx.ortLibPath || null);
      }
    } catch (err) {
      return {
        changed: false,
        components: [componentFailed(`${generator}-units`, err.message, err)],
        warnings: [],
      };
    }

    // The generators catch their own errors and return normally, so the
    // artifact check is the only honest evidence that anything happened.
    const after = ctx.platform === 'darwin' ? checkDarwin(ctx) : checkLinux(ctx);
    if (after.state !== 'healthy') {
      return {
        changed: true,
        components: [componentFailed(`${generator}-units`, after.summary)],
        warnings: [],
      };
    }
    return {
      changed: true,
      components: [componentOk(`${generator}-units`, after.summary)],
      warnings: [],
    };
  },

  // Exported for tests, which drive the predicates against fixture trees.
  _internals: {
    isUsableLibrary,
    unitIsEnabled,
    ortPathFromUnit,
    execStartFromUnit,
    ortPathFromPlist,
    programFromPlist,
    checkLinux,
    checkDarwin,
  },
};
