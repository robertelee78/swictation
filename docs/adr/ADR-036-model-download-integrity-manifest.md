# ADR-036: Model Download Integrity — Pinned Revisions and a Per-File Manifest

- **Status:** Accepted
- **Date:** 2026-08-08
- **Issue:** Audit Phase 3 (see ADR-034, ADR-035): model downloads had **zero** verification and resolved against mutable `main` refs. GPU tarballs have had SHA-512 via `checksums.txt` since gpu-libs-v1.2.0; the ~12 GB of model weights every install pulls had nothing.
- **Release:** v0.7.36 (planned)

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Context

`lib/model-downloader.js` fetched every model file from
`https://huggingface.co/<repo>/resolve/main/<file>` and accepted whatever came
back. Three consequences, in increasing order of severity:

1. **No content check.** A truncated transfer, a proxy-injected error page, or a
   corrupted disk write produced a file that "downloaded successfully" and then
   failed opaquely inside ONNX Runtime at first dictation.
2. **No pinning.** `main` is a branch. Three of the four model repos are
   third-party accounts (`jenerallee78`, `FluidInference`); a force-push or an
   account compromise silently changes what every subsequent install executes.
   Two of the four have already moved since this project first referenced them.
3. **Existence-only install detection.** `isModelDownloaded()` returned true if
   the filenames were present, so an install interrupted mid-write reported
   "already downloaded" forever and never self-healed.

The models are the largest attack surface in the install: `encoder.weights` for
1.1b is a 4.2 GB blob that the daemon memory-maps and executes as a graph.

## Decisions

1. **A generated integrity manifest is the source of truth:
   `npm-package/models.manifest.json`.**
   Per model key: the upstream `source` (repo + **40-char commit sha**, or the
   release URL for VAD) and `files[{path, sha256, size}]` for every file the
   downloader actually fetches — 5 models, 59 files, ~12.4 GB described.
   `.mlmodelc` bundles are directories, so the manifest tracks their five
   internal leaves, not the bundle name.

2. **The manifest is generated from upstream metadata, not from a local
   download** — `scripts/generate-model-manifest.js`, rerunnable, documented in
   its own header.
   HuggingFace's tree API returns `lfs.oid` for every git-LFS file, and that oid
   **is** the SHA-256 of the bytes `resolve/<rev>/<path>` serves (verified
   empirically against a downloaded object before relying on it). Every large
   file is LFS, so 36 of 59 hashes come from four JSON responses. The remaining
   23 are small plain git blobs whose `oid` is a git SHA-1 over a prefixed blob
   and therefore useless to us — those are downloaded (~4 MB total) and hashed
   locally, as is the VAD GitHub release asset. The generator **refuses** to
   download a plain blob over 8 MB rather than quietly pulling something large:
   a big file that is not LFS means an assumption broke.

3. **Downloads pin to the manifest revision.** `fileUrl()` builds
   `resolve/<revision>/<path>`; the hf-CLI tier passes `--revision <sha>`.
   Advancing a pin is a deliberate edit reviewed as a diff — which is the point.
   A changed sha256 on a file whose revision was not intentionally moved is a
   finding, not a rebase artifact.

4. **Verification happens before a file takes its real name.** Each file
   downloads to a staging path (`<dest>.download`, or `<dest>.partial` for the
   curl/VAD path) and is renamed only after passing. Size is checked first — a
   `stat()` that rules out truncation and error-pages without reading gigabytes
   — then SHA-256 is **streamed** (`checksumFile`), never buffered; these files
   reach 4.2 GB. On mismatch the staging file is **deleted** (never left as a
   resume base for the next attempt) and an `InstallError` `SW-E004` is raised.
   The hf-CLI tier writes files itself, so it verifies after the fact; a failure
   there rejects into the existing fallback and the direct-HTTP tier re-fetches
   at the pinned revision file by file.

5. **`isModelDownloaded()` checks size, not existence.** Cheap enough to run on
   every install and it catches the half-written tree the old check blessed.
   Content verification stays in the download path — hashing 7 GB on every
   startup is not a check anyone would keep, and a size-preserving tamper is
   caught when the download path re-verifies.

6. **Two explicit non-brick fences.**
   - *Unknown-to-manifest file* (manifest lagging a `MODELS` edit): warn, keep,
     do not fail.
   - *Manifest missing entirely*: fall back to today's behaviour — unverified
     download from `main` — behind a loud warning emitted **once** per
     downloader, not once per file across 59 files.
   An installer that refuses to install is worse than one that installs
   unverified and says so.

## Proof

- `scripts/generate-model-manifest.js` run for real against the live API;
  the committed manifest is its output.
- `tests/test-model-manifest.js` — 13 cases at first write, 23 after the
  review fixes below, wired into `npm test`
  (verify-pass, size-preserving tamper rejected, truncation rejected on size,
  missing file, manifest-backed `isModelDownloaded` accept/reject,
  missing-manifest fallback, warn-once, revision pinning, `main` fallback,
  unknown-file warns-not-fails, corrupt-file reported). `npm test` green.
- End-to-end against the live network: VAD downloaded and verified through the
  curl path; a HuggingFace file fetched at the pinned immutable ref; a re-run
  skipping an already-verified file; a size-preserving tamper detected on
  re-verify and replaced with clean bytes; and a deliberately wrong manifest
  hash proving `SW-E004` is raised with **nothing** written to the final path
  and the staging file removed.

## Known gaps (deliberate, not oversights)

- **The manifest is not signed.** It ships inside the npm tarball and inherits
  npm's provenance; it defends against upstream mutation and transport
  corruption, not against a compromised npm publish.
- `MODELS['0.6b-coreml'].size` advertises 2.67 GB while the declared file subset
  is 1.01 GB. The manifest now records real per-file sizes, so that display
  string is the stale value; cosmetic, untouched here.

## Closed after review (2026-08-08)

Four things were found by review after the write-up above; all are implemented
and fenced in `tests/test-model-manifest.js` (23 cases).

- **`postinstall.js` delegation is done, not deferred.** The gap listed above —
  postinstall's own name/existence `isModelDownloaded()` short-circuiting before
  any manifest check — is closed. It maps its model names onto downloader keys
  and calls `downloader.isModelDownloaded()` whenever a manifest is present,
  keeping its legacy existence checks as the fallback. A pre-existing corrupt
  tree now triggers a re-download instead of reading as installed.
- **A file rejected by the hf-CLI tier is quarantined.** That tier lets the CLI
  write under final names, so a rejection used to leave the bad file in place;
  if the direct-HTTP fallback then failed, the next run's size-only check blessed
  it — a same-size corruption was never re-hashed. Failures are now renamed to
  `<name>.corrupt.<timestamp>`, which keeps the bytes (they are the user's disk
  state, and evidence) while taking them out of every check.
- **A manifest entry must cover everything `MODELS` declares.** `isModelDownloaded`
  checked only the paths the entry happened to list, so a subset entry — or an
  empty one, where `[].every()` is vacuously true — could mark an incomplete
  model installed. An entry that does not cover every expanded file is now
  treated as unverifiable and falls back to the legacy existence result, never
  to a manifest-backed `true`. `scripts/generate-model-manifest.js` warns loudly
  when `--model=` runs with no manifest to merge into, which is how such an
  entry gets written.
- **Staging-name orphans.** Direct HTTP downloads to `<dest>.download`, so the
  helper's resume file moved to `<dest>.download.partial`; a pre-upgrade
  `<dest>.partial` is now migrated onto it rather than orphaned, and a complete
  `<dest>.download` (interrupted during hashing) goes straight to verification
  instead of re-fetching gigabytes to reach the same check.
