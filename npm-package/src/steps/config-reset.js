/**
 * config-reset — the PRE-download config pass (ADR-037 amendment 5).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * Config is two steps, not one, because it has two-pass semantics that a
 * single check/run pair cannot express. This pass runs BEFORE any model is
 * downloaded and carries `resetManagedOverride: true`: an
 * `stt_model_override` that postinstall itself wrote (recorded in the
 * sidecar) reverts to "auto" so this install re-tests the hardware. A GPU
 * that disappeared would otherwise keep forcing a branch that now errors.
 *
 * It must NOT run after the download phase — [[config-heal]] is that pass —
 * because by then postinstall has written a freshly verified model into the
 * override and the reset rule would undo it.
 *
 * The implementation is src/steps/config.js (ADR-035, shipped and tested).
 * This file is an adapter: shape translation and nothing else.
 */

const configStep = require('./config');
const { healthy, unhealthy, componentOk } = require('./health');

module.exports = {
  id: 'config-reset',
  // Reused verbatim as postinstall's phase banner — keep in sync with the plan.
  title: 'Configuring...',
  entrypoints: ['postinstall', 'setup'],
  after: [],
  forbidRoot: true,

  applies() {
    return true;
  },

  check() {
    const state = configStep.inspect();
    if (!state.exists) {
      return unhealthy('CONFIG_MISSING', 'config.toml does not exist', { evidence: [state.path] });
    }
    if (!state.parses) {
      return unhealthy('CONFIG_UNPARSEABLE', 'config.toml does not parse', {
        evidence: [state.path, state.parseError || 'unknown parse error'],
      });
    }
    // A parseable config is healthy. We do NOT flag an installer-written
    // override here: its staleness cannot be judged side-effect-free (gpu-info
    // is persisted, not re-detected), and flagging its mere presence cried wolf
    // on every fresh install (ADR-037 health round). The reset-to-auto that
    // re-tests hardware is an install action, done by run() in postinstall only.
    return healthy('CONFIG_OK', 'config.toml present and parseable', { evidence: [state.path] });
  },

  run(ctx) {
    const before = configStep.inspect();
    const result = configStep.run({
      log: ctx.log,
      generateDefaultConfig: ctx.generateDefaultConfig,
      // Reset an installer-written override to "auto" ONLY during the
      // postinstall pre-download pass, where the models phase then re-tests the
      // hardware and writes a fresh choice. In `setup`/doctor there is no such
      // re-test, so resetting a working override would just churn it (ADR-037
      // health round: full `swictation setup` must not rewrite a healthy one).
      resetManagedOverride: ctx.mode === 'postinstall',
    });
    return {
      changed: result.action !== 'kept',
      components: [componentOk('config-file', `${result.action} (${before.path})`)],
      warnings: result.action === 'replaced-invalid'
        ? ['config.toml did not parse and was replaced with defaults (backup kept)']
        : [],
    };
  },
};
