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
