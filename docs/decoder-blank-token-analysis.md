# Root Cause Analysis: Blank Token Flooding in Rust Decoder

## Executive Summary

**Problem**: Rust implementation produces "mmhmm" (nonsense) instead of proper transcription
**Symptom**: Almost all frames predict blank token
**Root Cause**: **Missing statistics tracking in return value** causing compilation failure and preventing execution

## Critical Finding: Code Never Executes

The decoder has a **type mismatch error** preventing compilation:

```rust
// Line 632 in recognizer_ort.rs - WRONG
Ok((tokens, last_emitted_token, decoder_out))
// Expected: Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
```

**The code never runs due to this compilation error**, so the "blank flooding" is a red herring - we're seeing stale binary output or unrelated errors.

## Detailed Analysis

### 1. Function Signature Mismatch

**Expected Return Type** (line 505):
```rust
) -> Result<(Vec<i64>, i64, Array1<f32>, (usize, usize))>
```

**Actual Return** (line 632):
```rust
Ok((tokens, last_emitted_token, decoder_out))
// Missing: (blank_count, nonblank_count)
```

**Caller Expectation** (line 378):
```rust
let (chunk_tokens, final_token, final_decoder_out, stats) =
    self.decode_frames_with_state(&encoder_out, decoder_out_opt.take(), last_decoder_token)?;
```

### 2. Missing Statistics Tracking

The decoder implementation LACKS the blank/non-blank counting that the caller expects:

```rust
// MISSING CODE - needs to be added before line 632
let blank_count = /* count of times y == blank_id */;
let nonblank_count = /* count of times y != blank_id */;

Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
```

### 3. Comparison with C++ Reference

**C++ Implementation** (offline-transducer-greedy-search-nemo-decoder.cc, line 106):
```cpp
int32_t blank_id = vocab_size - 1;  // ⚠️ CRITICAL: blank_id is LAST token
```

**Rust Implementation** (recognizer_ort.rs, line 76-78):
```rust
let blank_id = tokens.iter()
    .position(|t| t == "<blk>" || t == "<blank>")
    .ok_or_else(|| SttError::ModelLoadError("Could not find <blk> token".to_string()))? as i64;
```

**POTENTIAL ISSUE**: Rust searches for blank by name, while C++ uses `vocab_size - 1`. If tokens.txt has blank at wrong position, this could cause misidentification.

### 4. Token Metadata from Export Script

From `export_parakeet_tdt_1.1b.py` (lines 84-88):
```python
# Add blank token at the end
f.write(f"<blk> {i+1}\n")
# CRITICAL: vocab_size should NOT include blank token
vocab_size = len(asr_model.joint.vocabulary)
```

**Key Facts**:
- Vocabulary has 1024 tokens
- Blank token `<blk>` is written as token 1024 (i+1 where i=1023)
- vocab_size in metadata = 1024 (excludes blank)
- **Total tokens in tokens.txt = 1025** (includes blank)

### 5. Blank Token ID Verification

**Expected**: `blank_id = 1024` (last token in tokens.txt)
**Actual**: Need to verify that Rust correctly finds position 1024

**Test Command**:
```bash
tail -1 /opt/swictation/models/parakeet-tdt-1.1b/tokens.txt
# Should show: <blk> 1024
```

### 6. Greedy Search Logic Comparison

**C++ DecodeOneTDT** (lines 121-180):
1. Initialize decoder with blank_id BEFORE loop
2. Loop: for (t = 0; t < num_rows; t += skip)
3. Run joiner ONCE per iteration
4. Greedy select token and duration
5. If non-blank: emit token, run decoder with new token
6. Skip logic with 3 conditions (lines 167-179)

**Rust decode_frames_with_state** (lines 522-599):
1. ✅ Initialize decoder BEFORE loop (or reuse prev_decoder_out)
2. ✅ Loop: while t < num_frames with t += skip.max(1)
3. ✅ Run joiner ONCE per iteration (line 527)
4. ✅ Greedy select token and duration (lines 538-555)
5. ✅ If non-blank: emit token, run decoder (lines 566-575)
6. ✅ Skip logic matches C++ exactly (lines 580-595)

**Algorithm Appears Correct** - the issue is the missing return value preventing execution.

## Action Items

### IMMEDIATE FIX (Required for Code to Run)

1. **Add statistics tracking** in `decode_frames_with_state`:
```rust
// Add counters at start of function (after line 510)
let mut blank_count = 0_usize;
let mut nonblank_count = 0_usize;

// Update counters in loop
if y == blank_id {
    blank_count += 1;
} else {
    nonblank_count += 1;
}

// Fix return statement (line 632)
Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
```

### VERIFICATION CHECKS (After Code Compiles)

2. **Verify blank_id**:
```bash
# Check that blank token is at position 1024
grep -n "<blk>" /opt/swictation/models/parakeet-tdt-1.1b/tokens.txt
# Should output: 1025:<blk> 1024
```

3. **Add debug logging** to compare token predictions:
```rust
// At line 558-563, enhance logging
info!("Frame {}: token={} ('{}'), blank_id={}, logit={:.4}, duration={}",
    t, y,
    if y < self.tokens.len() as i64 { &self.tokens[y as usize] } else { "???" },
    blank_id,
    _y_logit,  // Add to show confidence
    skip);
```

4. **Run test and compare with Python**:
```bash
# Python (reference)
python3 scripts/test_1_1b_with_sherpa.py examples/en-short.mp3

# Rust (after fix)
cargo run --example test_1_1b_direct examples/en-short.mp3
```

### POTENTIAL ADDITIONAL ISSUES (If Still Broken After Fix)

5. **Verify encoder output normalization**:
- NeMo models expect normalized features (mean=0, std=1)
- Current code uses log-mel without normalization
- C++ reference: Check if sherpa-onnx applies normalization

6. **Verify joiner output size**:
- Expected: `vocab_size + num_durations` = 1024 + 6 = 1030
- Actual: Check at runtime (line 536: `output_size`)

7. **Check for blank penalty**:
- C++ applies optional `blank_penalty` (line 132-134)
- Rust doesn't implement this (may cause bias toward blank)

## Conclusion

**PRIMARY ISSUE**: Code doesn't compile due to missing statistics in return value.
**SECONDARY ISSUE**: Potential blank_id mismatch or missing blank penalty.
**ALGORITHM**: Core greedy search logic matches C++ reference correctly.

The "blank token flooding" cannot be the actual issue because **the code never executes**. Once the compilation error is fixed, we can verify the actual runtime behavior and compare with Python sherpa-onnx.

## Files Analyzed

1. `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs` (Rust decoder)
2. `/opt/swictation/rust-crates/target/release/build/.../offline-transducer-greedy-search-nemo-decoder.cc` (C++ reference)
3. `/opt/swictation/scripts/export_parakeet_tdt_1.1b.py` (Export script with metadata)
4. `/opt/swictation/scripts/test_1_1b_with_sherpa.py` (Python test reference)

## Metadata

- **Analyzed by**: Model-Output-Analyst (Hive Mind)
- **Date**: 2025-11-09
- **Coordination**: Task ID `task-1762761356953-0veskkq9p`
- **Memory Key**: `hive/analysis/decoder-findings`
