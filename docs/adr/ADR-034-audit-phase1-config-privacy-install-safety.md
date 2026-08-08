# ADR-034: Audit Phase 1 — Config Robustness, Transcript Privacy, Install Safety, Model Decision

- **Status:** Accepted
- **Date:** 2026-08-08
- **Issue:** Six-reviewer repo audit (4× Claude Fable 5 subagents, gpt-5.6-sol via codex, Kimi K3 via opencode) found first-contact breakage, a privacy leak, and destructive install paths; all findings verified against source at 435c3b7
- **Release:** v0.7.36 (planned)

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Context

A full-repo audit (installation, configuration, usability, daemon pipeline, STT model
landscape) produced ten critical findings. This ADR records the decisions for the subset
approved for immediate execution, plus the model decision. Findings were cross-verified:
every claim below was confirmed at the cited file:line before acceptance.

1. **Example config is unloadable as instructed.** `config/config.example.toml` says
   "copy this to ~/.config/swictation/config.toml", but `DaemonConfig` (config.rs:73) has
   no container-level `#[serde(default)]`, so the example's commented-out `socket_path`
   fails parsing (`missing field`). Tilde paths are never expanded (daemon resolves a
   literal `./~`). The example's 0.6B model dir (`sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx`)
   matches nothing the downloader creates (`parakeet-tdt-0.6b-v3-onnx`).
2. **Two divergent systemd generators.** `postinstall.js` renders
   `templates/swictation-daemon.service.template` (with `ORT_DYLIB_PATH`, marked
   "CRITICAL - without this, model produces blank output"); `bin/swictation`'s inline
   `generateSystemdService()` omits it. Six postinstall error handlers direct users to
   `swictation setup`, which overwrites the good unit with the broken one.
3. **Transcript text is persisted to journald.** `pipeline.rs:599` and `main.rs:781` log
   full dictated text at `info`; the installed unit sets `RUST_LOG=info`. This defeats
   `store_transcription_text=false` and the product's privacy promise.
4. **preuninstall sudo-deletes `/opt/swictation`.** The "legacy Python cleanup" list
   includes `/opt/swictation` — on any machine keeping a source checkout there, uninstall
   destroys it. Separately its trigger guard (`process.argv.includes('uninstall')`) never
   fires under npm, so cleanup both never runs normally AND is catastrophic when forced.
5. **macOS models download to a directory the daemon never reads.**
   `ModelDownloader` hardcodes `~/.local/share/swictation/models` on all platforms; the
   generated darwin config and the installed-check use `~/Library/Application
   Support/swictation/models`. Recovery hints print `swictation download-model`, which
   does not exist (`download-models` exists but rejects every recommended model name).
6. **Model claims are inverted.** Code comments state 1.1B = "5.77% WER (best)" and
   0.6B = "7-8%". NVIDIA's cards: 1.1B = 7.02% Open-ASR avg / 1.39% LS test-clean;
   0.6B-v3 = 6.34% avg / 1.93% LS-clean. The 5.77% figure is uncorroborated.

## Decision

1. **Every `DaemonConfig` field gets a FIELD-level `#[serde(default = …)]`** (revised
   during review from the originally drafted container-level attribute: container-level
   `serde(default)` constructs the full `Default` impl on every parse, and that impl is
   fallible and touches the filesystem — a complete custom config must never execute
   it). Every path field is tilde-expanded at load, including the first-run default
   branch. A round-trip test parses the shipped `config.example.toml` verbatim so the
   example can never drift from the parser again. The example's 0.6B path is corrected.
   `RUST_LOG` is honored via an env-filter (required for the debug opt-in in decision 3).
2. **`templates/swictation-daemon.service.template` is the single source of truth** for
   the systemd unit. `bin/swictation`'s inline generator is deleted; `swictation setup`
   renders the same template with the same substitutions postinstall uses.
3. **Transcript content never appears at `info` level.** Info logs carry length and
   latency only; full text moves to `debug!` (opt-in via `RUST_LOG=swictation_daemon=debug`).
4. **preuninstall drops `/opt/swictation`, all `sudo` escalation, and all system-path
   sweeps.** Cleanup gates on `npm_command === 'uninstall'` (stricter than the originally
   drafted `npm_lifecycle_event` gate: the lifecycle event is always `preuninstall` when the
   hook runs, so it cannot distinguish a true uninstall from an upgrade's replace) and
   touches only user-scope state. Distro-package hygiene is not npm's job.
   **Known limitation (honest):** npm ≥7 does not execute uninstall lifecycle scripts at
   all, so on modern npm this hook never fires and uninstalls still leave services
   pointing at deleted binaries. The safety fix here removes the catastrophic path;
   actual cleanup on modern npm requires a `swictation uninstall` command (Phase 2).
   The script remains manually runnable via `node preuninstall.js --force`.
5. **One JS paths module** (`npm-package/src/paths.js`) mirrors the Rust
   `swictation-paths` rules; `ModelDownloader` and all call sites take their model dir
   from it. `download-model` aliases `download-models`; valid model names derive from the
   downloader's `MODELS` table instead of a second hardcoded list.
6. **Model: stay on Parakeet-TDT-1.1B.** Its raw lowercase output is a feature — it makes
   Secretary Mode the sole authority on punctuation, which is the product's design intent
   (auto-punctuation is explicitly unwanted). The 0.6B remains the low-VRAM fallback. WER
   doc comments are corrected to the model-card numbers. Newer families were evaluated
   (Cohere Transcribe 03-2026, Moonshine v2, Nemotron-Speech, Canary-Qwen, Kyutai,
   Voxtral): the only credible future candidate is Cohere Transcribe (Apache-2.0,
   1.25% LS-clean, punctuation *toggleable*) — deferred behind a mandatory single-stream
   M1 latency spike before any port work.

## Open questions (deliberately not decided here)

- **Postinstall split** (minimal postinstall + idempotent `swictation setup` steps).
  `docs/archive/prd-postinstall-resilience.md:29` forbids it; the reason is not remembered and
  may be a Chesterton's fence. Requires a spike before superseding the PRD.
- **Stop-drain protocol and CHUNK_FRAMES padding** (pipeline): approved in principle,
  to be executed with regression tests encoding the original bugs (duplicate injection;
  long-audio truncation) — tracked for Phase 2, not this release.

## Consequences

- A copied example config now loads; missing keys fall back to compiled defaults.
- `swictation setup` repairs installs instead of breaking them.
- Dictated content stays out of journald at default configuration.
- `npm uninstall` can no longer destroy a source checkout; it also no longer pretends
  to do distro-level cleanup.
- macOS first-run downloads land where the daemon looks.
- The 6.96 GB 1.1B artifact remains the quality path by explicit decision, with honest
  WER numbers in the code.
