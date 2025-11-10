# Sherpa-ONNX vs Swictation: Implementation Comparison & Recommendations

**Analysis Date:** 2025-11-09
**Reviewer:** Implementation Reviewer (Hive Mind Swarm)
**Swarm ID:** swarm-1762744363981-g4ic134p6

---

## Executive Summary

After comprehensive analysis of both implementations, **our swictation implementation is remarkably close to sherpa-onnx** and already implements the correct TDT decoding algorithm. However, there are several optimizations and improvements we can adopt from sherpa-onnx's mature codebase.

**Key Finding:** The TDT decoder implementation in swictation (lines 437-572 of `recognizer_ort.rs`) is **correctly implemented** and matches the sherpa-onnx C++ reference exactly.

---

## 1. ONNX Runtime Setup & Configuration

### What Sherpa-ONNX Does:
```cpp
// session.cc lines 38-46
Ort::SessionOptions sess_opts;
sess_opts.SetIntraOpNumThreads(num_threads);
sess_opts.SetInterOpNumThreads(num_threads);

// Optional optimizations:
// sess_opts.SetGraphOptimizationLevel(ORT_ENABLE_EXTENDED);
// sess_opts.SetLogSeverityLevel(ORT_LOGGING_LEVEL_VERBOSE);
// sess_opts.EnableProfiling("profile");

// CUDA setup (lines 155-178)
OrtCUDAProviderOptions options;
options.device_id = provider_config->device;
options.cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic; // NOT Exhaustive!
sess_opts.AppendExecutionProvider_CUDA(options);
```

### What We Do:
```rust
// recognizer_ort.rs lines 88-105
let mut session_builder = Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_intra_threads(4)?;

if use_gpu {
    session_builder = session_builder
        .with_execution_providers([
            ep::CUDAExecutionProvider::default().build(),
            ep::CPUExecutionProvider::default().build(),
        ])?;
}
```

### ✅ Recommendations:

1. **Add Inter-Op Thread Configuration:**
   ```rust
   .with_inter_threads(4)?  // Add this!
   ```
   - Sherpa sets both intra and inter-op threads
   - We only set intra-op threads
   - **Difficulty:** Easy (5 minutes)

2. **Add CUDNN Algorithm Search Configuration:**
   ```rust
   ep::CUDAExecutionProvider::default()
       .with_cudnn_conv_algo_search(CudnnConvAlgoSearch::Heuristic)
       .build()
   ```
   - Sherpa uses `OrtCudnnConvAlgoSearchHeuristic`, NOT `Exhaustive`
   - Exhaustive is extremely slow
   - **Difficulty:** Easy (10 minutes)
   - **Impact:** Significant speedup for first inference

3. **Add Debug Profiling Support (Optional):**
   ```rust
   if debug {
       session_builder = session_builder.with_profiling(true)?;
   }
   ```
   - **Difficulty:** Easy (5 minutes)

---

## 2. Model Initialization & Metadata Reading

### What Sherpa-ONNX Does:
```cpp
// offline-transducer-nemo-model.cc lines 182-217
Ort::ModelMetadata meta_data = encoder_sess_->GetModelMetadata();

SHERPA_ONNX_READ_META_DATA(vocab_size_, "vocab_size");
vocab_size_ += 1;  // NeMo doesn't include blank in vocab_size!

SHERPA_ONNX_READ_META_DATA(subsampling_factor_, "subsampling_factor");
SHERPA_ONNX_READ_META_DATA_STR_ALLOW_EMPTY(normalize_type_, "normalize_type");
SHERPA_ONNX_READ_META_DATA(pred_rnn_layers_, "pred_rnn_layers");
SHERPA_ONNX_READ_META_DATA(pred_hidden_, "pred_hidden");
SHERPA_ONNX_READ_META_DATA_WITH_DEFAULT(feat_dim_, "feat_dim", -1);

// TDT detection from metadata URL!
std::string url;
SHERPA_ONNX_READ_META_DATA_STR_ALLOW_EMPTY(url, "url");
if (url.find("tdt") != std::string::npos) {
    is_tdt_ = 1;
}
```

### What We Do:
```rust
// recognizer_ort.rs lines 169-181
let transpose_input = if model_path.to_string_lossy().contains("1.1b") {
    false
} else if model_path.to_string_lossy().contains("0.6b") {
    true
} else {
    true  // Fallback
};
```

### ⚠️ Recommendations:

1. **Read Metadata from ONNX Model:**
   ```rust
   let metadata = encoder.metadata()?;
   let vocab_size: i64 = metadata.custom("vocab_size")?.parse()?;
   let vocab_size = vocab_size + 1; // Add blank token!

   let normalize_type: String = metadata.custom("normalize_type")
       .unwrap_or("".to_string());
   let feat_dim: i64 = metadata.custom("feat_dim")?.parse()?;

   // TDT detection from URL!
   if let Ok(url) = metadata.custom("url") {
       if url.contains("tdt") {
           is_tdt = true;
       }
   }
   ```
   - **Benefits:**
     - Automatic detection instead of path heuristics
     - More robust across model versions
     - Can detect feature normalization requirements
   - **Difficulty:** Medium (30 minutes)
   - **Priority:** HIGH - This is the proper way to handle model variants

2. **Add Feature Normalization Support:**
   - Sherpa supports "per_feature" normalization
   - Our current mel-spectrogram doesn't normalize
   - **Difficulty:** Medium (1-2 hours)
   - **Priority:** HIGH - May fix inference quality issues

---

## 3. TDT Decoder Implementation

### Side-by-Side Comparison:

| **Aspect** | **Sherpa-ONNX (C++)** | **Swictation (Rust)** | **Match?** |
|------------|----------------------|----------------------|-----------|
| Decoder initialization | `BuildDecoderInput(blank_id)` before loop | `run_decoder(&[initial_token])` before loop | ✅ YES |
| Main loop structure | `for (t = 0; t < num_rows; t += skip)` | `while t < num_frames { ... t += skip.max(1) }` | ✅ YES |
| Joiner call | Once per frame iteration | Once per frame iteration | ✅ YES |
| Token/duration split | `token_logits = p_logit[0..vocab]` | `token_logits = &logits[0..vocab_size]` | ✅ YES |
| Greedy selection | `std::max_element` | `max_by(partial_cmp)` | ✅ YES |
| Emit on non-blank | Update decoder immediately | Update decoder immediately | ✅ YES |
| Skip logic | 3 conditions (lines 167-179) | 3 conditions (lines 532-547) | ✅ YES |
| Cross-chunk state | Not implemented | **IMPLEMENTED!** | ✅ BETTER |

### ✅ Assessment:

**Our TDT decoder is CORRECT!** It matches sherpa-onnx exactly and even has a feature they don't have (cross-chunk state persistence).

**No changes needed to core decoder logic.**

---

## 4. Input Preprocessing

### What Sherpa-ONNX Does:
```cpp
// offline-transducer-nemo-model.cc lines 76-88
std::vector<Ort::Value> RunEncoder(Ort::Value features, Ort::Value features_length) {
    // (B, T, C) -> (B, C, T)
    features = Transpose12(allocator_, &features);

    std::array<Ort::Value, 2> encoder_inputs = {
        std::move(features),
        std::move(features_length)
    };

    auto encoder_out = encoder_sess_->Run(...);
    return encoder_out;
}
```

### What We Do:
```rust
// recognizer_ort.rs lines 379-399
let (shape, audio_data) = if self.transpose_input {
    // TRANSPOSE FOR 0.6B: (batch, features, time)
    let mut data = Vec::with_capacity(...);
    for col_idx in 0..num_features {
        for row in features.outer_iter() {
            data.push(row[col_idx]);
        }
    }
    (vec![batch_size, num_features, num_frames], data)
} else {
    // NO TRANSPOSE FOR 1.1B: (batch, time, features)
    let data: Vec<f32> = features.iter().copied().collect();
    (vec![batch_size, num_frames, num_features], data)
};
```

### ✅ Recommendations:

1. **Use Metadata Instead of Path Detection:**
   - Already covered in section 2
   - This makes our transpose logic more robust

2. **Consider Using ndarray Transpose:**
   ```rust
   use ndarray::s;
   let transposed = features.view().reversed_axes();
   ```
   - **Benefit:** More efficient than manual loop
   - **Difficulty:** Easy (15 minutes)
   - **Priority:** Low - Current implementation works

---

## 5. Output Processing & Token Decoding

### What Sherpa-ONNX Does:
```cpp
// Decoder output extraction (lines 632-663)
const float *p = encoder_out.GetTensorData<float>();
for (int32_t i = 0; i != batch_size; ++i) {
    const float *this_p = p + dim1 * dim2 * i;
    ans[i] = DecodeOneTDT(this_p, this_len, dim2, model_, blank_penalty_);
}
```

### What We Do:
```rust
// recognizer_ort.rs lines 730-754
tokens.iter()
    .filter_map(|&token_id| {
        if token_id != self.blank_id && token_id != self.unk_id {
            Some(self.tokens[idx].as_str())
        } else {
            None
        }
    })
    .collect::<Vec<_>>()
    .join("")
    .replace("▁", " ")  // BPE underscore handling
    .trim()
```

### ✅ Recommendations:

**No changes needed** - Our implementation is clean and correct.

---

## 6. Performance Optimizations

### What Sherpa-ONNX Does Better:

1. **Batch Processing:**
   ```cpp
   for (int32_t i = 0; i != batch_size; ++i) {
       ans[i] = DecodeOneTDT(...);
   }
   ```
   - Supports true batch inference
   - We're currently locked to batch_size=1

2. **Memory Reuse:**
   ```cpp
   Ort::Value cur_encoder_out = Ort::Value::CreateTensor(
       memory_info, const_cast<float *>(p) + t * num_cols, num_cols,
       encoder_shape.data(), encoder_shape.size());
   ```
   - Reuses memory without copying
   - Zero-copy tensor creation

### ⚠️ Recommendations:

1. **Add Batch Processing Support:**
   ```rust
   pub fn recognize_batch(&mut self, audio_paths: &[&Path]) -> Result<Vec<String>> {
       // Process multiple files in parallel
   }
   ```
   - **Difficulty:** Hard (4-6 hours)
   - **Priority:** Medium - Nice for server deployments

2. **Optimize Memory Allocations:**
   - Pre-allocate decoder states
   - Reuse tensors where possible
   - **Difficulty:** Medium (2-3 hours)
   - **Priority:** Low - Optimize after correctness confirmed

---

## 7. Code Organization & Architecture

### Sherpa-ONNX Structure:
```
offline-transducer-nemo-model.cc    # Model wrapper (ONNX sessions)
offline-transducer-greedy-search-nemo-decoder.cc  # Decoding algorithm
session.cc                          # Session configuration utilities
features.cc                         # Feature extraction
```

### Our Structure:
```rust
recognizer_ort.rs                   # Everything in one file (783 lines)
audio.rs                            # Audio processing
error.rs                            # Error types
```

### ✅ Recommendations:

**Current structure is fine for Rust** - Single-file modules are common and acceptable. Consider splitting only if:
- File grows beyond 1500 lines
- Multiple developers editing simultaneously
- **Priority:** Low

---

## 8. Error Handling & Robustness

### What Sherpa-ONNX Does:
```cpp
if (length_type != ONNX_TENSOR_ELEMENT_DATA_TYPE_INT32 &&
    length_type != ONNX_TENSOR_ELEMENT_DATA_TYPE_INT64) {
    SHERPA_ONNX_LOGE("Unsupported encoder_out_length data type: %d", ...);
    SHERPA_ONNX_EXIT(-1);
}
```

### What We Do:
```rust
.map_err(|e| SttError::InferenceError(format!("Encoder inference failed: {}", e)))?
```

### ✅ Recommendations:

**Our error handling is better!** We use proper Result types instead of exit(-1).

---

## 9. Testing & Validation

### What Sherpa-ONNX Has:
- C++ unit tests
- CXX-API examples
- Multiple model format support
- Cross-platform validation

### What We Have:
```rust
#[test]
#[ignore]
fn test_ort_recognizer_init() { ... }
```

### ⚠️ Recommendations:

1. **Add Comprehensive Tests:**
   ```rust
   #[test]
   fn test_tdt_decoder_logic() { ... }

   #[test]
   fn test_cross_chunk_state_persistence() { ... }

   #[test]
   fn test_metadata_reading() { ... }
   ```
   - **Difficulty:** Medium (3-4 hours)
   - **Priority:** HIGH

---

## Priority Matrix

### 🔴 HIGH Priority (Do First):

1. **Read ONNX Metadata for Model Detection** (30 min)
   - Replace path heuristics with metadata reading
   - Detect TDT from metadata URL
   - Read vocab_size, normalize_type, feat_dim

2. **Add Feature Normalization Support** (1-2 hours)
   - Implement "per_feature" normalization
   - May fix quality issues with certain models

3. **Add Comprehensive Tests** (3-4 hours)
   - Test TDT decoder logic
   - Test cross-chunk state
   - Test metadata reading

### 🟡 MEDIUM Priority (After HIGH):

4. **Add CUDNN Algorithm Configuration** (10 min)
   - Use Heuristic instead of Exhaustive
   - Significant speedup for first inference

5. **Add Inter-Op Thread Configuration** (5 min)
   - Complete ONNX Runtime setup

### 🟢 LOW Priority (Nice to Have):

6. **Batch Processing Support** (4-6 hours)
   - Process multiple files efficiently

7. **Memory Optimizations** (2-3 hours)
   - Pre-allocate, reuse tensors

8. **Use ndarray Transpose** (15 min)
   - More idiomatic Rust

---

## Bugs & Issues Found

### ✅ No Critical Bugs Found

Our implementation is **correct** and matches sherpa-onnx. The TDT decoder logic is identical.

### ⚠️ Potential Quality Issues:

1. **Missing Feature Normalization:**
   - Log: "Note: NVIDIA NeMo models typically expect normalized features (mean=0, std=1)"
   - We compute log-mel but don't normalize
   - Sherpa reads `normalize_type` from metadata and applies it
   - **This may explain any quality differences with official models**

---

## Implementation Difficulty Estimates

| **Task** | **Difficulty** | **Time** | **Impact** |
|----------|---------------|----------|-----------|
| Read ONNX metadata | Medium | 30 min | High |
| Feature normalization | Medium | 1-2 hrs | High |
| Add tests | Medium | 3-4 hrs | High |
| CUDNN config | Easy | 10 min | Medium |
| Inter-op threads | Easy | 5 min | Low |
| Batch processing | Hard | 4-6 hrs | Medium |
| Memory optimization | Medium | 2-3 hrs | Low |
| ndarray transpose | Easy | 15 min | Low |

---

## Conclusion

**Our swictation implementation is excellent!** The TDT decoder is correctly implemented and even has cross-chunk state persistence that sherpa-onnx doesn't implement in the offline version.

**Key Strengths:**
- ✅ Correct TDT decoding algorithm
- ✅ Cross-chunk state persistence
- ✅ Better error handling (Result types)
- ✅ Clean, idiomatic Rust code

**Key Improvements to Make:**
1. Read ONNX metadata instead of path heuristics
2. Implement feature normalization (may fix quality issues)
3. Add comprehensive tests
4. Complete ONNX Runtime configuration (inter-op threads, CUDNN)

**Total Estimated Work:** 6-8 hours for all HIGH priority items.

---

**Prepared by:** Implementation Reviewer
**Swarm:** Hive Mind (swarm-1762744363981-g4ic134p6)
**Status:** Analysis Complete ✅
