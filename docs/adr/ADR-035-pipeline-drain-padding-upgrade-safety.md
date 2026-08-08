# ADR-035: True-Length Decoding, Stop-Drain Protocol, Upgrade Safety, Panic Hardening

- **Status:** Accepted
- **Date:** 2026-08-08
- **Issue:** Phase 2 of the six-reviewer audit (see ADR-034): latency/hallucination bug in STT chunking, lost final utterance on stop, config/GPU-lib destruction on upgrade, silent STT death
- **Release:** v0.7.36 (planned)

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Decisions

1. **True-length decoding — no padding anywhere in the ONNX STT path.**
   `chunk_features()` padded every final chunk to `CHUNK_FRAMES = 10,000` mel frames
   (100 s at the 10 ms hop): a 3-second utterance ran the encoder over ~97 % synthetic
   frames, and because per-feature normalization ran BEFORE padding, the pad was
   mean-valued plausible input — the source of both the wasted latency and the phantom
   "um"-style hallucinations users reported. The encoder handles variable frame counts
   via dynamic shape inference (615 non-multiple frames verified per run_encoder's own
   note), so `CHUNK_FRAMES` is now a ceiling for bounded memory, never a pad target,
   and the length tensor is true by construction. **Fences kept as tests:**
   `chunk_features_preserves_every_frame_across_chunks` encodes the long-audio
   truncation bug the constant was originally raised to fix;
   `chunk_features_short_input_is_not_padded` encodes this fix.

2. **Stop-drain protocol — the final utterance is transcribed, exactly once.**
   `stop_recording()` used to discard the VAD flush (a workaround for a duplicate-
   injection bug) and never awaited the session tasks. New protocol: stop capture →
   replace the audio callback (dropping the channel's only sender) → the VAD task
   drains queued audio, flushes the detector, and forwards tail speech through the
   SAME channel streamed segments use → the single STT consumer drains everything →
   idle. Duplicate injection is structurally impossible (one consumer, one path) and
   nothing is discarded. `stop_recording()` is now sync and returns `DrainHandles`;
   **draining happens OUTSIDE the daemon state/pipeline locks** (in `toggle_inner`),
   because it runs STT inference and takes the metrics lock, and the metrics updater's
   lock order (metrics → state) would deadlock otherwise. Fences:
   `stop_drain_delivers_streamed_segments_then_flush_exactly_once` (the duplicate-
   injection bug), `drain_awaits_both_tasks_and_aborts_monitor`. The backpressure
   monitor's inverted exit (`break` on zero drops) is fixed by abort-on-stop.

3. **Upgrade safety.**
   - *Config:* `interactiveConfigMigration()` no longer clobbers `config.toml`. New
     step-contract module `src/steps/config.js` (check/run — the first step of the
     hybrid migration, see below): absent → defaults; unparseable → timestamped backup
     + defaults + warning; parseable → preserved **byte-for-byte** (comments included),
     healing only model-path keys whose configured target is missing while the platform
     default exists (the ADR-033 stale-path crash-loop case). Since ADR-034 every daemon
     key has a compiled default, partial configs are fully valid — postinstall never
     needs to reconstruct one. `smol-toml` (small, pure-JS) added for parse validation;
     the PRD's "no new dependencies" constraint dies with the PRD (see decision 5).
     Tests: `tests/test-config-step.js` (5 cases, wired into `npm test`).
   - *GPU libraries:* extraction retargeted from the npm-owned platform package to
     `getGpuLibsDir()` (`~/.local/share/swictation/gpu-libs`) — npm upgrades deleted
     the libraries while the metadata receipt survived, so the skip check reported
     "already installed" and the daemon silently fell back to CPU. The skip check now
     requires the metadata match AND a sentinel artifact (`libonnxruntime.so`) on disk.
     `detectCudaLibraryPaths()` and `generate-service.js` already prefer the new
     location (it was PRIORITY 1 dead code until now).

4. **Panic hardening + task-death surfacing.**
   - `total_cmp` replaces `partial_cmp().unwrap()` in both greedy argmaxes (a single
     NaN logit no longer kills the decode loop).
   - Joiner-output vs vocabulary size is validated per frame with a diagnostic error
     instead of a usize-underflow panic; `load_tokens` rejects blank lines (positional
     IDs — silently skipping would shift the whole vocabulary) and empty vocabularies.
   - When the STT task dies mid-session, the VAD task now sends an error through the
     transcription channel — previously recording, hotkeys, and metrics kept "working"
     while transcription was silently dead until restart.
   - **Deferred (deliberate):** the parking_lot/poison-free mutex sweep (~40 sites in
     pipeline.rs, main.rs, metrics collector). The known panic sites that triggered
     poisoning are eliminated above; the sweep is mechanical but wide and lands in the
     next batch, not as a rushed tail to this one.

5. **Postinstall hybrid migration — spike verdict accepted, PRD constraint superseded.**
   The spike reconstructed the fence: the all-in-one rule at
   `docs/archive/prd-postinstall-resilience.md:29` was a scoping note in an AI-generated
   PRD committed in the same commit as its own implementation (7484264); `swictation
   setup` predates it, no rationale exists anywhere, and postinstall's own recovery
   hints contradict it six times. **Verdict: hybrid** — postinstall still attempts every
   phase (the one-command promise holds), but each phase becomes an idempotent
   `check()`/`run()` step function shared with `swictation setup --<step>`/`--repair`.
   `src/steps/config.js` (decision 3) is the first such step; `generate-service.js`
   (ADR-034) proved the shape. Full migration (~5-6 days, phased) is scheduled work,
   not this release. Also noted for that work: `_totalPhases` is static while phase 5
   is conditional, and the phase [1/9] banner mislabels service teardown.

## Review round (gpt-5.6-sol + Kimi K3 over the diff)

Fixed before commit: config healing re-runs after model download (it ran before
models existed, so stale paths could never heal); the GPU metadata receipt is
invalidated before download so an interrupted extraction can't be blessed by a
surviving receipt + one sentinel; healing is guarded to simple single-line TOML
values (quoted keys / multiline strings are skipped rather than corrupted).

Deferred, accepted as known limitations of this release:
- PTT events queued during a slow stop-drain are discarded by the toggle
  handler's queue drain (a held press spanning the drain can invert state).
- `stt_model_override` written by postinstall after a model test is preserved
  by the config step as if user-authored; if the GPU later disappears the
  forced branch errors (visibly, post-hardening) instead of falling back.
- Session metrics include drain time as wall time; UI state reads Recording
  during the (typically sub-second) drain.
- Old GPU libraries under the platform package are not cleaned up, and
  postinstall's model verification may exercise the platform ORT while the
  service uses the user-data ORT.
Also corrected by review: the metrics updater currently releases `state`
before locking `metrics`, so the deadlock this ADR guards against is
prophylactic given the documented lock-order history, not currently live —
the drain-outside-locks design stands regardless.

## Consequences

- Linux ONNX dictation stops spending ~97 % of encoder compute on synthetic frames;
  phantom end-of-utterance "um" hallucinations lose their mechanism.
- The last words before the stop hotkey are transcribed; duplicated injection cannot
  recur through the drain path.
- Upgrades preserve user config byte-for-byte and can no longer silently strand GPU
  acceleration; a stale-receipt state heals itself by re-downloading.
- A bad logit or mismatched model artifacts produce a diagnosable error instead of a
  daemon that looks alive and types nothing.
- Honest verification caveat: the four model-dependent inference tests remain ignored
  in CI-less local runs; the ONNX end-to-end path needs a GPU box (RELEASE_CHECKLIST)
  before release.
