# CRITICAL FIX REQUIRED: Decoder Compilation Error

## 🚨 IMMEDIATE ACTION REQUIRED

The Rust decoder **DOES NOT COMPILE** due to a type mismatch error, which means all "blank token flooding" symptoms are from stale binaries or unrelated code.

## The Problem

**File**: `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs`
**Line**: 632
**Error**: Type mismatch - function returns 3 values but caller expects 4

```rust
// CURRENT CODE (LINE 632) - BROKEN ❌
Ok((tokens, last_emitted_token, decoder_out))

// EXPECTED RETURN TYPE (LINE 505) ✅
) -> Result<(Vec<i64>, i64, Array1<f32>, (usize, usize))>
//                                        ^^^^^^^^^^^^^^
//                                        Missing: (blank_count, nonblank_count)
```

## The Fix

### Step 1: Add Statistics Tracking

Insert after line 510 (after variable declarations):
```rust
let mut blank_count = 0_usize;
let mut nonblank_count = 0_usize;
```

### Step 2: Update Counters in Loop

Insert after line 555 (after skip calculation, before logging):
```rust
// Track statistics
if y == blank_id {
    blank_count += 1;
} else {
    nonblank_count += 1;
}
```

### Step 3: Fix Return Statement

Replace line 632:
```rust
// OLD ❌
Ok((tokens, last_emitted_token, decoder_out))

// NEW ✅
Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
```

## Verification After Fix

### 1. Blank Token ID Check

**Verified Facts**:
- Token file has **1025 lines** (0-indexed: 0 to 1024)
- Blank token `<blk>` is at **position 1024** (last line)
- Rust code searches by name: `position(|t| t == "<blk>")`
- **This will correctly find index 1024** ✅

**Proof**:
```bash
$ grep -n "<blk>" /opt/swictation/models/parakeet-tdt-1.1b/tokens.txt
1025:<blk> 1024
#    ^line number (1-indexed)
#         ^token text
#              ^token ID in file
```

**Rust 0-based index**: Line 1025 → Vec index 1024 ✅

### 2. Compare with C++ Reference

**C++ uses**: `blank_id = vocab_size - 1 = 1024 - 1 = 1023` ❓

**Wait - this is DIFFERENT!**

#### CRITICAL DISCOVERY

**C++ sherpa-onnx** (offline-transducer-greedy-search-nemo-decoder.cc, line 106):
```cpp
int32_t vocab_size = model->VocabSize();  // Returns 1024
int32_t blank_id = vocab_size - 1;        // blank_id = 1023 ❌
```

**Python export script** (export_parakeet_tdt_1.1b.py, lines 84-88):
```python
for i, s in enumerate(asr_model.joint.vocabulary):  # i goes 0-1023
    f.write(f"{s} {i}\n")
f.write(f"<blk> {i+1}\n")  # Writes: <blk> 1024
vocab_size = len(asr_model.joint.vocabulary)  # = 1024
```

**The MISMATCH**:
- Export script writes blank at position **1024**
- C++ expects blank at position **1023** (vocab_size - 1)
- Rust correctly finds blank at position **1024**

**THIS IS THE REAL BUG!** 🎯

## Root Cause Analysis Update

### Primary Issue (Prevents Execution)
❌ Missing statistics in return value → **Compilation error**

### Secondary Issue (Causes Wrong Output)
❌ **Blank token ID mismatch**:
- C++ sherpa-onnx uses `blank_id = vocab_size - 1 = 1023`
- Export writes `<blk>` at position `1024`
- Rust finds `<blk>` at position `1024` (CORRECT per export)
- **But C++ expects it at 1023!**

## Two Possible Solutions

### Option A: Fix Export Script (Recommended)
Change `export_parakeet_tdt_1.1b.py` line 85:
```python
# OLD
f.write(f"<blk> {i+1}\n")  # Writes at position 1024

# NEW - write at vocab_size-1 position
blank_id = len(asr_model.joint.vocabulary) - 1  # = 1023
f.write(f"<blk> {blank_id}\n")
```

Then re-export the model.

### Option B: Fix Rust Decoder (Quick Workaround)
Change `recognizer_ort.rs` line 76-78:
```rust
// Instead of searching by name
let blank_id = tokens.iter()
    .position(|t| t == "<blk>" || t == "<blank>")
    .ok_or_else(|| SttError::ModelLoadError("Could not find <blk> token".to_string()))? as i64;

// Use C++ convention
let blank_id = (tokens.len() - 1) as i64;  // vocab_size - 1
```

**Warning**: This assumes blank is ALWAYS last token.

## Complete Fix Steps

1. **Fix compilation error** (3 steps above)
2. **Choose blank_id strategy** (Option A or B)
3. **Test and compare**:
```bash
# Build
cd /opt/swictation/rust-crates/swictation-stt
cargo build --release --example test_1_1b_direct

# Run Rust
cargo run --release --example test_1_1b_direct /opt/swictation/examples/en-short.mp3

# Compare with Python
cd /opt/swictation
python3 scripts/test_1_1b_with_sherpa.py examples/en-short.mp3
```

## Expected Outcome

After fixing both issues:
1. Code compiles successfully ✅
2. Blank token ID matches C++ reference ✅
3. Greedy search algorithm executes correctly ✅
4. Output matches Python sherpa-onnx reference ✅

## Files to Modify

1. `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs` (3 changes)
2. `/opt/swictation/scripts/export_parakeet_tdt_1.1b.py` (1 change, optional)

## Priority

**CRITICAL** - Code does not execute until compilation error is fixed.

---

**Analysis by**: Model-Output-Analyst (Hive Mind)
**Date**: 2025-11-09
**Task**: decoder-blank-token-analysis
**Memory**: hive/analysis/decoder-findings
