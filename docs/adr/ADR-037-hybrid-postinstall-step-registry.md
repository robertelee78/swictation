# ADR-037: Hybrid Postinstall — Idempotent Step Registry Shared by Install, Setup, Repair, and Doctor

- **Status:** Accepted
- **Date:** 2026-08-09
- **Issue:** The 3,300-line all-or-nothing postinstall is the structural cause of the
  v0.7.29–0.7.35 install-fix treadmill (ADR-034/035 audit; spike evidence below)
- **Release:** v0.7.37 target (Phase A may ride v0.7.36 if the GPU-box session lands first)
- **Supersedes:** the "no separate setup step" constraint at
  `docs/archive/prd-postinstall-resilience.md:29`

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Context (spike findings, 2026-08-08)

The fence was investigated before removal. The PRD line forbidding a setup step was a
scoping note in an AI-generated PRD committed in the same commit as its own
implementation (7484264); `swictation setup` predates it; no ADR/changelog/commit
anywhere records a rationale; and postinstall's own error handlers direct users to
`swictation setup` six times. The legitimate concern behind the wording — install must
not depend on users discovering a second command — is honored by the hybrid design.

Phase map and failure accounting live in the spike report (session 2026-08-08):
9 phases, 112 catch blocks (17 fully silent), `main()` never rethrows and exits 0 by
policy, `_totalPhases` is static while phase 5 is conditional, and a checksum-load
failure in the optional GPU phase currently aborts phases 5–9.

## Decision

1. **Install state is derived from disk, never from a memory of having run.** Every
   step exposes `check(ctx)` — a side-effect-free predicate computed from artifacts
   (files, hashes, unit contents) — and `run(ctx)`, idempotent. `check()` is the
   definition of truth; ADR-035's receipt-vs-goods GPU bug is the canonical
   counterexample this rule exists to prevent.
2. **One registry, `npm-package/src/steps/index.js`.** Step contract:
   `{ id, title, needs: {network, sudo, session, npmLifecycle}, check(ctx), run(ctx) }`.
   The runner executes an ordered list, continue-on-error, capturing
   `{id, status: ok|failed|skipped|already, error?}` per step; it never aborts the list.
3. **Both entry points walk the same registry.** postinstall runs the full list
   (one-command UX unchanged; phase counter derived from the filtered registry, fixing
   the static-counter bug). `swictation setup` runs the registry minus
   `npmLifecycle: true` steps; `setup --<id>` runs one; `setup --repair` runs only
   steps whose `check()` fails; `setup --list` prints ids; `swictation doctor` runs
   every `check()` and prints a health table with the repair command per failure.
4. **Capability flags gate execution honestly.** A step whose `needs` aren't met
   (no session env under sudo installs, no network) is `skipped` with its repair
   command printed — never run-and-misbehave (the sudo-writes-into-/root case).
5. **The summary box becomes a per-step ledger.** npm still exits 0 (a nonzero
   postinstall fails the whole `npm install` and leaves the package half-unpacked —
   the existing policy stands); failures are named, not prevented.
6. **Phasing.** Phase A (this ADR's execution): registry + runner + the steps repair
   needs most — `config` (exists, ADR-035), `services` (wraps generate-service.js,
   ADR-034, plus launchd path), `models` (wraps ADR-036 downloader), `gpu-libs`
   (artifact-checked, ADR-035/036) — wired into postinstall in place, plus the new
   `setup` flags and `doctor`. Phase B (follow-up): remaining phases (platform,
   binaries, cleanup, integration, verify) migrate; postinstall shrinks to a ~150-line
   driver. Existing behavior parity is the acceptance bar for Phase A; no phase
   changes semantics while moving.

## Amendment — 2026-08-09 (pre-implementation design review, gpt-5.6-sol)

The contract above is revised as follows before any code exists:

1. `check(ctx)` returns structured health, not a boolean:
   `{state: healthy|unhealthy|blocked|not-applicable|unknown, code, summary, evidence[], repair}`.
   `run()` returns `{changed, components[], warnings[]}` (a step can partially succeed).
2. `applies(ctx)` is separate from capability gating: CPU-only machines get
   `not-applicable` for gpu-libs, never `skipped`. `check()` runs BEFORE capability
   gating (offline + intact artifacts reports `already`, not "network unavailable").
3. `needs` is replaced by precise fields: `entrypoints: ['postinstall','setup']`,
   `forbidRoot`/`targetUser`, `needsNetwork`, `needsSession`; plus soft `after: []`
   ordering (services follows gpu-libs but still generates a CPU-capable unit when
   GPU repair failed).
4. One immutable `ctx` carries resolved facts (mode, platform/arch, effective→target
   user/home, binaryPaths, gpuInfo, selected model, ortLibPath, logger, clock);
   steps never rediscover these from globals — recomputation under sudo/setup picks
   different users and artifacts.
5. Config migrates as TWO ordered step ids (`config-reset` pre-download,
   `config-heal` post-download) — one check/run pair cannot preserve its two-pass
   semantics.
6. Incremental wiring builds ONE nine-slot execution plan (four registry entries +
   five legacy adapters in original order); migrated call sites removed atomically
   including the model retry hidden in showNextSteps(); wrapped helpers refactored
   to return typed component results instead of swallowing failures.
7. gpu-libs `check()` without a per-extracted-file manifest returns `unknown`
   (degraded), never `healthy` — the receipt+sentinel pattern repeats the
   receipt-vs-goods bug at smaller scale. A gpu-libs file manifest is Phase-B work.
   Models use ADR-036 size verification labeled `size-verified`; deep hashing is a
   future `doctor --deep`.
8. handleSetup's autostart/hotkey/accessibility/hibernation flows are retained as
   legacy setup adapters — not dropped.
9. Doctor UX: one row per step (`STATUS ID CODE SUMMARY`), evidence + exact repair
   command beneath non-healthy rows only; context header (version, OS/arch,
   effective→target user, mode, selected GPU/model, log path); stable `--json` with
   schemaVersion; exit 0 healthy / 1 unhealthy-or-blocked / 2 internal. `setup
   --repair` exits nonzero when a selected repair fails; ONLY npm postinstall keeps
   the exit-zero policy. No `dryRun` branches in mutating code — doctor IS the
   read-only plan.

## Adjudication — 2026-08-09 (post-implementation review: gpt-5.6-sol + kimi)

The amendment above was implemented and re-reviewed. Outcomes, as built:

**Accepted from the first implementation** (they stand, and are now documented
contract rather than deviation): `run()` is still called on a step whose
`check()` is already healthy (`reassert`, default on for postinstall and
setup — an install asserts, it does not merely inspect; `doctor` never runs
and `--repair` pre-filters, so neither pays for it); the runner re-evaluates
`check()` after `run()` and reports `failed` when it does not come back
healthy, because the wrapped generators swallow their own errors and a clean
return proves nothing; `applies(ctx)` carries the platform/hardware scope; the
models phase banner is unconditional, which is what makes `[N/9]` honest.

**Required and now enforced:**

1. `check()` returns the five-state enum. Two invariants are tested directly,
   not merely implemented: **gpu-libs can never return `healthy`** while its
   only evidence is a receipt plus one sentinel file (`GPULIBS_UNVERIFIED`),
   and **models can never return `healthy` with no speech model on disk**.
   Both are asserted across every fixture, so a future edit cannot quietly
   relax them.
2. Invocation intent must not move artifact truth. The `requestedIds` context
   field is **deleted**, not merely unused — `check()` has no way to learn how
   it was invoked.
3. Fixture and CLI checks fail closed: a zero-byte (or non-regular)
   `libonnxruntime` counts as absent everywhere; an unrecognized model
   selection is `MODELS_UNKNOWN_SELECTION`, never a VAD-only pass; an
   undeterminable GPU variant is `GPULIBS_VARIANT_UNKNOWN`, never acceptance
   of whatever happens to be installed.
4. `showNextSteps()`'s second `autoDownloadModel()` call is removed. It was
   the reason cpu-only installs appeared to skip their model phase and then
   downloaded 2.5 GB during "Finalizing installation"; the models step is now
   the only download path, and cpu-only maps to `0.6b` in both entry points.
5. The two config passes both run in postinstall unconditionally —
   `config-heal` no longer sits inside the GPU model-test branch, so the
   ADR-033 stale-path heal is reachable on macOS and cpu-only machines.
6. The context is the only source of resolved facts: unit/plist paths derive
   from the **target** home (not `os.homedir()`), and the GPU variant is
   resolved once (`ctx.gpuVariant`) rather than re-probed inside `check()`.
   A target home that cannot be resolved is `unknown`, never healthy.
7. macOS `setup` generates LaunchAgents **without** loading them
   (`generateLaunchdServices(ort, {load:false})`); the autostart prompt is
   what loads them, so answering "No" means no. postinstall still loads inline.
8. Exit codes: `doctor` 0 healthy / 1 unhealthy-or-blocked / 2 internal, with
   a stable `--json` (schemaVersion 1). `setup --repair` exits nonzero on a
   failed repair, and a step named explicitly that does not run (blocked or
   not-applicable) is a failed request. Only npm postinstall keeps exit-zero.
9. `setup <bare-word>` is rejected exactly like `setup --dashed-typo`. Both
   used to fall through to a full interactive setup, which starts
   multi-gigabyte downloads the user never asked for.

**Deferred to Phase B:** the per-extracted-file gpu-libs manifest that would
let that step reach `healthy`, and `doctor --deep` (content hashing rather
than ADR-036 size verification).

## Phase B execution — 2026-08-09

Phase B is implemented. The registry is now the whole install: ten steps, and
`postinstall.js` no longer knows how to install anything.

**Migrated.** `platform`, `binaries`, `cleanup`, `integration` and `verify`
join the four Phase-A steps under the adjudicated contract. Registry order is
`platform → binaries → cleanup → config-reset → gpu-libs → models →
config-heal → services → integration → verify`, asserted as a dependency
order rather than a list.

- **platform** — the legacy `checkPlatform()` called `process.exit()` from
  three branches, which is precisely why doctor could never reuse it. The
  verdict and the decision to stop are now separate: the step reports, and the
  driver is the only caller that ends a run. An unsupported OS/arch is
  `unhealthy` with a repair line that says no repair is possible, not
  `not-applicable` — that state means "fine without it" and would paint a
  Windows box green. An old GLIBC/macOS is `unhealthy` (the install proceeds,
  as it always has, but "installed and nothing works" is not health); an
  unreadable version is `unknown`. Missing install directories are the one
  repairable failure, so `createDirectories()` is its `run()`.
- **binaries** — `entrypoints: ['postinstall','setup']`, but the nested
  `npm install -g` repair is gated to the npm lifecycle. From a user-invoked
  CLI that spawn can deadlock on the global lock and rewrites a tree nobody
  asked us to touch; `setup` names the command instead. This is the ONE step
  whose `check()` reads disk rather than `ctx.binaryPaths`: the package is the
  artifact under test and `run()` can install it, so a snapshot taken before
  `run()` would report failure immediately after success. The predicate is
  executability, not existence — the tarball-permissions failure is present,
  correct and unrunnable.
- **cleanup** — `entrypoints: ['postinstall']` only, and that is the point of
  the step rather than a detail: deleting another installation's files (with
  sudo, in `/usr/local`) is a thing an install may do and a diagnostic may
  not. `check()` deliberately names only what `run()` removes — a pip
  onnxruntime ≥1.22 is left alone, so listing it would ask for work the repair
  refuses and the step could never go green.
- **integration** — `needsSession: true`, which turns the spike's
  sudo-misdetection into an honest `blocked`. Under `sudo npm install -g` the
  session variables are root's, so a GNOME Wayland laptop read as bare X11,
  got xdotool instead of ydotool, and could not type a character; the GNOME
  shortcut went into root's dconf. The verdict covers only what `run()` can
  repair (injector + pipewire).
- **verify** — owns whether the unit is ENABLED; `services` owns whether it is
  CORRECT. Both read the same file and fail independently. A unit that exists
  and is not enabled is `unknown`, never `unhealthy`, and `run()` enables only
  under the npm lifecycle — under `setup` the autostart prompt owns that
  choice, the same fence as amendment P1.

**Driver.** `main()` builds one context, walks
`selectSteps({entrypoint:'postinstall'})`, and renders the ledger and next
steps: 73 lines, 51 non-comment. Two seams remain as prepare/follow-up hooks
keyed by step id — hardware detection (which produces the FACTS later steps
consume) and model test-loading (a cross-step judgement between `models` and
everything downstream). With those helpers and the ledger renderer the whole
driver section is 283 lines / 180 non-comment. Exit-0 policy and the
`install.log` lifecycle are unchanged; the single legacy exception (an Intel
Mac fails the whole `npm install`) is preserved deliberately and tested.

**gpu-libs reaches `healthy`.** `gpu-libs.manifest.json` is generated at
extraction time from the tarball's contents — each file hashed as it streams
into `getGpuLibsDir()`, one pass, so the digest is of the bytes that were
written rather than of a later re-read. It lives BESIDE the libraries, not in
the config dir, because the failure being defended against is exactly a record
outliving what it describes. With a manifest, `check()` verifies inventory and
sizes and may say `healthy`; without one it stays `unknown` exactly as before.
Amendment 7's invariant survives in its strongest form and is tested that way:
**gpu-libs can never report healthy without a per-file manifest.** The skip
path in `downloadGPULibraries()` now requires a matching manifest too — a
pre-Phase-B directory re-downloads once rather than being blessed on a receipt
and a sentinel.

**`doctor --deep`.** Steps may expose `deepCheck()`; `checkAll(steps, ctx,
{deep:true})` prefers it and falls back to `check()`, so `--deep` is a
stronger run of the SAME table, never a shorter one. gpu-libs hashes its
manifest; models streams ADR-036's `verifyFile()` over every manifested file
(measured: 17 files / 1.9 GB CoreML tree in 1.1 s on an M5 Max). `--json`
carries `depth: standard|deep`.

**macOS service continuity.** `launchdServiceState()` samples what is loaded
BEFORE plist regeneration boots it out — afterwards the answer is gone. When
`setup`'s autostart prompt is declined, `restoreLaunchdServices()` puts back
what was running: declining auto-start answers "should this start at login?",
it is not an instruction to stop a daemon the user had running. `RunAtLoad`
stays whatever the generated plist says.

**codex PARTIAL #6 closed.** `downloadGPULibraries()` takes
`{hasNvidiaGpu, gpuCompute}`; `generateSystemdService()` and
`generateLaunchdServices()` take `{binaryPaths, targetHome}`. The context
resolves compute capability ONCE (`ctx.gpuCompute`, with `gpuVariant` derived
from it) so the variant and the receipt can never describe different cards,
and the generators no longer call `resolveBinaryPaths()`/`os.homedir()` — the
combination that let a plist be written into one home while the `check()` that
"verified" it read another.

### Deviations from the brief

1. **`setup --cleanup` was a silent no-op** and is now a failed request.
   Filtering a named step out by entrypoint produced an empty plan, printed
   nothing, and exited 0 — indistinguishable from having done the work.
   `setup --list` labels install-only steps rather than hiding them, since
   doctor still reports them.
2. **NVIDIA hibernation is `unknown`, not `unhealthy`.** It is visible and
   real but unrepairable without root, so an unhealthy verdict would mark
   `integration` permanently failed on every affected laptop — a step that can
   never go green. `unknown` prints the evidence and `sudo swictation setup`,
   and leaves doctor's exit code at 0, matching how the installer has always
   treated it.
3. **macOS Accessibility is `unknown` on every Mac.** `AXIsProcessTrusted`
   answers for the calling process (node), not the daemon, and the TCC
   database is SIP-protected. Nothing readable describes the daemon's grant,
   so the row is permanently yellow-but-not-failing rather than a guess.
4. **`platform` gained a directory check** beyond "supported platform/arch".
   Without it `run()` would create directories no `check()` could see, which
   is the shape of bug this ADR exists to prevent.

**Proof.** `node --check` on every touched file; `npm test` green (126
step-registry tests, up from 74 — plus config-step, gpu-lib-cleanup and
model-manifest suites); `postinstall.js` still requirable side-effect-free
(asserted in a subprocess, not by inspection); `node bin/swictation doctor`,
`--json` and `--deep` exercised on darwin-arm64 with correct exit codes.

### Honesty round — 2026-08-09 (check review: kimi)

The five migrated checks were reviewed for states where they report health
they have not established. Eight were found, all confirmed RED against the
shipped code before any fix, each now holding a fixture:

1. **platform** — `sw_vers` exiting 0 with an unparseable string is not the
   same as `sw_vers` failing. `parseInt('garbage')` is NaN, `NaN < 14` is
   false, and the version gate fell straight through to `PLATFORM_OK`: the
   check vouching for an OS it never read. Now `PLATFORM_MACOS_UNPARSEABLE`
   (unknown), matching the contract the adjacent unreadable-version branch
   already followed.
2. **binaries** — `resolve() || ctx.binaryPaths` fell back to the very
   snapshot the comment beside it refuses to trust. A platform package removed
   after the context was built left the snapshot pointing at vanished paths and
   the check said `BINARIES_OK`. The fallback is gone; disk is the only
   authority, which is what that comment always claimed.
3. **binaries** — exists + non-empty + execute bit says nothing about whether
   the kernel can LOAD the image. An x86_64 daemon on Apple Silicon satisfied
   all three and then died with ENOEXEC on every launch. `check()` now reads
   the executable header (Mach-O `cputype` at offset 4, ELF `e_machine` at 18,
   universal binaries by slice) — a bounded read, no spawn — and reports
   `BINARIES_WRONG_ARCH`. An unrecognized format is `unknown`, never assumed
   loadable. `doctor --deep` goes further and actually spawns `--version`
   (`BINARIES_NOT_LOADABLE` / `BINARIES_PROBE_TIMEOUT` / `BINARIES_RUNS`),
   because a header the kernel accepts is still not a daemon whose dynamic
   linker resolves.
4. **cleanup** — the Python interpreter list was hardcoded 3.10–3.13 and went
   blind the day 3.14 shipped, on a schedule set by python.org rather than by
   anything in this repo. Versions are now discovered by reading
   `~/.local/lib/python*` and `~/Library/Python/*`.
5. **cleanup** — a `libonnxruntime.so` whose version could not be parsed
   returned "no conflict", making a library of unknown vintage invisible to
   both the check and the repair while it sat ahead of ours on the linker
   path. It now fails closed. Crucially the REPAIR was changed in the same
   commit: `cleanupOldOnnxRuntime()` had its own copy of both the directory
   list and the predicate, and it now calls the step's — two copies of this
   rule is exactly how a check starts naming artifacts the repair refuses to
   remove, producing a step that can never go green. A test asserts they are
   the same function object, not two that happen to agree.
6. **integration** — with no session variables at all (ssh without X
   forwarding, cron, a serial console) `describeSession()` reported "x11 /
   unknown" and the check confidently demanded xdotool: a verdict about a
   session that was never described. The `x11` default is a fallback for "not
   Wayland", not a positive finding. Now `INTEGRATION_NO_SESSION` (unknown)
   unless one of WAYLAND_DISPLAY / DISPLAY / XDG_SESSION_TYPE /
   XDG_CURRENT_DESKTOP / SWAYSOCK is actually set.
7. **integration** — `which ydotool` proves a file exists; whether a keystroke
   lands also needs ydotoold running and `/dev/uinput` writable, neither
   observable from here — and an installed binary with no daemon behind it is
   the commonest ydotool failure. The state stays `healthy` (the tools ARE
   installed, which is what `run()` installs) but the claim is now exactly
   that: `INTEGRATION_TOOLS_PRESENT`, "on PATH", with the unverified runtime
   liveness stated in the evidence.
8. **verify** — "will start on login" and "is running" are different claims,
   and only the second is what the reader came for. An enabled unit whose
   daemon exits at startup (missing model, bad ORT path) reported `VERIFY_OK`
   with the one fact that mattered buried in evidence under a green row.
   `is-active` now yields its RAW state instead of a boolean that collapsed
   `failed` into `inactive`: `active` → healthy, `failed` → `unhealthy`
   (`VERIFY_DAEMON_FAILED`), anything else → `unknown` (`VERIFY_NOT_RUNNING`)
   with the state named in the SUMMARY.

The round also caught two of this session's own fixtures asserting the right
outcome for the wrong reason: `fakePlatformPackage()` wrote a shebang script
as the daemon (which the new arch sniff correctly calls `unknown`), and the
first `--deep` binaries test monkeypatched a binding `inspectDeep` did not
read, so it passed only on a machine that happened to have the platform
package installed. Both fixtures now build a host-native header and drive an
explicit resolver seam.

`npm test` green at 140 step-registry tests. `doctor` on darwin-arm64 is
unchanged where it was already honest — same rows, same states — except
`binaries`, which now makes the stronger and still true claim "present,
executable, and built for this machine".

**Parity self-audit (same round).** Diffing every function defined at 26c3668
against the ones still reachable found one silent drop the honesty review had
no reason to look for: `checkNvidiaHibernation()` became unreferenced when the
"Verifying installation" phase was split, so laptop users lost the block
explaining CUDA 719/999 after suspend and the command that prevents it. The
one-line warning that replaced it was not a substitute. Restored, with a
fixture, since nothing about an uncalled function is visible in a passing test
suite. Also restored: the platform package name, bin and lib directories are
logged on SUCCESS again — the runner prints evidence only for non-healthy
steps, and install.log is what users attach to bug reports, so a green install
that records nothing about its own binaries is a support dead end.
(`downloadFile()` is also unreferenced, but was already dead at 26c3668 and is
marked `@deprecated`; left alone.)

### Parity round — 2026-08-09 (driver + ordering review: codex)

Codex found Q3 (manifest interruption) and Q5 (driver behaviour) sound, and
three defects. All three RED first:

1. **`cleanup` now precedes `binaries`.** The npm repair inside `binaries`
   reinstalls the platform package, overwriting the daemon executable in
   place; `cleanup` is what stops the running services, and it ran afterwards
   — so an upgrade could swap the binary underneath a live CUDA process. The
   legacy install had the identical hazard, but as an emergent property of
   where the calls sat in a 300-line function. That is the argument for a
   declarative registry in one sentence: the same bug, fixed by moving one
   line and asserted by one test. `binaries.after` is `['platform','cleanup']`;
   the edge is soft, so `setup` (where cleanup never runs) still works.
2. **A fatal platform verdict no longer disappears behind `PLATFORM_PARTIAL`.**
   The runner replaces `health` with a synthesized `_PARTIAL` record when a
   component fails — and a fatal platform step ALWAYS fails a component,
   because `run()` refuses to create directories on a machine the binaries
   cannot run on. `fatalPlatformExit()` read only `health`, found PARTIAL,
   called it non-fatal, and shipped an Intel Mac a full 10-phase install and
   an exit code of 0: the exception this ADR documents as preserved was in
   fact broken the moment it was written. Two changes: `runStep()` now keeps
   `checkHealth` (what `check()` actually said) alongside the record it
   displays — the synthesized PARTIAL is the right thing to SHOW and the wrong
   thing to be the only thing kept — and the driver consults both.
   **The test was the other half of the defect.** It exercised
   `fatalPlatformExit()` directly with a synthetic verdict, so it passed
   against broken behaviour; the failure lived between the runner and the
   predicate, where nothing was looking. The driver loop is now extracted as
   `runPlan(ctx, plan, results)` and the fence drives THAT, asserting both the
   exit code and that no downstream step ran.
3. **`launchctl print` exit 0 means loaded, not running.** Restore-after-
   decline therefore re-bootstrapped jobs that were loaded and STOPPED — and
   because the generated plists carry RunAtLoad, bootstrapping starts them.
   "Put the machine back as I found it" was starting services the user had
   deliberately stopped. `launchdServiceState()` now parses the print block
   (`{loaded, running}` per job, keyed off a TOP-LEVEL `pid`/`state` — the
   block nests `endpoints` sub-dictionaries with their own `state = active`
   lines, and a looser match reads an endpoint's liveness as the job's), and
   `restoreLaunchdServices()` restores only what was genuinely running. macOS
   `verify` gained the same loaded-vs-running distinction the systemd branch
   got in the honesty round, so both platforms now answer the question the
   user actually has.

`npm test` green at 143 step-registry tests. `doctor` on darwin-arm64 is
unchanged in every state; the only visible differences are the cleanup/binaries
row order and `verify` now saying "and the daemon is running", which it now
actually checks.

## Precedent

`src/steps/config.js` (ADR-035) and `src/generate-service.js` (ADR-034) are shipped,
tested, review-hardened step-shaped modules; postinstall.js is already requirable
without side effects (`require.main` guard). The contract generalizes what they proved.

## Consequences

- A failed phase produces a named repair command instead of a mystery reinstall.
- `--ignore-scripts` installs gain a working path (`swictation setup`).
- Divergent reimplementations of install logic become structurally impossible.
- Tests run steps against fixture directories (the config-step suite is the model).
- Verification of hardware-gated steps remains bounded to the GPU-box session that
  gates v0.7.36 (RELEASE_CHECKLIST); Phase A ships with `setup --repair`/`doctor` as
  the only user-visible change.
