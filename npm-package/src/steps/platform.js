/**
 * Platform-compatibility step (ADR-037 Phase B).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * The legacy checkPlatform() answered this question by calling process.exit()
 * from three different branches. That is why it could never be reused: doctor
 * cannot diagnose an install by killing its own process, and `setup` on an
 * Intel Mac would take the whole CLI down before printing anything useful.
 * The verdict and the decision to stop are now separate — this step reports,
 * and postinstall's driver is the only place that decides an unsupported
 * machine ends the run (preserving the legacy exit codes exactly).
 *
 * Three tiers of "unsupported", and the distinction is load-bearing:
 *   wrong OS / wrong arch → nothing can make this machine work. `unhealthy`
 *      with a repair line that says so plainly rather than naming a command
 *      that cannot help. Not `not-applicable`: that means "nothing to fix,
 *      ever, and the install is fine without it" (gpu-libs on a Mac), which
 *      would paint a Windows box green.
 *   OS too old (GLIBC < 2.39, macOS < 14) → the binaries are built against
 *      newer symbols. The install proceeds — as it always has — but the
 *      daemon may not start, and a check that called that healthy would be
 *      lying about the most common "it installed but nothing works" report.
 *   directories missing → the only genuinely repairable failure here, and the
 *      reason run() exists at all.
 *
 * `unknown` when the version cannot be read: no `ldd`, no `sw_vers`. That is
 * not permission to assume the best.
 */

const fs = require('fs');
const { execFileSync } = require('child_process');
const { healthy, unhealthy, unknown, componentOk, componentFailed } = require('./health');

const MIN_GLIBC = { major: 2, minor: 39 };
const MIN_MACOS_MAJOR = 14;

/** Codes the driver treats as "this machine cannot run swictation at all". */
const FATAL_CODES = new Set(['PLATFORM_UNSUPPORTED_OS', 'PLATFORM_UNSUPPORTED_ARCH']);

/** `ldd --version` first line, or null when glibc cannot be interrogated. */
function readGlibcVersion() {
  try {
    const out = execFileSync('ldd', ['--version'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    const match = out.match(/GLIBC\s+(\d+)\.(\d+)/i) || out.match(/\)\s+(\d+)\.(\d+)\s*$/m);
    if (!match) return null;
    return { major: parseInt(match[1], 10), minor: parseInt(match[2], 10) };
  } catch {
    return null;
  }
}

/** `sw_vers -productVersion`, or null. */
function readMacosVersion() {
  try {
    return execFileSync('sw_vers', ['-productVersion'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return null;
  }
}

/** The directories the install writes into, derived from the TARGET home. */
function requiredDirs() {
  const paths = require('../paths');
  return [paths.getConfigDir(), paths.getDataDir(), paths.getModelsDir(), paths.getCacheDir()];
}

function missingDirs() {
  return requiredDirs().filter(dir => {
    try {
      return !fs.statSync(dir).isDirectory();
    } catch {
      return true;
    }
  });
}

/**
 * The platform verdict. Pure: no exits, no writes, no prompts.
 * `probe` is injected so tests can drive every branch without a VM per case.
 */
function inspect(ctx, probe = { glibc: readGlibcVersion, macos: readMacosVersion }) {
  const { platform, arch } = ctx;

  if (platform !== 'linux' && platform !== 'darwin') {
    return unhealthy('PLATFORM_UNSUPPORTED_OS', `swictation does not support ${platform}`, {
      evidence: [`detected: ${platform} ${arch}`, 'supported: linux-x64, darwin-arm64'],
      repair: 'no repair is possible on this operating system',
    });
  }

  const wantedArch = platform === 'darwin' ? 'arm64' : 'x64';
  if (arch !== wantedArch) {
    return unhealthy('PLATFORM_UNSUPPORTED_ARCH',
      `swictation on ${platform} requires ${wantedArch}, not ${arch}`, {
        evidence: platform === 'darwin'
          ? ['Apple Silicon (M1 or later) is required; Intel Macs are not supported']
          : [`linux builds are ${wantedArch} only`],
        repair: 'no repair is possible on this hardware',
      });
  }

  if (platform === 'linux') {
    const glibc = probe.glibc();
    if (!glibc) {
      return unknown('PLATFORM_GLIBC_UNKNOWN', 'could not determine the GLIBC version', {
        evidence: ['ldd --version did not report a version',
          `swictation binaries need GLIBC ${MIN_GLIBC.major}.${MIN_GLIBC.minor}+`],
      });
    }
    if (glibc.major < MIN_GLIBC.major
      || (glibc.major === MIN_GLIBC.major && glibc.minor < MIN_GLIBC.minor)) {
      return unhealthy('PLATFORM_GLIBC_OLD',
        `GLIBC ${glibc.major}.${glibc.minor} is older than the required ${MIN_GLIBC.major}.${MIN_GLIBC.minor}`, {
          evidence: [
            'the daemon and UI binaries will fail to load on this distribution',
            'supported: Ubuntu 24.04 LTS+, Debian 13+ (Trixie), Fedora 39+',
          ],
          repair: 'upgrade the distribution — no swictation command can fix a GLIBC mismatch',
        });
    }
  } else {
    const version = probe.macos();
    if (!version) {
      return unknown('PLATFORM_MACOS_UNKNOWN', 'could not determine the macOS version', {
        evidence: ['sw_vers -productVersion did not answer',
          `swictation requires macOS ${MIN_MACOS_MAJOR}.0 (Sonoma) or newer`],
      });
    }
    const major = parseInt(version.split('.')[0], 10);
    // `sw_vers` exiting 0 with something unparseable is NOT the same as
    // `sw_vers` failing, and the difference used to be invisible: NaN < 14 is
    // false, so a garbage version fell straight through the gate below and the
    // step vouched for an OS it had not read. Unreadable is `unknown`,
    // whichever way the reading failed.
    if (!Number.isFinite(major)) {
      return unknown('PLATFORM_MACOS_UNPARSEABLE',
        `could not parse the macOS version from "${version}"`, {
          evidence: ['sw_vers -productVersion returned something unrecognizable',
            `swictation requires macOS ${MIN_MACOS_MAJOR}.0 (Sonoma) or newer`],
        });
    }
    if (major < MIN_MACOS_MAJOR) {
      return unhealthy('PLATFORM_MACOS_OLD',
        `macOS ${version} is older than the required ${MIN_MACOS_MAJOR}.0 (Sonoma)`, {
          evidence: ['supported: macOS 14.x (Sonoma), 15.x (Sequoia)'],
          repair: 'upgrade macOS — no swictation command can fix an OS version mismatch',
        });
    }
  }

  // Everything above is about the machine. This is the one thing the step can
  // actually repair, which is why it is checked last: an unsupported box must
  // not be told to run `setup --platform`.
  const missing = missingDirs();
  if (missing.length > 0) {
    return unhealthy('PLATFORM_DIRS_MISSING',
      `${missing.length} of the ${requiredDirs().length} install directories do not exist`, {
        evidence: missing,
      });
  }

  return healthy('PLATFORM_OK', `${platform} ${arch} is supported and the install directories exist`, {
    evidence: requiredDirs(),
  });
}

module.exports = {
  id: 'platform',
  title: 'Checking platform compatibility...',
  entrypoints: ['postinstall', 'setup'],
  after: [],
  // Directory creation writes into the target home; under `sudo npm i -g`
  // that resolves to root's, and every later check would read the real
  // user's empty one.
  forbidRoot: true,

  applies() {
    // Deliberately always in scope. An unsupported OS is a RESULT this step
    // reports, not a reason to omit the row — a doctor table that silently
    // drops the platform line on Windows tells the reader nothing.
    return true;
  },

  check(ctx) {
    return inspect(ctx);
  },

  run(ctx) {
    const postinstall = require('../../postinstall');
    const verdict = inspect(ctx);

    // Creating directories on a machine the binaries cannot run on leaves
    // litter and implies progress that is not happening.
    if (FATAL_CODES.has(verdict.code)) {
      return {
        changed: false,
        components: [componentFailed('install-directories', verdict.summary)],
        warnings: [],
      };
    }

    try {
      postinstall.createDirectories();
    } catch (err) {
      return {
        changed: false,
        components: [componentFailed('install-directories', err.message, err)],
        warnings: [],
      };
    }

    const still = missingDirs();
    const warnings = [];
    if (verdict.code === 'PLATFORM_GLIBC_OLD' || verdict.code === 'PLATFORM_MACOS_OLD') {
      warnings.push(`${verdict.summary} — the binaries may not run on this system`);
    }
    if (still.length > 0) {
      return {
        changed: true,
        components: [componentFailed('install-directories', `still missing: ${still.join(', ')}`)],
        warnings,
      };
    }
    return {
      changed: true,
      components: [componentOk('install-directories', 'install directories present')],
      warnings,
    };
  },

  // Exported for tests and for the postinstall driver, which needs to know
  // which verdicts end the run.
  _internals: {
    FATAL_CODES,
    MIN_GLIBC,
    MIN_MACOS_MAJOR,
    inspect,
    missingDirs,
    requiredDirs,
    readGlibcVersion,
    readMacosVersion,
  },
};
