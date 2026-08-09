/**
 * CUDA/ONNX-Runtime GPU library step — Linux + NVIDIA only. ADR-037.
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * This is the step ADR-035 was written about: the receipt is NOT the goods.
 * gpu-package-info.json lives in the config dir and survives every npm
 * upgrade; the 1.5 GB of libraries used to live inside the npm platform
 * package and were deleted by every npm upgrade. The skip check believed the
 * receipt, downloaded nothing, and the daemon silently fell back to CPU.
 *
 * Phase A could not do better than `unknown`, because receipt + one sentinel
 * .so is the SAME pattern at smaller scale: forty files extracted, exactly
 * one proven. Phase B pays that debt. `gpu-libs.manifest.json` is generated
 * FROM the extracted bytes — each file hashed as it streams into
 * getGpuLibsDir() — and lives beside the libraries rather than in the config
 * dir, so it cannot outlive the files it describes. With that manifest on
 * disk this check verifies the whole inventory and its sizes, and only then
 * may it say `healthy`.
 *
 * The escalation ladder is deliberate about what each rung proves:
 *   no manifest      → `unknown`. Pre-Phase-B directory: real files, nothing
 *                      that can vouch for them. Never healthy (amendment 7).
 *   inventory+sizes  → `healthy`. What check() runs; a stat() per file.
 *   sha256 per file  → `doctor --deep` via deepCheck(). Hashing 1.5 GB is not
 *                      something a check on every install may do, so it is an
 *                      explicit request, never a default.
 *
 * A Linux machine with no NVIDIA card is `not-applicable`, not `blocked`:
 * there is nothing to repair, ever, and doctor must not nag about it.
 */

const fs = require('fs');
const path = require('path');
const { healthy, unhealthy, unknown, componentOk, componentFailed } = require('./health');
const gpuManifest = require('../gpu-libs-manifest');

function receiptPath() {
  return path.join(require('../paths').getConfigDir(), 'gpu-package-info.json');
}

function sentinelPath() {
  return path.join(require('../paths').getGpuLibsDir(), 'libonnxruntime.so');
}

function readReceipt() {
  try {
    const parsed = JSON.parse(fs.readFileSync(receiptPath(), 'utf8'));
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * A sentinel that proves *something* was extracted.
 *
 * existsSync alone is not enough: an interrupted copy leaves a zero-byte
 * file, and a zero-byte libonnxruntime.so is not a library — it is the
 * receipt-vs-goods bug wearing the goods' name. This is the weakest claim
 * worth making, which is exactly why the verdict above it is `unknown`.
 */
function sentinelPresent() {
  try {
    const stat = fs.statSync(sentinelPath());
    return stat.isFile() && stat.size > 0;
  } catch {
    return false;
  }
}

/**
 * The verdict, as a plain function so deepCheck() can reuse the shallow rungs
 * without going back through module.exports.
 */
function inspect(ctx) {
    const postinstall = require('../../postinstall');
    const receipt = readReceipt();
    const sentinel = sentinelPath();

    if (!receipt) {
      return unhealthy('GPULIBS_NO_RECEIPT', 'no GPU library receipt on disk', {
        evidence: [receiptPath()],
      });
    }
    if (receipt.version !== postinstall.GPU_LIBS_VERSION) {
      return unhealthy('GPULIBS_STALE_VERSION',
        `receipt is v${receipt.version}, this package ships v${postinstall.GPU_LIBS_VERSION}`, {
          evidence: [receiptPath()],
        });
    }
    // The goods, independently of the receipt — the ADR-035 bug in one line.
    if (!sentinelPresent()) {
      return unhealthy('GPULIBS_MISSING',
        'receipt present but the libraries themselves are missing or empty', {
          evidence: [`receipt: ${receiptPath()}`, `expected a non-empty file at: ${sentinel}`],
        });
    }

    // `null` means the probe ran and could not tell. Accepting whatever is
    // installed in that case would be the receipt vouching for itself, so it
    // fails CLOSED into unknown rather than passing.
    const wanted = ctx.gpuVariant;
    if (wanted === null) {
      return unknown('GPULIBS_VARIANT_UNKNOWN',
        'cannot determine which library variant this GPU needs', {
          evidence: [
            `installed variant: ${receipt.variant}`,
            'nvidia-smi did not report a usable compute capability',
          ],
        });
    }
    if (wanted && receipt.variant !== wanted) {
      return unhealthy('GPULIBS_WRONG_VARIANT',
        `installed variant "${receipt.variant}" but this GPU wants "${wanted}"`, {
          evidence: [receiptPath()],
        });
    }

    // The per-file manifest is what separates "the receipt says so" from
    // "every file is here at the size it was written". Without it there is
    // still exactly one proven library, so the verdict stays `unknown`.
    const gpuLibsDir = require('../paths').getGpuLibsDir();
    const manifest = gpuManifest.readManifest(gpuLibsDir);
    if (!manifest) {
      return unknown('GPULIBS_UNVERIFIED',
        `v${receipt.version} (${receipt.variant}) present, but only the sentinel library is verified`, {
          evidence: [
            `receipt: ${receiptPath()}`,
            `sentinel: ${sentinel}`,
            `no per-file manifest at ${gpuManifest.manifestPath(gpuLibsDir)}`,
            'reinstalling regenerates it and promotes this row to healthy',
          ],
        });
    }

    // A manifest describing a different variant or version than the receipt
    // is two records disagreeing about the same directory. Neither can be
    // trusted over the other, so the honest verdict is that the set is wrong.
    if (manifest.variant !== receipt.variant || manifest.version !== receipt.version) {
      return unhealthy('GPULIBS_MANIFEST_MISMATCH',
        'the file manifest and the receipt describe different library sets', {
          evidence: [
            `receipt: v${receipt.version} (${receipt.variant})`,
            `manifest: v${manifest.version} (${manifest.variant})`,
          ],
        });
    }

    const inventory = gpuManifest.verifyInventory(manifest, gpuLibsDir);
    if (!inventory.ok) {
      return unhealthy('GPULIBS_INCOMPLETE',
        `${inventory.missing.length} missing and ${inventory.mismatched.length} wrong-sized library file(s)`, {
          evidence: [
            ...inventory.missing.slice(0, 5).map(name => `missing: ${name}`),
            ...inventory.mismatched.slice(0, 5),
            `manifest: ${gpuManifest.manifestPath(gpuLibsDir)}`,
          ],
        });
    }

    return healthy('GPULIBS_VERIFIED',
      `v${receipt.version} (${receipt.variant}): ${inventory.checked} files present at their recorded sizes`, {
        evidence: [
          `manifest: ${gpuManifest.manifestPath(gpuLibsDir)}`,
          'sizes only — run "swictation doctor --deep" to verify contents',
        ],
      });
}

/**
 * Full content verification — `doctor --deep` only.
 *
 * Sizes catch truncation and deletion; they cannot catch a file that was
 * replaced with something else of the same length, which is the case a
 * supply-chain question actually asks about. Hashing is streamed and never
 * buffered (individual CUDA libraries reach 500 MB), and it is only ever
 * reached because the user asked for it by name.
 */
async function inspectDeep(ctx) {
  const shallow = inspect(ctx);
  if (shallow.code !== 'GPULIBS_VERIFIED') return shallow;

  const gpuLibsDir = require('../paths').getGpuLibsDir();
  const manifest = gpuManifest.readManifest(gpuLibsDir);
  if (!manifest) return shallow;

  const result = await gpuManifest.verifyHashes(manifest, gpuLibsDir);
  if (!result.ok) {
    return unhealthy('GPULIBS_CORRUPT',
      `${result.failures.length} of ${result.checked} library file(s) failed sha256 verification`, {
        evidence: result.failures.slice(0, 10),
      });
  }
  return healthy('GPULIBS_HASH_VERIFIED',
    `${result.checked} files match the sha256 recorded when they were extracted`, {
      evidence: [`manifest: ${gpuManifest.manifestPath(gpuLibsDir)}`],
    });
}

module.exports = {
  id: 'gpu-libs',
  // postinstall prints this banner for the slot; on macOS the same slot runs
  // the CoreML runtime download, which is not a registry step.
  title: 'Downloading GPU libraries...',
  entrypoints: ['postinstall', 'setup'],
  after: [],
  needsNetwork: true,

  /**
   * Scope, not capability: Linux with an NVIDIA card. `hasNvidiaGpu` is a
   * resolved context fact so neither applies() nor check() shells out.
   */
  applies(ctx) {
    return ctx.platform === 'linux' && ctx.hasNvidiaGpu === true;
  },

  check: inspect,
  deepCheck: inspectDeep,

  async run(ctx) {
    const postinstall = require('../../postinstall');
    const before = sentinelPresent();
    try {
      // Hardware facts come from the context, not a second nvidia-smi probe
      // (ADR-037 amendment 4 / codex #6): a probe that disagrees with the one
      // that chose the variant writes a receipt describing a different GPU.
      await postinstall.downloadGPULibraries({
        hasNvidiaGpu: ctx.hasNvidiaGpu,
        gpuCompute: ctx.gpuCompute,
      });
    } catch (err) {
      return {
        changed: false,
        components: [componentFailed('gpu-libs-download', err.message, err)],
        warnings: [],
      };
    }

    const after = sentinelPresent();
    if (!after) {
      return {
        changed: false,
        components: [componentFailed('gpu-libs-download',
          `libraries still absent at ${sentinelPath()}`)],
        warnings: [],
      };
    }
    return {
      changed: !before,
      components: [componentOk('gpu-libs-download', before ? 'already installed' : 'installed')],
      warnings: [],
    };
  },

  // Exported for tests, which drive the predicate against fixture trees.
  _internals: { receiptPath, sentinelPath, readReceipt, sentinelPresent, gpuManifest },
};
