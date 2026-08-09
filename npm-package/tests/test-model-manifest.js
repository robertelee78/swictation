#!/usr/bin/env node
/**
 * Tests for model download integrity verification (ADR-036).
 *
 * Model downloads used to have no verification at all: files were pulled from
 * a mutable `resolve/main/` ref and accepted purely because the write
 * succeeded. These tests fence the four behaviours that make the manifest
 * worth having:
 *
 *   1. a byte-correct file verifies;
 *   2. a file whose bytes were tampered with is rejected even when its length
 *      is untouched — the size check alone is not integrity;
 *   3. a truncated/extended file is rejected by the cheap size check, before
 *      the expensive hash runs;
 *   4. a MISSING manifest degrades to the old existence-only behaviour with a
 *      warning, because an installer that refuses to install is worse than one
 *      that installs unverified.
 *
 * Three later regressions are fenced alongside them:
 *
 *   5. the hf-CLI tier writes files under their FINAL names, so a file that
 *      fails verification there must be quarantined — otherwise it survives to
 *      the next run, where the size-only `isModelDownloaded` blesses it;
 *   6. a manifest entry that covers only SOME of a model's declared files must
 *      not be allowed to vouch for the whole model;
 *   7. the direct-HTTP tier's staging rename moved the resume file, so a
 *      pre-upgrade `<dest>.partial` must be migrated rather than orphaned, and
 *      a complete `<dest>.download` must not be re-fetched.
 *
 * Run: node tests/test-model-manifest.js
 */

'use strict';

const assert = require('assert');
const crypto = require('crypto');
const { EventEmitter } = require('events');
const fs = require('fs');
const os = require('os');
const path = require('path');

const ModelDownloader = require('../lib/model-downloader');
const { loadManifest, verifyFile, MLMODELC_INTERNAL_FILES, expandModelFiles, MODELS } = ModelDownloader;
const { partialManifestNotice } = require('../scripts/generate-model-manifest');

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-model-manifest-'));

let passed = 0;
const failures = [];

async function test(name, fn) {
  try {
    await fn();
    passed += 1;
    console.log(`  ✓ ${name}`);
  } catch (err) {
    failures.push({ name, err });
    console.log(`  ✗ ${name}\n      ${err.message}`);
  }
}

const sha256 = (buf) => crypto.createHash('sha256').update(buf).digest('hex');

/** Write a fixture manifest + the model tree it describes. */
function makeFixture(dirName) {
  const root = path.join(scratch, dirName);
  const modelDir = path.join(root, 'models');
  const targetDir = path.join(modelDir, 'fixture-model');
  fs.mkdirSync(targetDir, { recursive: true });

  const good = Buffer.from('encoder weights, byte for byte\n');
  const tokens = Buffer.from('a\nb\nc\n');
  fs.writeFileSync(path.join(targetDir, 'encoder.onnx'), good);
  fs.writeFileSync(path.join(targetDir, 'tokens.txt'), tokens);

  const manifestPath = path.join(root, 'models.manifest.json');
  fs.writeFileSync(manifestPath, JSON.stringify({
    generated: '2026-08-08T00:00:00.000Z',
    models: {
      fixture: {
        name: 'Fixture Model',
        targetDir: 'fixture-model',
        source: { type: 'huggingface', repo: 'fixture/repo', revision: 'deadbeef' },
        files: [
          { path: 'encoder.onnx', sha256: sha256(good), size: good.length },
          { path: 'tokens.txt', sha256: sha256(tokens), size: tokens.length },
        ],
      },
    },
  }, null, 2));

  return { root, modelDir, targetDir, manifestPath, good, tokens };
}

/**
 * Fixture for a REAL `MODELS` key, so the manifest entry can be checked against
 * what the model actually declares. `coverFiles` narrows what the manifest
 * describes (to fake a `--model=` regeneration that never merged); `onDisk`
 * chooses what actually exists.
 */
function makeModelFixture(dirName, modelKey, { onDisk = [], coverFiles = null } = {}) {
  const model = MODELS[modelKey];
  const root = path.join(scratch, dirName);
  const modelDir = path.join(root, 'models');
  const targetDir = path.join(modelDir, model.targetDir);
  fs.mkdirSync(targetDir, { recursive: true });

  const contents = new Map();
  for (const rel of expandModelFiles(model)) contents.set(rel, Buffer.from(`${modelKey}:${rel}\n`));

  const covered = coverFiles || [...contents.keys()];
  const manifestPath = path.join(root, 'models.manifest.json');
  fs.writeFileSync(manifestPath, JSON.stringify({
    version: 1,
    models: {
      [modelKey]: {
        name: model.name,
        targetDir: model.targetDir,
        source: { type: 'huggingface', repo: model.repo, revision: 'a'.repeat(40) },
        files: covered.map(p => ({
          path: p,
          sha256: sha256(contents.get(p)),
          size: contents.get(p).length,
        })),
      },
    },
  }, null, 2));

  for (const rel of onDisk) {
    const abs = path.join(targetDir, rel);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, contents.get(rel));
  }

  return { root, modelDir, targetDir, manifestPath, contents };
}

/** A child process that exits with `code` and writes nothing itself. */
function fakeChild(code) {
  const child = new EventEmitter();
  setImmediate(() => child.emit('close', code));
  return child;
}

/** Downloader wired to a fixture, silent, and never touching the network. */
function fixtureDownloader(fx, extra = {}) {
  const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath, ...extra });
  d.log = () => {};
  return d;
}

(async () => {
  // ── 1. verify-pass ────────────────────────────────────────────────
  await test('a byte-correct file verifies against its manifest entry', async () => {
    const fx = makeFixture('pass');
    const entry = loadManifest(fx.manifestPath).models.fixture.files[0];
    const result = await verifyFile(path.join(fx.targetDir, 'encoder.onnx'), entry);
    assert.strictEqual(result.ok, true, `expected pass, got: ${result.reason}`);
  });

  await test('the shipped manifest parses and covers every declared model file', () => {
    const manifest = loadManifest();
    assert.ok(manifest, 'models.manifest.json must ship with the package');

    for (const [key, model] of Object.entries(ModelDownloader.MODELS)) {
      const entry = manifest.models[key];
      assert.ok(entry, `manifest is missing model "${key}"`);

      // .mlmodelc bundles are directories; the manifest tracks their leaves.
      const expected = [];
      for (const file of model.files) {
        if (file.endsWith('.mlmodelc')) {
          for (const internal of MLMODELC_INTERNAL_FILES) expected.push(`${file}/${internal}`);
        } else {
          expected.push(file);
        }
      }

      const covered = new Set(entry.files.map(f => f.path));
      for (const file of expected) {
        assert.ok(covered.has(file), `manifest for "${key}" is missing "${file}"`);
      }
      for (const f of entry.files) {
        assert.match(f.sha256, /^[0-9a-f]{64}$/, `bad sha256 for ${key}/${f.path}`);
        assert.ok(Number.isInteger(f.size) && f.size > 0, `bad size for ${key}/${f.path}`);
      }
      if (entry.source.type === 'huggingface') {
        assert.match(entry.source.revision, /^[0-9a-f]{40}$/,
          `"${key}" must pin a commit sha, not a mutable ref`);
      }
    }
  });

  // ── 2. corrupt-file reject (same size, different bytes) ───────────
  await test('a tampered file of the SAME size is rejected', async () => {
    const fx = makeFixture('corrupt');
    const target = path.join(fx.targetDir, 'encoder.onnx');
    const tampered = Buffer.from(fx.good);
    tampered[0] = tampered[0] ^ 0xff; // flip one bit, keep the length
    fs.writeFileSync(target, tampered);
    assert.strictEqual(fs.statSync(target).size, fx.good.length, 'fixture must keep the size identical');

    const entry = loadManifest(fx.manifestPath).models.fixture.files[0];
    const result = await verifyFile(target, entry);
    assert.strictEqual(result.ok, false, 'a size-preserving byte flip must not pass');
    assert.match(result.reason, /sha256|checksum/i);
  });

  // ── 3. size-mismatch reject ───────────────────────────────────────
  await test('a truncated file is rejected on size, without hashing', async () => {
    const fx = makeFixture('truncated');
    const target = path.join(fx.targetDir, 'encoder.onnx');
    fs.writeFileSync(target, fx.good.subarray(0, 5));

    const entry = loadManifest(fx.manifestPath).models.fixture.files[0];
    const result = await verifyFile(target, entry);
    assert.strictEqual(result.ok, false);
    assert.match(result.reason, /size/i);
  });

  await test('a missing file is rejected rather than throwing', async () => {
    const fx = makeFixture('absent');
    const entry = loadManifest(fx.manifestPath).models.fixture.files[0];
    const result = await verifyFile(path.join(fx.targetDir, 'nope.onnx'), entry);
    assert.strictEqual(result.ok, false);
    assert.match(result.reason, /missing|ENOENT/i);
  });

  // ── isModelDownloaded uses the manifest's sizes ───────────────────
  await test('isModelDownloaded accepts a correctly-sized tree', () => {
    const fx = makeFixture('sized-ok');
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    d.log = () => {};
    assert.strictEqual(d.isModelDownloaded('fixture'), true);
  });

  await test('isModelDownloaded rejects a wrong-sized file that merely exists', () => {
    const fx = makeFixture('sized-bad');
    fs.writeFileSync(path.join(fx.targetDir, 'encoder.onnx'), Buffer.alloc(3));
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    d.log = () => {};
    assert.strictEqual(
      d.isModelDownloaded('fixture'),
      false,
      'existence-only checks are what let a half-written model look installed'
    );
  });

  // ── 4. missing-manifest fallback ──────────────────────────────────
  await test('a missing manifest falls back to existence-only, loudly, without bricking', () => {
    const fx = makeFixture('no-manifest');
    fs.rmSync(fx.manifestPath);

    assert.strictEqual(loadManifest(fx.manifestPath), null);

    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    const warnings = [];
    d.log = (m) => warnings.push(String(m));

    // MODELS-declared model, files present on disk → still reports installed.
    fs.mkdirSync(path.join(fx.modelDir, 'silero-vad'), { recursive: true });
    fs.writeFileSync(path.join(fx.modelDir, 'silero-vad', 'silero_vad.onnx'), Buffer.alloc(8));

    assert.strictEqual(d.isModelDownloaded('vad'), true, 'install must not brick without a manifest');
    assert.ok(
      warnings.some(w => /manifest/i.test(w) && /WARNING|⚠/i.test(w)),
      `expected a loud manifest warning, got: ${JSON.stringify(warnings)}`
    );
  });

  await test('the loud manifest warning is emitted once, not per file', () => {
    const fx = makeFixture('warn-once');
    fs.rmSync(fx.manifestPath);
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    const warnings = [];
    d.log = (m) => { if (/manifest/i.test(String(m))) warnings.push(String(m)); };
    d.isModelDownloaded('vad');
    d.isModelDownloaded('0.6b');
    d.isModelDownloaded('1.1b');
    assert.strictEqual(warnings.length, 1, `expected exactly one warning, got ${warnings.length}`);
  });

  // ── URL pinning ───────────────────────────────────────────────────
  await test('download URLs pin to the manifest revision, never to main', () => {
    const fx = makeFixture('pinning');
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    d.log = () => {};
    const url = d.fileUrl('fixture', 'encoder.onnx');
    assert.strictEqual(url, 'https://huggingface.co/fixture/repo/resolve/deadbeef/encoder.onnx');
    assert.ok(!url.includes('/main/'), 'a mutable ref is the whole bug');
  });

  await test('without a manifest the URL falls back to main', () => {
    const fx = makeFixture('pinning-fallback');
    fs.rmSync(fx.manifestPath);
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    d.log = () => {};
    const url = d.fileUrl('0.6b', 'tokens.txt');
    assert.strictEqual(
      url,
      'https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3/resolve/main/tokens.txt'
    );
  });

  // ── unknown-to-manifest files warn, don't fail ────────────────────
  await test('a file absent from the manifest warns instead of failing the install', async () => {
    const fx = makeFixture('unknown-file');
    const stray = path.join(fx.targetDir, 'stray.onnx');
    fs.writeFileSync(stray, Buffer.from('not in the manifest'));

    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    const warnings = [];
    d.log = (m) => warnings.push(String(m));

    const report = await d.verifyModelFiles('fixture', fx.targetDir, ['encoder.onnx', 'tokens.txt', 'stray.onnx']);
    assert.strictEqual(report.ok, true, `unknown files must not fail: ${JSON.stringify(report.failures)}`);
    assert.deepStrictEqual(report.unverified, ['stray.onnx']);
    assert.ok(warnings.some(w => /stray\.onnx/.test(w)), 'the unknown file should be called out');
  });

  await test('verifyModelFiles reports a corrupt file as a failure', async () => {
    const fx = makeFixture('verify-fail');
    fs.writeFileSync(path.join(fx.targetDir, 'tokens.txt'), Buffer.from('x\ny\nz\n')); // same size
    const d = new ModelDownloader({ modelDir: fx.modelDir, manifestPath: fx.manifestPath });
    d.log = () => {};
    const report = await d.verifyModelFiles('fixture', fx.targetDir, ['encoder.onnx', 'tokens.txt']);
    assert.strictEqual(report.ok, false);
    assert.strictEqual(report.failures.length, 1);
    assert.strictEqual(report.failures[0].path, 'tokens.txt');
  });

  // ── 5. hf-CLI tier: a file that fails verification must be quarantined ──
  // The CLI writes under final names, so rejecting without moving the file
  // leaves it for the next run's size-only check to bless.
  await test('a corrupt hf-CLI file is quarantined, not left where the daemon loads from', async () => {
    const declared = expandModelFiles(MODELS['0.6b']);
    const fx = makeModelFixture('hf-corrupt', '0.6b', { onDisk: declared });
    const target = path.join(fx.targetDir, 'tokens.txt');
    const tampered = Buffer.from(fx.contents.get('tokens.txt'));
    tampered[0] = tampered[0] ^ 0xff; // same size, different bytes
    fs.writeFileSync(target, tampered);

    const d = fixtureDownloader(fx, { verbose: true });
    d._spawn = () => fakeChild(0);

    await assert.rejects(
      () => d._downloadWithHfCli('0.6b', MODELS['0.6b'], fx.targetDir),
      /integrity/i,
      'a size-preserving tamper must reject the CLI tier'
    );

    assert.ok(!fs.existsSync(target), 'the corrupt file must not keep its real name');
    const quarantined = fs.readdirSync(fx.targetDir).filter(f => /^tokens\.txt\.corrupt\.\d/.test(f));
    assert.strictEqual(quarantined.length, 1, `expected one quarantined file, got ${quarantined.join(', ')}`);
    assert.deepStrictEqual(
      fs.readFileSync(path.join(fx.targetDir, quarantined[0])),
      tampered,
      'quarantine keeps the bytes — deleting user disk state silently is not ours to do'
    );
    assert.strictEqual(
      d.isModelDownloaded('0.6b'),
      false,
      'the next run must re-download rather than count the quarantined file'
    );
  });

  await test('a manifest may not declare a quarantined path as an installed file', () => {
    const declared = expandModelFiles(MODELS['0.6b']);
    const fx = makeModelFixture('quarantine-declared', '0.6b', { onDisk: declared });
    const manifest = JSON.parse(fs.readFileSync(fx.manifestPath, 'utf8'));
    const tokens = manifest.models['0.6b'].files.find(f => f.path === 'tokens.txt');
    // A manifest regenerated from a tree that still held quarantined files.
    manifest.models['0.6b'].files.push({ ...tokens, path: 'tokens.txt.corrupt.1700000000000' });
    fs.writeFileSync(fx.manifestPath, JSON.stringify(manifest, null, 2));
    fs.copyFileSync(
      path.join(fx.targetDir, 'tokens.txt'),
      path.join(fx.targetDir, 'tokens.txt.corrupt.1700000000000')
    );

    assert.strictEqual(fixtureDownloader(fx).isModelDownloaded('0.6b'), false);
  });

  // ── 6. partial manifest entries may not vouch for a whole model ─────────
  await test('a manifest entry covering only SOME declared files falls back to the legacy check', () => {
    const fx = makeModelFixture('subset-entry', '0.6b', {
      coverFiles: ['tokens.txt'],
      onDisk: ['tokens.txt'],
    });
    assert.strictEqual(
      fixtureDownloader(fx).isModelDownloaded('0.6b'),
      false,
      'a subset entry must never report a model with four missing files as installed'
    );
  });

  await test('a manifest entry with an empty files array falls back to the legacy check', () => {
    const fx = makeModelFixture('empty-entry', '0.6b', { coverFiles: [], onDisk: [] });
    assert.strictEqual(
      fixtureDownloader(fx).isModelDownloaded('0.6b'),
      false,
      'every() over an empty list is vacuously true — that must not mean "installed"'
    );
  });

  await test('a manifest entry covering every declared file still reports installed', () => {
    const declared = expandModelFiles(MODELS['0.6b']);
    const fx = makeModelFixture('full-entry', '0.6b', { onDisk: declared });
    assert.strictEqual(fixtureDownloader(fx).isModelDownloaded('0.6b'), true);
  });

  await test('the generator warns when --model= would emit a partial manifest', () => {
    const notice = partialManifestNotice(['0.6b'], false);
    assert.ok(notice, '--model= with no manifest to merge into must warn');
    assert.match(notice, /PARTIAL MANIFEST/);
    assert.match(notice, /0\.6b/);
    assert.strictEqual(partialManifestNotice(['0.6b'], true), null, 'merging into an existing manifest is fine');
    assert.strictEqual(partialManifestNotice([], false), null, 'a full regeneration is fine');
  });

  // ── 7. staging-name migration and reuse ────────────────────────────────
  await test('a pre-upgrade <dest>.partial is migrated to the new staging resume name', async () => {
    const fx = makeModelFixture('legacy-partial', '0.6b');
    const legacyBytes = fx.contents.get('tokens.txt').subarray(0, 3);
    fs.writeFileSync(path.join(fx.targetDir, 'tokens.txt.partial'), legacyBytes);

    const d = fixtureDownloader(fx);
    const seen = new Map();
    d._download = async (url, staging) => {
      const rel = path.basename(staging, '.download');
      const resumeBase = `${staging}.partial`;
      seen.set(rel, fs.existsSync(resumeBase) ? fs.readFileSync(resumeBase) : null);
      fs.writeFileSync(staging, fx.contents.get(rel));
      fs.rmSync(resumeBase, { force: true });
    };

    await d.downloadModelDirect('0.6b');

    assert.deepStrictEqual(
      seen.get('tokens.txt'),
      legacyBytes,
      'the legacy partial must be handed to the downloader as its resume base'
    );
    assert.ok(
      !fs.existsSync(path.join(fx.targetDir, 'tokens.txt.partial')),
      'the orphaned pre-upgrade partial must not be left on disk'
    );
    assert.strictEqual(d.isModelDownloaded('0.6b'), true);
  });

  await test('a complete <dest>.download skips the network and goes straight to verification', async () => {
    const fx = makeModelFixture('completed-staging', '0.6b');
    for (const rel of expandModelFiles(MODELS['0.6b'])) {
      fs.writeFileSync(path.join(fx.targetDir, `${rel}.download`), fx.contents.get(rel));
    }

    const d = fixtureDownloader(fx);
    d._download = async () => { throw new Error('the network must not be touched'); };

    await d.downloadModelDirect('0.6b');

    assert.strictEqual(d.isModelDownloaded('0.6b'), true, 'the staged bytes must land under their real names');
  });

  await test('an incomplete <dest>.download is re-downloaded rather than trusted', async () => {
    const fx = makeModelFixture('truncated-staging', '0.6b');
    fs.writeFileSync(
      path.join(fx.targetDir, 'tokens.txt.download'),
      fx.contents.get('tokens.txt').subarray(0, 2)
    );

    const d = fixtureDownloader(fx);
    const fetched = [];
    d._download = async (url, staging) => {
      const rel = path.basename(staging, '.download');
      fetched.push(rel);
      fs.writeFileSync(staging, fx.contents.get(rel));
    };

    await d.downloadModelDirect('0.6b');

    assert.ok(fetched.includes('tokens.txt'), 'a short staging file is not a completed download');
    assert.strictEqual(d.isModelDownloaded('0.6b'), true);
  });

  await test('a right-sized but corrupt <dest>.download is still rejected, not promoted', async () => {
    const fx = makeModelFixture('corrupt-staging', '0.6b');
    const tampered = Buffer.from(fx.contents.get('tokens.txt'));
    tampered[0] = tampered[0] ^ 0xff;
    const staging = path.join(fx.targetDir, 'tokens.txt.download');
    fs.writeFileSync(staging, tampered);

    const d = fixtureDownloader(fx);
    d._download = async (url, dest) => {
      fs.writeFileSync(dest, fx.contents.get(path.basename(dest, '.download')));
    };

    await assert.rejects(() => d.downloadModelDirect('0.6b'), /SW-E004|Integrity/i);
    assert.ok(!fs.existsSync(staging), 'a bad staging file must not survive as the next resume base');
    assert.ok(!fs.existsSync(path.join(fx.targetDir, 'tokens.txt')), 'nothing may reach the final path');
  });

  fs.rmSync(scratch, { recursive: true, force: true });

  if (failures.length) {
    console.log(`\n${failures.length} model-manifest test(s) FAILED`);
    for (const f of failures) console.log(`\n  ✗ ${f.name}\n${f.err.stack}`);
    process.exit(1);
  }
  console.log(`\nAll ${passed} model-manifest tests passed`);
})();
