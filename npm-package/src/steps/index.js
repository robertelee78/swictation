/**
 * Install-step registry and runner (ADR-037 + its 2026-08-09 amendment).
 *
 * ── The fence this module exists to hold ────────────────────────────────
 * Install state is derived from disk, never from a memory of having run.
 * Every step exposes `check(ctx)` — side-effect-free, computed from
 * artifacts — and an idempotent `run(ctx)`. ADR-035's receipt-vs-goods GPU
 * bug is the canonical counterexample: a metadata receipt in the config dir
 * survived the npm upgrade that deleted 1.5 GB of libraries, the skip check
 * believed the receipt, and the daemon silently fell back to CPU.
 *
 * The runner enforces that rule for every step: after `run()` it
 * re-evaluates `check()`, and a step whose check does not come back healthy
 * is `failed` even when `run()` returned happily. Several of the wrapped
 * postinstall functions catch everything internally and return normally
 * after failing, so a clean return proves nothing on its own.
 *
 * ── Step contract ───────────────────────────────────────────────────────
 *   {
 *     id, title,
 *     entrypoints: ['postinstall','setup'],   // who may run it
 *     after: ['other-id'],                    // SOFT ordering, see below
 *     needsNetwork, needsSession, forbidRoot, // capability gating
 *     applies(ctx)  -> boolean                // is this step in scope here
 *     check(ctx)    -> health record (src/steps/health.js)
 *     run(ctx)      -> {changed, components[], warnings[]}
 *   }
 *
 * `applies()` is SEPARATE from capability gating (amendment 2). A Linux box
 * with no NVIDIA card is `not-applicable` for gpu-libs — there is nothing to
 * repair, ever — whereas an offline machine that still needs the libraries
 * is `blocked`, which is a temporary condition with a repair command. These
 * used to be the same "skipped" and the difference is the whole point.
 *
 * `after` is soft: it orders steps that are both in the plan and is ignored
 * when the dependency is absent or failed. services follows gpu-libs so the
 * unit is written against freshly installed libraries, but a failed gpu-libs
 * must still leave a CPU-capable unit behind rather than no unit at all.
 *
 * ── Runner semantics ────────────────────────────────────────────────────
 * The runner walks an ordered list, continue-on-error, and never aborts it.
 * Per step: `{id, title, status, health, changed, components, warnings, error?}`
 *   ok              check came back healthy after run()
 *   already         check was ALREADY healthy before run()
 *   failed          run() threw, a component failed, or check is still unhealthy
 *   blocked         capabilities missing — repair command printed
 *   not-applicable  out of scope on this machine
 *
 * check() runs BEFORE capability gating (amendment 2): an offline machine
 * whose artifacts are intact reports `already`, not "network unavailable".
 *
 * `reassert` (default true for postinstall and setup) still calls `run()` on
 * a healthy step, because that is what an install has always done and Phase A
 * holds behavior parity. `doctor` never runs anything; `--repair` pre-filters
 * to the unhealthy set, so it never sees a healthy step at all.
 */

const health = require('./health');
const { createContext, deriveContext, readSavedGpuInfo } = require('./context');

const { STATE } = health;

/** Steps in install order. postinstall, setup and doctor all walk this list. */
const STEPS = [
  require('./config-reset'),
  require('./gpu-libs'),
  require('./models'),
  require('./config-heal'),
  require('./services'),
];

const STATUS = {
  OK: 'ok',
  ALREADY: 'already',
  FAILED: 'failed',
  BLOCKED: 'blocked',
  NOT_APPLICABLE: 'not-applicable',
};

/** The command that repairs this step. */
function repairCommand(step) {
  return step.repair || `swictation setup --${step.id}`;
}

/** `config-reset` → `CONFIG_RESET`, so generated codes read like the rest. */
function codePrefix(step) {
  return step.id.toUpperCase().replace(/-/g, '_');
}

function getStep(id) {
  return STEPS.find(step => step.id === id) || null;
}

function appliesTo(step, ctx) {
  if (typeof step.applies !== 'function') return true;
  try {
    return step.applies(ctx) === true;
  } catch {
    return false;
  }
}

/**
 * Capability names this step needs that the context does not have.
 * Evaluated only when the step actually has work to do.
 */
function unmetCapabilities(step, ctx) {
  const caps = ctx.caps || {};
  const missing = [];
  if (step.needsNetwork && caps.network !== true) missing.push('network');
  if (step.needsSession && caps.session !== true) missing.push('session');
  // forbidRoot is not "needs root" — it is the sudo-writes-into-/root case:
  // running as root on behalf of another user resolves the wrong home.
  if (step.forbidRoot && ctx.elevatedForAnother) missing.push('non-root install');
  return missing;
}

/** Evaluate check(), never throwing: a broken check must not abort doctor. */
function checkStep(step, ctx) {
  try {
    return health.normalizeHealth(step.check(ctx), `${codePrefix(step)}_NO_HEALTH`);
  } catch (err) {
    return health.unknown(
      `${codePrefix(step)}_CHECK_ERROR`,
      `check() threw: ${err.message}`,
      { repair: repairCommand(step) }
    );
  }
}

/** Async-tolerant check for callers that may register async steps later. */
async function checkStepAsync(step, ctx) {
  try {
    return health.normalizeHealth(await step.check(ctx), `${codePrefix(step)}_NO_HEALTH`);
  } catch (err) {
    return health.unknown(
      `${codePrefix(step)}_CHECK_ERROR`,
      `check() threw: ${err.message}`,
      { repair: repairCommand(step) }
    );
  }
}

/**
 * Stable ordering that honours soft `after` edges.
 *
 * Registry order is the baseline; a step only moves later, and only past
 * dependencies that are actually present in this plan. A cycle degrades to
 * registry order rather than throwing — ordering is an optimization here,
 * not a correctness requirement, and refusing to install over a bad edge
 * would be worse than installing in a slightly wrong order.
 */
function orderSteps(list) {
  const present = new Set(list.map(s => s.id));
  const emitted = new Set();
  const ordered = [];
  const visiting = new Set();

  const visit = (step) => {
    if (emitted.has(step.id) || visiting.has(step.id)) return;
    visiting.add(step.id);
    for (const depId of step.after || []) {
      if (!present.has(depId)) continue;
      const dep = list.find(s => s.id === depId);
      if (dep) visit(dep);
    }
    visiting.delete(step.id);
    if (!emitted.has(step.id)) {
      emitted.add(step.id);
      ordered.push(step);
    }
  };

  for (const step of list) visit(step);
  return ordered;
}

/**
 * Ordered subset of the registry.
 * @param {object} [options]
 * @param {string[]} [options.ids] - only these ids
 * @param {'postinstall'|'setup'} [options.entrypoint] - drop steps that
 *   do not list it (what keeps npm-lifecycle-only work out of `setup`)
 * @param {object[]} [options.steps=STEPS]
 */
function selectSteps(options = {}) {
  const { ids, entrypoint, steps = STEPS } = options;
  const filtered = steps.filter(step => {
    if (ids && !ids.includes(step.id)) return false;
    if (entrypoint && !(step.entrypoints || []).includes(entrypoint)) return false;
    return true;
  });
  return orderSteps(filtered);
}

function emptyResult(step, status, healthRecord) {
  return {
    id: step.id,
    title: step.title,
    status,
    health: healthRecord,
    changed: false,
    components: [],
    warnings: [],
  };
}

/**
 * Run one step. Never throws.
 * @param {object} [options]
 * @param {boolean} [options.reassert=true] - call run() even when healthy
 * @param {boolean} [options.quiet=false]
 */
async function runStep(step, ctx, options = {}) {
  const { reassert = true, quiet = false } = options;
  const log = quiet ? () => {} : ctx.log;
  const repair = repairCommand(step);

  const announce = (record) => {
    if (record.state === STATE.HEALTHY) return;
    if (record.summary) log(record.state === STATE.UNHEALTHY ? 'red' : 'yellow', `  ${record.summary}`);
    for (const item of record.evidence) log('cyan', `    ${item}`);
    if (record.state !== STATE.NOT_APPLICABLE) log('cyan', `    Repair: ${record.repair || repair}`);
  };

  if (!appliesTo(step, ctx)) {
    const record = health.notApplicable(
      `${codePrefix(step)}_NOT_APPLICABLE`,
      `${step.title} does not apply on this system`
    );
    log('cyan', `  – ${record.summary}`);
    return emptyResult(step, STATUS.NOT_APPLICABLE, record);
  }

  // Before gating (amendment 2): intact artifacts beat a missing capability.
  const before = await checkStepAsync(step, ctx);

  if (before.state === STATE.NOT_APPLICABLE) {
    log('cyan', `  – ${before.summary}`);
    return emptyResult(step, STATUS.NOT_APPLICABLE, before);
  }

  if (before.state === STATE.HEALTHY && !reassert) {
    return emptyResult(step, STATUS.ALREADY, before);
  }

  const missing = unmetCapabilities(step, ctx);
  if (missing.length > 0) {
    // A healthy step with unmet capabilities is still healthy — there was
    // nothing it needed the capability for.
    if (before.state === STATE.HEALTHY) {
      return emptyResult(step, STATUS.ALREADY, before);
    }
    const record = health.blocked(
      `${codePrefix(step)}_BLOCKED`,
      `${step.title} requires ${missing.join(', ')}`,
      { evidence: before.evidence, repair: before.repair || repair }
    );
    announce(record);
    return emptyResult(step, STATUS.BLOCKED, record);
  }

  let ran;
  try {
    ran = health.normalizeRunResult(await step.run(ctx));
  } catch (err) {
    const record = health.unhealthy(
      `${codePrefix(step)}_RUN_ERROR`,
      `${step.title} failed: ${err.message}`,
      { evidence: before.evidence, repair }
    );
    announce(record);
    return { ...emptyResult(step, STATUS.FAILED, record), error: err };
  }

  const after = await checkStepAsync(step, ctx);
  const failedComponents = ran.components.filter(c => c.status === 'failed');
  const base = {
    id: step.id,
    title: step.title,
    health: after,
    changed: ran.changed,
    components: ran.components,
    warnings: ran.warnings,
  };

  for (const warning of ran.warnings) log('yellow', `  ⚠️  ${warning}`);

  // A typed component failure is a failure even if the overall check passes:
  // the UI service can fail while the daemon unit lands fine, and silently
  // reporting "ok" is what the amendment's typed results exist to prevent.
  if (failedComponents.length > 0) {
    const record = health.unhealthy(
      `${codePrefix(step)}_PARTIAL`,
      `${step.title}: ${failedComponents.map(c => c.id).join(', ')} failed`,
      { evidence: failedComponents.map(c => `${c.id}: ${c.summary}`), repair }
    );
    announce(record);
    return { ...base, status: STATUS.FAILED, health: record };
  }

  if (after.state === STATE.UNHEALTHY) {
    const record = { ...after, repair: after.repair || repair };
    announce(record);
    return { ...base, status: STATUS.FAILED, health: record };
  }

  if (after.state === STATE.BLOCKED) {
    announce(after);
    return { ...base, status: STATUS.BLOCKED };
  }

  if (after.state === STATE.NOT_APPLICABLE) {
    return { ...base, status: STATUS.NOT_APPLICABLE };
  }

  // healthy or unknown. `unknown` is a legitimate success for a step that
  // cannot fully verify itself yet (gpu-libs has no per-file manifest until
  // Phase B) — it is reported as such, never laundered into healthy.
  if (after.state === STATE.UNKNOWN) announce(after);
  const status = before.state === STATE.HEALTHY && after.state === STATE.HEALTHY
    ? STATUS.ALREADY
    : STATUS.OK;
  return { ...base, status };
}

/** Run an ordered list, continue-on-error. Never aborts, never throws. */
async function runSteps(steps, ctx, options = {}) {
  const results = [];
  for (const step of steps) {
    results.push(await runStep(step, ctx, options));
  }
  return results;
}

/**
 * Evaluate every check() without running anything — what `doctor` reports
 * and what `setup --repair` filters on.
 */
async function checkAll(steps, ctx) {
  const rows = [];
  for (const step of steps) {
    if (!appliesTo(step, ctx)) {
      rows.push({
        id: step.id,
        title: step.title,
        health: health.notApplicable(
          `${codePrefix(step)}_NOT_APPLICABLE`,
          `does not apply on ${ctx.platform} ${ctx.arch}`
        ),
        repair: repairCommand(step),
      });
      continue;
    }
    const record = await checkStepAsync(step, ctx);
    // Report a capability block only when there is work the block prevents.
    const missing = record.state === STATE.HEALTHY ? [] : unmetCapabilities(step, ctx);
    const finalRecord = missing.length > 0
      ? health.blocked(
        `${codePrefix(step)}_BLOCKED`,
        `${record.summary} (requires ${missing.join(', ')})`,
        { evidence: record.evidence, repair: record.repair || repairCommand(step) }
      )
      : record;
    rows.push({ id: step.id, title: step.title, health: finalRecord, repair: repairCommand(step) });
  }
  return rows;
}

/**
 * Steps `setup --repair` should act on: everything not healthy and not out
 * of scope. `unknown` is included — an unverifiable step is exactly the one
 * worth re-running.
 */
async function failingSteps(steps, ctx) {
  const rows = await checkAll(steps, ctx);
  const wanted = new Set(
    rows
      .filter(r => r.health.state !== STATE.HEALTHY && r.health.state !== STATE.NOT_APPLICABLE)
      .map(r => r.id)
  );
  return steps.filter(step => wanted.has(step.id));
}

/** Process exit code for a set of doctor rows (amendment 9). */
function exitCodeForRows(rows) {
  const bad = rows.some(r => r.health.state === STATE.UNHEALTHY || r.health.state === STATE.BLOCKED);
  return bad ? 1 : 0;
}

module.exports = {
  STEPS,
  STATUS,
  STATE,
  health,
  createContext,
  deriveContext,
  readSavedGpuInfo,
  repairCommand,
  getStep,
  appliesTo,
  unmetCapabilities,
  checkStep,
  checkStepAsync,
  orderSteps,
  selectSteps,
  runStep,
  runSteps,
  checkAll,
  failingSteps,
  exitCodeForRows,
};
