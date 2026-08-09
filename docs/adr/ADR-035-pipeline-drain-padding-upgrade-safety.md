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
     Tests: `tests/test-config-step.js` (7 cases, wired into `npm test`).
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
   - **Deferred (deliberate) — resolved 2026-08-08:** the parking_lot/poison-free mutex
     sweep (~40 sites in pipeline.rs, main.rs, metrics collector). The known panic sites
     that triggered poisoning were eliminated above; the sweep was mechanical but wide,
     so it landed as its own change rather than a rushed tail to this one.
     **Swept:** 46 lock sites — `pipeline.rs` (12: audio/vad/stt/metrics/session_id/
     broadcaster), `main.rs` (6: the metrics lock in toggle/updater, plus the
     `last_toggle` debounce mutex), `metrics/collector.rs` (28). `std::sync::Mutex` →
     `parking_lot::Mutex` in those three files only; `.lock()` returns the guard
     directly, so the `.unwrap()`s are gone and the VAD/STT `match`-on-lock-result arms
     (which `continue`d past a poisoned lock) collapse to direct locking — the failure
     mode they handled no longer exists. **Not touched:** every `tokio::sync` lock, and
     every lock ORDERING (the deadlock-history comments are preserved verbatim) — this
     was poison-elimination only. `database.rs`'s `Arc<Mutex<Connection>>` is still std
     (outside the scope this note named). Proof: `cargo test --workspace` green (142
     passed, 0 failed), `cargo clippy -p swictation-daemon -p swictation-metrics
     --all-targets` 0 errors and no new warnings, `cargo fmt --check` clean.

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
- ~~PTT events queued during a slow stop-drain are discarded by the toggle
  handler's queue drain (a held press spanning the drain can invert state).~~
  **Resolved 2026-08-08.** The drain no longer treats all queued events alike.
  `HotkeyManager::try_drain` (count-and-discard) is replaced by `drain_stale`,
  which reduces the batch through the pure, testable
  `reduce_drained_events(ptt_held, &[HotkeyEvent]) -> DrainOutcome`: stale
  `Toggle`s are counted and dropped (the toggle they would have driven has
  already run), while push-to-talk events are *replayed* against the hold
  state, because a press and its release are a pair — discarding the press
  while honouring the release is exactly what started a recording after the
  user let go. A press+release inside one batch cancels out; a batch that
  leaves the key held owes a press; one that leaves it released owes a release.
  The event loop now tracks `ptt_held` and a release with no matching active
  press is a no-op (key repeat collapses to one press). Fences:
  `drain_swallowing_a_press_does_not_leave_its_release_starting_a_recording`
  encodes the inversion, plus 7 sibling cases in `hotkey.rs`.
- ~~`stt_model_override` written by postinstall after a model test is preserved
  by the config step as if user-authored; if the GPU later disappears the
  forced branch errors (visibly, post-hardening) instead of falling back.~~
  **Resolved 2026-08-08.** Installer-authored values are now distinguishable
  from user-authored ones: `updateConfigWithTestedModel()` records what it wrote
  to a sidecar (`<config-dir>/postinstall-state.json`, key `managedOverride`),
  and `src/steps/config.js` reverts the override to `"auto"` when the config
  still holds exactly that value — so every install re-tests the hardware and a
  vanished GPU falls back instead of erroring. A value differing from the
  sidecar is the user's and is never touched. The reset is opt-in per call
  (`resetManagedOverride`) and wired only to the **pre-download** pass; the
  post-download pass must not undo the model just verified. Reuses the
  simple-single-line TOML guard from path healing. Tests:
  `tests/test-config-step.js` (managed value reset incl. the post-download
  no-op, user value preserved). The marker's *lifetime* was still wrong at this
  point — see the installer-safety round below.
- ~~Session metrics include drain time as wall time; UI state reads Recording
  during the (typically sub-second) drain.~~ **Resolved 2026-08-08.** Both
  halves come from the same instant. `stop_recording()` stamps `Instant::now()`
  the moment the mic detaches (audio stopped, callback replaced) and carries it
  on `DrainHandles::capture_stopped_at()`. *Metrics:*
  `MetricsCollector::end_session` takes an `Option<Instant>` end time —
  `None` keeps the old measure-to-now behavior for every other caller — and
  derives both the wall-clock stamp and `total_duration_s` from it, so STT
  inference on the tail utterance is no longer billed as dictation (and no
  longer deflates WPM). `end_session` still runs AFTER the drain: segments
  transcribed during it are added to the still-open session; only the end
  *timestamp* comes from before. *UI:* `toggle_inner` flips daemon state to
  Idle BEFORE draining. Broadcasting alone would not have held — the metrics
  updater re-broadcasts daemon state every second, so the internal state had
  to move too; it is taken and released on its own, adding no lock-order edge
  (the drain still happens outside all daemon locks). That internal flip is
  the load-bearing half: the explicit Idle broadcast was later moved behind
  `end_session` for ordering (see the wave-3 review round below), so the UI
  now learns of Idle from the updater's next tick during a drain.
  Fences: `end_session_at_capture_stop_excludes_the_drain` and
  `end_session_without_an_end_time_still_measures_up_to_now`.
- ~~Old GPU libraries under the platform package are not cleaned up, and
  postinstall's model verification may exercise the platform ORT while the
  service uses the user-data ORT.~~ **Resolved 2026-08-08.** Both halves of the
  split-brain are closed. *Cleanup:* after a successful extraction into
  `getGpuLibsDir()`, `cleanupSupersededGpuLibs()` removes shadowed copies from
  the platform package's `libDir` — restricted to shared-object names
  (`libfoo.so[.N…]`) the new directory also provides, so platform files that
  were never duplicated survive; it no-ops when the platform package is absent
  or the two paths resolve to the same directory, and every failure is logged
  and swallowed (a redundant library must never fail an install). *Verification
  parity:* `detectOrtLibrary()` now checks `getGpuLibsDir()` as PRIORITY 1
  (platform package demoted to 2, bundled to 3, Python to 4), mirroring
  `generate-service.js`'s LD_LIBRARY_PATH order — model verification and the
  generated service now load the same ONNX Runtime. Proof: the four cleanup
  behaviors above were exercised against the shipped function body with
  stubbed `require`/`log` over fixture directories. Two unsafe cases in that
  cleanup — and a gap in when it runs — are closed in the installer-safety
  round below.
Also corrected by review: the metrics updater currently releases `state`
before locking `metrics`, so the deadlock this ADR guards against is
prophylactic given the documented lock-order history, not currently live —
the drain-outside-locks design stands regardless.

**Smaller items — resolved 2026-08-08.** Three findings from the follow-up audit of
the VAD and injection path. They are grouped here because each was small and none
disturbs the drain design above.

- **`max_speech_duration` was never wired.** It was validated and documented
  ("segments longer than this are split"), but `VadDetector::new` did not pass it to
  the detector, so continuous speech grew one unbounded buffer that reached STT only
  when the stream ended. The duration now reaches the segmenter, which force-emits at
  the cap on a window boundary and stays triggered so the next segment continues.
  Consequence handled with it: one `process_audio` call spans many windows and can now
  complete more than one segment, where the old loop overwrote its result variable and
  kept only the last. Completed segments are queued in order (`drain_pending()` exposes
  any remainder); ~~with realistic settings the queue never fills, so pipeline behaviour
  is unchanged~~ — **wrong, corrected below:** nothing called `drain_pending()`, and the
  queue fills for exactly the speaker the cap exists for.
- **No pre-roll.** Buffering began at the first window that *crossed* the threshold,
  discarding the window containing the consonant that caused the crossing (~64 ms of
  word onset). The last two pre-trigger windows are now retained and prepended when
  speech triggers.
- **`inject_text` executed `<KEY:...>` markers found in its own input.** That input is
  STT output after the user's hot-reloadable correction table, so a dictated phrase —
  or a correction whose replacement contained a marker — could press arbitrary key
  combinations in whatever window had focus. A repo-wide sweep found no producer:
  midstream's `text-transform` has an `is_key_action` branch, but no rule in its table
  has a `<KEY:` replacement. The interpretation and the key-synthesis helpers are
  therefore removed and injection is always literal. Note this required fixing
  `macos_text_inject.rs` as well as `text_injection.rs` — the macOS injector carried
  its own copy of the parser, and `inject_text` delegates straight into it, so a
  Linux-only fix would have been a no-op on the platform the daemon ships on.

Proof: the segmentation rules moved into a session-free `SpeechSegmenter`, so pre-roll,
silence tolerance and the duration bounds are unit-testable by feeding probabilities
with no model on disk (4 tests). The split is additionally proven against the real
Silero model — `test_continuous_speech_is_split_at_max_speech`, `#[ignore]`d by default,
run locally with `ORT_DYLIB_PATH` set: 5.12 s of unbroken speech under a 1 s cap yields
5 bounded segments of 16384 samples totalling every sample fed, against a single
79872-sample segment before the fix. Literal injection is covered by a payload test
asserting the typed UTF-16 recombines to the input verbatim, markers included; it was
mutation-checked (reintroducing marker stripping fails it). `cargo test -p swictation-vad
-p swictation-daemon` green (84 passed, 0 failed), `cargo clippy --all-targets` 0 errors
with no warnings in the touched files, `cargo fmt --check` clean.

Verification caveats, in the same spirit as the note below: the `TextInjector` and
`MacOSTextInjector` construction tests still skip when Accessibility permission (or a
Linux injection tool) is absent, as they did before — the non-skipping payload test is
what actually exercises the literal path. The Linux `#[cfg]` branch of
`text_injection.rs` was edited but not compiled; no Linux target is installed on this
build host, and its change is the deletion of the marker functions plus a one-line
call.

## Review round (over the wave-3 VAD and injection work) — resolved 2026-08-08

Four findings against the smaller-items work above. The first two are dictation
loss and dictation corruption respectively; both were introduced by wiring
`max_speech`.

- **P1 — the queued VAD segments had no consumer.** `process_audio` returns the
  oldest segment a call completed and queues the rest in `pending_segments`,
  because with `max_speech` splitting one call can finish several. Nothing ever
  called `drain_pending()`: the queue was written, documented, and never read,
  so every segment past the first was dropped — the note above ("with realistic
  settings the queue never fills") was an assumption, and it is exactly the
  speaker who never pauses, whom the cap exists for, who violates it. The
  pipeline's VAD task now drains after every call and forwards all of them in
  order, and the stop path drains BEFORE flushing (`flush()` pushes onto the
  same queue and returns its front, so flushing first would reorder the tail).
  `start_recording` also resets the detector, so a segment finished in one
  session can never lead the next one. Both orderings live in pure functions
  (`payloads_from_call`, `payloads_at_stop`) rather than in the task loop.
- **P1 — a cap-split followed by silence emitted a segment of pure silence.**
  The force-emit at the cap deliberately stays `triggered`, but it leaves the
  buffer EMPTY. The silence that follows is then buffered under the tolerance
  and closed by `min_silence` as a segment of its own, and it passes
  `min_speech` on length alone — so silence was handed to STT and whatever it
  hallucinated was injected. The segmenter now tracks whether any window at or
  above the threshold entered the buffer since the last emit; the silence-end
  and flush paths emit only if it did, and the cap path takes the buffer either
  way so it stays bounded even under a pathological `min_silence >=
  max_speech`. Content, not length, now decides.
- **P2 — the stop broadcasts were unordered against the next toggle's.**
  `end_session` and the Idle state change were two independently spawned tasks,
  so they could run in either order and either could land after the *next*
  toggle's `start_session`/Recording pair — a replayed push-to-talk press
  starts that next toggle immediately. They are now one spawned task
  (`end_session` then Idle) awaited before the stop branch returns, so
  `toggle_lock` orders them ahead of the next toggle. Still spawned rather than
  inlined, so a panicking client broadcast cannot take the toggle down. No lock
  is held at that point, so this adds no lock-order edge; the deadlock-history
  comments are untouched. Consequence, accepted: the explicit Idle broadcast
  now follows the drain instead of preceding it, so during a drain the UI
  learns of Idle from the metrics updater's next tick (≤1 s) rather than
  immediately — the internal state flip, which is what the updater reads, still
  happens before the drain.
- **Stale doc strings after the `<KEY:...>` removal.** Three places still
  described marker interpretation as live behaviour: `text_injection.rs`'s
  module summary ("with keyboard shortcut support"), the text-injection box in
  `docs/architecture/ARCHITECTURE_DIAGRAM.md` ("`<KEY:...>` markers → key
  events"), and `docs/specs/macos-support-specification.md`, which still
  specified `send_cgevent_keys()` and its `parse_modifiers()` helper. All three
  now state the literal-injection reality; the spec's two functions are deleted
  and replaced with a comment saying why there is deliberately no
  key-combination path.

Accepted limitation (P3): segments drained after the stop hotkey carry
`Utc::now()` timestamps taken during the drain, while the session's end
timestamp is retro-stamped to the instant capture detached. A tail segment can
therefore have a timestamp later than its own session's end. Metrics totals are
unaffected (segments are summed, not clipped to the window), but a consumer
reconstructing a timeline from segment timestamps should treat the session end
as the moment dictation stopped, not as an upper bound on segment times.

Proof: `cargo test --workspace` green (165 passed, 0 failed). The cap-split fix
is fenced by three segmenter tests — `cap_split_followed_by_silence_emits_nothing`
and `cap_split_followed_by_stop_flushes_nothing` (both fail against the previous
implementation; verified) and `cap_split_then_more_speech_emits_that_speech`,
which pins that the fix does not swallow real speech recorded after a split. The
forwarding orderings are fenced by `every_segment_one_vad_call_completed_is_forwarded_in_order`
and `stop_forwards_queued_segments_before_the_flush` in `pipeline.rs`. That the
queue is load-bearing rather than theoretical is proven against the real Silero
model by `one_call_queues_the_segments_it_cannot_return_and_reset_clears_them`
(`#[ignore]`d, run locally with `ORT_DYLIB_PATH` set): one call over 8 windows
under a one-window cap returns 1 segment and queues 7, the 8 recombine to the
input in order, and `reset()` empties the queue. `cargo clippy -p swictation-vad
-p swictation-daemon --all-targets` 0 errors, no warnings in the touched files;
`cargo fmt --check` clean.

Verification caveat: the model-backed test aborts the test binary at process
teardown (`mutex lock failed` out of the ORT dylib's static destructors) after
reporting `ok`. This is pre-existing and unrelated — the older
`test_continuous_speech_is_split_at_max_speech` does the same on this host — but
it means the ignored tests cannot be gated on a clean exit code, only on their
reported result.

## Installer-safety round (codex over the wave-3 JS) — resolved 2026-08-08

Four findings against the upgrade-safety code this ADR introduced. All four are
in the installer's JS; none touches the daemon.

- **GPU cleanup could delete the fallback ONNX Runtime.** `libonnxruntime.so`
  exists in *both* the downloaded GPU set and the platform package, where
  `packages/linux-x64/scripts/build.sh` (step 8) ships it deliberately as the
  CPU/fallback runtime and `generate-service.js` points LD_LIBRARY_PATH at it
  whenever gpu-libs has no ORT of its own. "Superseded" matched it by name, so
  cleanup removed the only runtime a GPU-less install could fall back to. The
  name is now on a `PLATFORM_OWNED_LIBS` keep-list.
- **GPU cleanup could mutate another install's tree.** `resolve-binary`
  consults `npm root -g` *before* the local tree, so a local
  `npm install swictation` resolves the GLOBAL platform package — and the
  local postinstall would then delete out of it. Cleanup now runs only when
  the resolved `libDir` lies inside the `node_modules` tree this package
  itself lives in, compared after `fs.realpathSync` on both sides so a
  symlinked install (npm link, pnpm store) cannot alias past the guard. A dev
  checkout has no `node_modules` ancestor and is skipped outright.
- **Cleanup never ran on the upgrade that needed it most.** The call sat inside
  the `!skipDownload` branch. But the shadowing copies are restored by
  reinstalling the *platform package*, which is exactly the upgrade whose
  gpu-libs receipt still matches — so it downloads nothing, skips the branch,
  and leaves both copies on disk with LD_LIBRARY_PATH order deciding which
  CUDA/ORT build the daemon loads. The call moved out of the branch, gated on
  the gpu-libs sentinel so it still proves the authoritative set is present.
- **The managed-override marker was never consumed (ABA).** `managedOverride`
  was written but never deleted, so it stayed a permanent claim on the key.
  Once the installer had written `0.6b-gpu`, a user who later chose that same
  value deliberately would have it reset to `"auto"` on the next install —
  indistinguishable from the installer's own write. The marker is now
  one-shot: `clearManagedOverride()` deletes it after a completed reset, and
  also as soon as the configured value differs from it (ownership has moved to
  the user), so the A-B-A return can never be misread.
- **Recording the override failed open.** `recordManagedOverride()` ran
  *after* the forced model was written to `config.toml`, and its failure was
  ignored — leaving a forced model that looks user-authored forever, which no
  install could re-test. It now runs first, and the config write is skipped
  with a warning if it fails: `"auto"` stays, and the daemon's own detection
  picks a model at runtime. Relatedly, `readPostinstallState()` returned
  whatever `JSON.parse` produced, so a sidecar holding `null`, an array or a
  scalar was indexed as a record; it now validates a plain object first.

Proof: `node --check` on every touched file; `tests/test-config-step.js`
extended to 10 cases (marker consumed by a completed reset; the ABA case where
a user re-selects a previously-managed value after an edit and must never be
reset; non-object sidecar JSON ignored); new `tests/test-gpu-lib-cleanup.js`
(6 cases) covers the keep-list, the shared-object name filter, the containment
guard incl. the `node_modules-old` prefix sibling, symlink defeat, and the dev
checkout. Both new fences were mutation-checked — reverting the fix fails them.
`npm test` green. To make the cleanup guards testable, `postinstall.js` now
exports its pure helpers and guards the auto-run with `require.main === module`;
`node postinstall.js` is unaffected (verified both directions: requiring it
returns in ~5 ms without installing, running it still prints the install
banner).

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
