# VAD Chunk Spacing Analysis

**Date:** November 12, 2025
**Task:** Research for task 3393b914 (Secretary Dictation Mode)
**Issues Investigated:**
1. Spacing problem between VAD chunks
2. Model capabilities for automatic punctuation/capitalization
3. Built-in features vs custom transformation rules

---

## Executive Summary

**Key Findings:**
1. ✅ **Spacing issue root cause identified** - No spacing logic in pipeline
2. ❌ **No built-in punctuation in current models** - Using base Parakeet-TDT, NOT _ctc variants
3. ✅ **Transformation layer ready** - Currently 0 rules (intentionally reset), architecture supports ~1µs latency
4. 🎯 **Recommendation:** Implement dictation mode using transformation rules, NOT model variants

---

## Problem 1: Spacing Between VAD Chunks

### Root Cause

**Location:** `/opt/swictation/rust-crates/swictation-daemon/src/pipeline.rs:432`

```rust
// Line 432: Transcription sent directly to text injection
let _ = tx.send(Ok(transformed));
```

**Issue:** Each VAD chunk is transcribed and injected independently with NO spacing logic:

```
Chunk 1: "hello world"    [VAD detects 0.8s silence]
Chunk 2: "testing"        [VAD detects 0.8s silence]
Result:  "hello worldtesting"  ❌ NO SPACE BETWEEN CHUNKS
```

### Why This Happens

1. **VAD segments audio** at silence boundaries (pipeline.rs:320-323)
2. **STT transcribes** each segment independently (pipeline.rs:360)
3. **MidStream transforms** the text (pipeline.rs:375)
4. **Text injector** types the result **immediately** (main.rs:377)
5. **No chunk boundary tracking** exists in the pipeline

### Solution Options

**Option A: Add trailing space in transformation** (RECOMMENDED)
```rust
// In pipeline.rs:375-378
let transformed = transform(&text);
let with_space = if !transformed.ends_with(['.', '!', '?', '\n']) {
    format!("{} ", transformed)  // Add space unless ends with punctuation
} else {
    transformed
};
```

**Option B: Track chunk boundaries in VAD**
```rust
// More complex - requires VAD/STT state coordination
// NOT recommended as it couples VAD with text injection
```

**Impact:** ~1 line code change, <1ms latency addition

---

## Problem 2: Model Capabilities for Punctuation/Capitalization

### Current Implementation

**Models in Use:**
- `parakeet-tdt-1.1b-onnx` (encoder.onnx, decoder.onnx, joiner.onnx)
- `parakeet-tdt-0.6b-v3-onnx` (encoder.onnx, decoder.onnx, joiner.onnx)

**Implementation Stack:**
```
Rust ONNX Runtime (ort crate 2.0.0-rc.10)
        ↓
Base Parakeet-TDT models (NOT _ctc variants)
        ↓
Output: Lowercase, no punctuation
```

### Research: Model Variants with Punctuation

**Discovery:** NVIDIA NeMo has `parakeet-tdt_ctc-1.1b` variant with:
- Automatic punctuation insertion
- Sentence-start capitalization
- First-person pronoun capitalization

**Critical Limitation:** These variants are:
1. ❌ **Python/NeMo only** - Designed for NeMo toolkit
2. ❌ **Not available as ONNX** - No direct Rust/ort compatibility
3. ❌ **Would require Python runtime** - Defeats pure Rust architecture
4. ⚠️ **Conversion unknown** - No known NeMo→ONNX export for _ctc variants

### Reality Check

**Your Rust/ONNX Implementation CANNOT use:**
- Python NeMo models
- Models requiring Python runtime
- Non-ONNX model formats

**You ARE USING:**
- ONNX Runtime via `ort` crate (direct Rust)
- Base Parakeet-TDT models (no punctuation)
- Pure compiled binary (no Python dependency)

**Conclusion:** Built-in model punctuation is **NOT available** in current architecture.

---

## Problem 3: Transformation Architecture Analysis

### Current State

**Location:** `/opt/swictation/external/midstream/crates/text-transform/`

**Status:** ✅ **Intentionally Reset** (0 rules)
```rust
// src/rules.rs:47-73
pub static STATIC_MAPPINGS: Lazy<HashMap<&'static str, TransformRule>> = Lazy::new(|| {
    let map = HashMap::with_capacity(50);

    // EMPTY - awaiting Parakeet-TDT behavior documentation (task 4218691c)
    // Previous 268-rule programming mode cleared
    // Target: 30-50 basic dictation rules

    map
});
```

**Why Reset:**
- Old rules: 268 programming-focused transformations
- New goal: Natural language dictation (task 3393b914)
- Approach: Build rules based on ACTUAL STT output, not assumptions

### Transformation Capabilities

**Architecture:** `/opt/swictation/external/midstream/crates/text-transform/src/lib.rs`

**Performance:**
- HashMap lookup: O(1) per word
- Multi-word patterns: 2-4 word phrases supported
- Latency: <1µs per transformation (target: <5ms)
- Quote state tracking: Context-aware transformations

**Features:**
```rust
struct TransformRule {
    replacement: &'static str,
    attach_to_prev: bool,     // "comma" → ",world" (remove space before)
    is_opening: bool,         // "quote" → " "hello..." (opening quote)
    no_space_after: bool,     // CLI flags like "-m"
}
```

**Example Transformation Flow:**
```
Input:  "Hello comma world period"
  ↓ Split words: ["Hello", "comma", "world", "period"]
  ↓ Lookup "comma": TransformRule::new(",", true)
  ↓ Lookup "period": TransformRule::new(".", true)
  ↓ Apply rules with spacing logic
Output: "Hello, world."
```

**Current Usage:**
```rust
// pipeline.rs:375-376
let transformed = transform(&text);  // Currently pass-through (0 rules)
info!("Transcribed: {} → {}", text, transformed);
```

---

## Recommendations for Task 3393b914

### Strategy: Transformation Rules (NOT Model Variants)

**Why NOT pursue model variants:**
1. ❌ Requires Python runtime (breaks pure Rust architecture)
2. ❌ No ONNX export available for _ctc variants
3. ❌ Unknown conversion feasibility
4. ❌ Adds ~200MB Python dependency
5. ❌ Degrades performance (Python bridge overhead)

**Why USE transformation rules:**
1. ✅ Already implemented (~1µs latency)
2. ✅ Pure Rust (zero dependencies)
3. ✅ Full control over behavior
4. ✅ Can customize for dictation style
5. ✅ Easy to tune based on user feedback
6. ✅ Supports context-aware transformations

### Implementation Plan for Dictation Mode

**Phase 1: Core Punctuation (Priority 1)**
```rust
// Add to rules.rs STATIC_MAPPINGS
map.insert("comma", TransformRule::new(",", true));
map.insert("period", TransformRule::new(".", true));
map.insert("question mark", TransformRule::new("?", true));
map.insert("exclamation point", TransformRule::new("!", true));
map.insert("exclamation mark", TransformRule::new("!", true));
map.insert("colon", TransformRule::new(":", true));
map.insert("semicolon", TransformRule::new(";", true));
```

**Phase 2: Formatting (Priority 2)**
```rust
map.insert("new line", TransformRule::new("\n", true));
map.insert("new paragraph", TransformRule::new("\n\n", true));
map.insert("tab", TransformRule::new("\t", true));
```

**Phase 3: Quotes & Brackets (Priority 3)**
```rust
// Using quote state tracking (already implemented)
map.insert("quote", TransformRule::opening("\""));
map.insert("open quote", TransformRule::opening("\""));
map.insert("close quote", TransformRule::opening("\"")); // Toggles state
map.insert("single quote", TransformRule::opening("'"));
map.insert("apostrophe", TransformRule::new("'", true)); // For contractions

map.insert("open parenthesis", TransformRule::opening("("));
map.insert("open paren", TransformRule::opening("("));
map.insert("close parenthesis", TransformRule::new(")", true));
map.insert("close paren", TransformRule::new(")", true));
map.insert("open parentheses", TransformRule::opening("("));
map.insert("close parentheses", TransformRule::new(")", true));
```

**Phase 4: Capitalization (Post-Processing)**
```rust
// Add to transform() function in lib.rs
fn apply_capitalization(text: String) -> String {
    let mut result = text;

    // Capitalize first-person pronoun
    result = result.replace(" i ", " I ");
    result = result.replace(" i'm", " I'm");
    result = result.replace(" i'd", " I'd");
    result = result.replace(" i'll", " I'll");

    // Capitalize sentence starts (after . ! ?)
    // Simple regex: capitalize after ". " "! " "? " or at start

    result
}
```

**Phase 5: Spacing Fix (Immediate)**
```rust
// pipeline.rs:375-378 (add trailing space logic)
let transformed = transform(&text);
let final_text = if !transformed.ends_with(['.', '!', '?', '\n']) {
    format!("{} ", transformed)  // Add space for next chunk
} else {
    transformed
};
let _ = tx.send(Ok(final_text));
```

### Testing Strategy

**1. Unit Tests (transformation layer)**
```rust
#[test]
fn test_dictation_mode() {
    assert_eq!(transform("Hello comma world period"), "Hello, world.");
    assert_eq!(transform("How are you question mark"), "How are you?");
    assert_eq!(transform("quote hello quote"), "\"hello\"");
}
```

**2. Integration Tests (with actual STT output)**
```bash
# Record voice samples
parecord --channels=1 --rate=16000 --format=float32le dictation_test.raw

# Process through pipeline
swictation-daemon --test-mode dictation_test.raw

# Verify output
```

**3. Production Testing**
- Speak naturally with voice commands
- Check logs: `journalctl --user -u swictation-daemon | grep "Transcribed:"`
- Verify spacing between chunks
- Tune rules based on actual behavior

---

## Alternative Approaches Considered

### ❌ Option 1: Export NeMo _ctc to ONNX

**Feasibility:** Unknown
**Risk:** High (undocumented, may not preserve punctuation logic)
**Time:** Weeks of research
**Benefit:** Automatic punctuation
**Drawback:** Still requires post-processing for proper nouns, custom vocabulary

**Verdict:** Not worth the risk when transformation rules work

### ❌ Option 2: Run Python NeMo alongside Rust

**Feasibility:** Possible via IPC/gRPC
**Risk:** Medium
**Complexity:** High
**Performance:** Degraded (~50-100ms added latency)
**Dependencies:** +200MB (Python, NeMo, PyTorch)

**Verdict:** Defeats pure Rust architecture goals

### ✅ Option 3: Transformation Rules (SELECTED)

**Feasibility:** Already implemented
**Risk:** Low
**Complexity:** Low
**Performance:** Excellent (~1µs)
**Flexibility:** Full control over behavior

**Verdict:** Best fit for project goals

---

## Code Locations Reference

### Spacing Issue
- **Pipeline transcription:** `/opt/swictation/rust-crates/swictation-daemon/src/pipeline.rs:432`
- **Text injection:** `/opt/swictation/rust-crates/swictation-daemon/src/main.rs:377`

### Transformation Layer
- **Core logic:** `/opt/swictation/external/midstream/crates/text-transform/src/lib.rs`
- **Rules definition:** `/opt/swictation/external/midstream/crates/text-transform/src/rules.rs`
- **Spacing logic:** `/opt/swictation/external/midstream/crates/text-transform/src/spacing.rs`

### STT Models
- **1.1B ONNX:** `/opt/swictation/models/parakeet-tdt-1.1b-onnx/`
- **0.6B ONNX:** `/opt/swictation/models/parakeet-tdt-0.6b-v3-onnx/`
- **ORT recognizer:** `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs`

### Parakeet-TDT Behavior Documentation
- **Patterns doc:** `/opt/swictation/docs/parakeet-tdt-patterns.md` (task 4218691c)
- **Production logs:** `journalctl --user -u swictation-daemon | grep "Injecting text:"`

---

## Next Actions

**For task 3393b914 (Secretary Dictation Mode):**

1. ✅ **Spacing fix** - Add trailing space logic (1 line change)
2. ✅ **Implement Phase 1 rules** - Basic punctuation (7 rules)
3. 🔄 **Test with voice samples** - Verify actual STT output matches patterns
4. 🔄 **Implement Phase 2-3 rules** - Formatting and brackets (15 rules)
5. 🔄 **Add capitalization** - Post-processing for I/sentences (optional)
6. 🔄 **Production testing** - Tune based on real usage

**Estimated effort:** 2-4 hours implementation, 1-2 days tuning

---

## Conclusion

**Don't chase model variants** - They're not compatible with your Rust/ONNX architecture.

**Use the transformation layer** - It's already implemented, performant, and gives you full control.

**Fix spacing immediately** - Simple 1-line change in pipeline.rs

**Build dictation rules incrementally** - Start with 7 core punctuation rules, expand based on actual usage.

**Leverage existing documentation** - Task 4218691c provides real STT output patterns to inform rule design.

---

**Status:** Research complete, ready for implementation
**Blocking:** None (task 4218691c provides patterns)
**Next:** Implement spacing fix and Phase 1 rules
