# Sherpa-ONNX ONNX Runtime Implementation Analysis

**Research Date:** 2025-11-09
**Researcher:** Sherpa-ONNX Researcher Agent
**Swarm ID:** swarm-1762744363981-g4ic134p6
**Codebase:** /var/tmp/sherpa-onnx (64,330 lines of C++ code)

---

## Executive Summary

Sherpa-ONNX is a comprehensive speech recognition framework that demonstrates **production-ready patterns** for ONNX Runtime integration. Key strengths:

- ✅ **Robust multi-provider support** (CPU, CUDA, TensorRT, CoreML, NNAPI, DirectML, XNNPACK)
- ✅ **Clean session management** with centralized configuration
- ✅ **Memory-efficient tensor operations** with proper lifetime management
- ✅ **Cross-platform compatibility** (Windows, macOS, Linux, Android, iOS, WASM)
- ✅ **Extensive model support** (Transducer, CTC, Whisper, Paraformer, NeMo models)
- ✅ **Helper utilities** for common ONNX operations

---

## 1. Core Architecture

### 1.1 Session Management (`session.h/cc`)

**Key Pattern: Centralized Session Options Factory**

```cpp
// Central session options builder with provider selection
Ort::SessionOptions GetSessionOptionsImpl(
    int32_t num_threads,
    const std::string &provider_str,
    const ProviderConfig *provider_config = nullptr);
```

**Critical Design Decisions:**
- Single source of truth for session configuration
- Thread count configuration (intra-op and inter-op)
- Provider fallback chain (e.g., TensorRT → CUDA → CPU)
- Compile-time feature detection with `#if` guards

### 1.2 Execution Provider Architecture

**Supported Providers:**
```cpp
enum class Provider {
    kCPU,          // Default, always available
    kCUDA,         // NVIDIA GPUs with CUDA
    kTRT,          // NVIDIA TensorRT optimization
    kCoreML,       // Apple Silicon/iOS
    kXnnpack,      // ARM NEON optimization
    kNNAPI,        // Android Neural Networks API
    kDirectML      // DirectX ML (Windows)
};
```

**Provider Configuration Pattern:**
```cpp
// Comprehensive TensorRT configuration
struct TensorrtConfig {
    int64_t trt_max_workspace_size = 2147483647;
    int32_t trt_max_partition_iterations = 10;
    int32_t trt_min_subgraph_size = 5;
    bool trt_fp16_enable = true;
    bool trt_engine_cache_enable = true;
    std::string trt_engine_cache_path = ".";
    // ... 10+ configuration options
};

// CUDA-specific configuration
struct CudaConfig {
    int32_t cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic;
};
```

**Provider Selection Logic:**
```cpp
switch (p) {
    case Provider::kCUDA: {
        OrtCUDAProviderOptions options;
        options.device_id = provider_config->device;
        options.cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic;
        sess_opts.AppendExecutionProvider_CUDA(options);
        break;
    }
    case Provider::kTRT: {
        // TensorRT with detailed configuration
        OrtTensorRTProviderOptionsV2 *tensorrt_options = nullptr;
        api.CreateTensorRTProviderOptions(&tensorrt_options);
        api.UpdateTensorRTProviderOptions(tensorrt_options, keys, values, count);
        sess_opts.AppendExecutionProvider_TensorRT_V2(*tensorrt_options);
        // Intentional fall-through to CUDA if TRT fails
    }
    // ... other providers
}
```

---

## 2. Model Implementation Patterns

### 2.1 Transducer Model Architecture

**Three-Session Pattern for RNN-T:**
```cpp
class OfflineTransducerModel::Impl {
private:
    Ort::Env env_;
    Ort::SessionOptions sess_opts_;

    // Three separate ONNX models
    std::unique_ptr<Ort::Session> encoder_sess_;
    std::unique_ptr<Ort::Session> decoder_sess_;
    std::unique_ptr<Ort::Session> joiner_sess_;

    // Input/output name management
    std::vector<std::string> encoder_input_names_;
    std::vector<const char *> encoder_input_names_ptr_;
    std::vector<std::string> encoder_output_names_;
    std::vector<const char *> encoder_output_names_ptr_;
    // ... similar for decoder and joiner
};
```

**Initialization Pattern:**
```cpp
void InitEncoder(void *model_data, size_t model_data_length) {
    // 1. Create session from in-memory buffer
    encoder_sess_ = std::make_unique<Ort::Session>(
        env_, model_data, model_data_length, sess_opts_);

    // 2. Extract input/output names
    GetInputNames(encoder_sess_.get(), &encoder_input_names_,
                  &encoder_input_names_ptr_);
    GetOutputNames(encoder_sess_.get(), &encoder_output_names_,
                   &encoder_output_names_ptr_);

    // 3. Read model metadata
    Ort::ModelMetadata meta_data = encoder_sess_->GetModelMetadata();
    if (config_.debug) {
        PrintModelMetadata(os, meta_data);
    }
}
```

**Inference Execution:**
```cpp
std::pair<Ort::Value, Ort::Value> RunEncoder(
    Ort::Value features, Ort::Value features_length) {

    std::array<Ort::Value, 2> encoder_inputs = {
        std::move(features), std::move(features_length)
    };

    auto encoder_out = encoder_sess_->Run(
        {},                                    // Run options (empty)
        encoder_input_names_ptr_.data(),       // Input names
        encoder_inputs.data(),                 // Input tensors
        encoder_inputs.size(),                 // Input count
        encoder_output_names_ptr_.data(),      // Output names
        encoder_output_names_ptr_.size()       // Output count
    );

    return {std::move(encoder_out[0]), std::move(encoder_out[1])};
}
```

### 2.2 CTC Model Architecture

**Simpler Single-Session Pattern:**
```cpp
class OfflineZipformerCtcModel::Impl {
private:
    Ort::Env env_;
    Ort::SessionOptions sess_opts_;
    std::unique_ptr<Ort::Session> sess_;  // Single session

    std::vector<std::string> input_names_;
    std::vector<const char *> input_names_ptr_;
    std::vector<std::string> output_names_;
    std::vector<const char *> output_names_ptr_;
};
```

---

## 3. ONNX Runtime API Usage Patterns

### 3.1 Input/Output Name Management

**Critical Helper Functions:**
```cpp
// Get all input names from session
void GetInputNames(Ort::Session *sess,
                   std::vector<std::string> *input_names,
                   std::vector<const char *> *input_names_ptr) {
    Ort::AllocatorWithDefaultOptions allocator;
    size_t node_count = sess->GetInputCount();

    input_names->resize(node_count);
    input_names_ptr->resize(node_count);

    for (size_t i = 0; i != node_count; ++i) {
        #if ORT_API_VERSION >= 12
            auto v = sess->GetInputNameAllocated(i, allocator);
            (*input_names)[i] = v.get();
        #else
            auto v = sess->GetInputName(i, allocator);
            (*input_names)[i] = v;
            allocator.Free(allocator, v);
        #endif
        (*input_names_ptr)[i] = (*input_names)[i].c_str();
    }
}
```

**Why Two Vectors?**
- `input_names_`: Owns the string storage
- `input_names_ptr_`: Provides raw C-string pointers for ONNX Runtime API
- Ensures memory safety and prevents dangling pointers

### 3.2 Tensor Creation Patterns

**Pattern 1: Tensor from CPU Memory**
```cpp
auto memory_info = Ort::MemoryInfo::CreateCpu(
    OrtDeviceAllocator, OrtMemTypeDefault);

std::array<int64_t, 2> shape = {num_frames, feat_dim};

Ort::Value tensor = Ort::Value::CreateTensor(
    memory_info,
    features_vec[i].data(),      // Data pointer
    features_vec[i].size(),      // Data size in elements
    shape.data(),                // Shape array
    shape.size()                 // Shape rank
);
```

**Pattern 2: Tensor with Allocator**
```cpp
Ort::Value decoder_input = Ort::Value::CreateTensor<int64_t>(
    Allocator(),                 // OrtAllocator*
    shape.data(),                // Shape
    shape.size()                 // Rank
);

// Get mutable pointer and fill data
int64_t *p = decoder_input.GetTensorMutableData<int64_t>();
std::copy(begin, end, p);
```

### 3.3 Metadata Reading Pattern

**Custom Metadata Macro:**
```cpp
#define SHERPA_ONNX_READ_META_DATA(dst, src_key) \
    do { \
        auto src_value = LookupCustomModelMetaData( \
            meta_data, #src_key, allocator); \
        if (src_value.empty()) { \
            SHERPA_ONNX_LOGE("Missing %s in metadata", #src_key); \
            exit(-1); \
        } \
        std::istringstream is(src_value); \
        is >> dst; \
        SHERPA_ONNX_LOGE("%s: %s", #src_key, src_value.c_str()); \
    } while (0)

// Usage:
Ort::ModelMetadata meta_data = decoder_sess_->GetModelMetadata();
Ort::AllocatorWithDefaultOptions allocator;
SHERPA_ONNX_READ_META_DATA(vocab_size_, "vocab_size");
SHERPA_ONNX_READ_META_DATA(context_size_, "context_size");
```

---

## 4. Audio Preprocessing Pipeline

### 4.1 Feature Extraction Configuration

```cpp
struct FeatureExtractorConfig {
    int32_t sampling_rate = 16000;       // Target sample rate
    int32_t feature_dim = 80;            // Mel bins
    float low_freq = 20.0f;              // Mel filterbank low cutoff
    float high_freq = -400.0f;           // Relative to Nyquist
    float dither = 0.0f;                 // Dithering for hard zeros
    bool normalize_samples = true;        // [-1, 1] normalization
    float frame_shift_ms = 10.0f;        // 10ms frame shift
    float frame_length_ms = 25.0f;       // 25ms frame length
    float preemph_coeff = 0.97f;         // Pre-emphasis
    std::string window_type = "povey";   // Window function
    bool remove_dc_offset = true;        // Remove DC component

    // NeMo-specific normalization
    std::string nemo_normalize_type;     // "per_feature" or other
};
```

**Feature Extraction API:**
```cpp
class FeatureExtractor {
public:
    void AcceptWaveform(int32_t sampling_rate,
                       const float *waveform,
                       int32_t n) const;

    void InputFinished() const;

    int32_t NumFramesReady() const;

    // Get flattened 2-D tensor (n_frames, feature_dim)
    std::vector<float> GetFrames(int32_t frame_index, int32_t n) const;

    int32_t FeatureDim() const;
};
```

---

## 5. Utility Functions (`onnx-utils.h/cc`)

### 5.1 Tensor Operations

```cpp
// Deep copy tensor
Ort::Value Clone(OrtAllocator *allocator, const Ort::Value *v);

// Shallow copy (creates view)
Ort::Value View(Ort::Value *v);

// Fill tensor with value
template <typename T = float>
void Fill(Ort::Value *tensor, T value) {
    auto n = tensor->GetTypeInfo()
        .GetTensorTypeAndShapeInfo().GetElementCount();
    auto p = tensor->GetTensorMutableData<T>();
    std::fill(p, p + n, value);
}

// Extract single frame from encoder output
Ort::Value GetEncoderOutFrame(OrtAllocator *allocator,
                              Ort::Value *encoder_out,
                              int32_t frame_index);

// Repeat tensor for beam search
Ort::Value Repeat(OrtAllocator *allocator,
                 Ort::Value *cur_encoder_out,
                 const std::vector<int32_t> &hyps_num_split);
```

### 5.2 Debugging Utilities

```cpp
// Print tensor shape
void PrintShape(const Ort::Value *v);

// Print 1-D, 2-D, 3-D, 4-D tensors
template <typename T = float>
void Print1D(const Ort::Value *v);
void Print2D(const Ort::Value *v);
void Print3D(const Ort::Value *v);
void Print4D(const Ort::Value *v);

// Print all model metadata
void PrintModelMetadata(std::ostream &os,
                       const Ort::ModelMetadata &meta_data);

// Compute statistics
float ComputeSum(const Ort::Value *v, int32_t n = -1);
float ComputeMean(const Ort::Value *v, int32_t n = -1);
```

---

## 6. Key Learnings for Our Implementation

### 6.1 Best Practices Observed

✅ **1. Session Options Centralization**
- Create a single `GetSessionOptions()` function
- Encapsulate all provider logic in one place
- Support compile-time feature detection

✅ **2. Input/Output Name Management**
- Store both `std::string` and `const char*` versions
- Extract names once during initialization
- Use helper functions to avoid repetition

✅ **3. Memory Management**
- Use `std::unique_ptr` for ONNX sessions
- Prefer `std::move` for `Ort::Value` transfers
- Create tensors with proper memory info

✅ **4. Model Metadata Usage**
- Store configuration in ONNX model metadata
- Read vocab size, context size, etc. from model
- Validate required metadata at load time

✅ **5. Multi-Provider Support**
- Implement graceful fallback chains
- Detect available providers at runtime
- Log provider selection clearly

✅ **6. Error Handling**
- Check for provider availability before use
- Provide clear error messages with context
- Exit gracefully on critical failures

### 6.2 Patterns to Adopt

**Session Initialization:**
```cpp
// Our target pattern
Ort::Env env(ORT_LOGGING_LEVEL_WARNING);
Ort::SessionOptions sess_opts = GetSessionOptions(config);

// Load from memory buffer (better than file path)
auto buffer = ReadFile(model_path);
auto session = std::make_unique<Ort::Session>(
    env, buffer.data(), buffer.size(), sess_opts);
```

**Inference Execution:**
```cpp
// Prepare inputs
std::array<Ort::Value, 2> inputs = {
    std::move(audio_features),
    std::move(feature_lengths)
};

// Run inference
auto outputs = session->Run(
    {},                          // Run options
    input_names_ptr.data(),      // Input names
    inputs.data(),               // Input tensors
    inputs.size(),               // Input count
    output_names_ptr.data(),     // Output names
    output_names_ptr.size()      // Output count
);

// Access outputs
float* logits = outputs[0].GetTensorData<float>();
```

### 6.3 Architecture Recommendations

**For Parakeet-TDT (CTC-based model):**

1. **Single-Session Architecture** (like ZipformerCtcModel)
   - One ONNX session for the entire model
   - Simpler than multi-session transducer approach

2. **Session Management**
   ```cpp
   class ParakeetTDTModel {
   private:
       Ort::Env env_;
       Ort::SessionOptions sess_opts_;
       std::unique_ptr<Ort::Session> session_;

       std::vector<std::string> input_names_;
       std::vector<const char*> input_names_ptr_;
       std::vector<std::string> output_names_;
       std::vector<const char*> output_names_ptr_;

       int32_t vocab_size_;
       int32_t subsampling_factor_;
   };
   ```

3. **Provider Selection**
   - Start with CPU for testing
   - Add CUDA support for production
   - Consider XNNPACK for ARM devices

4. **Input Processing**
   - Detect input format automatically (80 vs 240 features)
   - Apply proper transpose if needed
   - Use helper functions for tensor creation

---

## 7. Code Organization

```
sherpa-onnx/csrc/
├── session.{h,cc}              # Session options factory
├── onnx-utils.{h,cc}           # Tensor utilities
├── provider.{h,cc}             # Provider enumeration
├── provider-config.{h,cc}      # Provider configuration
├── features.{h,cc}             # Audio preprocessing
├── offline-ctc-model.{h,cc}    # CTC model base
├── offline-zipformer-ctc-model.{h,cc}  # CTC implementation
├── offline-transducer-model.{h,cc}     # RNN-T implementation
└── macros.h                    # Metadata reading macros
```

**Total LOC:** 64,330 lines across ~400 files

---

## 8. Comparison: Sherpa-ONNX vs Our Current Implementation

| Aspect | Sherpa-ONNX | Our Implementation | Action Needed |
|--------|-------------|-------------------|---------------|
| **Session Management** | Centralized factory function | Direct construction | ✅ Adopt factory pattern |
| **Provider Support** | 7 providers with configs | CPU only | ⚠️ Add CUDA support |
| **Input Names** | Helper function extraction | Manual specification | ✅ Use GetInputNames() |
| **Output Names** | Helper function extraction | Manual specification | ✅ Use GetOutputNames() |
| **Metadata Reading** | Macros + utilities | Not used | ✅ Read vocab_size from model |
| **Memory Management** | unique_ptr everywhere | Raw pointers | ✅ Use smart pointers |
| **Error Handling** | Extensive logging | Basic | ⚠️ Improve error messages |
| **Tensor Utilities** | 20+ helper functions | Basic | ⚠️ Add Clone(), View(), etc. |

---

## 9. Immediate Action Items

### High Priority
1. ✅ **Adopt session factory pattern** from `session.cc`
2. ✅ **Implement GetInputNames/GetOutputNames** helpers
3. ✅ **Add proper input/output name management** (dual vector approach)
4. ✅ **Read model metadata** for vocab_size and other params

### Medium Priority
5. ⚠️ **Add CUDA provider support** with proper fallback
6. ⚠️ **Implement tensor utility functions** (Clone, View, Fill)
7. ⚠️ **Improve error messages** with context

### Low Priority
8. 📋 **Add debug utilities** (PrintShape, Print2D, etc.)
9. 📋 **Consider multi-provider support** for deployment flexibility
10. 📋 **Extract model type from metadata** for auto-detection

---

## 10. Critical Code Snippets for Reference

### Complete Initialization Sequence
```cpp
// 1. Create environment
Ort::Env env(ORT_LOGGING_LEVEL_WARNING);

// 2. Configure session options with provider
Ort::SessionOptions sess_opts;
sess_opts.SetIntraOpNumThreads(num_threads);
sess_opts.SetInterOpNumThreads(num_threads);

// Add provider (CPU, CUDA, etc.)
if (use_cuda) {
    OrtCUDAProviderOptions cuda_opts;
    cuda_opts.device_id = 0;
    cuda_opts.cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic;
    sess_opts.AppendExecutionProvider_CUDA(cuda_opts);
}

// 3. Load model from memory
auto buffer = ReadFile(model_path);
auto session = std::make_unique<Ort::Session>(
    env, buffer.data(), buffer.size(), sess_opts);

// 4. Extract I/O names
std::vector<std::string> input_names;
std::vector<const char*> input_names_ptr;
GetInputNames(session.get(), &input_names, &input_names_ptr);

std::vector<std::string> output_names;
std::vector<const char*> output_names_ptr;
GetOutputNames(session.get(), &output_names, &output_names_ptr);

// 5. Read metadata
Ort::ModelMetadata meta_data = session->GetModelMetadata();
Ort::AllocatorWithDefaultOptions allocator;
auto vocab_size_str = LookupCustomModelMetaData(
    meta_data, "vocab_size", allocator);
int32_t vocab_size = std::stoi(vocab_size_str);
```

### Complete Inference Sequence
```cpp
// 1. Create input tensor
auto memory_info = Ort::MemoryInfo::CreateCpu(
    OrtDeviceAllocator, OrtMemTypeDefault);

std::array<int64_t, 3> shape = {batch_size, num_frames, feat_dim};
Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
    memory_info, audio_features.data(), audio_features.size(),
    shape.data(), shape.size());

// 2. Run inference
std::array<Ort::Value, 1> inputs = {std::move(input_tensor)};
auto outputs = session->Run(
    {},                          // Run options
    input_names_ptr.data(),      // Input names
    inputs.data(),               // Input tensors
    1,                           // Input count
    output_names_ptr.data(),     // Output names
    output_names_ptr.size()      // Output count
);

// 3. Extract results
auto output_shape = outputs[0].GetTensorTypeAndShapeInfo().GetShape();
const float* logits = outputs[0].GetTensorData<float>();
// Process logits...
```

---

## Conclusion

Sherpa-ONNX demonstrates **production-grade ONNX Runtime integration** with:
- Clean separation of concerns (session, model, features)
- Robust multi-provider support with graceful fallbacks
- Extensive utility functions for common operations
- Strong error handling and debugging capabilities
- Cross-platform compatibility

**Key Takeaway:** We should adopt their session management pattern, input/output name extraction helpers, and tensor utility functions to improve our Parakeet-TDT implementation's robustness and maintainability.

---

## References

- **Codebase:** https://github.com/k2-fsa/sherpa-onnx
- **Location Analyzed:** /var/tmp/sherpa-onnx
- **Primary Files:**
  - `sherpa-onnx/csrc/session.{h,cc}`
  - `sherpa-onnx/csrc/onnx-utils.{h,cc}`
  - `sherpa-onnx/csrc/offline-transducer-model.cc`
  - `sherpa-onnx/csrc/offline-ctc-model.cc`
  - `sherpa-onnx/csrc/provider-config.h`
