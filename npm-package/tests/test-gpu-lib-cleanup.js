#!/usr/bin/env node
/**
 * Tests for the GPU library cleanup guards in postinstall.js (ADR-035).
 *
 * Cleanup deletes files out of an npm package directory, so it needs two
 * fences that are easy to lose in a refactor:
 *   1. the platform package's own libonnxruntime.so is never a candidate —
 *      it is the CPU/fallback ONNX Runtime a GPU-less install depends on;
 *   2. it only ever touches the node_modules tree this package lives in,
 *      because resolve-binary can resolve a GLOBAL platform package from a
 *      LOCAL install and a local postinstall must not mutate a global tree.
 *
 * Run: node tests/test-gpu-lib-cleanup.js
 */

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

const { PLATFORM_OWNED_LIBS, ownNodeModulesRoot, isWithin, supersededLibNames } = require('../postinstall');

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'swictation-gpu-cleanup-'));

let passed = 0;
function test(name, fn) {
  fn();
  passed += 1;
  console.log(`  ✓ ${name}`);
}

test('requiring postinstall.js does not run the installer', () => {
  // Reaching this line at all proves it: a real run would have started
  // downloading before the require returned.
  assert.ok(typeof supersededLibNames === 'function');
});

test('platform-owned libonnxruntime.so is never superseded', () => {
  const downloaded = [
    'libonnxruntime.so',
    'libonnxruntime_providers_cuda.so',
    'libcudnn.so.9.15.1',
    'libcublas.so.12',
  ];
  const superseded = supersededLibNames(downloaded);
  assert.strictEqual(
    superseded.has('libonnxruntime.so'),
    false,
    'the platform package ships this as the CPU/fallback ORT; deleting it breaks CPU-only installs'
  );
  assert.strictEqual(superseded.has('libonnxruntime_providers_cuda.so'), true);
  assert.strictEqual(superseded.has('libcudnn.so.9.15.1'), true);
  assert.strictEqual(superseded.has('libcublas.so.12'), true);
  assert.strictEqual(superseded.size, 3);
  assert.ok(PLATFORM_OWNED_LIBS.has('libonnxruntime.so'));
});

test('non-shared-object names are never superseded', () => {
  const superseded = supersededLibNames([
    'README.md',
    'libfoo.solid',
    'libfoo.so.beta',
    'swictation-daemon',
    'libfoo.so',
  ]);
  assert.deepStrictEqual([...superseded], ['libfoo.so']);
});

test('containment guard accepts the tree we live in and rejects a sibling', () => {
  const localRoot = path.join(scratch, 'project', 'node_modules');
  const globalRoot = path.join(scratch, 'usr', 'lib', 'node_modules');
  const localLib = path.join(localRoot, '@agidreams', 'linux-x64', 'lib');
  const globalLib = path.join(globalRoot, '@agidreams', 'linux-x64', 'lib');

  assert.strictEqual(isWithin(localRoot, localLib), true);
  assert.strictEqual(isWithin(localRoot, localRoot), true);
  assert.strictEqual(
    isWithin(localRoot, globalLib),
    false,
    'a local install must never delete out of the global tree'
  );
  // Prefix-only sibling: /a/node_modules must not swallow /a/node_modules-old.
  assert.strictEqual(isWithin(localRoot, `${localRoot}-old`), false);
});

test('ownNodeModulesRoot finds the nearest node_modules through symlinks', () => {
  const root = path.join(scratch, 'app', 'node_modules');
  const pkgDir = path.join(root, 'swictation');
  fs.mkdirSync(pkgDir, { recursive: true });
  assert.strictEqual(ownNodeModulesRoot(pkgDir), fs.realpathSync(root));

  // npm link / pnpm store: the package dir is a symlink out of the tree, so a
  // textual path check would pass where the real location is elsewhere.
  const realPkg = path.join(scratch, 'store', 'swictation');
  fs.mkdirSync(realPkg, { recursive: true });
  const linked = path.join(root, 'linked-pkg');
  fs.symlinkSync(realPkg, linked, 'dir');
  assert.strictEqual(
    ownNodeModulesRoot(linked),
    null,
    'realpath must defeat symlink aliasing: the real location is outside any node_modules'
  );
});

test('a dev checkout has no node_modules ancestor, so cleanup is skipped', () => {
  const devDir = path.join(scratch, 'checkout', 'npm-package');
  fs.mkdirSync(devDir, { recursive: true });
  assert.strictEqual(ownNodeModulesRoot(devDir), null);
});

fs.rmSync(scratch, { recursive: true, force: true });
console.log(`\nAll ${passed} gpu-lib-cleanup tests passed`);
