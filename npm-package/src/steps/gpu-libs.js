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
 * The amendment (item 7) goes further, and it is right to: receipt + one
 * sentinel .so is the SAME pattern at smaller scale. Forty files are
 * extracted and exactly one is proven. So this check never returns
 * `healthy` — the best it can honestly say is `unknown`, and `unknown` is
 * reported as such rather than laundered into a green row. Promoting it
 * requires a per-extracted-file manifest, which is Phase-B work.
 *
 * Because it is never healthy, the runner always calls run() on a GPU box.
 * That is correct and cheap: downloadGPULibraries() does its own receipt
 * check and, on the skip path, still removes superseded shadowing copies
 * left inside the npm platform package — work the check cannot express and
 * which a short-circuit would silently drop.
 *
 * A Linux machine with no NVIDIA card is `not-applicable`, not `blocked`:
 * there is nothing to repair, ever, and doctor must not nag about it.
 */

const fs = require('fs');
const path = require('path');
// No `healthy` import: this check cannot return it — see the fence above.
const { unhealthy, unknown, componentOk, componentFailed } = require('./health');

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

  check(ctx) {
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

    // Deliberately NOT healthy (amendment 7): one sentinel cannot vouch for
    // the whole extracted set. Phase B adds the per-file manifest that can.
    return unknown('GPULIBS_UNVERIFIED',
      `v${receipt.version} (${receipt.variant}) present, but only the sentinel library is verified`, {
        evidence: [
          `receipt: ${receiptPath()}`,
          `sentinel: ${sentinel}`,
          'per-file integrity manifest is Phase-B work (ADR-037 amendment 7)',
        ],
      });
  },

  async run(ctx) {
    const postinstall = require('../../postinstall');
    const before = sentinelPresent();
    try {
      await postinstall.downloadGPULibraries();
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
  _internals: { receiptPath, sentinelPath, readReceipt, sentinelPresent },
};
