/**
 * config-heal — the POST-download config pass (ADR-037 amendment 5).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * Healing a stale model path requires the platform-default target to EXIST,
 * so it cannot work before the download phase — which is exactly why this is
 * a separate step ordered `after: ['models']` rather than a second call to
 * [[config-reset]]. ADR-033's crash loop was a config pointing at a model
 * directory a migration had moved; the heal only fires when the configured
 * path is gone and the default is present, so a path the user deliberately
 * customized to a real location is never touched.
 *
 * `resetManagedOverride` is deliberately OFF here: this pass runs after
 * postinstall wrote a freshly verified model into the override, and the
 * reset rule would undo it.
 */

const configStep = require('./config');
const { healthy, unhealthy, componentOk } = require('./health');

module.exports = {
  id: 'config-heal',
  title: 'Healing configuration...',
  entrypoints: ['postinstall', 'setup'],
  after: ['models'],
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
    if (state.healableKeys.length > 0) {
      return unhealthy('CONFIG_STALE_PATHS',
        `config.toml points at ${state.healableKeys.length} missing model path(s)`, {
          evidence: state.healableKeys,
        });
    }
    return healthy('CONFIG_HEALED', 'no stale model paths in config.toml', {
      evidence: [state.path],
    });
  },

  run(ctx) {
    const result = configStep.run({
      log: ctx.log,
      generateDefaultConfig: ctx.generateDefaultConfig,
      resetManagedOverride: false,
    });
    return {
      changed: result.healedKeys.length > 0,
      components: [componentOk('model-paths', result.healedKeys.length > 0
        ? `healed ${result.healedKeys.join(', ')}`
        : 'nothing to heal')],
      warnings: [],
    };
  },
};
