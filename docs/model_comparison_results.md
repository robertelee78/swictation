# STT Model Comparison: 0.6B vs 1.1B Secretary Mode Analysis

**Date**: 2025-11-24
**Test**: `/opt/swictation/rust-crates/swictation-stt/examples/compare_models.rs`

## Executive Summary

The regression in Secretary Mode with the 1.1B model is caused by fundamentally different tokenizers and training approaches between the two models.

### Root Cause

- **0.6B Model**: Uses special tokens (`<|pnc|>`, `<|itn|>`) and outputs punctuation symbols directly (`,` and `.`)
- **1.1B Model**: Uses BPE subword tokenization and outputs punctuation as **words** (`comma`, `period`)

## Detailed Findings

### 1. Tokenizer Comparison

#### 0.6B Model Tokenizer
- **Total tokens**: 8,193
- **Type**: Character/word-based with special control tokens
- **Special tokens**:
  - `<|pnc|>` - Punctuation normalization control
  - `<|itn|>` - Inverse text normalization control
  - `<|nospeech|>`, `<|endoftext|>`, etc.
- **Punctuation handling**: Direct symbol tokens (`,`, `.`, `?`, `!`)

Sample tokens:
```
<unk> 0
<|nospeech|> 1
<pad> 2
<|endoftext|> 3
<|startoftranscript|> 4
<|pnc|> 5
<|nopnc|> 6
<|itn|> 8
```

#### 1.1B Model Tokenizer
- **Total tokens**: 1,025
- **Type**: BPE (Byte-Pair Encoding) subword tokenization
- **BPE markers**: Uses `▁` (U+2581) to mark word boundaries
- **Punctuation handling**: Words like "comma", "period", "question mark"

Sample tokens:
```
<unk> 0
▁t 1
▁th 2
▁a 3
▁i 4
▁the 5
re 6
▁w 7
```

**Critical finding**: NO tokens found for "period" or "comma" as words in the tokenizer file, which means the model outputs these as character sequences from BPE subwords.

### 2. Model Architecture Differences

| Feature | 0.6B | 1.1B |
|---------|------|------|
| Vocab size | 8,193 | 1,025 |
| Decoder hidden size | 640 | 640 |
| Mel features | 128 | 80 |
| Transpose input | true | false |
| Tokenization | Character/word + special | BPE subwords |

### 3. Transcription Behavior

#### Test Results (from test audio files)

**English Test Audio (en.wav)**:
```
0.6B: "Hello, world."        ← Punctuation as symbols
1.1B: "Hello, world period."  ← Punctuation as words
```

**Analysis**:
- 0.6B: `period` ✗/word, `.` ✓/symbol
- 1.1B: `period` ✓/word, `.` ✗/symbol

### 4. Secretary Mode Impact

#### Current Behavior
The Secretary Mode text replacement in `swictation-daemon` expects:
- Input: "Hello comma world period"
- Output: "Hello, world."

#### Why 0.6B Works
The 0.6B model was trained to:
1. Recognize punctuation control tokens (`<|pnc|>`)
2. Output punctuation symbols directly
3. No post-processing needed

#### Why 1.1B Fails
The 1.1B model:
1. Has no special punctuation tokens
2. Outputs punctuation as spelled-out words
3. **Never receives "comma" or "period" as input from audio** - these come from Secretary Mode post-processing
4. The model's raw output already contains symbols, but Secretary Mode tries to replace words that don't exist

## Recommended Solutions

### Option 1: Remove Secretary Mode for 1.1B (Recommended)
Since 1.1B already outputs punctuation symbols correctly, Secretary Mode is **unnecessary**:

```rust
// In swictation-daemon's transcription handler
fn apply_secretary_mode(text: &str, model: &ModelType) -> String {
    match model {
        ModelType::Parakeet06B => {
            // 0.6B needs word→symbol conversion
            text.replace(" comma ", ", ")
                .replace(" period ", ". ")
        }
        ModelType::Parakeet11B => {
            // 1.1B already outputs symbols
            text.to_string()  // No conversion needed
        }
    }
}
```

### Option 2: Inverse Processing for 1.1B
If Secretary Mode must work with 1.1B, reverse the logic:

```rust
fn apply_secretary_mode_1_1b(text: &str) -> String {
    // Convert symbols to words (opposite of 0.6B)
    text.replace(",", " comma")
        .replace(".", " period")
        .replace("?", " question mark")
        .replace("!", " exclamation point")
}
```

### Option 3: Post-Training Fine-Tuning
Fine-tune 1.1B to output punctuation as words (expensive, not recommended).

## Test Files

### Existing Tests Found
- `/opt/swictation/rust-crates/swictation-stt/examples/test_0_6b_config.rs` - 0.6B config test
- `/opt/swictation/rust-crates/swictation-stt/examples/test_1_1b_regression.rs` - 1.1B regression test
- `/opt/swictation/rust-crates/swictation-stt/examples/test_1_1b.rs` - Full 1.1B pipeline test
- `/opt/swictation/rust-crates/swictation-stt/examples/compare_models.rs` - **NEW**: Side-by-side comparison

### Test Audio Files
Located in: `/home/robert/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx/test_wavs/`
- `en.wav` (184 KB) - English
- `de.wav` (121 KB) - German
- `es.wav` (235 KB) - Spanish
- `fr.wav` (219 KB) - French

### Model Directories

#### 0.6B Model
Path: `/home/robert/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx/`

Key files:
- `encoder.onnx` (41 MB)
- `encoder.weights` (2.3 GB)
- `decoder.onnx` (47 MB)
- `joiner.onnx` (25 MB)
- `tokens.txt` (94 KB, 8,193 tokens)
- `test_wavs/` (test audio files)

#### 1.1B Model
Path: `/home/robert/.local/share/swictation/models/parakeet-tdt-1.1b-onnx/`

Key files:
- `encoder.onnx` (42 MB)
- `encoder.weights` (4.2 GB)
- `encoder.int8.onnx` (1.1 GB) - Quantized version
- `encoder.int8.weights` (2.1 GB) - Quantized weights
- `decoder.onnx` (29 MB)
- `decoder.int8.onnx` (7 MB)
- `joiner.onnx` (7 MB)
- `joiner.int8.onnx` (2 MB)
- `tokens.txt` (only 1,025 tokens)
- Hundreds of layer weight files (split format)

**Notable**: The 1.1B model has INT8 quantized versions for reduced memory usage.

## Running the Comparison Test

```bash
cd /opt/swictation/rust-crates

# Set ONNX Runtime library path
export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")

# Build and run
cargo build --release --example compare_models
./target/release/examples/compare_models
```

## Conclusion

**The 1.1B model is NOT broken** - it works correctly and outputs punctuation symbols as designed. The issue is that Secretary Mode's text replacement logic was designed for the 0.6B model's behavior (punctuation as words) and is being incorrectly applied to the 1.1B model which already outputs symbols.

**Immediate fix**: Disable Secretary Mode word-to-symbol replacement for the 1.1B model, or detect model type and skip the replacement step.

## Technical Details

### Token Conversion Logic
Current implementation in `recognizer_ort.rs`:
```rust
fn tokens_to_text(&self, tokens: &[i64]) -> String {
    tokens
        .iter()
        .filter_map(|&token_id| {
            let idx = token_id as usize;
            if idx < self.tokens.len() &&
               token_id != self.blank_id &&
               token_id != self.unk_id {
                Some(self.tokens[idx].as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .replace("▁", " ")  // BPE underscore → space
        .trim()
        .to_string()
}
```

This correctly converts BPE subwords to text with proper spacing. The punctuation symbols come naturally from the BPE tokenization.

## References

- Test implementation: `/opt/swictation/rust-crates/swictation-stt/examples/compare_models.rs`
- STT recognizer: `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs`
- Model configs: Lines 39-80 in `recognizer_ort.rs`
