# Implementation Specification: Windowed Chunking for CoreML Recognizer

**File**: `rust-crates/swictation-stt/src/recognizer_coreml.rs`
**Author**: Agent #4 (Architect), CFA Swarm
**Status**: Implementation-Ready

---

## Problem Statement

The CoreML encoder is traced with a fixed 15-second window (240,000 samples at 16 kHz). Audio beyond 15 seconds is silently truncated at lines 252-258. The ORT backend handles this via feature-level chunking, but the CoreML encoder is fused (raw audio in, features out), so we must chunk at the audio sample level.

---

## 1. Constants

Add one new constant alongside the existing `MAX_AUDIO_SAMPLES` (line 249):

```
const MAX_AUDIO_SAMPLES: usize = 240_000;  // existing (15s * 16kHz)
const CHUNK_SAMPLES: usize = 240_000;      // new: chunk size = encoder window
```

These are intentionally equal. Defining `CHUNK_SAMPLES` separately makes it trivial to change the chunk size independently of the model's max window later (e.g., for overlap).

**Lines affected**: Near line 249.

---

## 2. Decoder State Carry-Over Structure

### 2.1 New Struct: `DecoderCarryState`

Define a struct to hold all state that crosses chunk boundaries. Place it inside `mod inner`, before the `CoreMLRecognizer` struct (around line 80).

```
struct DecoderCarryState {
    /// LSTM hidden state h: shape [2, 1, hidden_size], flattened to Vec<f32>
    /// Length: 2 * hidden_size (e.g., 1280 for hidden_size=640)
    state_h: Vec<f32>,

    /// LSTM cell state c: shape [2, 1, hidden_size], flattened to Vec<f32>
    /// Length: 2 * hidden_size
    state_c: Vec<f32>,

    /// Decoder embedding output from the last run_decoder() call.
    /// Shape: [640] (decoder_hidden_size). This is the "decoder_out" vector
    /// that feeds into the joiner on the next frame.
    decoder_out: Vec<f32>,

    /// The last token that was fed to run_decoder().
    /// For the first chunk, this is blank_id. For subsequent chunks,
    /// it is the last emitted non-blank token (or blank_id if the
    /// previous chunk emitted nothing).
    last_token: i64,
}
```

**Types and sizes summary**:

| Field | Type | Length | Shape Origin |
|-------|------|--------|-------------|
| `state_h` | `Vec<f32>` | `2 * hidden_size` (1280) | `[2, 1, 640]` flattened |
| `state_c` | `Vec<f32>` | `2 * hidden_size` (1280) | `[2, 1, 640]` flattened |
| `decoder_out` | `Vec<f32>` | `hidden_size` (640) | `[1, 1, 640]` squeezed |
| `last_token` | `i64` | 1 | scalar |

### 2.2 Constructor

```
impl DecoderCarryState {
    fn initial(hidden_size: usize, blank_id: i64) -> Self {
        Self {
            state_h: vec![0.0f32; 2 * hidden_size],
            state_c: vec![0.0f32; 2 * hidden_size],
            decoder_out: Vec::new(),  // empty signals "needs bootstrap"
            last_token: blank_id,
        }
    }
}
```

An empty `decoder_out` is the sentinel for "first chunk -- must run initial decoder call with blank_id."

---

## 3. Modified `recognize_samples()` -- The Chunk Loop

**Lines affected**: 243-300 (entire function body replaced).

### 3.1 New Signature

The public signature does NOT change:

```
pub fn recognize_samples(&mut self, samples: &[f32]) -> Result<String>
```

### 3.2 Pseudocode

```
fn recognize_samples(&mut self, samples: &[f32]) -> Result<String> {
    // --- STEP 1: Early return for empty audio ---
    if samples.is_empty() {
        return Ok(String::new());
    }

    // --- STEP 2: Compute chunks ---
    // num_chunks = ceil(samples.len() / CHUNK_SAMPLES)
    // At least 1 chunk.
    let num_chunks = (samples.len() + CHUNK_SAMPLES - 1) / CHUNK_SAMPLES;

    // --- STEP 3: Initialize carry state ONCE (outside loop) ---
    let hidden_size = self.config.decoder_hidden_size;
    let mut carry = DecoderCarryState::initial(hidden_size, self.blank_id);
    let mut all_tokens: Vec<i64> = Vec::new();

    // --- STEP 4: Chunk loop ---
    for chunk_idx in 0..num_chunks {
        let start = chunk_idx * CHUNK_SAMPLES;
        let end = (start + CHUNK_SAMPLES).min(samples.len());
        let chunk_samples = &samples[start..end];
        let actual_length = chunk_samples.len();

        // Pad chunk to MAX_AUDIO_SAMPLES (encoder's fixed window)
        let audio: Vec<f32> = if actual_length >= MAX_AUDIO_SAMPLES {
            chunk_samples[..MAX_AUDIO_SAMPLES].to_vec()
        } else {
            let mut padded = vec![0.0f32; MAX_AUDIO_SAMPLES];
            padded[..actual_length].copy_from_slice(chunk_samples);
            padded
        };

        // Run encoder on this chunk
        let (encoder_features, encoder_dim, valid_frames) =
            self.run_encoder(&audio, actual_length)?;

        // Load carry state into self.decoder_state_h / self.decoder_state_c
        self.decoder_state_h = carry.state_h.clone();
        self.decoder_state_c = carry.state_c.clone();

        // Decode this chunk (modified signature -- see Section 4)
        let (chunk_tokens, final_carry) = self.tdt_greedy_decode(
            &encoder_features,
            encoder_dim,
            valid_frames,
            &carry,  // provides decoder_out and last_token
        )?;

        // Accumulate tokens
        all_tokens.extend(chunk_tokens);

        // Carry state forward to next chunk
        carry = final_carry;
    }

    // --- STEP 5: Convert accumulated tokens to text ---
    let text = self.tokens_to_text(&all_tokens);
    Ok(text)
}
```

### 3.3 Key Design Points

1. **Decoder state reset happens exactly once** -- inside `DecoderCarryState::initial()`. It never resets between chunks. This fixes the bug identified by Agent #2 (lines 287-289 currently reset every call).

2. **The existing `run_encoder()` is called unchanged** per chunk. The `actual_length` input correctly tells the encoder how many samples are real vs. padding.

3. **The loop structure is overlap-agnostic** -- see Section 7 for how overlap slots in without touching `tdt_greedy_decode()`.

---

## 4. Modified `tdt_greedy_decode()` Contract

**Lines affected**: 549-693 (signature change + bootstrap logic).

### 4.1 New Signature

```
fn tdt_greedy_decode(
    &mut self,
    encoder_features: &[f32],
    encoder_dim: usize,
    num_frames: usize,
    prior_state: &DecoderCarryState,
) -> Result<(Vec<i64>, DecoderCarryState)>
```

**Changes from current**:
- **Added parameter**: `prior_state: &DecoderCarryState` -- carries decoder embedding and last token from previous chunk.
- **Changed return type**: From `Result<Vec<i64>>` to `Result<(Vec<i64>, DecoderCarryState)>` -- must return final state for the next chunk.

### 4.2 Bootstrap Logic (First Chunk vs. Subsequent)

Replace the current unconditional `self.run_decoder(blank_id)` at line 564 with:

```
// Determine initial decoder_out for this chunk
let mut decoder_out = if prior_state.decoder_out.is_empty() {
    // First chunk: bootstrap from blank_id (current behavior)
    self.run_decoder(self.blank_id)?
} else {
    // Subsequent chunk: reuse decoder_out from end of previous chunk
    prior_state.decoder_out.clone()
};
```

The LSTM states (`self.decoder_state_h`, `self.decoder_state_c`) are already loaded by the caller (see Section 3.2, step before the call). `run_decoder()` reads and writes them via `self`.

### 4.3 Return Value Construction

At the end of the function (after the while loop, around line 692), before returning:

```
let final_carry = DecoderCarryState {
    state_h: self.decoder_state_h.clone(),
    state_c: self.decoder_state_c.clone(),
    decoder_out: decoder_out,  // the last decoder_out used
    last_token: if tokens.is_empty() { prior_state.last_token } else { *tokens.last().unwrap() },
};

Ok((tokens, final_carry))
```

### 4.4 No Other Changes to Decode Logic

The entire while loop (lines 570-675) remains exactly as-is. The greedy search, duration skip logic, joiner calls, and `run_decoder()` calls inside the loop are unchanged. Only the initialization (top) and return (bottom) of the function change.

---

## 5. `run_decoder()` -- No Changes Required

The current `run_decoder()` (lines 432-495) reads from `self.decoder_state_h` / `self.decoder_state_c` and writes updated states back to them. This already works correctly for the carry-over design because:

1. The caller loads carry state into `self.decoder_state_h/c` before calling `tdt_greedy_decode()`.
2. `run_decoder()` updates `self.decoder_state_h/c` after each call.
3. At the end, we read `self.decoder_state_h/c` back into `DecoderCarryState`.

No signature or behavioral changes needed.

---

## 6. `run_encoder()` -- No Changes Required

The current `run_encoder()` (lines 340-414) already:
- Accepts padded audio and an `actual_length` parameter
- Returns valid frame count via the model's `obj` output
- Works correctly for partial chunks (shorter than 15s)

No changes needed.

---

## 7. Edge Cases

### 7.1 Empty Audio (0 samples)

- `recognize_samples()` returns `Ok(String::new())` immediately (new early return at the top).
- No encoder or decoder calls are made.

### 7.2 Audio Shorter Than 15 Seconds (e.g., 5 seconds = 80,000 samples)

- `num_chunks = 1`.
- Single iteration: chunk is padded to 240,000, `actual_length = 80,000`.
- Encoder reports correct `valid_frames` for the real audio.
- `DecoderCarryState::initial()` gives fresh zero states.
- `decoder_out` is empty, so bootstrap from `blank_id` occurs.
- Behavior is identical to the current code, minus the bug of always resetting state (irrelevant for a single chunk).

### 7.3 Audio Exactly 15 Seconds (240,000 samples)

- `num_chunks = ceil(240000 / 240000) = 1`.
- Single chunk, no padding needed. `actual_length = 240000`.
- Identical to current behavior.

### 7.4 Audio 15.001 Seconds (240,001 samples)

- `num_chunks = ceil(240001 / 240000) = 2`.
- **Chunk 0**: samples[0..240000], `actual_length = 240000`. Full chunk.
- **Chunk 1**: samples[240000..240001], `actual_length = 1`. Padded to 240,000 with zeros.
  - The encoder will see `audio_length = 1` and report very few (possibly 0-1) valid frames.
  - If `valid_frames = 0`, the decode loop runs 0 iterations and returns no tokens. This is correct -- 1 sample (62.5 microseconds) carries no speech.
- Decoder state carries over from chunk 0 to chunk 1, but since chunk 1 emits nothing, the final tokens are just those from chunk 0.

### 7.5 Audio 30 Seconds (480,000 samples)

- `num_chunks = 2`.
- **Chunk 0**: samples[0..240000], full 15 seconds.
- **Chunk 1**: samples[240000..480000], full 15 seconds.
- Decoder LSTM state carries from chunk 0 to chunk 1, preserving language model context.
- Tokens from both chunks are concatenated.

### 7.6 Audio 60+ Seconds

- `num_chunks = 4+`. Each chunk processed sequentially with state carry-over.
- Performance: each 15s chunk takes roughly the same wall-clock time. Total time scales linearly.

---

## 8. Future Overlap Support (Option B)

The design deliberately keeps all chunking logic in `recognize_samples()` and keeps `tdt_greedy_decode()` overlap-agnostic. To add overlap later:

### 8.1 What Changes (Only in `recognize_samples()`)

1. Add a constant: `const OVERLAP_SAMPLES: usize = 2 * 16000;` (2 seconds).
2. Compute stride: `const STRIDE_SAMPLES: usize = CHUNK_SAMPLES - OVERLAP_SAMPLES;` (208,000 = 13 seconds).
3. Change the chunk loop to use stride-based iteration:

```
let num_chunks = if samples.len() <= CHUNK_SAMPLES {
    1
} else {
    1 + (samples.len() - CHUNK_SAMPLES + STRIDE_SAMPLES - 1) / STRIDE_SAMPLES
};

for chunk_idx in 0..num_chunks {
    let start = chunk_idx * STRIDE_SAMPLES;
    let end = (start + CHUNK_SAMPLES).min(samples.len());
    let chunk_samples = &samples[start..end];
    // ... rest identical ...
}
```

4. After decoding each chunk (except the first), discard tokens that fall within the overlap region. The overlap region corresponds to the first N encoder frames of the chunk, where N can be computed from the encoder's downsampling ratio (approximately `OVERLAP_SAMPLES / 1280` frames, but use the encoder's `valid_frames` output for the overlap portion to be precise).

### 8.2 What Does NOT Change

- `tdt_greedy_decode()` signature and logic -- unchanged.
- `run_encoder()` -- unchanged.
- `run_decoder()` -- unchanged.
- `DecoderCarryState` -- unchanged.

### 8.3 Why This Works

The decoder state carry-over is still valid with overlap because the decoder LSTM state at the end of chunk N reflects the full linguistic context. When chunk N+1 starts decoding (even from overlapping audio), the LSTM state provides continuity. The overlap only helps the encoder produce better features at chunk boundaries -- the decoder state carry handles the language model continuity.

---

## 9. Lines-to-Change Summary

| Line(s) | Current Code | Change |
|----------|-------------|--------|
| ~80 (new) | -- | Add `DecoderCarryState` struct definition |
| 249 | `const MAX_AUDIO_SAMPLES` | Add `const CHUNK_SAMPLES` alongside |
| 252-258 | Pad/truncate to single window | Replace with chunk loop (Section 3) |
| 260-298 | Single encoder call + state reset + single decode | Replace with chunk loop body |
| 287-289 | `self.decoder_state_h = vec![0.0...]` (reset every call) | **Remove** -- state reset moves to `DecoderCarryState::initial()` |
| 549 | `fn tdt_greedy_decode(...)` | Add `prior_state` param, change return type (Section 4.1) |
| 564 | `let mut decoder_out = self.run_decoder(blank_id)?` | Conditional bootstrap (Section 4.2) |
| 692 | `Ok(tokens)` | Construct and return `(tokens, final_carry)` (Section 4.3) |

### Lines That Do NOT Change

- `run_encoder()` (340-414) -- untouched
- `run_decoder()` (432-495) -- untouched
- `run_joiner()` (509-539) -- untouched
- `tokens_to_text()` (700-719) -- untouched
- TDT greedy while loop body (570-675) -- untouched
- `CoreMLRecognizer::new()` (139-233) -- untouched
- `recognize_file()` (303-311) -- untouched

---

## 10. Removal of Debug Logging

While implementing, the developer should consider removing or gating behind `debug!()` the following temporary debug lines that are unrelated to chunking but clutter the output:

- Lines 277-284 (DEBUG stride extraction and Python reference comparison)
- Lines 482-489 (DEBUG h state logging)
- Lines 604-622 (DEBUG frame 0 logits)

These are diagnostic aids from the initial CoreML bring-up and are not needed in production.

---

## 11. Testing Strategy

### Unit Tests (no model required)

1. **Chunk count calculation**: Verify `num_chunks` for 0, 1, 239999, 240000, 240001, 480000 samples.
2. **DecoderCarryState::initial()**: Verify zero states, empty decoder_out, blank_id token.
3. **Chunk boundary math**: Verify `start`/`end` indices for each chunk.

### Integration Tests (model required)

1. **Regression**: 5-second audio file produces identical output before and after the change.
2. **15-second boundary**: Audio at exactly 14.9s, 15.0s, and 15.1s all produce correct output.
3. **Long audio**: 30-second and 60-second audio files produce complete transcriptions with no truncation.
4. **Quality**: Compare word error rate of chunked 30s audio vs. a known-good transcript.
