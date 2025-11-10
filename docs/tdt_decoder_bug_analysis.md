# TDT Decoder Bug Analysis - sherpa-onnx C++ vs Our Rust Implementation

## Reference Implementation
File: `sherpa-onnx/csrc/offline-transducer-greedy-search-nemo-decoder.cc`
Function: `DecodeOneTDT` (lines 97-183)

## Critical Bugs Found

### BUG #1: WRONG LOOP STRUCTURE ❌ CRITICAL

**Our Code (lines 459-526):**
```rust
loop {
    let decoder_out = self.run_decoder(&decoder_state)?;
    let logits = self.run_joiner(&encoder_frame, &decoder_out)?;

    if y != blank_id {
        // emit token
        decoder_state = vec![y];
        symbols_this_frame += 1;
        // CONTINUES LOOP - runs joiner multiple times per frame!
    } else {
        break;
    }
}
```
- Has INNER LOOP that continues until blank
- Runs joiner MULTIPLE times per frame
- Emits multiple tokens from same encoder frame

**C++ Reference (lines 121-180):**
```cpp
for (int32_t t = 0; t < num_rows; t += skip) {
    Ort::Value logit = model->RunJoiner(...);  // ONCE per frame!

    if (y != blank_id) {
        // emit token, run decoder again
        tokens_this_frame += 1;
    }

    // Skip logic determines next frame
    // NO INNER LOOP!
}
```
- NO inner loop
- Runs joiner ONCE per frame
- Emits AT MOST one token per frame
- Uses tokens_this_frame counter and skip logic to handle multiple emissions

**Impact:** Our code produces wrong token sequences because it's emitting multiple tokens per frame when it shouldn't.

---

### BUG #2: MISSING DECODER INITIALIZATION ❌ CRITICAL

**Our Code (line 447):**
```rust
let mut decoder_state: Vec<i64> = vec![blank_id];
// Main loop starts immediately
while t < num_frames {
    loop {
        let decoder_out = self.run_decoder(&decoder_state)?;  // First call in loop!
        ...
    }
}
```
- Decoder is NOT run before the main loop
- First decoder call happens inside the frame loop

**C++ Reference (lines 108-113):**
```cpp
auto decoder_input_pair = BuildDecoderInput(blank_id, model->Allocator());

std::pair<Ort::Value, std::vector<Ort::Value>> decoder_output_pair =
    model->RunDecoder(std::move(decoder_input_pair.first),
                      std::move(decoder_input_pair.second),
                      model->GetDecoderInitStates(1));  // Run BEFORE loop!

for (int32_t t = 0; t < num_rows; t += skip) {
    // Use decoder_output_pair from above
    Ort::Value logit = model->RunJoiner(..., View(&decoder_output_pair.first));
    ...
}
```
- Runs decoder ONCE with blank_id BEFORE the main loop
- Gets initial decoder output states
- Uses these states for first joiner call

**Impact:** We're using uninitialized decoder output, causing joiner to produce garbage.

---

### BUG #3: DECODER STATE NOT UPDATED CORRECTLY ❌ CRITICAL

**Our Code (line 504):**
```rust
if y != blank_id {
    tokens.push(y);
    decoder_state = vec![y];  // Just change token ID
    symbols_this_frame += 1;
    // Continue loop - next iteration calls run_decoder with new token
}
```
- Sets `decoder_state` to new token ID
- Relies on next loop iteration to run decoder
- Decoder output is stale until next iteration

**C++ Reference (lines 157-162):**
```cpp
if (y != blank_id) {
    ans.tokens.push_back(y);

    decoder_input_pair = BuildDecoderInput(y, model->Allocator());

    decoder_output_pair =
        model->RunDecoder(std::move(decoder_input_pair.first),
                          std::move(decoder_input_pair.second),
                          std::move(decoder_output_pair.second));  // Update immediately!

    tokens_this_frame += 1;
}
```
- Runs decoder IMMEDIATELY after emitting token
- Updates decoder_output_pair with new states
- Next joiner call uses fresh decoder output

**Impact:** Decoder output is always one step behind, causing wrong predictions.

---

### BUG #4: WRONG SKIP LOGIC ❌ IMPORTANT

**Our Code (lines 510-516):**
```rust
if y != blank_id {
    // emit
} else {
    // Blank token - use duration for skip
    if duration > 0 && !tokens.is_empty() {
        skip = duration;
    } else {
        skip = 1;
    }
    break;
}
```
- Only sets skip when blank is encountered
- Condition checks if tokens were emitted
- Break immediately after setting skip

**C++ Reference (lines 147-179):**
```cpp
// Duration is ALWAYS calculated (line 148-150)
skip = static_cast<int32_t>(std::distance(
    duration_logits,
    std::max_element(duration_logits, duration_logits + num_durations)));

if (y != blank_id) {
    // emit token, update decoder
    tokens_this_frame += 1;
}

// Skip logic AFTER emission check:
if (skip > 0) {
    tokens_this_frame = 0;
}

if (tokens_this_frame >= max_tokens_per_frame) {
    tokens_this_frame = 0;
    skip = 1;
}

if (y == blank_id && skip == 0) {
    tokens_this_frame = 0;
    skip = 1;
}
```
- Duration skip is ALWAYS calculated before blank check
- Multiple conditions determine final skip value
- Uses tokens_this_frame counter
- Forces skip=1 if blank with skip=0

**Impact:** Frame skipping is incorrect, causing misalignment with audio timeline.

---

### BUG #5: MISSING tokens_this_frame COUNTER ❌ IMPORTANT

**Our Code:**
- Has `symbols_this_frame` but only uses it for max check
- Resets only when max reached
- No interaction with skip logic

**C++ Reference:**
```cpp
int32_t tokens_this_frame = 0;

// When token emitted:
tokens_this_frame += 1;

// Reset logic:
if (skip > 0) {
    tokens_this_frame = 0;  // Reset when advancing frame
}

if (tokens_this_frame >= max_tokens_per_frame) {
    tokens_this_frame = 0;
    skip = 1;  // Force advance
}

if (y == blank_id && skip == 0) {
    tokens_this_frame = 0;
    skip = 1;  // Force advance
}
```
- Counter tracks tokens emitted from current frame
- Reset when frame advances (skip > 0)
- Reset when max reached
- Reset when blank with no skip

**Impact:** Can't properly track multiple emissions from same frame.

---

## Root Cause Summary

Our implementation has the **fundamentally wrong structure**:

1. ❌ Uses nested loop (outer frame loop + inner emission loop)
2. ❌ Runs joiner multiple times per frame
3. ❌ Doesn't initialize decoder before main loop
4. ❌ Doesn't update decoder immediately after emission
5. ❌ Wrong skip calculation logic

**The correct structure should be:**
1. ✅ Initialize decoder ONCE before loop
2. ✅ Single frame loop with skip-based advancement
3. ✅ Run joiner ONCE per frame iteration
4. ✅ If non-blank: emit + run decoder immediately + increment counter
5. ✅ Calculate skip based on duration, counter, and blank status
6. ✅ Reset counter when advancing frames

## Next Steps

1. Rewrite `decode_frames()` to match C++ structure exactly
2. Add `decoder_output` storage (not just token IDs)
3. Run decoder before main loop
4. Remove inner loop
5. Implement correct skip logic
6. Test with both 0.6B and 1.1B models
