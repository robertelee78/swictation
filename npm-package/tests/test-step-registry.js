#!/usr/bin/env node
/**
 * Tests for the install-step registry and runner (ADR-037 + amendment).
 *
 * Three halves (the third is the CLI, which is where a typo once started a
 * 1.9 GB download):
 *   1. Runner semantics against fixture steps — ordering incl. soft `after`
 *      edges, continue-on-error, the five statuses, applies() vs capability
 *      gating, and the rule that makes the whole thing trustworthy: a step
 *      whose check() is not healthy after run() is FAILED, no matter how
 *      cleanly run() returned. Most wrapped postinstall functions swallow
 *      their own errors, so without that rule the ledger prints "ok" over a
 *      broken install.
 *   2. Each step's check() against fixture directories, following
 *      tests/test-config-step.js: isolate HOME, build the artifacts, assert
 *      the health record. Model trees are fabricated as SPARSE files at the
 *      exact sizes models.manifest.json declares, so the real
 *      manifest-aware ModelDownloader runs against a real tree without
 *      writing gigabytes.
 *   3. CLI contracts: doctor's exit codes and --json schema, setup's flag
 *      validation.
 *
 * Run: node tests/test-step-registry.js
 */

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

// Isolate HOME so paths.js resolves into a scratch dir (test-config-step.js
// establishes this pattern; os.homedir() honours $HOME on POSIX).
const scratchHome = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-step-registry-'));
process.env.HOME = scratchHome;
// The context reads these; pin them so a developer's shell cannot flip a test.
delete process.env.SWICTATION_OFFLINE;
delete process.env.npm_config_offline;
delete process.env.SUDO_USER;

const steps = require('../src/steps');
const { STATE, STATUS } = steps;
const health = require('../src/steps/health');
const servicesStep = require('../src/steps/services');
const modelsStep = require('../src/steps/models');
const gpuLibsStep = require('../src/steps/gpu-libs');
const configReset = require('../src/steps/config-reset');
const configHeal = require('../src/steps/config-heal');
const { getConfigDir, getModelsDir, getGpuLibsDir } = require('../src/paths');

const PKG_ROOT = path.join(__dirname, '..');
const MANIFEST = require('../models.manifest.json');

let passed = 0;
let currentLog = [];

function test(name, fn) {
  currentLog = [];
  fn();
  passed += 1;
  console.log(`  ✓ ${name}`);
}

async function asyncTest(name, fn) {
  currentLog = [];
  await fn();
  passed += 1;
  console.log(`  ✓ ${name}`);
}

/** Recording logger: the runner's user-facing output is part of its contract. */
const recorder = (color, message) => currentLog.push(`${color}:${message}`);
const loggedText = () => currentLog.join('\n');

function ctxWith(overrides = {}) {
  return steps.createContext({ mode: 'setup', log: recorder, ...overrides });
}

/** A fixture step whose check() answers from a mutable box. */
function fixtureStep(id, options = {}) {
  const state = { healthy: options.healthy === true, ran: 0 };
  return {
    id,
    title: options.title || `step ${id}`,
    entrypoints: options.entrypoints || ['postinstall', 'setup'],
    after: options.after || [],
    needsNetwork: options.needsNetwork,
    needsSession: options.needsSession,
    forbidRoot: options.forbidRoot,
    state,
    applies: () => options.applies !== false,
    check() {
      if (options.checkThrows) throw new Error('check exploded');
      if (options.checkState) return health.makeHealth(options.checkState, 'FIXTURE', 'fixture state');
      return state.healthy
        ? health.healthy('FIXTURE_OK', 'fixture is healthy')
        : health.unhealthy('FIXTURE_BAD', 'fixture is not healthy');
    },
    run() {
      state.ran += 1;
      if (options.order) options.order.push(id);
      if (options.runThrows) throw new Error(`${id} blew up`);
      if (options.healsOnRun !== false) state.healthy = true;
      return {
        changed: true,
        components: options.failComponent
          ? [health.componentFailed('sub', 'sub-unit failed')]
          : [health.componentOk('sub', 'fine')],
        warnings: options.warn ? ['a warning'] : [],
      };
    },
  };
}

console.log('\nRunner semantics:');

(async () => {

await asyncTest('runs steps in list order', async () => {
  const order = [];
  const list = ['a', 'b', 'c'].map(id => fixtureStep(id, { order }));
  await steps.runSteps(list, ctxWith());
  assert.deepStrictEqual(order, ['a', 'b', 'c']);
});

await asyncTest('continue-on-error: a failed step never aborts the list', async () => {
  const order = [];
  const list = [
    fixtureStep('a', { order }),
    fixtureStep('boom', { order, runThrows: true }),
    fixtureStep('c', { order }),
  ];
  const results = await steps.runSteps(list, ctxWith());
  assert.deepStrictEqual(order, ['a', 'boom', 'c'], 'every step still ran');
  assert.deepStrictEqual(results.map(r => r.status), [STATUS.OK, STATUS.FAILED, STATUS.OK]);
  assert.strictEqual(results[1].error.message, 'boom blew up');
  assert.strictEqual(results[1].health.code, 'BOOM_RUN_ERROR');
});

await asyncTest('ok: check unhealthy before, healthy after run()', async () => {
  const step = fixtureStep('a');
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.OK);
  assert.strictEqual(result.health.state, STATE.HEALTHY);
  assert.strictEqual(step.state.ran, 1);
});

await asyncTest('already: healthy before run — and reassert still runs it', async () => {
  const step = fixtureStep('a', { healthy: true });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.ALREADY);
  assert.strictEqual(
    step.state.ran, 1,
    'postinstall reasserts every step, which is what an install has always done'
  );
});

await asyncTest('already: reassert:false leaves a healthy step untouched', async () => {
  const step = fixtureStep('a', { healthy: true });
  const result = await steps.runStep(step, ctxWith(), { reassert: false });
  assert.strictEqual(result.status, STATUS.ALREADY);
  assert.strictEqual(step.state.ran, 0);
});

await asyncTest('failed: run() returned cleanly but check() is still unhealthy', async () => {
  const step = fixtureStep('a', { healsOnRun: false });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.FAILED,
    'check() is the definition of truth, not run()\'s return value');
  assert.match(loggedText(), /Repair: swictation setup --a/);
});

await asyncTest('failed: a typed component failure beats an otherwise healthy check', async () => {
  const step = fixtureStep('a', { failComponent: true });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.FAILED);
  assert.strictEqual(result.health.code, 'A_PARTIAL');
  assert.deepStrictEqual(result.components.map(c => c.status), ['failed']);
});

await asyncTest('warnings from run() are surfaced, not swallowed', async () => {
  const step = fixtureStep('a', { warn: true });
  const result = await steps.runStep(step, ctxWith());
  assert.deepStrictEqual(result.warnings, ['a warning']);
  assert.match(loggedText(), /a warning/);
});

await asyncTest('not-applicable: applies() false means nothing to repair, ever', async () => {
  const step = fixtureStep('a', { applies: false });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.NOT_APPLICABLE);
  assert.strictEqual(step.state.ran, 0);
  assert.ok(!loggedText().includes('Repair:'), 'an out-of-scope step has no repair command');
});

await asyncTest('blocked: unmet capabilities gate execution and print a repair hint', async () => {
  const step = fixtureStep('a', { needsNetwork: true });
  const ctx = ctxWith({ caps: { network: false, session: true, root: false } });
  const result = await steps.runStep(step, ctx);
  assert.strictEqual(result.status, STATUS.BLOCKED);
  assert.strictEqual(step.state.ran, 0, 'a gated step must never run-and-misbehave');
  assert.match(result.health.summary, /requires network/);
  assert.match(loggedText(), /Repair: swictation setup --a/);
});

await asyncTest('check() runs BEFORE gating: intact artifacts beat a missing capability', async () => {
  // The amendment's example: offline, but everything is already on disk.
  const step = fixtureStep('a', { healthy: true, needsNetwork: true });
  const ctx = ctxWith({ caps: { network: false, session: true, root: false } });
  const result = await steps.runStep(step, ctx, { reassert: false });
  assert.strictEqual(result.status, STATUS.ALREADY,
    'reporting "network unavailable" for a step with nothing to do is a lie');
  assert.strictEqual(result.health.state, STATE.HEALTHY);
});

await asyncTest('blocked: forbidRoot fires only when root acts for another user', async () => {
  const step = fixtureStep('a', { forbidRoot: true });
  const asUser = await steps.runStep(step, ctxWith({ users: { uid: 1000, isRoot: false, effectiveUser: 'u', targetUser: 'u', targetHome: '/home/u', elevatedForAnother: false } }));
  assert.strictEqual(asUser.status, STATUS.OK);

  const viaSudo = await steps.runStep(fixtureStep('a', { forbidRoot: true }), ctxWith({
    users: { uid: 0, isRoot: true, effectiveUser: 'root', targetUser: 'u', targetHome: '/home/u', elevatedForAnother: true },
  }));
  assert.strictEqual(viaSudo.status, STATUS.BLOCKED);
  assert.match(viaSudo.health.summary, /non-root install/);
});

await asyncTest('unknown after run() is success, but is reported as unknown', async () => {
  // gpu-libs can never say healthy until Phase B ships a per-file manifest.
  const step = fixtureStep('a', { checkState: STATE.UNKNOWN });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.status, STATUS.OK, 'an unverifiable step is not a failed step');
  assert.strictEqual(result.health.state, STATE.UNKNOWN,
    'and it must never be laundered into healthy');
});

await asyncTest('a throwing check() reports unknown instead of aborting the run', async () => {
  const step = fixtureStep('a', { checkThrows: true });
  const result = await steps.runStep(step, ctxWith());
  assert.strictEqual(result.health.state, STATE.UNKNOWN);
  assert.match(result.health.summary, /check\(\) threw: check exploded/);
});

await asyncTest('checkAll reports without running anything', async () => {
  const list = [fixtureStep('a', { healthy: true }), fixtureStep('b')];
  const rows = await steps.checkAll(list, ctxWith());
  assert.deepStrictEqual(rows.map(r => r.health.state), [STATE.HEALTHY, STATE.UNHEALTHY]);
  assert.deepStrictEqual(list.map(s => s.state.ran), [0, 0], 'doctor must never mutate the system');
});

await asyncTest('checkAll reports a block only when the block prevents work', async () => {
  const ctx = ctxWith({ caps: { network: false, session: true, root: false } });
  const rows = await steps.checkAll([
    fixtureStep('done', { healthy: true, needsNetwork: true }),
    fixtureStep('todo', { needsNetwork: true }),
  ], ctx);
  assert.strictEqual(rows[0].health.state, STATE.HEALTHY);
  assert.strictEqual(rows[1].health.state, STATE.BLOCKED);
});

await asyncTest('failingSteps selects unhealthy and unknown, never healthy or n/a', async () => {
  const list = [
    fixtureStep('good', { healthy: true }),
    fixtureStep('bad'),
    fixtureStep('murky', { checkState: STATE.UNKNOWN }),
    fixtureStep('moot', { applies: false }),
  ];
  const failing = await steps.failingSteps(list, ctxWith());
  assert.deepStrictEqual(failing.map(s => s.id), ['bad', 'murky'],
    'an unverifiable step is exactly the one worth re-running');
});

test('orderSteps honours soft after-edges and keeps registry order otherwise', () => {
  const list = [fixtureStep('services', { after: ['gpu-libs'] }), fixtureStep('gpu-libs')];
  assert.deepStrictEqual(steps.orderSteps(list).map(s => s.id), ['gpu-libs', 'services']);
});

test('orderSteps ignores an after-edge whose target is not in the plan', () => {
  const list = [fixtureStep('services', { after: ['gpu-libs'] })];
  assert.deepStrictEqual(steps.orderSteps(list).map(s => s.id), ['services'],
    'a failed or absent gpu-libs must still leave a CPU-capable unit behind');
});

test('orderSteps degrades to registry order on a cycle rather than throwing', () => {
  const list = [fixtureStep('a', { after: ['b'] }), fixtureStep('b', { after: ['a'] })];
  assert.deepStrictEqual(steps.orderSteps(list).map(s => s.id).sort(), ['a', 'b']);
});

test('selectSteps filters by id and by entrypoint', () => {
  assert.deepStrictEqual(
    steps.selectSteps({ ids: ['services', 'config-reset'] }).map(s => s.id),
    ['config-reset', 'services']
  );
  const list = [fixtureStep('both'), fixtureStep('installOnly', { entrypoints: ['postinstall'] })];
  assert.deepStrictEqual(
    steps.selectSteps({ steps: list, entrypoint: 'setup' }).map(s => s.id), ['both']);
  assert.deepStrictEqual(
    steps.selectSteps({ steps: list, entrypoint: 'postinstall' }).map(s => s.id),
    ['both', 'installOnly']);
});

test('the real registry order is a dependency order', () => {
  const plan = steps.selectSteps({ entrypoint: 'postinstall' }).map(s => s.id);
  const before = (a, b) => assert.ok(plan.indexOf(a) < plan.indexOf(b), `${a} must precede ${b}`);

  assert.strictEqual(plan[0], 'platform',
    'nothing may be written before the platform is known to be supported');
  before('platform', 'cleanup');
  // The npm repair inside `binaries` overwrites the daemon binary in place.
  // Doing that while the previous version is still running is how a live
  // CUDA process gets its executable swapped underneath it; `cleanup` is what
  // stops the services, so it has to have already run. The legacy phase order
  // had this backwards too — but ordering is declarative now, so it is fixable
  // in one line instead of being a property of where the calls happened to sit.
  before('cleanup', 'binaries');
  before('binaries', 'services');
  before('cleanup', 'services');
  before('config-reset', 'models');
  before('models', 'config-heal');
  before('gpu-libs', 'services');
  before('services', 'integration');
  before('services', 'verify');
});

test('every registered step satisfies the amended contract', () => {
  for (const step of steps.STEPS) {
    assert.strictEqual(typeof step.id, 'string', 'id');
    assert.strictEqual(typeof step.title, 'string', `${step.id}: title`);
    assert.strictEqual(typeof step.check, 'function', `${step.id}: check`);
    assert.strictEqual(typeof step.run, 'function', `${step.id}: run`);
    assert.strictEqual(typeof step.applies, 'function', `${step.id}: applies`);
    assert.ok(Array.isArray(step.entrypoints) && step.entrypoints.length > 0,
      `${step.id}: entrypoints`);
    assert.ok(Array.isArray(step.after), `${step.id}: after`);
    for (const dep of step.after) {
      assert.ok(steps.getStep(dep), `${step.id}: after references unknown step "${dep}"`);
    }
  }
});

test('health records normalize junk instead of crashing doctor', () => {
  assert.strictEqual(health.normalizeHealth(null).state, STATE.UNKNOWN);
  assert.strictEqual(health.normalizeHealth(true).state, STATE.UNKNOWN);
  assert.strictEqual(health.normalizeHealth({ state: 'nonsense' }).state, STATE.UNKNOWN);
  const good = health.normalizeHealth(health.healthy('C', 's', { evidence: ['e'] }));
  assert.deepStrictEqual([good.state, good.code, good.evidence], [STATE.HEALTHY, 'C', ['e']]);
});

console.log('\nContext resolution:');

test('the context is frozen and derives rather than mutates', () => {
  const ctx = ctxWith();
  assert.ok(Object.isFrozen(ctx));
  const derived = steps.deriveContext(ctx, { selectedModel: '1.1b' });
  assert.strictEqual(derived.selectedModel, '1.1b');
  assert.notStrictEqual(ctx.selectedModel, '1.1b', 'the original must be untouched');
  assert.ok(Object.isFrozen(derived));
});

test('sudo on behalf of a user has no session and a different target home', () => {
  const { resolveUsers, resolveCapabilities } = require('../src/steps/context');
  const realGetuid = process.getuid;
  process.getuid = () => 0;
  try {
    const users = resolveUsers({ SUDO_USER: '__definitely_not_a_user__' }, 'linux');
    assert.strictEqual(users.elevatedForAnother, true);
    assert.strictEqual(users.targetUser, '__definitely_not_a_user__');
    assert.strictEqual(resolveCapabilities({}, users).session, false,
      'writing a unit into /root when the install belongs to someone else is the bug');
  } finally {
    process.getuid = realGetuid;
  }
});

test('network is declared, not probed', () => {
  const { resolveCapabilities } = require('../src/steps/context');
  assert.strictEqual(resolveCapabilities({}).network, true);
  assert.strictEqual(resolveCapabilities({ SWICTATION_OFFLINE: '1' }).network, false);
  assert.strictEqual(resolveCapabilities({ npm_config_offline: 'true' }).network, false);
});

console.log('\nconfig-reset / config-heal:');

const configStep = require('../src/steps/config');
const DEFAULTS = 'stt_model_override = "auto"\n';
const configCtx = () => ctxWith({ generateDefaultConfig: () => DEFAULTS });

function freshConfigDir() {
  fs.rmSync(getConfigDir(), { recursive: true, force: true });
  fs.mkdirSync(getConfigDir(), { recursive: true });
}

test('config-reset: absent config is unhealthy, healthy once run', () => {
  fs.rmSync(getConfigDir(), { recursive: true, force: true });
  assert.strictEqual(configReset.check(configCtx()).code, 'CONFIG_MISSING');
  configReset.run(configCtx());
  assert.strictEqual(configReset.check(configCtx()).state, STATE.HEALTHY);
});

test('config-reset: an unparseable config names the parse error as evidence', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'audio_device_index = null\n');
  const record = configReset.check(configCtx());
  assert.strictEqual(record.code, 'CONFIG_UNPARSEABLE');
  assert.strictEqual(record.evidence.length, 2, 'path plus the parser message');
});

// Regression fence (ADR-037 health round): a parseable config is HEALTHY even
// with an installer-written override present. Flagging its mere presence made
// every brand-new install report UNHEALTHY and prompted a pointless reset of a
// correct value. config-reset never judges override staleness — it can't do so
// honestly side-effect-free (gpu-info is persisted, not re-detected).
test('config-reset: a parseable config with an installer override is healthy', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b-coreml"\n');
  configStep.recordManagedOverride('1.1b-coreml');
  assert.strictEqual(configReset.check(configCtx()).state, STATE.HEALTHY);
  assert.strictEqual(configReset.check(configCtx()).code, 'CONFIG_OK');
});

// The installer override reverts to "auto" during the POSTINSTALL pre-download
// pass, so the models phase re-tests the hardware.
test('config-reset: run() resets an installer override to auto during postinstall', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b-gpu"\n');
  configStep.recordManagedOverride('1.1b-gpu');
  configReset.run(ctxWith({ generateDefaultConfig: () => DEFAULTS, mode: 'postinstall' }));
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /stt_model_override = "auto"/);
});

// Codex health-round fence: full `swictation setup` (mode=setup) must NOT churn
// a working installer override — there is no re-test to write it back.
test('config-reset: run() leaves an installer override untouched in setup mode', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b-coreml"\n');
  configStep.recordManagedOverride('1.1b-coreml');
  configReset.run(ctxWith({ generateDefaultConfig: () => DEFAULTS, mode: 'setup' }));
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /stt_model_override = "1\.1b-coreml"/);
});

test('config-reset: a user-authored override is never touched', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "0.6b-cpu"\n');
  configReset.run(configCtx());
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /stt_model_override = "0\.6b-cpu"/);
});

test('config-heal: a stale model path is unhealthy and names the key', () => {
  freshConfigDir();
  const defaultDir = path.join(getModelsDir(), 'parakeet-tdt-1.1b-onnx');
  fs.mkdirSync(defaultDir, { recursive: true });
  fs.writeFileSync(configStep.configPath(),
    `stt_1_1b_model_path = "${scratchHome}/gone/parakeet-tdt-1.1b-onnx"\n`);

  const record = configHeal.check(configCtx());
  assert.strictEqual(record.code, 'CONFIG_STALE_PATHS');
  assert.deepStrictEqual(record.evidence, ['stt_1_1b_model_path']);

  configHeal.run(configCtx());
  assert.strictEqual(configHeal.check(configCtx()).state, STATE.HEALTHY);
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /parakeet-tdt-1\.1b-onnx/);
  fs.rmSync(getModelsDir(), { recursive: true, force: true });
});

test('config-heal: a path the user pointed somewhere real is not "stale"', () => {
  freshConfigDir();
  const customDir = path.join(scratchHome, 'my-models', 'parakeet-tdt-1.1b-onnx');
  fs.mkdirSync(customDir, { recursive: true });
  fs.mkdirSync(path.join(getModelsDir(), 'parakeet-tdt-1.1b-onnx'), { recursive: true });
  fs.writeFileSync(configStep.configPath(), `stt_1_1b_model_path = "${customDir}"\n`);
  assert.strictEqual(configHeal.check(configCtx()).state, STATE.HEALTHY);
  fs.rmSync(getModelsDir(), { recursive: true, force: true });
});

test('config-heal never resets the override the post-download pass just wrote', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b"\n');
  configStep.recordManagedOverride('1.1b');
  configHeal.run(configCtx());
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /stt_model_override = "1\.1b"/,
    'the model was just verified against this hardware; resetting it would undo that');
});

console.log('\nservices step:');

const SYSTEMD_UNIT_DIR = path.join(scratchHome, '.config', 'systemd', 'user');
const UNIT_PATH = path.join(SYSTEMD_UNIT_DIR, 'swictation-daemon.service');

/** A real, NON-EMPTY file: a zero-byte library is not a library. */
function touchFile(name) {
  const dir = path.join(scratchHome, 'libs');
  fs.mkdirSync(dir, { recursive: true });
  const p = path.join(dir, name);
  fs.writeFileSync(p, 'not-really-a-library');
  return p;
}

function writeUnit({ ortLine, execStart }) {
  fs.mkdirSync(SYSTEMD_UNIT_DIR, { recursive: true });
  fs.writeFileSync(UNIT_PATH, [
    '[Service]',
    `ExecStart=${execStart}`,
    'Environment="RUST_LOG=info"',
    ortLine,
    '',
  ].filter(Boolean).join('\n'));
}

const linuxCtx = (binaryPaths) => ctxWith({ platform: 'linux', binaryPaths: binaryPaths || null });

test('services (linux): no unit file', () => {
  fs.rmSync(SYSTEMD_UNIT_DIR, { recursive: true, force: true });
  assert.strictEqual(servicesStep.check(linuxCtx()).code, 'SERVICES_UNIT_MISSING');
});

test('services (linux): a unit without ORT_DYLIB_PATH is the ADR-034 blank-output unit', () => {
  writeUnit({ execStart: touchFile('swictation-daemon'), ortLine: null });
  assert.strictEqual(servicesStep.check(linuxCtx()).code, 'SERVICES_NO_ORT');
});

test('services (linux): ORT_DYLIB_PATH pointing at a missing file', () => {
  writeUnit({
    execStart: touchFile('swictation-daemon'),
    ortLine: 'Environment="ORT_DYLIB_PATH=/definitely/not/here/libonnxruntime.so"',
  });
  assert.strictEqual(servicesStep.check(linuxCtx()).code, 'SERVICES_ORT_MISSING');
});

test('services (linux): a unit left pointing at the previous version binary', () => {
  const oldBin = touchFile('swictation-daemon');
  const lib = touchFile('libonnxruntime.so');
  writeUnit({ execStart: oldBin, ortLine: `Environment="ORT_DYLIB_PATH=${lib}"` });
  const record = servicesStep.check(linuxCtx({ daemon: path.join(scratchHome, 'v2', 'swictation-daemon') }));
  assert.strictEqual(record.code, 'SERVICES_EXEC_STALE',
    'an npm upgrade moves the platform package; the old unit no longer starts');
});

test('services (linux): a complete unit is healthy', () => {
  const bin = touchFile('swictation-daemon');
  const lib = touchFile('libonnxruntime.so');
  writeUnit({ execStart: bin, ortLine: `Environment="ORT_DYLIB_PATH=${lib}"` });
  assert.strictEqual(servicesStep.check(linuxCtx({ daemon: bin })).state, STATE.HEALTHY);
});

const LAUNCH_AGENTS_DIR = path.join(scratchHome, 'Library', 'LaunchAgents');
const PLIST_PATH = path.join(LAUNCH_AGENTS_DIR, 'com.swictation.daemon.plist');
const darwinCtx = () => ctxWith({ platform: 'darwin' });

function writePlist({ program, ortPath, keepPlaceholders = false }) {
  fs.mkdirSync(LAUNCH_AGENTS_DIR, { recursive: true });
  let content = fs.readFileSync(
    path.join(PKG_ROOT, 'templates', 'macos', 'com.swictation.daemon.plist'), 'utf8');
  if (!keepPlaceholders) {
    content = content
      .replace(/\{\{DAEMON_PATH\}\}/g, program)
      .replace(/\{\{ORT_DYLIB_PATH\}\}/g, ortPath)
      .replace(/\{\{LOG_DIR\}\}/g, path.join(scratchHome, 'logs'))
      .replace(/\{\{HOME\}\}/g, scratchHome);
  }
  fs.writeFileSync(PLIST_PATH, content);
}

test('services (darwin): no plist', () => {
  fs.rmSync(LAUNCH_AGENTS_DIR, { recursive: true, force: true });
  assert.strictEqual(servicesStep.check(darwinCtx()).code, 'SERVICES_PLIST_MISSING');
});

test('services (darwin): unsubstituted template placeholders', () => {
  writePlist({ keepPlaceholders: true });
  assert.strictEqual(servicesStep.check(darwinCtx()).code, 'SERVICES_PLIST_TEMPLATE');
});

test('services (darwin): a missing launcher wrapper (SIP strips DYLD_*)', () => {
  writePlist({ program: path.join(scratchHome, 'gone', 'launcher'), ortPath: touchFile('libonnxruntime.dylib') });
  assert.strictEqual(servicesStep.check(darwinCtx()).code, 'SERVICES_LAUNCHER_MISSING');
});

test('services (darwin): ORT_DYLIB_PATH pointing at a missing dylib', () => {
  writePlist({ program: touchFile('launcher'), ortPath: '/definitely/not/here/libonnxruntime.dylib' });
  assert.strictEqual(servicesStep.check(darwinCtx()).code, 'SERVICES_ORT_MISSING');
});

test('services (darwin): launcher + real dylib is healthy', () => {
  writePlist({ program: touchFile('launcher'), ortPath: touchFile('libonnxruntime.dylib') });
  assert.strictEqual(servicesStep.check(darwinCtx()).state, STATE.HEALTHY);
});

test('services: a zero-byte ONNX Runtime is not a runtime', () => {
  const emptyLib = path.join(scratchHome, 'libs', 'empty-libonnxruntime.dylib');
  fs.mkdirSync(path.dirname(emptyLib), { recursive: true });
  fs.writeFileSync(emptyLib, '');
  writePlist({ program: touchFile('launcher'), ortPath: emptyLib });
  assert.strictEqual(servicesStep.check(darwinCtx()).code, 'SERVICES_ORT_MISSING',
    'an interrupted download leaves an empty file that existsSync accepts');
});

test('services reads the paths the CONTEXT resolved, not os.homedir()', () => {
  // Under sudo those differ; a step that reads one while writing the other
  // reports healthy over an install that does not work.
  const elsewhere = path.join(scratchHome, 'other-home');
  const plistPath = path.join(elsewhere, 'Library', 'LaunchAgents', 'com.swictation.daemon.plist');
  const ctx = ctxWith({ platform: 'darwin', daemonPlistPath: plistPath });
  const record = servicesStep.check(ctx);
  assert.strictEqual(record.code, 'SERVICES_PLIST_MISSING');
  assert.ok(record.evidence[0].startsWith(elsewhere),
    `evidence must name the context path, got ${record.evidence[0]}`);
});

test('services: an unresolvable target home is unknown, not healthy or missing', () => {
  const ctx = ctxWith({ platform: 'darwin', daemonPlistPath: null });
  const record = servicesStep.check(ctx);
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'SERVICES_NO_TARGET_HOME');
});

console.log('\nmodels step:');

/**
 * Materialize a model tree at the exact sizes models.manifest.json declares,
 * using sparse files: isModelDownloaded() stats every file, so this exercises
 * the real manifest-aware predicate without writing gigabytes.
 */
function fabricateModel(key, { corruptFirstFile = false } = {}) {
  const entry = MANIFEST.models[key];
  assert.ok(entry, `manifest has no entry for ${key}`);
  const root = path.join(getModelsDir(), entry.targetDir);
  entry.files.forEach((file, index) => {
    const target = path.join(root, file.path);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, '');
    fs.truncateSync(target, corruptFirstFile && index === 0 ? Math.max(0, file.size - 1) : file.size);
  });
}

function clearModels() {
  fs.rmSync(getModelsDir(), { recursive: true, force: true });
}

const modelCtx = (selectedModel) => ctxWith({ selectedModel, gpuInfo: null });

test('models: no recommendation recorded is unknown, not unhealthy', () => {
  clearModels();
  const record = modelsStep.check(modelCtx(null));
  assert.strictEqual(record.state, STATE.UNKNOWN,
    'nothing is broken — we simply have no record of what this machine chose');
  assert.strictEqual(record.code, 'MODELS_NO_SELECTION');
});

test('models: a recommended model that is absent', () => {
  clearModels();
  const record = modelsStep.check(modelCtx('1.1b-coreml'));
  assert.strictEqual(record.code, 'MODELS_MISSING');
  assert.match(record.summary, /vad, 1\.1b-coreml/);
});

test('models: cpu-only requires the 0.6b model, not nothing', () => {
  clearModels();
  const record = modelsStep.check(modelCtx('cpu-only'));
  assert.strictEqual(record.code, 'MODELS_MISSING');
  assert.match(record.summary, /0\.6b/,
    'install has always fetched a model for cpu-only — it just did it invisibly, late');
});

test('models: VAD alone is not enough — the daemon needs both', () => {
  clearModels();
  fabricateModel('vad');
  assert.match(modelsStep.check(modelCtx('1.1b-coreml')).summary, /missing model\(s\): 1\.1b-coreml/);
});

test('models: VAD + the recommended model at manifest sizes is size-verified', () => {
  clearModels();
  fabricateModel('vad');
  fabricateModel('1.1b-coreml');
  const record = modelsStep.check(modelCtx('1.1b-coreml'));
  assert.strictEqual(record.state, STATE.HEALTHY);
  assert.strictEqual(record.code, 'MODELS_SIZE_VERIFIED');
  assert.match(record.summary, /size-verified/,
    'the label must say what was actually proven — sizes, not contents');
});

test('models: a truncated file fails the check (the half-written-tree bug)', () => {
  clearModels();
  fabricateModel('vad');
  fabricateModel('1.1b-coreml', { corruptFirstFile: true });
  assert.strictEqual(modelsStep.check(modelCtx('1.1b-coreml')).state, STATE.UNHEALTHY,
    'existence is not enough — sizes must match the manifest');
});

test('models: requiredKeys always includes VAD, and is null for an unknown key', () => {
  assert.deepStrictEqual(modelsStep._internals.requiredKeys('1.1b-gpu'), ['vad', '1.1b']);
  assert.deepStrictEqual(modelsStep._internals.requiredKeys('cpu-only'), ['vad', '0.6b']);
  assert.strictEqual(modelsStep._internals.requiredKeys('nonsense'), null);
});

test('models: an unrecognized selection fails CLOSED, even with VAD on disk', () => {
  clearModels();
  fabricateModel('vad');
  const record = modelsStep.check(modelCtx('parakeet-tdt-9.9b-imaginary'));
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'MODELS_UNKNOWN_SELECTION',
    'a 629 KB VAD file must never vouch for a machine with no speech model');
});

test('models never reports healthy with zero speech models on disk', () => {
  // The adjudicated invariant, stated directly.
  clearModels();
  for (const selection of ['cpu-only', '0.6b-gpu', '1.1b-coreml', 'nonsense', null]) {
    const record = modelsStep.check(modelCtx(selection));
    assert.notStrictEqual(record.state, STATE.HEALTHY, `selection=${selection}`);
  }
  fabricateModel('vad');
  for (const selection of ['cpu-only', '1.1b-coreml']) {
    assert.notStrictEqual(modelsStep.check(modelCtx(selection)).state, STATE.HEALTHY,
      `VAD alone must not make ${selection} healthy`);
  }
});

console.log('\ngpu-libs step:');

const postinstall = require('../postinstall');
const RECEIPT_PATH = path.join(getConfigDir(), 'gpu-package-info.json');
const SENTINEL_PATH = path.join(getGpuLibsDir(), 'libonnxruntime.so');

/** The variant is a RESOLVED CONTEXT FACT — the step must never re-probe it. */
const gpuCtx = (gpuVariant) => ctxWith({ platform: 'linux', hasNvidiaGpu: true, gpuVariant });

function writeReceipt(receipt) {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  fs.writeFileSync(RECEIPT_PATH, JSON.stringify(receipt));
}

/** A non-empty sentinel: the zero-byte case has its own test below. */
function writeSentinel() {
  fs.mkdirSync(getGpuLibsDir(), { recursive: true });
  fs.writeFileSync(SENTINEL_PATH, 'not-really-a-library');
}

function clearGpuLibs() {
  fs.rmSync(RECEIPT_PATH, { force: true });
  fs.rmSync(getGpuLibsDir(), { recursive: true, force: true });
}

// sm_86 selects the "modern" variant; assert that rather than hardcoding it,
// so the test tracks the shipped table instead of duplicating it.
const MODERN = postinstall.selectGPUPackageVariant(86).variant;

test('gpu-libs: not applicable on macOS, and on Linux without an NVIDIA card', () => {
  assert.strictEqual(steps.appliesTo(gpuLibsStep, ctxWith({ platform: 'darwin' })), false);
  assert.strictEqual(
    steps.appliesTo(gpuLibsStep, ctxWith({ platform: 'linux', hasNvidiaGpu: false })), false,
    'a CPU-only Linux box has nothing to repair — doctor must not nag it forever');
  assert.strictEqual(steps.appliesTo(gpuLibsStep, gpuCtx(MODERN)), true);
});

test('gpu-libs: the variant comes from the context, never a fresh probe', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  writeSentinel();
  // If check() re-probed the hardware it would call this and blow up.
  const saved = postinstall.detectGPUComputeCapability;
  postinstall.detectGPUComputeCapability = () => {
    throw new Error('check() must not probe: the variant is a resolved ctx fact');
  };
  try {
    assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).state, STATE.UNKNOWN);
  } finally {
    postinstall.detectGPUComputeCapability = saved;
  }
});

test('gpu-libs: GPU present, no receipt', () => {
  clearGpuLibs();
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_NO_RECEIPT');
});

test('gpu-libs: a receipt for an older package version', () => {
  clearGpuLibs();
  writeReceipt({ version: '0.0.1', variant: MODERN });
  writeSentinel();
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_STALE_VERSION');
});

test('gpu-libs: the ADR-035 bug — receipt survived, libraries did not', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  const record = gpuLibsStep.check(gpuCtx(MODERN));
  assert.strictEqual(record.code, 'GPULIBS_MISSING');
  assert.strictEqual(record.state, STATE.UNHEALTHY,
    'the receipt must never vouch for absent goods');
});

test('gpu-libs: a zero-byte sentinel is not a library', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  fs.mkdirSync(getGpuLibsDir(), { recursive: true });
  fs.writeFileSync(SENTINEL_PATH, '');
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_MISSING',
    'an interrupted copy leaves an empty file that existsSync happily accepts');
});

test('gpu-libs: a directory named like the sentinel is not a library either', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  fs.mkdirSync(SENTINEL_PATH, { recursive: true });
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_MISSING');
  fs.rmSync(SENTINEL_PATH, { recursive: true, force: true });
});

test('gpu-libs: libraries built for a different architecture', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: 'legacy' });
  writeSentinel();
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_WRONG_VARIANT');
});

test('gpu-libs: an undetermined variant fails CLOSED into unknown, never passes', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  writeSentinel();
  const record = gpuLibsStep.check(gpuCtx(null));
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'GPULIBS_VARIANT_UNKNOWN',
    'accepting whatever is installed when the probe failed is the receipt vouching for itself');
});

test('gpu-libs: a matching receipt plus sentinel, with no manifest, is UNKNOWN', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  writeSentinel();
  const record = gpuLibsStep.check(gpuCtx(MODERN));
  assert.strictEqual(record.state, STATE.UNKNOWN,
    'one sentinel .so cannot vouch for forty extracted files (ADR-037 amendment 7)');
  assert.strictEqual(record.code, 'GPULIBS_UNVERIFIED');
  assert.ok(record.evidence.some(e => /manifest/.test(e)),
    'the report must say why it cannot know, not just that it does not');
});

test('gpu-libs can never report healthy WITHOUT a per-file manifest', () => {
  // Amendment 7 in its strongest form, and it survives Phase B unchanged:
  // the manifest is the ONLY thing that may promote this row to healthy, so
  // no future edit can green it on the strength of a receipt again.
  const fixtures = [
    () => clearGpuLibs(),
    () => { writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN }); writeSentinel(); },
    () => { writeReceipt({ version: '0.0.1', variant: MODERN }); writeSentinel(); },
  ];
  for (const setUp of fixtures) {
    clearGpuLibs();
    setUp();
    for (const variant of [MODERN, null, undefined]) {
      assert.notStrictEqual(gpuLibsStep.check(gpuCtx(variant)).state, STATE.HEALTHY);
    }
  }
});

console.log('\ngpu-libs per-file manifest (ADR-037 Phase B):');

const gpuLibsManifest = require('../src/gpu-libs-manifest');

/**
 * A library set plus the manifest that describes it, both written the way
 * downloadGPULibraries() writes them: the manifest's hashes come from
 * copyAndHash() streaming the real bytes, never from the fixture restating
 * what it just wrote. A fixture that computed the hash a second way could
 * agree with a check that was also wrong.
 */
async function installGpuLibs({ names = ['libonnxruntime.so', 'libcudnn.so.9'], variant = MODERN, version = postinstall.GPU_LIBS_VERSION } = {}) {
  clearGpuLibs();
  const staging = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-gpu-src-'));
  fs.mkdirSync(getGpuLibsDir(), { recursive: true });
  const files = [];
  for (const name of names) {
    const src = path.join(staging, name);
    fs.writeFileSync(src, `ELF-ish contents of ${name}`.repeat(4));
    files.push(await gpuLibsManifest.copyAndHash(src, path.join(getGpuLibsDir(), name), name));
  }
  gpuLibsManifest.writeManifest(getGpuLibsDir(), { variant, version, files });
  writeReceipt({ version, variant });
  fs.rmSync(staging, { recursive: true, force: true });
  return files;
}

await asyncTest('gpu-libs: a complete manifested set is HEALTHY at last', async () => {
  await installGpuLibs();
  const record = gpuLibsStep.check(gpuCtx(MODERN));
  assert.strictEqual(record.state, STATE.HEALTHY);
  assert.strictEqual(record.code, 'GPULIBS_VERIFIED');
  assert.match(record.summary, /2 files/);
  assert.ok(record.evidence.some(e => /doctor --deep/.test(e)),
    'a size-only verdict must say so, exactly as the models step does');
});

await asyncTest('gpu-libs: a deleted library breaks the manifest, not just the sentinel', async () => {
  await installGpuLibs();
  fs.rmSync(path.join(getGpuLibsDir(), 'libcudnn.so.9'));
  const record = gpuLibsStep.check(gpuCtx(MODERN));
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'GPULIBS_INCOMPLETE',
    'the sentinel is still present — only the manifest can see this');
  assert.ok(record.evidence.some(e => /libcudnn\.so\.9/.test(e)));
});

await asyncTest('gpu-libs: a truncated library is caught by size alone', async () => {
  await installGpuLibs();
  fs.truncateSync(path.join(getGpuLibsDir(), 'libcudnn.so.9'), 3);
  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).code, 'GPULIBS_INCOMPLETE');
});

await asyncTest('gpu-libs: a manifest describing a different variant than the receipt', async () => {
  await installGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: 'legacy' });
  // The context wants what the receipt claims, so the variant check passes
  // and the two records' disagreement is the only thing left to catch it.
  const record = gpuLibsStep.check(gpuCtx('legacy'));
  assert.strictEqual(record.code, 'GPULIBS_MANIFEST_MISMATCH');
  assert.strictEqual(record.state, STATE.UNHEALTHY);
});

await asyncTest('gpu-libs --deep: same-size different-contents is invisible to check(), caught by deepCheck()', async () => {
  const files = await installGpuLibs();
  const target = path.join(getGpuLibsDir(), 'libcudnn.so.9');
  const size = files.find(f => f.path === 'libcudnn.so.9').size;
  fs.writeFileSync(target, 'X'.repeat(size));

  assert.strictEqual(gpuLibsStep.check(gpuCtx(MODERN)).state, STATE.HEALTHY,
    'sizes match, so the standard check cannot and does not claim otherwise');
  const deep = await gpuLibsStep.deepCheck(gpuCtx(MODERN));
  assert.strictEqual(deep.state, STATE.UNHEALTHY);
  assert.strictEqual(deep.code, 'GPULIBS_CORRUPT');
  assert.ok(deep.evidence.some(e => /libcudnn\.so\.9/.test(e)));
});

await asyncTest('gpu-libs --deep: an intact set verifies by content', async () => {
  await installGpuLibs();
  const deep = await gpuLibsStep.deepCheck(gpuCtx(MODERN));
  assert.strictEqual(deep.state, STATE.HEALTHY);
  assert.strictEqual(deep.code, 'GPULIBS_HASH_VERIFIED');
});

await asyncTest('gpu-libs --deep never upgrades a verdict the standard check rejected', async () => {
  clearGpuLibs();
  const deep = await gpuLibsStep.deepCheck(gpuCtx(MODERN));
  assert.strictEqual(deep.code, 'GPULIBS_NO_RECEIPT',
    '--deep is a stronger check, never a second opinion');
});

test('the manifest lives beside the libraries, so it cannot outlive them', () => {
  assert.strictEqual(
    path.dirname(gpuLibsManifest.manifestPath(getGpuLibsDir())), getGpuLibsDir(),
    'a manifest in the config dir would repeat the ADR-035 receipt bug exactly');
  assert.notStrictEqual(path.dirname(gpuLibsManifest.manifestPath(getGpuLibsDir())), getConfigDir());
});

test('an empty or malformed manifest reads as absent, never as "nothing to check"', () => {
  // `files: []` would make every `.every()` vacuously true — a manifest that
  // vouches for a directory by describing none of it.
  fs.mkdirSync(getGpuLibsDir(), { recursive: true });
  for (const junk of ['', 'not json', '{}', '{"files":[]}']) {
    fs.writeFileSync(gpuLibsManifest.manifestPath(getGpuLibsDir()), junk);
    assert.strictEqual(gpuLibsManifest.readManifest(getGpuLibsDir()), null, junk);
  }
  clearGpuLibs();
});

await asyncTest('models --deep: sizes pass while contents do not', async () => {
  clearModels();
  fabricateModel('vad');
  fabricateModel('1.1b-coreml');
  assert.strictEqual(modelsStep.check(modelCtx('1.1b-coreml')).state, STATE.HEALTHY);

  // Sparse files are all zero bytes, so they are the right SIZE and the wrong
  // CONTENT — exactly the case --deep exists for, and the reason the shallow
  // check labels itself size-verified instead of verified.
  const deep = await modelsStep.deepCheck(modelCtx('1.1b-coreml'));
  assert.strictEqual(deep.state, STATE.UNHEALTHY);
  assert.strictEqual(deep.code, 'MODELS_CORRUPT');
  assert.ok(deep.evidence.length > 0, 'the failing files must be named');
});

await asyncTest('models --deep never upgrades a verdict the standard check rejected', async () => {
  clearModels();
  const deep = await modelsStep.deepCheck(modelCtx('1.1b-coreml'));
  assert.strictEqual(deep.code, 'MODELS_MISSING');
});

console.log('\nplatform step:');

const platformStep = require('../src/steps/platform');
const { inspect: inspectPlatform } = platformStep._internals;
const okProbe = { glibc: () => ({ major: 2, minor: 39 }), macos: () => '14.4' };

function makeInstallDirs() {
  for (const dir of platformStep._internals.requiredDirs()) fs.mkdirSync(dir, { recursive: true });
}

test('platform: an unsupported OS is unhealthy, not "not applicable"', () => {
  const record = inspectPlatform(ctxWith({ platform: 'win32', arch: 'x64' }), okProbe);
  assert.strictEqual(record.state, STATE.UNHEALTHY,
    'not-applicable means "fine without it" — that would paint a Windows box green');
  assert.strictEqual(record.code, 'PLATFORM_UNSUPPORTED_OS');
  assert.match(record.repair, /no repair is possible/,
    'naming a command that cannot help is worse than saying there is none');
});

test('platform: an Intel Mac and a non-x64 Linux are both wrong-arch', () => {
  assert.strictEqual(
    inspectPlatform(ctxWith({ platform: 'darwin', arch: 'x64' }), okProbe).code,
    'PLATFORM_UNSUPPORTED_ARCH');
  assert.strictEqual(
    inspectPlatform(ctxWith({ platform: 'linux', arch: 'arm64' }), okProbe).code,
    'PLATFORM_UNSUPPORTED_ARCH');
});

test('platform: the step reports; only the driver decides an unsupported box stops', () => {
  // The legacy checkPlatform() called process.exit() from three branches,
  // which is why doctor could never reuse it.
  const record = inspectPlatform(ctxWith({ platform: 'win32', arch: 'x64' }), okProbe);
  assert.ok(platformStep._internals.FATAL_CODES.has(record.code));
  assert.ok(!platformStep._internals.FATAL_CODES.has('PLATFORM_GLIBC_OLD'),
    'an old distribution still installs — it always has');
});

test('platform: GLIBC older than 2.39 is unhealthy, not a warning', () => {
  const record = inspectPlatform(
    ctxWith({ platform: 'linux', arch: 'x64' }),
    { ...okProbe, glibc: () => ({ major: 2, minor: 35 }) });
  assert.strictEqual(record.code, 'PLATFORM_GLIBC_OLD');
  assert.match(record.summary, /2\.35/);
});

test('platform: an unreadable GLIBC version is unknown, never assumed good', () => {
  const record = inspectPlatform(
    ctxWith({ platform: 'linux', arch: 'x64' }), { ...okProbe, glibc: () => null });
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'PLATFORM_GLIBC_UNKNOWN');
});

test('platform: an UNPARSEABLE macOS version is unknown, not silently supported', () => {
  // `sw_vers` exiting 0 with garbage is not the same as `sw_vers` failing.
  // parseInt('garbage') is NaN, NaN < 14 is false, and the version gate used
  // to fall straight through to PLATFORM_OK — the check vouching for an OS
  // it could not read at all.
  for (const garbage of ['garbage', 'Version', '.', 'macOS']) {
    const record = inspectPlatform(
      ctxWith({ platform: 'darwin', arch: 'arm64' }), { ...okProbe, macos: () => garbage });
    assert.notStrictEqual(record.state, STATE.HEALTHY, `sw_vers said: ${garbage}`);
    assert.strictEqual(record.state, STATE.UNKNOWN, `sw_vers said: ${garbage}`);
  }
});

test('platform: macOS older than Sonoma is unhealthy; unreadable is unknown', () => {
  assert.strictEqual(
    inspectPlatform(ctxWith({ platform: 'darwin', arch: 'arm64' }), { ...okProbe, macos: () => '13.6' }).code,
    'PLATFORM_MACOS_OLD');
  assert.strictEqual(
    inspectPlatform(ctxWith({ platform: 'darwin', arch: 'arm64' }), { ...okProbe, macos: () => null }).state,
    STATE.UNKNOWN);
});

test('platform: missing install directories are the one repairable failure here', () => {
  for (const dir of platformStep._internals.requiredDirs()) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  const supported = ctxWith({ platform: process.platform, arch: process.arch });
  const record = inspectPlatform(supported, okProbe);
  assert.strictEqual(record.code, 'PLATFORM_DIRS_MISSING');
  assert.ok(record.evidence.length > 0);

  makeInstallDirs();
  assert.strictEqual(inspectPlatform(supported, okProbe).state, STATE.HEALTHY);
});

test('platform: an unsupported machine is never told to create directories', () => {
  for (const dir of platformStep._internals.requiredDirs()) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  const result = platformStep.run(ctxWith({ platform: 'win32', arch: 'x64' }));
  assert.deepStrictEqual(result.components.map(c => c.status), ['failed']);
  assert.strictEqual(platformStep._internals.missingDirs().length,
    platformStep._internals.requiredDirs().length,
    'creating directories on a machine the binaries cannot run on is litter');
  makeInstallDirs();
});

console.log('\nbinaries step:');

const binariesStep = require('../src/steps/binaries');
const { inspect: inspectBinaries } = binariesStep._internals;

/** A 16-byte Mach-O 64 header — enough to name the architecture, nothing more. */
function machoHeader(cpuType) {
  const header = Buffer.alloc(32);
  header.writeUInt32LE(0xfeedfacf, 0);
  header.writeInt32LE(cpuType, 4);
  return header;
}

/** The same for ELF64 little-endian: magic, class, and e_machine at 18. */
function elfHeader(machine) {
  const header = Buffer.alloc(64);
  Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01]).copy(header, 0);
  header.writeUInt16LE(machine, 18);
  return header;
}

const CPU_ARM64 = 0x0100000c;
const CPU_X86_64 = 0x01000007;
const EM_X86_64 = 62;
const EM_AARCH64 = 183;

/** A binary this host can actually load, so fixtures test what they name. */
function nativeHeader() {
  if (process.platform === 'darwin') {
    return machoHeader(process.arch === 'arm64' ? CPU_ARM64 : CPU_X86_64);
  }
  return elfHeader(process.arch === 'arm64' ? EM_AARCH64 : EM_X86_64);
}

function fakePlatformPackage({ daemonMode = 0o755, ui = false, daemonBytes = nativeHeader() } = {}) {
  const root = path.join(scratchHome, 'platform-pkg');
  fs.mkdirSync(path.join(root, 'bin'), { recursive: true });
  const daemon = path.join(root, 'bin', 'swictation-daemon');
  fs.writeFileSync(daemon, daemonBytes);
  fs.chmodSync(daemon, daemonMode);
  const uiPath = path.join(root, 'bin', 'swictation-ui');
  if (ui) {
    fs.writeFileSync(uiPath, '#!/bin/sh\n');
    fs.chmodSync(uiPath, 0o755);
  } else {
    fs.rmSync(uiPath, { force: true });
  }
  return { packageName: '@agidreams/fixture', daemon, ui: uiPath, libDir: root, binDir: path.join(root, 'bin') };
}

test('binaries: no platform package names the npm behaviour that causes it', () => {
  const record = inspectBinaries(ctxWith({ binaryPaths: null }), () => null);
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'BINARIES_NO_PLATFORM_PKG');
  assert.match(record.repair, /npm install -g swictation/);
});

test('binaries: a present-but-non-executable daemon is not a working install', () => {
  const paths = fakePlatformPackage({ daemonMode: 0o644 });
  const record = inspectBinaries(ctxWith(), () => paths);
  assert.strictEqual(record.code, 'BINARIES_UNUSABLE');
  assert.match(record.summary, /not executable/,
    'an existence check calls this healthy while every launch fails with EACCES');
});

test('binaries: a zero-byte daemon is not a daemon', () => {
  const paths = fakePlatformPackage();
  fs.writeFileSync(paths.daemon, '');
  fs.chmodSync(paths.daemon, 0o755);
  assert.strictEqual(inspectBinaries(ctxWith(), () => paths).code, 'BINARIES_UNUSABLE');
});

test('binaries: the UI is optional — its absence is evidence, never a failure', () => {
  const withoutUi = inspectBinaries(ctxWith(), () => fakePlatformPackage({ ui: false }));
  assert.strictEqual(withoutUi.state, STATE.HEALTHY);
  assert.ok(withoutUi.evidence.some(e => /ui: not installed/.test(e)));

  const withUi = inspectBinaries(ctxWith(), () => fakePlatformPackage({ ui: true }));
  assert.strictEqual(withUi.state, STATE.HEALTHY);
  assert.ok(withUi.evidence.some(e => /swictation-ui/.test(e)));
});

test('binaries reads DISK, not the context snapshot run() may have invalidated', () => {
  // The inverse of the services rule, and deliberate: run() can install the
  // package, so a check consulting a pre-run snapshot would report the step
  // failed immediately after it succeeded.
  const staleCtx = ctxWith({ binaryPaths: null });
  const record = inspectBinaries(staleCtx, () => fakePlatformPackage());
  assert.strictEqual(record.state, STATE.HEALTHY,
    'disk wins over a snapshot taken before the package was installed');
});

test('binaries: a package that vanished AFTER the context was built is unhealthy', () => {
  // The mirror of the test above, and the one that matters: `|| ctx.binaryPaths`
  // fell back to the very snapshot the comment beside it refuses to trust. An
  // upgrade that removes the platform package mid-install left the snapshot
  // pointing at paths that no longer exist and the check said BINARIES_OK.
  const ctx = ctxWith({ binaryPaths: fakePlatformPackage() });
  const record = inspectBinaries(ctx, () => null);
  assert.strictEqual(record.state, STATE.UNHEALTHY,
    'disk said the package is gone; a remembered snapshot must not overrule it');
  assert.strictEqual(record.code, 'BINARIES_NO_PLATFORM_PKG');
});

const daemonWithHeader = (header) => fakePlatformPackage({ daemonBytes: header });

test('binaries: a daemon built for the wrong architecture is not a working install', () => {
  // exists + non-empty + executable says nothing about whether the kernel can
  // load it. An x86_64 daemon on Apple Silicon passes all three and then dies
  // with ENOEXEC on every launch, which no user can tell from "it hangs".
  const wrong = ctxWith({ platform: 'darwin', arch: 'arm64' });
  const record = inspectBinaries(wrong, () => daemonWithHeader(machoHeader(CPU_X86_64)));
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'BINARIES_WRONG_ARCH');
  assert.match(record.summary, /x86_64/);
});

test('binaries: a matching architecture still passes, on both platforms', () => {
  assert.strictEqual(
    inspectBinaries(ctxWith({ platform: 'darwin', arch: 'arm64' }),
      () => daemonWithHeader(machoHeader(CPU_ARM64))).state,
    STATE.HEALTHY);
  assert.strictEqual(
    inspectBinaries(ctxWith({ platform: 'linux', arch: 'x64' }),
      () => daemonWithHeader(elfHeader(EM_X86_64))).state,
    STATE.HEALTHY);
  assert.strictEqual(
    inspectBinaries(ctxWith({ platform: 'linux', arch: 'x64' }),
      () => daemonWithHeader(elfHeader(EM_AARCH64))).code,
    'BINARIES_WRONG_ARCH');
});

await asyncTest('binaries --deep: a header that loads is not a daemon that runs', async () => {
  // The header proves the kernel would accept the image. It cannot prove the
  // dynamic linker resolves every symbol — a daemon built against a newer
  // GLIBC, or missing its ONNX Runtime, passes the header check and dies.
  const { inspectDeep } = binariesStep._internals;
  const onDisk = () => fakePlatformPackage();

  const dead = await inspectDeep(ctxWith(), async () => ({ status: 'failed', reason: 'ENOEXEC' }), onDisk);
  assert.strictEqual(dead.state, STATE.UNHEALTHY);
  assert.strictEqual(dead.code, 'BINARIES_NOT_LOADABLE');

  const hung = await inspectDeep(ctxWith(), async () => ({ status: 'timeout' }), onDisk);
  assert.strictEqual(hung.state, STATE.UNKNOWN, 'a hung daemon must not hang doctor OR fail it');
  assert.strictEqual(hung.code, 'BINARIES_PROBE_TIMEOUT');

  const alive = await inspectDeep(ctxWith(), async () => ({ status: 'ok', output: 'swictation 1.2.3' }), onDisk);
  assert.strictEqual(alive.code, 'BINARIES_RUNS');

  // And it must not vouch for a package that is not there at all.
  const absent = await inspectDeep(ctxWith(), async () => ({ status: 'ok' }), () => null);
  assert.strictEqual(absent.code, 'BINARIES_NO_PLATFORM_PKG');
});

test('binaries: an unrecognized executable format is unknown, never assumed loadable', () => {
  const record = inspectBinaries(ctxWith(),
    () => daemonWithHeader(Buffer.from('#!/bin/sh\necho hi\n')));
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'BINARIES_FORMAT_UNKNOWN');
});

await asyncTest('binaries: the nested npm repair is postinstall-only', async () => {
  // From `swictation setup` a global npm install can deadlock on the global
  // lock and rewrites a tree the user did not ask us to touch.
  const npmCalls = [];
  const realExecSync = require('child_process').execSync;
  require('child_process').execSync = (cmd, ...rest) => {
    if (/npm install/.test(cmd)) { npmCalls.push(cmd); return ''; }
    return realExecSync(cmd, ...rest);
  };
  const realIsInstalled = require('../src/resolve-binary').isPlatformPackageInstalled;
  require('../src/resolve-binary').isPlatformPackageInstalled = () => false;
  try {
    const setupResult = await binariesStep.run(ctxWith({ mode: 'setup' }));
    assert.strictEqual(npmCalls.length, 0, 'setup must never spawn a global npm install');
    assert.ok(setupResult.warnings.some(w => /npm install -g swictation/.test(w)),
      'it names the command instead of running it');
    assert.deepStrictEqual(
      setupResult.components.filter(c => c.id === 'platform-package').map(c => c.status), ['skipped']);
  } finally {
    require('child_process').execSync = realExecSync;
    require('../src/resolve-binary').isPlatformPackageInstalled = realIsInstalled;
  }
});

console.log('\ncleanup step:');

const cleanupStep = require('../src/steps/cleanup');

test('cleanup never runs from `swictation setup`', () => {
  assert.deepStrictEqual(cleanupStep.entrypoints, ['postinstall'],
    'deleting another installation\'s files is an install\'s business, not a diagnostic\'s');
  assert.ok(!steps.selectSteps({ entrypoint: 'setup' }).some(s => s.id === 'cleanup'));
  assert.ok(steps.selectSteps({ entrypoint: 'postinstall' }).some(s => s.id === 'cleanup'));
});

test('cleanup: a Python-era systemd unit is a shadow worth naming', () => {
  const linuxCtx2 = ctxWith({ platform: 'linux', users: {
    uid: 1000, isRoot: false, effectiveUser: 'u', targetUser: 'u',
    targetHome: scratchHome, elevatedForAnother: false,
  } });
  const unit = path.join(scratchHome, '.config', 'systemd', 'user', 'swictation.service');
  fs.mkdirSync(path.dirname(unit), { recursive: true });
  fs.writeFileSync(unit, '[Service]\n');

  const record = cleanupStep.check(linuxCtx2);
  assert.strictEqual(record.code, 'CLEANUP_LEGACY_ARTIFACTS');
  assert.ok(record.evidence.some(e => e.includes('swictation.service')));

  fs.rmSync(unit);
  assert.strictEqual(cleanupStep.check(linuxCtx2).state, STATE.HEALTHY);
});

test('cleanup: the plists THIS version generates are not "legacy"', () => {
  // Listing them would make the check permanently red the moment the services
  // step succeeded.
  fs.mkdirSync(LAUNCH_AGENTS_DIR, { recursive: true });
  fs.writeFileSync(path.join(LAUNCH_AGENTS_DIR, 'com.swictation.daemon.plist'), '<plist/>');
  fs.writeFileSync(path.join(LAUNCH_AGENTS_DIR, 'com.swictation.ui.plist'), '<plist/>');
  const ctx = ctxWith({ platform: 'darwin', users: {
    uid: 501, isRoot: false, effectiveUser: 'u', targetUser: 'u',
    targetHome: scratchHome, elevatedForAnother: false,
  } });
  assert.deepStrictEqual(cleanupStep._internals.legacyPlists(scratchHome, 'darwin'), []);

  const stale = path.join(LAUNCH_AGENTS_DIR, 'com.swictation.tray.plist');
  fs.writeFileSync(stale, '<plist/>');
  assert.strictEqual(cleanupStep.check(ctx).code, 'CLEANUP_LEGACY_ARTIFACTS');
  fs.rmSync(stale);
});

test('cleanup: python versions are discovered, not hardcoded', () => {
  // The list stopped at 3.13. Python 3.14 shipped; an onnxruntime 1.20 under
  // it shadows ours exactly as hard as one under 3.12, and the check simply
  // could not see it.
  const dir = path.join(scratchHome, '.local', 'lib', 'python3.14', 'site-packages', 'onnxruntime', 'capi');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'libonnxruntime.so.1.20.1'), '');
  const ctx = ctxWith({ platform: 'linux', users: {
    uid: 1000, isRoot: false, effectiveUser: 'u', targetUser: 'u',
    targetHome: scratchHome, elevatedForAnother: false,
  } });
  const record = cleanupStep.check(ctx);
  assert.strictEqual(record.code, 'CLEANUP_LEGACY_ARTIFACTS');
  assert.ok(record.evidence.some(e => e.includes('python3.14')), 'a future interpreter is still an interpreter');
  fs.rmSync(path.join(scratchHome, '.local', 'lib'), { recursive: true, force: true });
});

test('cleanup: an UNPARSEABLE onnxruntime version fails closed, not silent', () => {
  // A libonnxruntime whose version cannot be read is not "fine" — it is a
  // library of unknown vintage sitting ahead of ours on the linker path.
  // Returning false made it invisible to both the check and the repair.
  const dir = path.join(scratchHome, '.local', 'lib', 'python3.12', 'site-packages', 'onnxruntime', 'capi');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'libonnxruntime.so'), '');
  assert.strictEqual(
    cleanupStep._internals.hasConflictingOrt(path.dirname(dir), 'linux'), true,
    'unknown vintage must be treated as conflicting');
  fs.rmSync(path.join(scratchHome, '.local', 'lib'), { recursive: true, force: true });
});

test('cleanup: the check and the repair share ONE conflict predicate', () => {
  // Two copies of this rule is how a check starts asking for work the repair
  // refuses to do — a step that can never go green.
  assert.strictEqual(
    require('../postinstall')._ortConflictPredicate,
    cleanupStep._internals.hasConflictingOrt);
});

test('cleanup: a NEWER pip onnxruntime is left alone, so the check must not name it', () => {
  // run() only removes <1.22; reporting a 1.23 tree would ask for work the
  // repair refuses to do, and the step could never go green.
  const dir = path.join(scratchHome, '.local', 'lib', 'python3.12', 'site-packages', 'onnxruntime', 'capi');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'libonnxruntime.so.1.23.0'), '');
  assert.strictEqual(
    cleanupStep._internals.hasConflictingOrt(path.dirname(dir), 'linux'), false);

  fs.writeFileSync(path.join(dir, 'libonnxruntime.so.1.20.1'), '');
  assert.strictEqual(
    cleanupStep._internals.hasConflictingOrt(path.dirname(dir), 'linux'), true);
  fs.rmSync(path.join(scratchHome, '.local', 'lib'), { recursive: true, force: true });
});

console.log('\nintegration step:');

const integrationStep = require('../src/steps/integration');
const { describeSession, requiredInjector, inspectLinux: inspectIntegrationLinux } =
  integrationStep._internals;
const allPresent = { onPath: () => true, hibernation: () => null };

test('integration: each session gets the injector its compositor actually needs', () => {
  const cases = [
    [{ WAYLAND_DISPLAY: 'wayland-0', XDG_CURRENT_DESKTOP: 'ubuntu:GNOME' }, 'ydotool'],
    [{ WAYLAND_DISPLAY: 'wayland-0', SWAYSOCK: '/run/sway' }, 'wtype'],
    [{ XDG_SESSION_TYPE: 'wayland', XDG_CURRENT_DESKTOP: 'KDE' }, 'wtype'],
    [{ DISPLAY: ':0' }, 'xdotool'],
  ];
  for (const [env, expected] of cases) {
    assert.strictEqual(requiredInjector(describeSession(env)), expected, JSON.stringify(env));
  }
});

test('integration: an empty environment reads as X11/unknown — the sudo shape', () => {
  // This is the misdetection itself: root has no session vars, so a GNOME
  // Wayland laptop looks like a bare X11 box and gets xdotool.
  const session = describeSession({});
  assert.strictEqual(session.displayServer, 'x11');
  assert.strictEqual(requiredInjector(session), 'xdotool');
});

await asyncTest('integration: under sudo-for-another that misdetection becomes BLOCKED', async () => {
  const ctx = ctxWith({
    platform: 'linux',
    env: {},
    users: { uid: 0, isRoot: true, effectiveUser: 'root', targetUser: 'u',
      targetHome: '/home/u', elevatedForAnother: true },
  });
  const result = await steps.runStep(integrationStep, ctx);
  assert.strictEqual(result.status, STATUS.BLOCKED,
    'a confidently wrong install is worse than a named, repairable block');
  assert.match(result.health.summary, /session/);
});

test('integration: a missing injector is unhealthy and says which one', () => {
  const ctx = ctxWith({ platform: 'linux', env: { WAYLAND_DISPLAY: 'wayland-0', XDG_CURRENT_DESKTOP: 'GNOME' } });
  const record = inspectIntegrationLinux(ctx, { onPath: (t) => t !== 'ydotool', hibernation: () => null });
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'INTEGRATION_NO_INJECTOR');
  assert.match(record.summary, /ydotool/);
});

test('integration: missing pipewire is unhealthy — there is nothing to transcribe', () => {
  const ctx = ctxWith({ platform: 'linux', env: { DISPLAY: ':0' } });
  const record = inspectIntegrationLinux(ctx, { onPath: (t) => t !== 'pipewire', hibernation: () => null });
  assert.strictEqual(record.code, 'INTEGRATION_NO_AUDIO');
});

test('integration: unconfigured NVIDIA hibernation is unknown, never a failed step', () => {
  // run() cannot fix it without root, so unhealthy would mark this step
  // permanently failed on every affected laptop.
  const ctx = ctxWith({ platform: 'linux', env: { DISPLAY: ':0' } });
  const record = inspectIntegrationLinux(ctx, {
    onPath: () => true,
    hibernation: () => ({ needsConfiguration: true, isLaptop: true, hasNvidiaGpu: true, distribution: 'ubuntu' }),
  });
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'INTEGRATION_HIBERNATION');
  assert.strictEqual(record.repair, 'sudo swictation setup');
});

test('integration: a sessionless environment yields unknown, not a wrong verdict', () => {
  // ssh without X forwarding, a cron job, doctor from a serial console: no
  // session variables at all. describeSession() called that "x11 / unknown"
  // and the check confidently demanded xdotool — a verdict about a session
  // that was never described. The empty env is the ABSENCE of evidence.
  for (const env of [{}, { TERM: 'xterm', PATH: '/usr/bin' }]) {
    const record = inspectIntegrationLinux(
      ctxWith({ platform: 'linux', env }), { onPath: () => false, hibernation: () => null });
    assert.strictEqual(record.state, STATE.UNKNOWN, JSON.stringify(env));
    assert.strictEqual(record.code, 'INTEGRATION_NO_SESSION', JSON.stringify(env));
  }
});

test('integration: a described session still gets a real verdict', () => {
  // The guard above must not swallow the case it was added around.
  const record = inspectIntegrationLinux(
    ctxWith({ platform: 'linux', env: { XDG_SESSION_TYPE: 'x11', DISPLAY: ':0' } }),
    { onPath: () => false, hibernation: () => null });
  assert.strictEqual(record.code, 'INTEGRATION_NO_INJECTOR');
});

test('integration: healthy claims tools ON PATH, not that injection works', () => {
  // `which ydotool` proves a file exists. It says nothing about ydotoold
  // running or /dev/uinput being writable, which is what actually decides
  // whether a keystroke ever lands. The summary must claim only the former.
  const ctx = ctxWith({ platform: 'linux', env: { DISPLAY: ':0' } });
  const record = inspectIntegrationLinux(ctx, allPresent);
  assert.strictEqual(record.state, STATE.HEALTHY);
  assert.strictEqual(record.code, 'INTEGRATION_TOOLS_PRESENT');
  assert.match(record.summary, /on PATH/);
  assert.ok(record.evidence.some(e => /unverified/.test(e)),
    'the gap between "installed" and "working" belongs in the evidence');
});

await asyncTest('integration keeps the legacy NVIDIA hibernation guidance, not just a warning', async () => {
  // The migration orphaned checkNvidiaHibernation(): the legacy phase printed
  // a block explaining what a defunct GPU after hibernation looks like and the
  // exact command that fixes it. A one-line warning is not that block, and a
  // laptop user who loses CUDA after every suspend needs the block.
  const postinstallModule = require('../postinstall');
  const realWayland = postinstallModule.setupWaylandIntegration;
  const realHibernation = postinstallModule.checkNvidiaHibernation;
  let guidance = 0;
  postinstallModule.setupWaylandIntegration = async () => ({
    textInjectionTool: 'xdotool', pipewireInstalled: true, gnomeShortcuts: false,
  });
  postinstallModule.checkNvidiaHibernation = async () => { guidance += 1; };
  try {
    const result = await integrationStep.run(ctxWith({ platform: 'linux', env: { DISPLAY: ':0' } }));
    assert.strictEqual(guidance, 1, 'the legacy guidance block must still be printed');
    assert.ok(Array.isArray(result.warnings));
  } finally {
    postinstallModule.setupWaylandIntegration = realWayland;
    postinstallModule.checkNvidiaHibernation = realHibernation;
  }
});

test('integration (darwin): accessibility is unverifiable, so it is never healthy', () => {
  const record = integrationStep.check(ctxWith({ platform: 'darwin' }));
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'INTEGRATION_TCC_UNKNOWN',
    'AXIsProcessTrusted answers for node, not for the daemon — guessing would be a lie');
});

console.log('\nverify step:');

const verifyStep = require('../src/steps/verify');
const { inspectLinux: inspectVerifyLinux, inspectDarwin: inspectVerifyDarwin } = verifyStep._internals;

test('verify: no unit yet defers to services rather than double-reporting it', () => {
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: path.join(scratchHome, 'nope.service') });
  const record = inspectVerifyLinux(ctx, () => true);
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'VERIFY_NO_UNIT');
  assert.strictEqual(record.repair, 'swictation setup --services');
});

test('verify: an installed-but-disabled unit is unknown, never unhealthy', () => {
  // Declining auto-start is a legitimate configuration; painting it red
  // trains people to ignore a red doctor.
  writeUnit({ execStart: touchFile('swictation-daemon'), ortLine: `Environment="ORT_DYLIB_PATH=${touchFile('libonnxruntime.so')}"` });
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: UNIT_PATH });
  const record = inspectVerifyLinux(ctx, () => false);
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'VERIFY_NOT_ENABLED');
  assert.match(record.repair, /systemctl --user enable/);
});

test('verify: "disabled" and "systemd did not answer" are different answers', () => {
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: UNIT_PATH });
  assert.strictEqual(inspectVerifyLinux(ctx, () => null).code, 'VERIFY_NO_SYSTEMD');
  assert.strictEqual(inspectVerifyLinux(ctx, () => false).code, 'VERIFY_NOT_ENABLED');
  assert.strictEqual(inspectVerifyLinux(ctx, () => true, () => 'active').state, STATE.HEALTHY);
});

test('verify: enabled-but-not-running is NOT VERIFY_OK, and says so in the summary', () => {
  // "Will start on login" and "is running" are different claims. The daemon
  // exits at startup when its model is missing; reporting VERIFY_OK there put
  // the one fact that mattered into evidence, under a green row nobody reads.
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: UNIT_PATH });

  const inactive = inspectVerifyLinux(ctx, () => true, () => 'inactive');
  assert.notStrictEqual(inactive.state, STATE.HEALTHY);
  assert.strictEqual(inactive.state, STATE.UNKNOWN);
  assert.strictEqual(inactive.code, 'VERIFY_NOT_RUNNING');
  assert.match(inactive.summary, /not running/,
    'the fact belongs in the summary, not buried in the evidence list');

  const unknownState = inspectVerifyLinux(ctx, () => true, () => null);
  assert.strictEqual(unknownState.state, STATE.UNKNOWN);
  assert.strictEqual(unknownState.code, 'VERIFY_ACTIVE_UNKNOWN');
});

test('verify: a unit systemd reports as FAILED is unhealthy, not merely unknown', () => {
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: UNIT_PATH });
  const record = inspectVerifyLinux(ctx, () => true, () => 'failed');
  assert.strictEqual(record.state, STATE.UNHEALTHY);
  assert.strictEqual(record.code, 'VERIFY_DAEMON_FAILED');
  assert.match(record.summary, /failed/);
});

test('verify: optional tools are inventory, never a verdict', () => {
  const ctx = ctxWith({ platform: 'linux', systemdUnitPath: UNIT_PATH });
  const record = inspectVerifyLinux(ctx, () => true, () => 'active');
  assert.strictEqual(record.state, STATE.HEALTHY, 'a missing `hf` must not fail an install');
  assert.ok(record.evidence.some(e => /hf \(/.test(e)), 'but it is still reported');
});

test('verify (darwin): a plist nothing loaded is unknown with the bootstrap command', () => {
  writePlist({ program: touchFile('launcher'), ortPath: touchFile('libonnxruntime.dylib') });
  const ctx = ctxWith({ platform: 'darwin', daemonPlistPath: PLIST_PATH });
  const off = { loaded: false, running: false };
  const up = { loaded: true, running: true };

  const notLoaded = inspectVerifyDarwin(ctx, () => ({ daemon: off, ui: off }));
  assert.strictEqual(notLoaded.code, 'VERIFY_NOT_LOADED');
  assert.match(notLoaded.repair, /launchctl bootstrap/);

  const loaded = inspectVerifyDarwin(ctx, () => ({ daemon: up, ui: off }));
  assert.strictEqual(loaded.state, STATE.HEALTHY);
  assert.ok(loaded.evidence.some(e => /ui agent: not loaded/.test(e)));
});

await asyncTest('verify: `setup` never enables autostart — the prompt owns that choice', async () => {
  const postinstallModule = require('../postinstall');
  const real = postinstallModule.enableAndStartService;
  let called = 0;
  postinstallModule.enableAndStartService = async () => { called += 1; return { enabled: true, started: true }; };
  try {
    const setupRun = await verifyStep.run(ctxWith({ platform: 'linux', mode: 'setup' }));
    assert.strictEqual(called, 0, 'enabling here would answer the prompt before it is asked');
    assert.deepStrictEqual(
      setupRun.components.filter(c => c.id === 'autostart').map(c => c.status), ['skipped']);

    await verifyStep.run(ctxWith({ platform: 'linux', mode: 'postinstall' }));
    assert.strictEqual(called, 1, 'an npm install still enables it, as it always has');
  } finally {
    postinstallModule.enableAndStartService = real;
  }
});

console.log('\npostinstall driver:');

/** Run a snippet in a fresh node, with HOME still pointed at the scratch dir. */
function spawnSyncNode(source) {
  return require('child_process').spawnSync(process.execPath, ['-e', source], {
    encoding: 'utf8',
    env: { ...process.env, HOME: scratchHome },
  });
}

test('every driver hook is keyed to a step that actually exists', () => {
  // The hooks are how hardware facts reach the context between phases. A key
  // that no longer names a step fails silently — the fact is simply never
  // resolved and every later step reads a null.
  const { PREPARE, FOLLOW_UP } = postinstall._driver;
  for (const [label, hooks] of [['PREPARE', PREPARE], ['FOLLOW_UP', FOLLOW_UP]]) {
    for (const id of Object.keys(hooks)) {
      assert.ok(steps.getStep(id), `${label} hook "${id}" names no registered step`);
      assert.strictEqual(typeof hooks[id], 'function', `${label}.${id}`);
    }
  }
});

/**
 * A stand-in platform step that reports `code` and fails a component, which
 * is what a real fatal platform step does: run() refuses to create
 * directories on a machine the binaries cannot run on.
 */
function fatalPlatformStub(code) {
  return {
    id: 'platform',
    title: 'Checking platform compatibility...',
    entrypoints: ['postinstall'],
    after: [],
    applies: () => true,
    check: () => health.unhealthy(code, `${code} summary`, { evidence: ['because'] }),
    run: () => ({ changed: false, components: [health.componentFailed('install-directories', 'refused')], warnings: [] }),
  };
}

await asyncTest('the driver preserves the legacy exit codes, THROUGH the real driver', async () => {
  // An Intel Mac has always failed the whole `npm install`; every other
  // unsupported machine has always exited 0. Only postinstall keeps an
  // exception to the exit-zero policy, and this is it.
  //
  // This drives runPlan() rather than fatalPlatformExit() directly, because
  // the interesting failure lives BETWEEN them: the runner replaces a step's
  // health with a synthesized `PLATFORM_PARTIAL` when a component fails, and
  // a fatal check whose code has been overwritten stops looking fatal. Testing
  // the predicate in isolation passes while the driver ships an Intel Mac an
  // exit code of 0 and a full 10-phase install it cannot use.
  const { runPlan } = postinstall._driver;
  const laterRan = [];
  const later = fixtureStep('config-reset', { order: laterRan });

  const mac = await runPlan(ctxWith({ platform: 'darwin', mode: 'postinstall' }),
    [fatalPlatformStub('PLATFORM_UNSUPPORTED_ARCH'), later], []);
  assert.strictEqual(mac.exitCode, 1, 'an Intel Mac fails the whole npm install, as it always has');
  assert.deepStrictEqual(laterRan, [], 'and nothing downstream runs on a machine that cannot use it');

  const linux = await runPlan(ctxWith({ platform: 'linux', mode: 'postinstall' }),
    [fatalPlatformStub('PLATFORM_UNSUPPORTED_ARCH')], []);
  assert.strictEqual(linux.exitCode, 0);

  const windows = await runPlan(ctxWith({ platform: 'win32', mode: 'postinstall' }),
    [fatalPlatformStub('PLATFORM_UNSUPPORTED_OS')], []);
  assert.strictEqual(windows.exitCode, 0);

  // A non-fatal platform verdict must not stop anything.
  const oldDistro = await runPlan(ctxWith({ platform: 'linux', mode: 'postinstall' }),
    [fatalPlatformStub('PLATFORM_GLIBC_OLD'), fixtureStep('config-reset', { order: laterRan })], []);
  assert.strictEqual(oldDistro.exitCode, null,
    'an old distribution installs and warns, exactly as it always has');
  assert.deepStrictEqual(laterRan, ['config-reset'], 'and the rest of the plan still runs');
});

await asyncTest('the runner keeps the post-run check record when it synthesizes a PARTIAL', async () => {
  // Losing it is what let a fatal platform verdict disappear behind
  // PLATFORM_PARTIAL. The synthesized record is the right thing to SHOW; it
  // is the wrong thing to be the only thing kept.
  const result = await steps.runStep(fixtureStep('a', { failComponent: true }), ctxWith());
  assert.strictEqual(result.health.code, 'A_PARTIAL');
  assert.strictEqual(result.checkHealth.code, 'FIXTURE_OK',
    'what check() actually said must survive the component-failure override');
});

test('requiring postinstall runs no install', () => {
  // The require.main guard. Every step module requires this file for the
  // implementations it wraps, so a stray side effect would fire on `doctor`.
  const result = spawnSyncNode(`
    const before = Date.now();
    require(${JSON.stringify(path.join(PKG_ROOT, 'postinstall.js'))});
    process.stdout.write('QUIET');
  `);
  assert.strictEqual(result.status, 0, result.stderr);
  assert.strictEqual(result.stdout, 'QUIET',
    `requiring postinstall.js printed: ${JSON.stringify(result.stdout)}`);
});

console.log('\nmacOS service continuity:');

test('launchd state is sampled before the bootout, or the answer is gone', () => {
  const postinstallModule = require('../postinstall');
  assert.strictEqual(typeof postinstallModule.launchdServiceState, 'function');
  assert.strictEqual(typeof postinstallModule.restoreLaunchdServices, 'function');
  const state = postinstallModule.launchdServiceState();
  assert.deepStrictEqual(Object.keys(state).sort(), ['daemon', 'ui']);
  for (const value of Object.values(state)) {
    assert.deepStrictEqual(Object.keys(value).sort(), ['loaded', 'running']);
  }
});

test('launchd: LOADED and RUNNING are read as different facts', () => {
  // `launchctl print` exiting 0 means the job is bootstrapped, not that it has
  // a process. Treating exit 0 as "running" made restore-after-decline START
  // a service the user had deliberately stopped — the exact opposite of
  // putting the machine back as it was found.
  const { parseLaunchctlPrint } = require('../postinstall')._launchd;

  const running = parseLaunchctlPrint([
    'gui/502/com.swictation.daemon = {',
    '\tactive count = 3',
    '\tstate = running',
    '\tpid = 844',
    '\tendpoints = {',
    '\t\tstate = active',
    '\t}',
    '}',
  ].join('\n'));
  assert.deepStrictEqual(running, { loaded: true, running: true });

  const stopped = parseLaunchctlPrint([
    'gui/502/com.swictation.daemon = {',
    '\tactive count = 0',
    '\tstate = not running',
    '\tendpoints = {',
    '\t\tstate = active',   // a nested endpoint is not the job
    '\t}',
    '}',
  ].join('\n'));
  assert.deepStrictEqual(stopped, { loaded: true, running: false },
    'loaded with no pid is a job that exists and is not running');
});

test('restore only bootstraps what was RUNNING, never what was merely loaded', () => {
  const postinstallModule = require('../postinstall');
  const stopped = { loaded: true, running: false };
  const off = { loaded: false, running: false };

  // Loaded but stopped: bootstrapping it would start it (RunAtLoad), which is
  // starting a service the user had stopped.
  assert.deepStrictEqual(
    postinstallModule.restoreLaunchdServices({ daemon: stopped, ui: stopped }, scratchHome), []);
  assert.deepStrictEqual(
    postinstallModule.restoreLaunchdServices({ daemon: off, ui: off }, scratchHome), []);
  // Running, but the plist is gone: there is nothing to bootstrap FROM.
  assert.deepStrictEqual(
    postinstallModule.restoreLaunchdServices(
      { daemon: { loaded: true, running: true }, ui: { loaded: true, running: true } },
      path.join(scratchHome, 'no-such-home')), []);
});

test('verify (darwin) distinguishes a loaded agent from a running one', () => {
  writePlist({ program: touchFile('launcher'), ortPath: touchFile('libonnxruntime.dylib') });
  const ctx = ctxWith({ platform: 'darwin', daemonPlistPath: PLIST_PATH });
  const loadedNotRunning = inspectVerifyDarwin(ctx, () => ({
    daemon: { loaded: true, running: false }, ui: { loaded: false, running: false },
  }));
  assert.strictEqual(loadedNotRunning.state, STATE.UNKNOWN);
  assert.strictEqual(loadedNotRunning.code, 'VERIFY_NOT_RUNNING');
  assert.match(loadedNotRunning.summary, /not running/);

  const running = inspectVerifyDarwin(ctx, () => ({
    daemon: { loaded: true, running: true }, ui: { loaded: true, running: true },
  }));
  assert.strictEqual(running.state, STATE.HEALTHY);
});

console.log('\nCLI entry points:');

const { spawnSync } = require('child_process');
const CLI = path.join(PKG_ROOT, 'bin', 'swictation');

function cli(...argv) {
  return spawnSync(process.execPath, [CLI, ...argv], {
    encoding: 'utf8',
    env: { ...process.env, HOME: scratchHome },
  });
}

test('setup rejects an unrecognized flag instead of running everything', () => {
  // Falling through to a full setup would start multi-gigabyte downloads the
  // user never asked for — a typo must not do that.
  const result = cli('setup', '--nonsense');
  assert.strictEqual(result.status, 1);
  assert.match(result.stderr, /Unknown setup option\(s\): --nonsense/);
});

test('setup rejects a BARE positional typo the same way as a dashed one', () => {
  // `setup models` used to fall through to a full interactive setup, which
  // is the same multi-gigabyte hazard the dashed check was added to stop.
  const result = cli('setup', 'models');
  assert.strictEqual(result.status, 1);
  assert.match(result.stderr, /Unexpected argument: models/);
  assert.match(result.stderr, /did you mean "--models"/);
});

test('setup: naming a step that cannot run here exits nonzero', () => {
  // `swictation setup --gpu-libs` on a Mac must not exit 0 as though it had
  // installed something.
  const result = cli('setup', '--gpu-libs');
  assert.strictEqual(result.status, 1, 'a targeted step that did not run is a failed request');
});

test('setup --list names every step id, and labels the ones it cannot run', () => {
  const result = cli('setup', '--list');
  assert.strictEqual(result.status, 0);
  for (const step of steps.STEPS) assert.ok(result.stdout.includes(step.id), step.id);
  assert.match(result.stdout, /cleanup.*install only/,
    'listing a step it will refuse to run without saying so is an invitation to a no-op');
});

test('setup refuses an install-only step instead of silently doing nothing', () => {
  // `--cleanup` filtered to an empty plan, printed nothing, and exited 0 —
  // indistinguishable from having deleted the legacy installations it names.
  const result = cli('setup', '--cleanup');
  assert.strictEqual(result.status, 1);
  assert.match(result.stderr, /Cannot run "cleanup" from swictation setup/);
  assert.match(result.stderr, /runs only during: postinstall/);
});

test('doctor exits 1 when something is unhealthy, 0 when nothing is', () => {
  fs.rmSync(getConfigDir(), { recursive: true, force: true });
  const broken = cli('doctor');
  assert.strictEqual(broken.status, 1);
  assert.match(broken.stdout, /UNHEALTHY/);
  assert.match(broken.stdout, /repair: swictation setup --config-reset/);
});

test('doctor --json emits a stable schema with a context header', () => {
  fs.rmSync(getConfigDir(), { recursive: true, force: true });
  const result = cli('doctor', '--json');
  const payload = JSON.parse(result.stdout);
  assert.strictEqual(payload.schemaVersion, 1);
  assert.deepStrictEqual(
    steps.STEPS.map(s => s.id), payload.steps.map(s => s.id),
    'every step appears, in registry order');
  for (const row of payload.steps) {
    for (const key of ['id', 'title', 'state', 'code', 'summary', 'evidence', 'repair']) {
      assert.ok(key in row, `${row.id} is missing "${key}"`);
    }
    assert.ok(steps.health.ALL_STATES.includes(row.state), `${row.id}: bad state ${row.state}`);
  }
  for (const key of ['version', 'platform', 'effectiveUser', 'targetUser', 'mode', 'selectedModel', 'logPath']) {
    assert.ok(key in payload.context, `context is missing "${key}"`);
  }
});

test('doctor prints the context header before the table', () => {
  const result = cli('doctor');
  assert.match(result.stdout, /doctor\n\s+platform\s+darwin/);
  assert.match(result.stdout, /\n\s+user\s+/);
  assert.match(result.stdout, /\n\s+log\s+/);
});

test('doctor --deep reports the same rows, labelled as a deeper check', () => {
  // A stronger run of the SAME table — a --deep that quietly omitted rows
  // without a deepCheck() would be a different, shorter report.
  const standard = JSON.parse(cli('doctor', '--json').stdout);
  const deep = JSON.parse(cli('doctor', '--deep', '--json').stdout);
  assert.strictEqual(standard.depth, 'standard');
  assert.strictEqual(deep.depth, 'deep');
  assert.deepStrictEqual(deep.steps.map(s => s.id), standard.steps.map(s => s.id));
  for (const row of deep.steps) {
    assert.ok(steps.health.ALL_STATES.includes(row.state), `${row.id}: bad state ${row.state}`);
  }
});

await asyncTest('a step without a deepCheck falls back to its check rather than vanishing', async () => {
  const plain = fixtureStep('plain', { healthy: true });
  const record = await steps.checkStepDeep(plain, ctxWith());
  assert.strictEqual(record.code, 'FIXTURE_OK');
});

await asyncTest('a throwing deepCheck is unknown, not a crashed doctor', async () => {
  const exploding = {
    ...fixtureStep('boom', { healthy: true }),
    deepCheck: () => { throw new Error('hash failed'); },
  };
  const record = await steps.checkStepDeep(exploding, ctxWith());
  assert.strictEqual(record.state, STATE.UNKNOWN);
  assert.strictEqual(record.code, 'BOOM_DEEP_CHECK_ERROR');
});

console.log(`\n✅ ${passed} step-registry tests passed`);
fs.rmSync(scratchHome, { recursive: true, force: true });

})().catch(err => {
  console.error(`\n❌ ${err.message}`);
  console.error(err.stack);
  try { fs.rmSync(scratchHome, { recursive: true, force: true }); } catch { /* ignore */ }
  process.exit(1);
});
