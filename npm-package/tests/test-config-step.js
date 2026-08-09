#!/usr/bin/env node
/**
 * Tests for src/steps/config.js — the upgrade-safe config step (ADR-035).
 *
 * Regression fence: interactiveConfigMigration() used to clobber config.toml
 * with defaults on every install/upgrade. These tests pin the new contract:
 * absent → create; unparseable → backup + replace; parseable → preserve
 * byte-for-byte except stale model-path healing.
 *
 * Run: node tests/test-config-step.js
 */

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

// Isolate HOME so paths.js resolves into a scratch dir.
const scratchHome = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-config-step-'));
process.env.HOME = scratchHome;

const configStep = require('../src/steps/config');
const { getConfigDir, getModelsDir } = require('../src/paths');

const quiet = () => {};
const DEFAULTS = [
  '# Swictation Configuration (test defaults)',
  'stt_model_override = "auto"',
  'vad_threshold = 0.25',
  '',
].join('\n');
const ctx = { log: quiet, generateDefaultConfig: () => DEFAULTS };

let passed = 0;
function test(name, fn) {
  // Fresh config dir per test
  fs.rmSync(getConfigDir(), { recursive: true, force: true });
  fn();
  passed += 1;
  console.log(`  ✓ ${name}`);
}

test('absent config is created with defaults', () => {
  const result = configStep.run(ctx);
  assert.strictEqual(result.action, 'created');
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), DEFAULTS);
  assert.strictEqual(configStep.check(), true);
});

test('unparseable config is backed up and replaced', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  fs.writeFileSync(configStep.configPath(), 'audio_device_index = null\n'); // invalid TOML
  assert.strictEqual(configStep.check(), false);
  const result = configStep.run(ctx);
  assert.strictEqual(result.action, 'replaced-invalid');
  const backups = fs.readdirSync(getConfigDir()).filter(f => f.includes('.bak.'));
  assert.strictEqual(backups.length, 1, 'timestamped backup must exist');
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), DEFAULTS);
});

test('valid customized config is preserved byte-for-byte', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const userConfig = [
    '# my precious hand-tuned config',
    'vad_threshold = 0.5',
    'stt_model_override = "0.6b-cpu"',
    '',
    '[hotkeys]',
    'toggle = "Ctrl+Alt+Z"',
    '',
  ].join('\n');
  fs.writeFileSync(configStep.configPath(), userConfig);
  const result = configStep.run(ctx);
  assert.strictEqual(result.action, 'kept');
  assert.strictEqual(
    fs.readFileSync(configStep.configPath(), 'utf8'),
    userConfig,
    'user config must survive the step unchanged, comments included'
  );
  const backups = fs.readdirSync(getConfigDir()).filter(f => f.includes('.bak.'));
  assert.strictEqual(backups.length, 0, 'no backup when nothing was modified');
});

test('stale model path is healed when default exists, custom lines preserved', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const defaultModelDir = path.join(getModelsDir(), 'parakeet-tdt-1.1b-onnx');
  fs.mkdirSync(defaultModelDir, { recursive: true });
  const staleConfig = [
    '# comment survives healing',
    'vad_threshold = 0.5',
    `stt_1_1b_model_path = "${scratchHome}/old-location/parakeet-tdt-1.1b-onnx"`,
    '',
  ].join('\n');
  fs.writeFileSync(configStep.configPath(), staleConfig);
  const result = configStep.run(ctx);
  assert.strictEqual(result.action, 'healed');
  assert.deepStrictEqual(result.healedKeys, ['stt_1_1b_model_path']);
  const content = fs.readFileSync(configStep.configPath(), 'utf8');
  assert.ok(content.includes('# comment survives healing'));
  assert.ok(content.includes('vad_threshold = 0.5'));
  assert.ok(content.includes(JSON.stringify(defaultModelDir)));
  const backups = fs.readdirSync(getConfigDir()).filter(f => f.includes('.bak.'));
  assert.strictEqual(backups.length, 1, 'healing modifies the file, so it must back up first');
});

test('custom path pointing at a REAL location is never touched', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const customDir = path.join(scratchHome, 'my-models', 'parakeet-tdt-1.1b-onnx');
  fs.mkdirSync(customDir, { recursive: true });
  const defaultModelDir = path.join(getModelsDir(), 'parakeet-tdt-1.1b-onnx');
  fs.mkdirSync(defaultModelDir, { recursive: true });
  const userConfig = `stt_1_1b_model_path = ${JSON.stringify(customDir)}\n`;
  fs.writeFileSync(configStep.configPath(), userConfig);
  const result = configStep.run(ctx);
  assert.strictEqual(result.action, 'kept');
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), userConfig);
});

test('installer-written override resets to auto so hardware is re-tested', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const installerConfig = [
    '# comment survives the reset',
    'vad_threshold = 0.5',
    'stt_model_override = "0.6b-gpu"',
    '',
  ].join('\n');
  fs.writeFileSync(configStep.configPath(), installerConfig);
  configStep.recordManagedOverride('0.6b-gpu');

  // Post-download pass (no flag) must not undo the model postinstall just
  // verified — the reset belongs to the pre-download pass alone.
  assert.strictEqual(configStep.run(ctx).action, 'kept');
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), installerConfig);

  const result = configStep.run({ ...ctx, resetManagedOverride: true });
  assert.strictEqual(result.action, 'healed');
  assert.strictEqual(result.resetOverride, true);
  const content = fs.readFileSync(configStep.configPath(), 'utf8');
  assert.ok(content.includes('stt_model_override = "auto"'));
  assert.ok(content.includes('# comment survives the reset'));
  assert.ok(content.includes('vad_threshold = 0.5'));
  const backups = fs.readdirSync(getConfigDir()).filter(f => f.includes('.bak.'));
  assert.strictEqual(backups.length, 1, 'the reset modifies the file, so it must back up first');
});

test('user-authored override is never reset', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const userConfig = 'stt_model_override = "1.1b-cpu"\n';
  fs.writeFileSync(configStep.configPath(), userConfig);
  // Sidecar remembers a DIFFERENT value: the user edited it since.
  configStep.recordManagedOverride('0.6b-gpu');

  const result = configStep.run({ ...ctx, resetManagedOverride: true });
  assert.strictEqual(result.action, 'kept');
  assert.strictEqual(result.resetOverride, false);
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), userConfig);
  const backups = fs.readdirSync(getConfigDir()).filter(f => f.includes('.bak.'));
  assert.strictEqual(backups.length, 0, 'no backup when nothing was modified');
});

test('a completed reset consumes the marker, so re-choosing that model sticks', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "0.6b-gpu"\n');
  configStep.recordManagedOverride('0.6b-gpu');

  assert.strictEqual(configStep.run({ ...ctx, resetManagedOverride: true }).resetOverride, true);
  assert.strictEqual(
    configStep.readPostinstallState().managedOverride,
    undefined,
    'the marker is a one-shot claim; the reset must consume it'
  );

  // The user now deliberately picks the same model the installer once forced.
  const userConfig = 'stt_model_override = "0.6b-gpu"\n';
  fs.writeFileSync(configStep.configPath(), userConfig);
  const result = configStep.run({ ...ctx, resetManagedOverride: true });
  assert.strictEqual(result.action, 'kept');
  assert.strictEqual(result.resetOverride, false);
  assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), userConfig);
});

test('ABA: re-selecting the old managed value after an edit is never reset', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  // Installer wrote 0.6b-gpu and claimed it.
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "0.6b-gpu"\n');
  configStep.recordManagedOverride('0.6b-gpu');

  // The user edits it to something else; the install that sees this must let
  // go of the claim, because the key is now the user's.
  fs.writeFileSync(configStep.configPath(), 'stt_model_override = "1.1b-cpu"\n');
  assert.strictEqual(configStep.run({ ...ctx, resetManagedOverride: true }).action, 'kept');
  assert.strictEqual(
    configStep.readPostinstallState().managedOverride,
    undefined,
    'the user took ownership, so the stale marker must be dropped'
  );

  // Later the user deliberately goes back to 0.6b-gpu — the A-B-A that a
  // surviving marker would silently undo.
  const userConfig = 'stt_model_override = "0.6b-gpu"\n';
  fs.writeFileSync(configStep.configPath(), userConfig);
  const result = configStep.run({ ...ctx, resetManagedOverride: true });
  assert.strictEqual(result.action, 'kept');
  assert.strictEqual(result.resetOverride, false);
  assert.strictEqual(
    fs.readFileSync(configStep.configPath(), 'utf8'),
    userConfig,
    'a deliberate user choice must survive even when it equals a former managed value'
  );
});

test('non-object sidecar JSON is ignored rather than indexed', () => {
  fs.mkdirSync(getConfigDir(), { recursive: true });
  const userConfig = 'stt_model_override = "0.6b-gpu"\n';

  for (const junk of ['null', '["0.6b-gpu"]', '"0.6b-gpu"', '42', 'not json at all']) {
    fs.writeFileSync(configStep.statePath(), junk);
    assert.deepStrictEqual(configStep.readPostinstallState(), {}, `sidecar ${junk} must read as no state`);

    fs.writeFileSync(configStep.configPath(), userConfig);
    const result = configStep.run({ ...ctx, resetManagedOverride: true });
    assert.strictEqual(result.action, 'kept', `sidecar ${junk} must not trigger a reset`);
    assert.strictEqual(fs.readFileSync(configStep.configPath(), 'utf8'), userConfig);
  }
});

fs.rmSync(scratchHome, { recursive: true, force: true });
console.log(`\nAll ${passed} config-step tests passed`);
