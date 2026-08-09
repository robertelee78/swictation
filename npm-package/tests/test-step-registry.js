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

test('the real registry orders config-reset before models before config-heal', () => {
  const plan = steps.selectSteps({ entrypoint: 'postinstall' }).map(s => s.id);
  assert.ok(plan.indexOf('config-reset') < plan.indexOf('models'),
    'the override reset must precede the download it re-tests hardware for');
  assert.ok(plan.indexOf('models') < plan.indexOf('config-heal'),
    'healing a stale path needs the default target to exist, so models runs first');
  assert.ok(plan.indexOf('gpu-libs') < plan.indexOf('services'),
    'the unit is written against freshly installed libraries');
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

test('config-reset: a pending installer-written override is unhealthy until consumed', () => {
  freshConfigDir();
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b"\n');
  configStep.recordManagedOverride('1.1b');
  assert.strictEqual(configReset.check(configCtx()).code, 'CONFIG_OVERRIDE_PENDING');

  configReset.run(configCtx());
  assert.match(fs.readFileSync(configStep.configPath(), 'utf8'), /stt_model_override = "auto"/,
    'the installer-written override reverts so this install re-tests the hardware');
  assert.strictEqual(configReset.check(configCtx()).state, STATE.HEALTHY);
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

test('gpu-libs: a matching receipt plus sentinel is UNKNOWN, never healthy', () => {
  clearGpuLibs();
  writeReceipt({ version: postinstall.GPU_LIBS_VERSION, variant: MODERN });
  writeSentinel();
  const record = gpuLibsStep.check(gpuCtx(MODERN));
  assert.strictEqual(record.state, STATE.UNKNOWN,
    'one sentinel .so cannot vouch for forty extracted files (ADR-037 amendment 7)');
  assert.strictEqual(record.code, 'GPULIBS_UNVERIFIED');
  assert.ok(record.evidence.some(e => /Phase-B/.test(e)),
    'the report must say why it cannot know, not just that it does not');
});

test('gpu-libs can never report healthy, under any fixture', () => {
  // The strongest form of amendment 7: not "does not today", but "cannot".
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

test('setup --list names every step id', () => {
  const result = cli('setup', '--list');
  assert.strictEqual(result.status, 0);
  for (const step of steps.STEPS) assert.ok(result.stdout.includes(step.id), step.id);
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

console.log(`\n✅ ${passed} step-registry tests passed`);
fs.rmSync(scratchHome, { recursive: true, force: true });

})().catch(err => {
  console.error(`\n❌ ${err.message}`);
  console.error(err.stack);
  try { fs.rmSync(scratchHome, { recursive: true, force: true }); } catch { /* ignore */ }
  process.exit(1);
});
