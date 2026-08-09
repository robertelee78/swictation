/**
 * Speech-model step (ADR-037, wrapping the ADR-036 downloader).
 *
 * ── Fence ───────────────────────────────────────────────────────────────
 * The predicate is ModelDownloader.isModelDownloaded(), never a bare
 * existsSync on the model directory. ADR-036 made that method manifest-aware:
 * it compares every file's SIZE against models.manifest.json, which is what
 * catches the half-written 7 GB tree that an existence check happily reported
 * as installed, and it refuses to count a quarantined `.corrupt.*` file.
 * Reimplementing the check here would resurrect exactly that bug, so this
 * module only decides WHICH models to ask about.
 *
 * The verdict is labelled `size-verified` (amendment 7) rather than plain
 * healthy, because that is what it is: sizes match the manifest, contents
 * were verified at download time, and re-hashing 7 GB on every doctor run is
 * a future `doctor --deep`, not a default.
 *
 * Which models: whatever `ctx.selectedModel` resolved to — a context fact,
 * never rediscovered here (amendment 4) — plus VAD, because
 * autoDownloadModel() fetches ['vad', model] and a daemon without VAD never
 * starts transcribing.
 *
 * cpu-only is a real download, not a no-op. Before ADR-037 the install did
 * fetch a model for cpu-only machines — just late and invisibly, inside
 * showNextSteps()'s second autoDownloadModel() call. That call site is gone;
 * this step owns model downloads outright, so cpu-only maps to 0.6b here.
 */

const { healthy, unhealthy, unknown, componentOk, componentFailed } = require('./health');

/**
 * postinstall's recommendation vocabulary → ModelDownloader keys.
 * Mirrors the modelMap inside autoDownloadModel(); used only to answer
 * check(). run() hands the raw recommendation back to autoDownloadModel and
 * lets it do its own mapping, so there is still one download path.
 */
const DOWNLOADER_KEYS = {
  '0.6b': '0.6b',
  '0.6b-gpu': '0.6b',
  '1.1b': '1.1b',
  '1.1b-gpu': '1.1b',
  'cpu-only': '0.6b',
  '0.6b-coreml': '0.6b-coreml',
  '1.1b-coreml': '1.1b-coreml',
};

/**
 * Downloader keys that must be present on disk for `recommendation`, or null
 * when the recommendation is not one this package knows how to download.
 *
 * Null fails CLOSED at the call site. The previous shape returned ['vad']
 * for an unknown key, so a machine whose recommendation had been renamed or
 * corrupted would report healthy on the strength of a 629 KB VAD file with
 * no speech model on disk at all.
 */
function requiredKeys(recommendation) {
  const key = DOWNLOADER_KEYS[recommendation];
  return key ? ['vad', key] : null;
}

function downloaderFor() {
  const ModelDownloader = require('../../lib/model-downloader.js');
  return new ModelDownloader({ modelDir: require('../paths').getModelsDir() });
}

module.exports = {
  id: 'models',
  // Reused verbatim as postinstall's phase banner — keep in sync with the plan.
  title: 'Downloading speech models...',
  entrypoints: ['postinstall', 'setup'],
  after: ['gpu-libs'],
  needsNetwork: true,
  forbidRoot: true,

  applies() {
    return true;
  },

  check(ctx) {
    const recommendation = ctx.selectedModel;
    if (!recommendation) {
      // Not unhealthy: nothing is broken, we simply have no record of what
      // this machine decided. Running the step resolves it.
      return unknown('MODELS_NO_SELECTION',
        'no model recommendation has been recorded for this machine', {
          evidence: ['gpu-info.json is absent or has no recommendedModel'],
        });
    }

    const required = requiredKeys(recommendation);
    if (!required) {
      return unhealthy('MODELS_UNKNOWN_SELECTION',
        `"${recommendation}" is not a model this package can download`, {
          evidence: [
            `known selections: ${Object.keys(DOWNLOADER_KEYS).join(', ')}`,
            'nothing can vouch for a model that has no download path',
          ],
        });
    }

    let downloader;
    try {
      downloader = downloaderFor();
    } catch (err) {
      return unknown('MODELS_DOWNLOADER_UNAVAILABLE',
        `model downloader unavailable: ${err.message}`);
    }

    const missing = required.filter(key => !downloader.isModelDownloaded(key));
    if (missing.length > 0) {
      return unhealthy('MODELS_MISSING', `missing model(s): ${missing.join(', ')}`, {
        evidence: [
          `selected: ${recommendation}`,
          `models dir: ${require('../paths').getModelsDir()}`,
        ],
      });
    }

    return healthy('MODELS_SIZE_VERIFIED',
      `${required.join(' + ')} present (size-verified against models.manifest.json)`, {
        evidence: [`selected: ${recommendation}`],
      });
  },

  async run(ctx) {
    const postinstall = require('../../postinstall');
    const recommendation = ctx.selectedModel;

    if (!recommendation) {
      return {
        changed: false,
        components: [componentFailed('model-download',
          'no model selected — GPU detection has not run for this machine')],
        warnings: [],
      };
    }

    const required = requiredKeys(recommendation);
    if (!required) {
      return {
        changed: false,
        components: [componentFailed('model-download',
          `"${recommendation}" is not a model this package can download`)],
        warnings: [],
      };
    }

    const before = downloaderFor();
    const already = required.every(key => before.isModelDownloaded(key));

    const ok = await postinstall.autoDownloadModel(recommendation);
    if (!ok) {
      return {
        changed: false,
        components: [componentFailed('model-download', `download failed for ${recommendation}`)],
        warnings: [],
      };
    }
    return {
      changed: !already,
      components: [componentOk('model-download', already ? 'already present' : `downloaded ${recommendation}`)],
      warnings: [],
    };
  },

  // Exported for tests, which drive the predicate against fixture trees.
  _internals: { DOWNLOADER_KEYS, requiredKeys },
};
