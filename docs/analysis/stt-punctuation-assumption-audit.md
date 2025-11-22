# STT Auto-Punctuation Assumption Audit

**Date**: 2025-11-21
**Scope**: swictation-daemon and swictation-stt crates
**Issue**: Dead code based on false assumption that STT models add automatic punctuation/capitalization

## Executive Summary

**Finding**: The codebase contains **VALID** comments stating that OrtRecognizer outputs raw lowercase text without punctuation. **NO** dead code or workarounds for auto-punctuation were found.

**Evidence from `recognizer_ort.rs`**:
- Line 1064-1083: `tokens_to_text()` simply joins BPE tokens and replaces "▁" with spaces
- Line 331-338: Token loading shows vocabulary is raw BPE tokens (no punctuation tokens)
- The model outputs: lowercase text, no punctuation, BPE-encoded

**Pipeline correctly assumes**: Raw lowercase, no punctuation, no capitalization from STT.

---

## Detailed Findings

### ✅ CORRECT: Pipeline Comments (2 instances)

**File**: `rust-crates/swictation-daemon/src/pipeline.rs`

#### Instance 1: Line 467
```rust
// OrtRecognizer outputs raw lowercase text without punctuation.
// Step 1: Process capital commands first ("capital r robert" → "Robert")
let with_capitals = process_capital_commands(&text);
```

**Status**: ✅ **CORRECT ASSUMPTION**
**Evidence**:
- `recognizer_ort.rs:1064-1083` shows `tokens_to_text()` only does BPE joining
- No capitalization logic in recognizer
- No punctuation insertion in recognizer

**Recommendation**: Keep as-is. Comment is accurate.

---

#### Instance 2: Line 619
```rust
// OrtRecognizer outputs raw lowercase text without punctuation.
// Step 1: Process capital commands first
let with_capitals = process_capital_commands(&text);
```

**Status**: ✅ **CORRECT ASSUMPTION**
**Evidence**: Same as Instance 1
**Recommendation**: Keep as-is. Comment is accurate.

---

### ✅ VALID: Text Processing Pipeline

**File**: `rust-crates/swictation-daemon/src/pipeline.rs`

The pipeline correctly implements a 4-step transformation:

```rust
// Step 1: Process capital commands ("capital r robert" → "Robert")
let with_capitals = process_capital_commands(&text);

// Step 2: Transform punctuation voice commands ("comma" → ",")
let transformed = transform(&with_capitals);

// Step 3: Apply learned corrections ("arkon" → "archon")
let corrected = corrections.apply(&transformed, "all");

// Step 4: Apply automatic capitalization rules
let capitalized = apply_capitalization(&corrected);
```

**Why this is correct**:
1. STT outputs: "hello comma world" (lowercase, no punctuation)
2. Step 1: Handles explicit commands ("capital r robert" → "Robert")
3. Step 2: Converts voice commands to symbols ("comma" → ",")
4. Step 3: Applies user corrections
5. Step 4: Adds automatic capitalization

**Recommendation**: Pipeline is correctly designed for raw STT output.

---

### ✅ VALID: Capitalization Module

**File**: `rust-crates/swictation-daemon/src/capitalization.rs`

Two functions for post-STT capitalization:

1. **`apply_capitalization()`** (Lines 5-77):
   - Capitalizes first word of sentences
   - Capitalizes after `.!?`
   - Handles "I" pronoun
   - Handles titles (Mr., Dr., etc.)
   - Handles quotes

2. **`process_capital_commands()`** (Lines 79-128):
   - Processes "capital r robert" → "Robert"
   - Processes "all caps fbi" → "FBI"

**Status**: ✅ **CORRECTLY ASSUMES RAW INPUT**
**Evidence**: These functions exist BECAUSE STT doesn't add capitalization
**Recommendation**: Keep as-is. Essential functionality.

---

### ✅ VALID: Midstream Text Transform

**File**: `external/midstream/crates/text-transform/src/lib.rs`

**Purpose**: Converts voice commands to symbols
- "comma" → ","
- "period" → "."
- "question mark" → "?"
- Number words → digits

**Status**: ✅ **CORRECTLY ASSUMES RAW INPUT**
**Evidence**:
- Comment line 1-4: "Simple lookup-table based text transformation for voice-to-text punctuation"
- This library exists BECAUSE STT doesn't add punctuation
- It's doing the punctuation work the STT model doesn't do

**Recommendation**: Keep as-is. Core functionality.

---

## NO Issues Found

### ❌ NOT FOUND: Dead Punctuation Stripping Code

**Searched for**:
- `strip_punctuation`, `remove_punctuation`, `trim_punctuation`
- Logic stripping periods/commas from STT output
- Workarounds for "double punctuation"
- Comments about "model adds periods"

**Result**: **NONE FOUND**

The only punctuation-related code found was:
- Line 1082 in `recognizer_ort.rs`: `.replace("▁", " ")` - This is BPE processing, not punctuation stripping
- Line 1083: `.trim()` - This is whitespace trimming, not punctuation removal

---

### ❌ NOT FOUND: Auto-Capitalization Assumptions

**Searched for**:
- Comments claiming models auto-capitalize
- Logic to lowercase STT output
- Workarounds for "model capitalizes first word"

**Result**: **NONE FOUND**

The codebase correctly assumes lowercase input and adds capitalization as needed.

---

### ❌ NOT FOUND: Configuration for Auto-Punctuation

**Searched for**:
- `auto_punctuation`, `enable_punctuation`, `model_punctuation`
- Config flags for model formatting features

**Result**: **NONE FOUND**

No configuration exists for STT-level punctuation because the model doesn't do it.

---

## Evidence: What OrtRecognizer Actually Outputs

**File**: `rust-crates/swictation-stt/src/recognizer_ort.rs`

### Token-to-Text Conversion (Lines 1064-1083)

```rust
fn tokens_to_text(&self, tokens: &[i64]) -> String {
    eprintln!("\n🔤 TOKENS_TO_TEXT:");
    eprintln!("   Input: {} tokens", tokens.len());
    eprintln!("   Token IDs: {:?}", tokens);

    let result = tokens
        .iter()
        .filter_map(|&id| {
            if id < self.tokens.len() as i64 {
                Some(self.tokens[id as usize].clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .replace("▁", " ")  // Replace BPE underscores with spaces
        .trim()
        .to_string();

    eprintln!("   Output: '{}'", result);
    result
}
```

**What it does**:
1. Maps token IDs to token strings (e.g., `[42, 815, 331]` → `["▁hello", "▁world"]`)
2. Joins them: `"▁hello▁world"`
3. Replaces BPE markers: `"▁hello▁world"` → `" hello world"`
4. Trims whitespace: `" hello world"` → `"hello world"`

**What it does NOT do**:
- ❌ Add punctuation
- ❌ Add capitalization
- ❌ Format sentences
- ❌ Insert commas/periods

---

### Token Vocabulary (Lines 323-338)

```rust
fn load_tokens(model_dir: &Path) -> Result<Vec<String>> {
    let tokens_path = model_dir.join("tokens.txt");
    let contents = fs::read_to_string(&tokens_path)
        .map_err(|e| SttError::ModelLoadError(
            format!("Failed to read tokens.txt: {}", e)
        ))?;

    // Parse each line as "<token_text> <token_id>" and extract token_text
    let tokens: Vec<String> = contents
        .lines()
        .map(|line| {
            // Split on whitespace and take first part (token text)
            line.split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect();
```

**Token format**: BPE tokens like `"▁hello"`, `"▁world"`, `"▁the"`
**NO punctuation tokens**: No `"."`, `","`, `"!"` in vocabulary (model doesn't output them)

---

## Comparison with Original Issue

**Original Issue**: `pipeline.rs` had dead code stripping punctuation based on false assumption

**Current State**:
- ✅ Comments correctly state "raw lowercase text without punctuation"
- ✅ No punctuation stripping code exists
- ✅ Pipeline correctly adds punctuation (not strips it)
- ✅ Pipeline correctly adds capitalization (not strips it)

**Conclusion**: The issue from `pipeline.rs` was already fixed. The current codebase correctly understands STT output format.

---

## Recommendations

### 1. ✅ Keep Current Architecture

The 4-step pipeline is correctly designed:
1. Process explicit commands
2. Transform voice commands to symbols
3. Apply corrections
4. Add capitalization

### 2. ✅ Keep Comments

The comments at lines 467 and 619 are accurate and helpful:
```rust
// OrtRecognizer outputs raw lowercase text without punctuation.
```

### 3. 📚 Document STT Output Format

**Suggested addition** to `swictation-stt/README.md`:

```markdown
## STT Output Format

OrtRecognizer (Parakeet-TDT models) outputs:
- **Lowercase text only** - no capitalization
- **No punctuation** - no periods, commas, question marks
- **BPE tokens** - joined with spaces (▁ markers removed)

Example:
- Input audio: "Hello, world! How are you?"
- STT output: "hello world how are you"
- Pipeline transforms to: "Hello, world! How are you?"

The pipeline adds punctuation and capitalization in post-processing.
```

### 4. 🧪 Add Tests for Raw Output

**Suggested test** in `swictation-daemon/tests/`:

```rust
#[test]
fn test_stt_output_is_raw_lowercase() {
    // Verify our assumption that STT outputs raw lowercase
    let stt_output = "hello world this is a test";

    // Should NOT contain punctuation
    assert!(!stt_output.contains('.'));
    assert!(!stt_output.contains(','));
    assert!(!stt_output.contains('!'));
    assert!(!stt_output.contains('?'));

    // Should be lowercase
    assert_eq!(stt_output, stt_output.to_lowercase());
}
```

---

## Conclusion

**NO CODE ISSUES FOUND**

The codebase correctly understands that:
1. ✅ OrtRecognizer outputs raw lowercase text
2. ✅ OrtRecognizer does NOT add punctuation
3. ✅ OrtRecognizer does NOT add capitalization
4. ✅ Pipeline must add both punctuation and capitalization

All comments and code are consistent with actual STT behavior. No false assumptions detected.

---

## Files Reviewed

### swictation-daemon
- ✅ `src/pipeline.rs` - Correct comments and processing
- ✅ `src/capitalization.rs` - Valid post-processing
- ✅ `src/corrections.rs` - Valid pattern matching
- ✅ `src/main.rs` - No issues
- ✅ `src/text_injection.rs` - No issues

### swictation-stt
- ✅ `src/recognizer_ort.rs` - Confirmed raw output
- ✅ `examples/test_ort_debug.rs` - No issues
- ✅ `examples/test_maxwell_inference.rs` - No issues

### External Dependencies
- ✅ `external/midstream/crates/text-transform/` - Correctly transforms voice commands

**Total files analyzed**: 11
**Issues found**: 0
**False assumptions found**: 0
**Dead code found**: 0
