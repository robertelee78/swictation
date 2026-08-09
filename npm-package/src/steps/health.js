/**
 * The health vocabulary every step's check() speaks (ADR-037, amendment 1).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * A boolean cannot tell the difference between "this machine has no NVIDIA
 * GPU so there is nothing to install", "the libraries are missing", and "I
 * have no way to know whether these 40 extracted files are intact". All
 * three used to collapse into `false`, and the third one collapsing into
 * `true` is precisely the ADR-035 receipt-vs-goods bug: a check that cannot
 * verify something must say so, not vouch for it.
 *
 * States:
 *   healthy         the artifacts this step owns are present and correct
 *   unhealthy       they are absent or wrong — `repair` says how to fix it
 *   blocked         cannot be established here (no session, root install)
 *   not-applicable  out of scope on this machine — nothing to fix, ever
 *   unknown         genuinely undetermined; NEVER a synonym for healthy
 *
 * `code` is a stable machine-readable token (doctor --json consumers and
 * support reports key off it); `summary` is one human line; `evidence` is
 * the concrete paths/values the verdict was computed from, so a bug report
 * carries its own proof; `repair` is the exact command to run.
 */

const STATE = {
  HEALTHY: 'healthy',
  UNHEALTHY: 'unhealthy',
  BLOCKED: 'blocked',
  NOT_APPLICABLE: 'not-applicable',
  UNKNOWN: 'unknown',
};

const ALL_STATES = Object.values(STATE);

/** Every state except `healthy` warrants a repair line in doctor. */
function isHealthy(health) {
  return !!health && health.state === STATE.HEALTHY;
}

/** States that mean "no action is possible or needed here". */
function isTerminal(health) {
  return !!health && (health.state === STATE.NOT_APPLICABLE || health.state === STATE.BLOCKED);
}

function makeHealth(state, code, summary, options = {}) {
  return {
    state,
    code,
    summary,
    evidence: options.evidence || [],
    repair: options.repair || null,
  };
}

const healthy = (code, summary, options) => makeHealth(STATE.HEALTHY, code, summary, options);
const unhealthy = (code, summary, options) => makeHealth(STATE.UNHEALTHY, code, summary, options);
const blocked = (code, summary, options) => makeHealth(STATE.BLOCKED, code, summary, options);
const notApplicable = (code, summary, options) => makeHealth(STATE.NOT_APPLICABLE, code, summary, options);
const unknown = (code, summary, options) => makeHealth(STATE.UNKNOWN, code, summary, options);

/**
 * Coerce whatever a step returned into a valid health record. A step that
 * returns nonsense is a bug in the step, not a reason to crash doctor.
 */
function normalizeHealth(value, fallbackCode = 'E_NO_HEALTH') {
  if (!value || typeof value !== 'object' || !ALL_STATES.includes(value.state)) {
    return unknown(fallbackCode, 'check() did not return a health record');
  }
  return {
    state: value.state,
    code: typeof value.code === 'string' ? value.code : fallbackCode,
    summary: typeof value.summary === 'string' ? value.summary : '',
    evidence: Array.isArray(value.evidence) ? value.evidence : [],
    repair: typeof value.repair === 'string' ? value.repair : null,
  };
}

/** A component is one unit of work inside run() that can fail on its own. */
function component(id, status, summary, options = {}) {
  return { id, status, summary, error: options.error || null };
}

const componentOk = (id, summary) => component(id, 'ok', summary);
const componentFailed = (id, summary, error) => component(id, 'failed', summary, { error });
const componentSkipped = (id, summary) => component(id, 'skipped', summary);

/** Normalize a run() return value into {changed, components, warnings}. */
function normalizeRunResult(value) {
  if (!value || typeof value !== 'object') {
    return { changed: false, components: [], warnings: [] };
  }
  return {
    changed: value.changed === true,
    components: Array.isArray(value.components) ? value.components : [],
    warnings: Array.isArray(value.warnings) ? value.warnings : [],
  };
}

module.exports = {
  STATE,
  ALL_STATES,
  isHealthy,
  isTerminal,
  makeHealth,
  healthy,
  unhealthy,
  blocked,
  notApplicable,
  unknown,
  normalizeHealth,
  component,
  componentOk,
  componentFailed,
  componentSkipped,
  normalizeRunResult,
};
