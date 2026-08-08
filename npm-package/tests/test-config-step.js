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

fs.rmSync(scratchHome, { recursive: true, force: true });
console.log(`\nAll ${passed} config-step tests passed`);
