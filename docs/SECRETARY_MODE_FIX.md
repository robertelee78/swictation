# Secretary Mode Fix for 1.1B Model

## Problem Summary

Secretary Mode text replacement is designed to convert spoken punctuation words into symbols:
- "period" → "."
- "comma" → ","
- "question mark" → "?"
- "exclamation point" → "!"

**This works for 0.6B but BREAKS for 1.1B because the models have fundamentally different behavior.**

## Root Cause Analysis

### 0.6B Model Behavior
```
User says: "Hello comma world period"
       ↓
Model outputs: "Hello comma world period"  ← Words
       ↓
Secretary Mode: Replace "comma" → "," and "period" → "."
       ↓
Final result: "Hello, world."  ✓ CORRECT
```

### 1.1B Model Behavior
```
User says: "Hello comma world period"
       ↓
Model outputs: "Hello, world."  ← Already has symbols!
       ↓
Secretary Mode: Try to replace "comma" and "period" (not found)
       ↓
Final result: "Hello, world."  ← Unchanged, but confusing
```

## The Real Issue

**The 1.1B model NEVER outputs "comma" or "period" as words** - it's trained with BPE tokenization and outputs punctuation symbols directly from the acoustic model.

When users dictate punctuation in Secretary Mode:
1. With 0.6B: Audio "comma" → Model outputs "comma" → Secretary Mode converts to ","
2. With 1.1B: Audio "comma" → Model outputs "," directly → Secretary Mode has nothing to do

## Proof from Code

### Secretary Mode Rules
Location: `/opt/swictation/external/midstream/crates/text-transform/src/v3/static_rules.rs`

```rust
// Lines 69-84: Punctuation rules (Secretary mode)
StaticRule {
    from: "period".to_string(),
    to: ".".to_string(),
    mode: Some("Secretary".to_string()),
    word_boundary: true,
    case_sensitive: false,
},
StaticRule {
    from: "comma".to_string(),
    to: ",".to_string(),
    mode: Some("Secretary".to_string()),
    word_boundary: true,
    case_sensitive: false,
},
```

### Model Tokenizers

**0.6B tokens.txt** (8,193 tokens):
```
<unk> 0
<|nospeech|> 1
<|pnc|> 5        ← Punctuation Normalization Control
<|nopnc|> 6
...
, 154            ← Comma as word token
. 187            ← Period as word token
```

**1.1B tokens.txt** (1,025 tokens):
```
<unk> 0
▁t 1             ← BPE subword
▁th 2
▁a 3
...
▁com 234         ← Part of "common", NOT "comma"
▁per 445         ← Part of "person", NOT "period"
```

The 1.1B tokenizer has NO dedicated tokens for "comma" or "period" as words - these would be built from BPE subwords like `▁com + ma` or `▁per + iod`, but the model is trained to output the punctuation symbols instead.

## Test Results

From `/opt/swictation/rust-crates/swictation-stt/examples/compare_models.rs`:

```
File: en.wav
  0.6B: "Ask not what your country can do for you comma ask what you can do for your country period"
  1.1B: "Ask not what your country can do for you, ask what you can do for your country."

  Analysis:
    0.6B: period=-/word, comma=-/word (needs conversion)
    1.1B: period=-/symbol, comma=-/symbol (already correct)
```

## Solution Options

### Option 1: Model Detection (Recommended)
Add model type detection and skip Secretary Mode for 1.1B:

```rust
// In swictation-daemon or text-transform
pub enum ModelType {
    Parakeet06B,
    Parakeet11B,
}

impl TransformMode {
    pub fn apply(&self, text: &str, model: ModelType) -> String {
        match (self, model) {
            (TransformMode::Secretary, ModelType::Parakeet06B) => {
                // Apply word→symbol conversion
                self.apply_rules(text)
            }
            (TransformMode::Secretary, ModelType::Parakeet11B) => {
                // 1.1B already outputs symbols
                text.to_string()
            }
            _ => self.apply_rules(text)
        }
    }
}
```

### Option 2: Intelligent Detection
Detect if text already has punctuation symbols:

```rust
pub fn apply_secretary_mode(text: &str) -> String {
    // If text already has punctuation symbols, don't apply rules
    if text.contains(&['.', ',', '?', '!'][..]) {
        return text.to_string();
    }

    // Otherwise apply word→symbol conversion
    text.replace("period", ".")
        .replace("comma", ",")
        .replace("question mark", "?")
        .replace("exclamation point", "!")
}
```

### Option 3: Configuration Flag
Add a setting to disable Secretary Mode punctuation conversion:

```toml
[secretary_mode]
convert_punctuation = false  # For 1.1B model
```

## Implementation Plan

1. **Detect model type** in swictation-daemon when loading STT
2. **Pass model type** to text-transform crate
3. **Skip punctuation rules** for 1.1B in Secretary Mode
4. **Keep all other rules** (capitalization, spacing, etc.)

## Files to Modify

1. `/opt/swictation/rust-crates/swictation-daemon/src/stt_engine.rs`
   - Detect model type from path or config
   - Store model type in engine state

2. `/opt/swictation/external/midstream/crates/text-transform/src/lib.rs`
   - Add `ModelType` parameter to transform functions
   - Skip punctuation rules for 1.1B

3. `/opt/swictation/external/midstream/crates/text-transform/src/v3/static_rules.rs`
   - Add model type awareness to rule application
   - Conditional rule execution

## Testing

Use the comparison test to verify:

```bash
cd /opt/swictation/rust-crates
export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
./target/release/examples/compare_models
```

Expected output:
- 0.6B: "... comma ... period" → Secretary Mode converts to symbols
- 1.1B: "... , ... ." → Secretary Mode leaves unchanged

## Conclusion

**Secretary Mode is NOT broken** - it was designed for 0.6B's behavior. The 1.1B model doesn't need punctuation conversion because it outputs symbols directly. The fix is to detect the model type and skip punctuation replacement for 1.1B while keeping all other Secretary Mode features.

## Quick Fix

For immediate testing, modify `static_rules.rs` line 69-98:

```rust
// Only load punctuation rules if using 0.6B model
if cfg!(feature = "model_0_6b") {  // Add feature flag
    let punctuation_rules = vec![
        // ... existing rules
    ];
    self.rules.insert("Secretary".to_string(), punctuation_rules);
}
```

Or simply comment out the punctuation rules for now when using 1.1B.
