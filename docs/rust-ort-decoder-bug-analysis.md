# Rust ORT Decoder Bug Analysis

## Problem Summary

The Rust OrtRecognizer produces "mmhmm" blank token spam instead of correct transcriptions, with 90.6% blank predictions.

## Root Cause Identified

**After emitting the first non-blank token, the blank probability explodes:**

```
Frame 5 (after emitting 'm'): blank prob = 93.34%
Frame 6: blank prob = 98.48%
Frame 7: blank prob = 99.19%
Frame 8: blank prob = 99.92%
```

**This indicates the decoder RNN state is NOT being properly managed after token emission.**

## Detailed Analysis

### What SHOULD Happen (According to sherpa-onnx C++)

When a non-blank token is emitted:
1. Run decoder with the NEW token → get NEW decoder_out
2. Use this NEW decoder_out for subsequent joiner calls
3. The decoder state updates internally to reflect the new token history

### What IS Happening in Our Rust Code

Looking at `decode_frames_with_state()`:

```rust
// When non-blank token 'y' is predicted:
if y != blank_id {
    tokens.push(y);
    // ✅ CORRECT: Update decoder with new token
    decoder_out = self.run_decoder(&[y])?;
    last_emitted_token = y;
}
```

This LOOKS correct! We DO update `decoder_out` after emission.

### The Hidden Bug: Decoder State Persistence

The problem is likely in HOW we call `run_decoder`:

```rust
fn run_decoder(&mut self, tokens: &[i64]) -> Result<Array1<f32>> {
    // We pass tokens: &[y] - a SINGLE token
    // But the decoder expects CUMULATIVE history!

    let targets_i32: Vec<i32> = tokens.iter().map(|&t| t as i32).collect();
    let targets = Tensor::from_array((
        vec![batch_size, seq_len],  // seq_len = 1!
        targets_i32.into_boxed_slice(),
    ))
```

**CRITICAL**: We're passing `seq_len=1` every time, meaning we're telling the decoder "this is the FIRST token in the sequence" over and over!

## The Fix

The decoder needs to know the FULL token history, not just the latest token. We need to:

1. **Maintain cumulative token history** within each chunk
2. **Pass the FULL history to decoder** each time
3. **Extract the LAST decoder output** from the result

OR (simpler):

1. Keep passing single tokens BUT
2. Ensure decoder RNN states are correctly propagated
3. The states ALREADY encode the history!

## Hypothesis

The current code SHOULD work because:
- We maintain `decoder_state1` and `decoder_state2` as instance variables
- `run_decoder()` updates these states after each call
- The states encode the full history

So why does it fail? Let me check if the state updates are working...

Looking at `run_decoder()`:

```rust
// Extract decoder output
let decoder_out_3d = ...;
let last_frame = decoder_out_3d.slice(s![0, .., seq - 1]).to_owned();

// Update states
if let Ok((state_shape, state_data)) = outputs[2].try_extract_tensor::<f32>() {
    self.decoder_state1 = Some(Array3::from_shape_vec(...).unwrap());
}
```

**AHA!** The states ARE being updated. So the bug must be elsewhere.

## New Hypothesis: Decoder Input Mismatch

What if the issue is that after emitting a token, we should:
1. NOT immediately run the decoder again
2. WAIT for the next encoder frame
3. THEN run joiner with (new_encoder_frame, previous_decoder_out)

Let me check the C++ reference again...

## Python sherpa-onnx Comparison Needed

We need to run the Python version with the SAME model and compare:
1. Logit distributions
2. Token emissions
3. Decoder state progression

This will tell us if:
- The model export is broken
- Our Rust implementation has a logic error
- There's a data type/precision issue
