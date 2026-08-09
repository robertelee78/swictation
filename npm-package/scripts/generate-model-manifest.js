#!/usr/bin/env node
'use strict';

/**
 * Regenerates npm-package/models.manifest.json — the integrity manifest that
 * lib/model-downloader.js verifies every downloaded model file against.
 *
 * Usage:
 *   node scripts/generate-model-manifest.js            # rewrite the manifest
 *   node scripts/generate-model-manifest.js --dry-run  # print, write nothing
 *   node scripts/generate-model-manifest.js --model=0.6b [--model=vad]
 *
 * Rerun this whenever lib/model-downloader.js MODELS changes (a new model, a
 * new file in an existing model) or when a pinned upstream revision should be
 * advanced. Advancing a revision is a deliberate act: the manifest is what
 * stops a rewritten upstream branch from silently changing what users install,
 * so review the diff — a changed sha256 on a file whose revision you did NOT
 * intend to move is a finding, not a rebase artifact.
 *
 * How the hashes are obtained WITHOUT downloading multi-GB models:
 *   HuggingFace's tree API (`/api/models/<repo>/tree/main?recursive=true`)
 *   returns, for every git-LFS file, `lfs.oid` — which is by definition the
 *   SHA-256 of the object's content, the same bytes `resolve/<rev>/<path>`
 *   serves. Every large model file is LFS, so the whole ~17 GB corpus is
 *   described from one JSON response per repo.
 *
 *   Small files committed as plain git blobs are the exception: their `oid` is
 *   a git SHA-1 over a prefixed blob, not a SHA-256 of the content, so those
 *   are downloaded (all are well under 2 MB) and hashed locally. The VAD model
 *   is a GitHub release asset with no such API, so it is downloaded and hashed
 *   the same way.
 *
 * The pinned `revision` is the repo's current commit sha (`sha` on
 * `/api/models/<repo>`), so downloads resolve against an immutable ref instead
 * of a branch that upstream can move under us.
 */

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

const ModelDownloader = require('../lib/model-downloader');
const { MODELS, MLMODELC_INTERNAL_FILES } = ModelDownloader;

const MANIFEST_PATH = path.join(__dirname, '..', 'models.manifest.json');
const HF_API = 'https://huggingface.co/api/models';

// Plain-blob files are fetched to be hashed. Anything larger than this in the
// plain-blob path means an assumption broke (a big file stopped being LFS) and
// we should notice rather than quietly pull it.
const MAX_PLAIN_FETCH_BYTES = 8 * 1024 * 1024;

const args = process.argv.slice(2);
const dryRun = args.includes('--dry-run');
const onlyModels = args
  .filter(a => a.startsWith('--model='))
  .map(a => a.slice('--model='.length));

function log(msg) {
  console.log(msg);
}

/** Expand the MODELS file list: a .mlmodelc bundle is a directory of leaves. */
function expandFiles(model) {
  const out = [];
  for (const file of model.files) {
    if (file.endsWith('.mlmodelc')) {
      for (const internal of MLMODELC_INTERNAL_FILES) out.push(`${file}/${internal}`);
    } else {
      out.push(file);
    }
  }
  return out;
}

async function fetchJson(url) {
  const res = await fetch(url, { headers: { 'User-Agent': 'swictation-manifest-generator' } });
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText} for ${url}`);
  return res.json();
}

async function fetchAndHash(url) {
  const res = await fetch(url, { headers: { 'User-Agent': 'swictation-manifest-generator' } });
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText} for ${url}`);
  const buf = Buffer.from(await res.arrayBuffer());
  return { sha256: crypto.createHash('sha256').update(buf).digest('hex'), size: buf.length };
}

/** Build the manifest entry for one HuggingFace-hosted model. */
async function buildHuggingFaceEntry(key, model) {
  log(`\n▸ ${key} — ${model.repo}`);

  const info = await fetchJson(`${HF_API}/${model.repo}`);
  const revision = info.sha;
  if (!/^[0-9a-f]{40}$/.test(revision || '')) {
    throw new Error(`${model.repo}: expected a 40-char commit sha, got ${JSON.stringify(revision)}`);
  }
  log(`  revision ${revision} (lastModified ${info.lastModified})`);

  const tree = await fetchJson(`${HF_API}/${model.repo}/tree/${revision}?recursive=true`);
  const byPath = new Map();
  for (const entry of tree) {
    if (entry.type === 'file') byPath.set(entry.path, entry);
  }

  const wanted = expandFiles(model);
  const files = [];
  let fetched = 0;

  for (const filePath of wanted) {
    const entry = byPath.get(filePath);
    if (!entry) {
      throw new Error(
        `${model.repo}: declared file "${filePath}" does not exist at ${revision}. ` +
        'Fix lib/model-downloader.js MODELS or pick a revision that has it.'
      );
    }

    if (entry.lfs && entry.lfs.oid) {
      // git-lfs oid IS the sha256 of the content the resolve endpoint serves.
      files.push({ path: filePath, sha256: entry.lfs.oid, size: entry.lfs.size });
      continue;
    }

    // Plain git blob: `oid` is a SHA-1 over a prefixed blob, useless to us.
    if (entry.size > MAX_PLAIN_FETCH_BYTES) {
      throw new Error(
        `${model.repo}: "${filePath}" is a ${entry.size}-byte plain blob (not LFS); ` +
        'refusing to download it just to hash it. Investigate before raising the cap.'
      );
    }
    const url = `https://huggingface.co/${model.repo}/resolve/${revision}/${encodeURI(filePath)}`;
    const { sha256, size } = await fetchAndHash(url);
    if (size !== entry.size) {
      throw new Error(`${model.repo}: "${filePath}" served ${size} bytes, tree said ${entry.size}`);
    }
    fetched += 1;
    files.push({ path: filePath, sha256, size });
  }

  const total = files.reduce((n, f) => n + f.size, 0);
  log(`  ${files.length} files, ${(total / 1024 ** 3).toFixed(2)} GB ` +
      `(${files.length - fetched} from LFS oids, ${fetched} hashed locally)`);

  return {
    name: model.name,
    targetDir: model.targetDir,
    source: { type: 'huggingface', repo: model.repo, revision },
    files,
  };
}

/** Build the manifest entry for a direct-URL model (the VAD release asset). */
async function buildDirectUrlEntry(key, model) {
  log(`\n▸ ${key} — ${model.directUrl}`);
  const { sha256, size } = await fetchAndHash(model.directUrl);
  log(`  ${model.files[0]}: ${size} bytes, sha256 ${sha256}`);
  return {
    name: model.name,
    targetDir: model.targetDir,
    source: { type: 'url', url: model.directUrl },
    files: [{ path: model.files[0], sha256, size }],
  };
}

/**
 * The banner for a `--model=` run with no manifest to merge into.
 *
 * That combination cannot produce a shippable manifest: the output describes
 * only the named models, and lib/model-downloader.js now treats an entry that
 * does not cover a model's declared files as unverifiable — so every other
 * model silently drops back to an existence check. It is still a legitimate
 * intermediate (bootstrapping a new model, checking one repo), so this warns
 * loudly rather than refusing.
 *
 * @param {string[]} onlyModels  the --model= values, empty for a full run
 * @param {boolean} manifestExists
 * @returns {string|null} the banner, or null when the run is fine
 */
function partialManifestNotice(onlyModels, manifestExists) {
  if (onlyModels.length === 0 || manifestExists) return null;
  const rule = '  ' + '='.repeat(72);
  return [
    '',
    rule,
    '  ⚠️  PARTIAL MANIFEST — NOT SUITABLE FOR SHIPPING',
    '',
    `  --model=${onlyModels.join(',')} was given, but there is no existing`,
    `  ${path.basename(MANIFEST_PATH)} to merge into, so the output will describe ONLY`,
    '  those models. Every other model becomes unverifiable at install time and',
    '  falls back to an existence check.',
    '',
    '  Re-run without --model= before publishing.',
    rule,
    '',
  ].join('\n');
}

async function main() {
  const keys = Object.keys(MODELS).filter(k => onlyModels.length === 0 || onlyModels.includes(k));
  if (keys.length === 0) {
    throw new Error(`No models matched --model=${onlyModels.join(',')}`);
  }

  // A partial regeneration merges into the existing manifest rather than
  // truncating it — otherwise `--model=vad` would silently drop every other
  // model's hashes.
  const manifestExists = fs.existsSync(MANIFEST_PATH);
  let existing = { models: {} };
  if (onlyModels.length > 0 && manifestExists) {
    existing = JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8'));
  }

  // Warned up front so it is seen before the network work, and again at the end
  // so it is the last thing on screen.
  const notice = partialManifestNotice(onlyModels, manifestExists);
  if (notice) log(notice);

  const models = { ...existing.models };
  for (const key of keys) {
    const model = MODELS[key];
    models[key] = model.directUrl
      ? await buildDirectUrlEntry(key, model)
      : await buildHuggingFaceEntry(key, model);
  }

  const manifest = {
    _comment: 'Generated by scripts/generate-model-manifest.js — do not hand-edit. See ADR-036.',
    version: 1,
    generated: new Date().toISOString(),
    models,
  };

  const json = JSON.stringify(manifest, null, 2) + '\n';

  if (dryRun) {
    log(`\n--dry-run: would write ${json.length} bytes to ${MANIFEST_PATH}`);
    if (notice) log(notice);
    return;
  }

  fs.writeFileSync(MANIFEST_PATH, json);
  const fileCount = Object.values(models).reduce((n, m) => n + m.files.length, 0);
  log(`\n✓ Wrote ${MANIFEST_PATH}`);
  log(`  ${Object.keys(models).length} models, ${fileCount} files`);
  if (notice) log(notice);
}

module.exports = { partialManifestNotice, expandFiles };

if (require.main === module) {
  main().catch(err => {
    console.error(`\n❌ Manifest generation failed: ${err.message}`);
    process.exit(1);
  });
}
