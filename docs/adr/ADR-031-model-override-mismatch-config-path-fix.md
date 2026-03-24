# ADR-031: Model Override Mismatch & macOS Config Path Fix

- **Status:** Accepted
- **Date:** 2026-03-24
- **Issue:** [#5](https://github.com/anthropics/swictation/issues/5) — `stt_model_override="1.1b-coreml"` crashes with mutex error; config path mismatch
- **Release:** v0.7.33

## Mantra

> DO NOT BE LAZY. No shortcuts. Never make assumptions. Always dive deep.
> Measure 3x, cut once. Chesterton's fence.

## Context

On macOS (Apple Silicon), `swictation v0.7.32` crashes on startup when `stt_model_override = "1.1b-coreml"` is set in config. Setting `stt_model_override = "auto"` works perfectly and loads the exact same 1.1B CoreML model through a different code path.

Investigation revealed two compounding bugs:

1. **Model override string mismatch:** The postinstall script, CLI parser, and dry-run handler all use `"1.1b-coreml"`, but the actual model loading match block in `pipeline.rs` only recognizes `"coreml-native"`. The value `"1.1b-coreml"` falls through to the error arm, causing a failed init. The subsequent mutex crash is a side effect of partial teardown of VAD/ORT/CoreML global state in a Tokio async context.

2. **Config path mismatch (macOS only):** The postinstall script unconditionally writes config to `~/.config/swictation/config.toml` on all platforms. On macOS, the daemon reads from `~/Library/Application Support/swictation/config.toml` (via `dirs::config_dir()`). The daemon never sees the postinstall-written config, silently creates a default config with `stt_model_override = "auto"`, and appears to work — masking Bug #1.

Bug #2 masks Bug #1: on a fresh macOS install, the daemon runs with `"auto"` (from the default config it generated) rather than `"1.1b-coreml"` (from the config postinstall wrote to the wrong path). The crash only manifests when a user manually places the config in the correct Application Support location.

### Evidence from live system

Two config files found on the developer's macOS machine:

| Field | `~/.config/swictation/config.toml` (postinstall wrote) | `~/Library/Application Support/swictation/config.toml` (daemon reads) |
|---|---|---|
| `stt_model_override` | `"1.1b-coreml"` | `"auto"` |
| `stt_coreml_model_path` | **missing** | present |
| `hotkeys.toggle` | `Super+Shift+D` (wrong) | `Ctrl+Shift+D` (correct) |
| `hotkeys.push_to_talk` | `Super+Space` (wrong) | `Ctrl+Space` (correct) |
| `stt_0_6b_model_path` | `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx` (stale) | `parakeet-tdt-0.6b-v3-onnx` (correct) |

## Decision

Fix all 9 items in a single PR/commit targeting v0.7.33.

## Fix Items

### Item 1: Add `"1.1b-coreml"` match arm in pipeline.rs

**Why:** `pipeline.rs` match block at line ~107 only recognizes `"coreml-native"` for the CoreML path, but every other part of the system (CLI, dry-run, postinstall) uses `"1.1b-coreml"`. The mismatch causes the forced override to hit the error arm and crash.

**What:** Add `"1.1b-coreml"` as an alias alongside `"coreml-native"` in the match arm (`"1.1b-coreml" | "coreml-native" =>`). Both strings route to `CoreMLRecognizer::new()`. This preserves backwards compatibility for anyone who manually typed `"coreml-native"` into their config.

**File:** `rust-crates/swictation-daemon/src/pipeline.rs`

### Item 2: Fix model size in CoreML log messages

**Why:** Both the forced and auto CoreML paths log `"Loading Parakeet-TDT-0.6B via native CoreML"` — but the model is 1.1B, not 0.6B. Misleading for debugging.

**What:** Change `"0.6B"` to `"1.1B"` in both log messages (forced path at ~line 152, auto path at ~line 185). Also fix the success messages.

**File:** `rust-crates/swictation-daemon/src/pipeline.rs`

### Item 3: Update stt_model_override doc comment

**Why:** The doc comment in `config.rs` lists `"coreml-native"` as the macOS option. The canonical user-facing name is `"1.1b-coreml"`.

**What:** Update the comment to list `"1.1b-coreml"` as primary, note `"coreml-native"` as accepted alias.

**File:** `rust-crates/swictation-daemon/src/config.rs`

### Item 4: Use swictation_paths for config path in daemon

**Why:** `config.rs` calls `dirs::config_dir()` directly instead of using the project's canonical `swictation_paths::config_dir()`. While the result currently agrees, this bypasses the single source of truth and invites future drift.

**What:** Replace the manual `dirs::config_dir()` logic in `default_config_path()` with `swictation_paths::config_dir()`. Add `swictation_paths` as a dependency if not already present.

**File:** `rust-crates/swictation-daemon/src/config.rs`, `rust-crates/swictation-daemon/Cargo.toml`

### Item 5: Add platform-aware getConfigDir() in postinstall.js

**Why:** All 10 config path references in postinstall.js are hardcoded to `~/.config/swictation/` regardless of platform. On macOS, this is the wrong location — the daemon reads from `~/Library/Application Support/swictation/`.

**What:** Add a `getConfigDir()` helper that returns:
- macOS: `~/Library/Application Support/swictation/`
- Linux: `~/.config/swictation/`

Replace all 10 hardcoded `path.join(os.homedir(), '.config', 'swictation')` occurrences with calls to this helper.

**File:** `npm-package/postinstall.js`

### Item 6: Clean up stale ~/.config/swictation/ on macOS

**Why:** Users who installed v0.7.30–0.7.32 on macOS have a stale config at `~/.config/swictation/` that was never read by the daemon. Leaving it creates confusion.

**What:** During postinstall on macOS, if `~/.config/swictation/` exists, remove it and log a message explaining the migration to the correct Application Support path.

**File:** `npm-package/postinstall.js`

### Item 7: Always emit stt_coreml_model_path on macOS

**Why:** The postinstall-generated config on macOS was missing `stt_coreml_model_path` entirely. If the config were read by the daemon, the CoreML model path would fall back to default — which might work, but is fragile and inconsistent.

**What:** Ensure the config generation logic always includes `stt_coreml_model_path` when running on macOS (darwin).

**File:** `npm-package/postinstall.js`

### Item 8: Fix hotkey defaults in postinstall config

**Why:** Postinstall writes `Super+Shift+D` and `Super+Space` on macOS. The correct macOS hotkeys are `Ctrl+Shift+D` for toggle. `push_to_talk` should not be emitted (removed in v0.7.31, ADR-028).

**What:** Platform-branch the hotkey generation:
- macOS: `toggle = "Ctrl+Shift+D"`, no `push_to_talk`
- Linux: `toggle = "Super+Shift+D"`, no `push_to_talk`

**File:** `npm-package/postinstall.js`

### Item 9: Fix 0.6b model path prefix

**Why:** Postinstall writes the 0.6b model path with a stale `sherpa-onnx-nemo-` prefix (`sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx`). The actual downloaded directory is `parakeet-tdt-0.6b-v3-onnx`.

**What:** Remove the `sherpa-onnx-nemo-` prefix from the 0.6b model path in config generation.

**File:** `npm-package/postinstall.js`

## Files Changed

| File | Items |
|------|-------|
| `rust-crates/swictation-daemon/src/pipeline.rs` | #1, #2 |
| `rust-crates/swictation-daemon/src/config.rs` | #3, #4 |
| `rust-crates/swictation-daemon/Cargo.toml` | #4 (add swictation_paths dep) |
| `npm-package/postinstall.js` | #5, #6, #7, #8, #9 |

## Risks

- **Item 1 (alias):** Low risk. Adding a match arm, not removing one.
- **Item 4 (swictation_paths):** Low risk. The path output should be identical; this is a refactor for correctness.
- **Item 5 (10 path replacements):** Medium risk. Must verify each replacement individually — some paths are for `config.toml`, others for `gpu-info.json` or `gpu-package-info.json`.
- **Item 6 (cleanup):** Low risk. Only removes the old incorrect path on macOS. Logs the action.

## Verification

1. `cargo fmt --all && cargo clippy --all-targets` — no warnings
2. `cargo test --workspace` — all tests pass
3. `cargo audit` — no new advisories
4. Fresh macOS install test: verify config written to `~/Library/Application Support/swictation/config.toml` with correct values
5. Verify `stt_model_override = "1.1b-coreml"` starts daemon successfully (no mutex crash)
6. Verify `stt_model_override = "auto"` still works
7. Verify `stt_model_override = "coreml-native"` still works (backwards compat)
8. Verify `~/.config/swictation/` is cleaned up on macOS
9. Verify Linux paths unchanged (`~/.config/swictation/` on both sides)
