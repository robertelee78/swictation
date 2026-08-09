/**
 * Platform-package binaries step (ADR-037 Phase B).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * The daemon and UI do not ship in the `swictation` package; they come from
 * an optionalDependency (@agidreams/linux-x64, @agidreams/darwin-arm64) that
 * npm silently skips more often than anyone expects — a mirror that does not
 * carry it, `--no-optional`, a platform npm guessed wrong. postinstall's
 * answer was to shell out to `npm install -g` mid-lifecycle and repair it.
 *
 * That repair is deliberately POSTINSTALL-ONLY, and the distinction is the
 * whole reason this is its own step. Inside `npm install` we are already the
 * npm lifecycle: another `npm install -g` is a nested invocation of the tool
 * that is currently running, which npm tolerates because it is the documented
 * escape hatch. From `swictation setup` — a plain CLI a user may have run
 * under any shell, in any directory, possibly while another npm holds the
 * global lock — spawning a global install is a different and much worse act:
 * it can deadlock on that lock, rewrite a global tree the user did not ask us
 * to touch, and it needs write access to a prefix `setup` has no business
 * assuming. So `setup` reports the problem and names the command instead.
 *
 * The check is not "the file exists". A binary without the execute bit is the
 * classic tarball-permissions failure: present, correct, and unrunnable — and
 * an existence check calls that healthy while every launch fails with EACCES.
 * Nor is it "the file is executable": exists + non-empty + mode says nothing
 * about whether this kernel can LOAD it. An x86_64 daemon on Apple Silicon
 * satisfies all three and then dies with ENOEXEC on every launch, which no
 * user can distinguish from "it hangs". So check() reads the 20-byte
 * executable header and compares the architecture it declares against the one
 * we are running on — no spawn, no timeout, no output to parse. Actually
 * EXECUTING the thing is `doctor --deep`'s job.
 */

const fs = require('fs');
const path = require('path');
const { healthy, unhealthy, unknown, componentOk, componentFailed, componentSkipped } = require('./health');

/** node's `process.arch` → the CPU each executable format names it by. */
const MACHO_CPU = { arm64: 0x0100000c, x64: 0x01000007 };
const ELF_MACHINE = { arm64: 183 /* EM_AARCH64 */, x64: 62 /* EM_X86_64 */ };
const MACHO_CPU_NAMES = { 0x0100000c: 'arm64', 0x01000007: 'x86_64' };
const ELF_MACHINE_NAMES = { 183: 'arm64', 62: 'x86_64', 3: 'i386', 40: 'arm' };

/**
 * The architecture an executable declares, or null when the format is not one
 * we recognize (a shell-script wrapper is a legitimate, unreadable-by-magic
 * case — hence null rather than a failure).
 *
 * Reads 20 bytes. Mach-O keeps `cputype` at offset 4; ELF keeps `e_machine`
 * at offset 18. Universal ("fat") Mach-O binaries list several, so any
 * matching slice counts.
 *
 * @returns {{arch: string|null, format: string, all?: string[]}|null}
 */
function declaredArchitecture(filePath) {
  let header = Buffer.alloc(4096);
  let read = 0;
  let fd;
  try {
    fd = fs.openSync(filePath, 'r');
    read = fs.readSync(fd, header, 0, header.length, 0);
  } catch {
    return null;
  } finally {
    if (fd !== undefined) {
      try { fs.closeSync(fd); } catch { /* already gone */ }
    }
  }
  if (read < 20) return null;
  header = header.subarray(0, read);

  const le = header.readUInt32LE(0);
  const be = header.readUInt32BE(0);

  // Mach-O 64 (0xfeedfacf) and 32 (0xfeedface), either byte order.
  if (le === 0xfeedfacf || le === 0xfeedface) {
    const cpu = header.readInt32LE(4);
    return { format: 'mach-o', arch: MACHO_CPU_NAMES[cpu] || `cputype ${cpu}` };
  }
  if (be === 0xfeedfacf || be === 0xfeedface) {
    const cpu = header.readInt32BE(4);
    return { format: 'mach-o', arch: MACHO_CPU_NAMES[cpu] || `cputype ${cpu}` };
  }

  // Universal binary: a count, then that many 20-byte arch descriptors.
  if (be === 0xcafebabe) {
    const count = Math.min(header.readUInt32BE(4), 32);
    const all = [];
    for (let i = 0; i < count; i++) {
      const offset = 8 + i * 20;
      if (offset + 4 > read) break;
      const cpu = header.readInt32BE(offset);
      all.push(MACHO_CPU_NAMES[cpu] || `cputype ${cpu}`);
    }
    return all.length > 0 ? { format: 'universal', arch: all[0], all } : null;
  }

  if (header[0] === 0x7f && header[1] === 0x45 && header[2] === 0x4c && header[3] === 0x46) {
    const machine = header[5] === 2 ? header.readUInt16BE(18) : header.readUInt16LE(18);
    return { format: 'elf', arch: ELF_MACHINE_NAMES[machine] || `e_machine ${machine}` };
  }

  return null;
}

/** Whether `declared` covers the architecture this process is running as. */
function architectureMatches(declared, nodeArch) {
  if (!declared) return null;
  const wanted = MACHO_CPU_NAMES[MACHO_CPU[nodeArch]] || ELF_MACHINE_NAMES[ELF_MACHINE[nodeArch]] || nodeArch;
  const offered = declared.all || [declared.arch];
  return offered.includes(wanted);
}

/** A file that exists, is regular, non-empty, and carries an execute bit. */
function isExecutable(filePath) {
  try {
    const stat = fs.statSync(filePath);
    return stat.isFile() && stat.size > 0 && (stat.mode & 0o111) !== 0;
  } catch {
    return false;
  }
}

/** Why `filePath` is not a runnable binary, or null when it is one. */
function binaryProblem(label, filePath) {
  if (!filePath) return `${label}: no path resolved`;
  let stat;
  try {
    stat = fs.statSync(filePath);
  } catch {
    return `${label}: missing (${filePath})`;
  }
  if (!stat.isFile()) return `${label}: not a regular file (${filePath})`;
  if (stat.size === 0) return `${label}: zero bytes (${filePath})`;
  if ((stat.mode & 0o111) === 0) return `${label}: not executable (${filePath})`;
  return null;
}

/** The CLI wrapper this package ships itself. */
function cliPath() {
  return path.join(__dirname, '..', '..', 'bin', 'swictation');
}

/** Resolve the platform package from disk, or null when it is not installed. */
function resolveFromDisk() {
  try {
    return require('../resolve-binary').resolveBinaryPaths();
  } catch {
    return null;
  }
}

/**
 * @param {object} ctx
 * @param {function} [resolve] - seam over disk resolution.
 *
 * This is the one step that reads the platform package from DISK rather than
 * from `ctx.binaryPaths`, and deliberately so. Everywhere else the context
 * snapshot is the right answer — it stops services.run() from writing a unit
 * against a different package than the one the install verified. Here the
 * package IS the artifact under test, and run() can install it: a check that
 * consulted a snapshot taken before run() would report the step failed
 * immediately after it succeeded. Reading state from a memory of what was
 * true earlier is the exact mistake ADR-035 is about, pointed the other way.
 *
 * Which is why there is NO fallback to `ctx.binaryPaths` when disk says the
 * package is gone. Falling back would resurrect the same bug in the same
 * function: an upgrade that removes the platform package mid-install leaves
 * the snapshot pointing at paths that no longer exist, and a check consulting
 * it reports a healthy install over an empty directory.
 */
function inspect(ctx, resolve = resolveFromDisk) {
  const binaryPaths = resolve();

  if (!binaryPaths) {
    return unhealthy('BINARIES_NO_PLATFORM_PKG',
      'the platform package that carries the daemon and UI is not installed', {
        evidence: [
          `expected a package for ${ctx.platform}-${ctx.arch}`,
          'npm skips optionalDependencies silently — --no-optional, a mirror without',
          'the package, or a platform mismatch all produce exactly this state',
        ],
        repair: 'npm install -g swictation --force',
      });
  }

  const problems = [];
  const daemonProblem = binaryProblem('daemon', binaryPaths.daemon);
  if (daemonProblem) problems.push(daemonProblem);

  // The CLI wrapper is ours; if it lost its execute bit nothing the user
  // types works, including the repair command doctor is about to print.
  const cliProblem = binaryProblem('swictation CLI', cliPath());
  if (cliProblem) problems.push(cliProblem);

  if (problems.length > 0) {
    return unhealthy('BINARIES_UNUSABLE', problems[0], {
      evidence: [...problems, `package: ${binaryPaths.packageName || 'unknown'}`],
    });
  }

  // The UI is genuinely optional — a headless Linux box has no tray — so its
  // absence is evidence, never a failure.
  const evidence = [
    `package: ${binaryPaths.packageName}`,
    `daemon: ${binaryPaths.daemon}`,
  ];
  if (isExecutable(binaryPaths.ui)) {
    evidence.push(`ui: ${binaryPaths.ui}`);
  } else {
    evidence.push('ui: not installed (optional — the daemon runs without a tray)');
  }

  // Loadability, as far as a header read can establish it.
  const declared = declaredArchitecture(binaryPaths.daemon);
  if (!declared) {
    return unknown('BINARIES_FORMAT_UNKNOWN',
      'the daemon is not in a recognized executable format', {
        evidence: [...evidence,
          'neither Mach-O nor ELF — a wrapper script is legitimate, a corrupt download is not',
          'run "swictation doctor --deep" to actually execute it'],
      });
  }
  const matches = architectureMatches(declared, ctx.arch);
  if (matches === false) {
    return unhealthy('BINARIES_WRONG_ARCH',
      `the daemon is built for ${declared.all ? declared.all.join('/') : declared.arch}, not ${ctx.arch}`, {
        evidence: [...evidence,
          `format: ${declared.format}`,
          'every launch fails with ENOEXEC, which looks exactly like a hang'],
        repair: 'npm install -g swictation --force',
      });
  }
  evidence.push(`architecture: ${declared.arch} (${declared.format})`);

  return healthy('BINARIES_OK', 'daemon and CLI are present, executable, and built for this machine', { evidence });
}

/**
 * Actually run the daemon — `doctor --deep` only.
 *
 * The header read proves the kernel would accept the image. It cannot prove
 * the dynamic linker will resolve every symbol: a daemon built against a
 * newer GLIBC, or one whose ONNX Runtime is missing, loads and dies. Spawning
 * `--version` is the cheapest question that exercises the whole chain, and it
 * is bounded hard because a hung daemon must not hang doctor.
 */
async function inspectDeep(ctx, spawnProbe, resolve = resolveFromDisk) {
  const shallow = inspect(ctx, resolve);
  if (shallow.code !== 'BINARIES_OK' && shallow.code !== 'BINARIES_FORMAT_UNKNOWN') return shallow;

  const binaryPaths = resolve();
  if (!binaryPaths) return shallow;

  const probe = spawnProbe || defaultVersionProbe;
  const result = await probe(binaryPaths.daemon);

  if (result.status === 'timeout') {
    return unknown('BINARIES_PROBE_TIMEOUT',
      'the daemon did not answer --version within the probe timeout', {
        evidence: [binaryPaths.daemon, 'it may be waiting on a device, a socket, or a model'],
      });
  }
  if (result.status === 'failed') {
    return unhealthy('BINARIES_NOT_LOADABLE',
      `the daemon could not be executed: ${result.reason}`, {
        evidence: [binaryPaths.daemon,
          'the file is present and correctly shaped, but this machine cannot run it'],
        repair: 'npm install -g swictation --force',
      });
  }
  return healthy('BINARIES_RUNS', 'the daemon executes and reports its version', {
    evidence: [binaryPaths.daemon, ...(result.output ? [result.output.trim().slice(0, 120)] : [])],
  });
}

/** Spawn `<daemon> --version`, bounded. Never throws. */
function defaultVersionProbe(daemon, timeout = 5000) {
  const { spawnSync } = require('child_process');
  const result = spawnSync(daemon, ['--version'], { encoding: 'utf8', timeout, windowsHide: true });
  if (result.error && result.error.code === 'ETIMEDOUT') return { status: 'timeout' };
  if (result.error) return { status: 'failed', reason: result.error.code || result.error.message };
  if (result.status !== 0) {
    const stderr = (result.stderr || '').trim().split('\n')[0] || `exit ${result.status}`;
    return { status: 'failed', reason: stderr.slice(0, 120) };
  }
  return { status: 'ok', output: result.stdout || result.stderr || '' };
}

module.exports = {
  id: 'binaries',
  title: 'Verifying platform binaries...',
  // Runs from both entry points; only the npm-lifecycle REPAIR is gated,
  // because `setup` on a machine with no platform package must still be able
  // to say so, fix permissions, and name the command that fixes the rest.
  entrypoints: ['postinstall', 'setup'],
  // After cleanup, not merely after platform: run() can reinstall the platform
  // package, which overwrites the daemon executable in place. Cleanup is what
  // stops the running services first. The edge is soft, so `setup` — where
  // cleanup never runs — simply follows platform.
  after: ['platform', 'cleanup'],
  needsNetwork: true,

  applies(ctx) {
    return ctx.platform === 'linux' || ctx.platform === 'darwin';
  },

  check(ctx) {
    return inspect(ctx);
  },

  deepCheck(ctx) {
    return inspectDeep(ctx);
  },

  run(ctx) {
    const postinstall = require('../../postinstall');
    const components = [];
    const warnings = [];
    let changed = false;

    const { isPlatformPackageInstalled } = require('../resolve-binary');
    let installed;
    try {
      installed = isPlatformPackageInstalled();
    } catch {
      installed = false;
    }

    if (!installed) {
      if (ctx.mode === 'postinstall') {
        const result = installPlatformPackage(ctx);
        components.push(result.component);
        changed = changed || result.changed;
      } else {
        // See the fence: a global npm install from a user-invoked CLI is a
        // different act than one inside the npm lifecycle, and not ours to
        // perform unasked.
        components.push(componentSkipped('platform-package',
          'not installed — reinstall the package to fetch it'));
        warnings.push(
          'The platform binaries are missing. Run: npm install -g swictation --force'
        );
      }
    } else {
      components.push(componentOk('platform-package', 'already installed'));
    }

    try {
      postinstall.ensureBinaryPermissions();
      components.push(componentOk('permissions', 'execute bits asserted'));
    } catch (err) {
      components.push(componentFailed('permissions', err.message, err));
    }

    // Logged on success too, deliberately. The runner prints a step's evidence
    // only when something is wrong, but "which platform package, and where" is
    // the first question every binary bug report has to answer — and
    // install.log is what users attach. A green install that records nothing
    // about its own binaries is a support dead end.
    const resolved = resolveFromDisk();
    if (resolved) {
      ctx.log('cyan', `  Platform package: ${resolved.packageName}`);
      ctx.log('cyan', `  Binaries: ${resolved.binDir}`);
      ctx.log('cyan', `  Libraries: ${resolved.libDir}`);
    }

    return { changed, components, warnings };
  },

  _internals: {
    isExecutable,
    binaryProblem,
    cliPath,
    inspect,
    inspectDeep,
    resolveFromDisk,
    declaredArchitecture,
    architectureMatches,
    defaultVersionProbe,
  },
};

/**
 * The npm-lifecycle-only repair. Kept out of run()'s body so the gate above
 * reads as one decision rather than a nested branch inside a loop of them.
 */
function installPlatformPackage(ctx) {
  const postinstall = require('../../postinstall');
  const { detectPlatform } = require('../resolve-binary');

  let platformInfo;
  try {
    platformInfo = detectPlatform();
  } catch (err) {
    return { changed: false, component: componentFailed('platform-package', err.message, err) };
  }

  if (platformInfo.supported === false) {
    return {
      changed: false,
      component: componentFailed('platform-package',
        `unsupported platform: ${platformInfo.platform}-${platformInfo.arch}`),
    };
  }

  const packageName = platformInfo.packageName;
  const version = require('../../package.json').version;
  ctx.log('yellow', '\n⚠ Platform package not installed automatically');
  ctx.log('cyan', `   Installing ${packageName}@${version}...`);

  try {
    require('child_process').execSync(`npm install -g ${packageName}@${version}`, {
      stdio: 'inherit',
      encoding: 'utf8',
    });
  } catch (err) {
    return {
      changed: false,
      component: componentFailed('platform-package',
        `npm install -g ${packageName} failed: ${err.message}`, err),
    };
  }

  // resolve-binary caches nothing across a require, but postinstall may
  // already have loaded it with the package absent.
  try {
    delete require.cache[require.resolve('../resolve-binary')];
  } catch {
    /* not cached */
  }
  if (!require('../resolve-binary').isPlatformPackageInstalled()) {
    return {
      changed: true,
      component: componentFailed('platform-package',
        'npm reported success but the package is still not resolvable'),
    };
  }
  postinstall.log('green', `✓ Successfully installed ${packageName}`);
  return { changed: true, component: componentOk('platform-package', `installed ${packageName}`) };
}
