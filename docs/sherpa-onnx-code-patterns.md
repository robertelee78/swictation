# Sherpa-ONNX Code Patterns to Adopt

Quick reference guide for implementing ONNX Runtime patterns based on Sherpa-ONNX analysis.

---

## 1. Session Factory Pattern

```cpp
// session.h
namespace sherpa_onnx {

Ort::SessionOptions GetSessionOptionsImpl(
    int32_t num_threads,
    const std::string &provider_str,
    const ProviderConfig *provider_config = nullptr);

template <typename T>
Ort::SessionOptions GetSessionOptions(const T &config) {
    return GetSessionOptionsImpl(config.num_threads, config.provider);
}

}  // namespace sherpa_onnx
```

**Usage:**
```cpp
Ort::Env env(ORT_LOGGING_LEVEL_WARNING);
Ort::SessionOptions sess_opts = GetSessionOptions(config);
auto session = std::make_unique<Ort::Session>(
    env, model_data, model_size, sess_opts);
```

---

## 2. Input/Output Name Extraction

```cpp
// onnx-utils.h
void GetInputNames(Ort::Session *sess,
                   std::vector<std::string> *input_names,
                   std::vector<const char *> *input_names_ptr);

void GetOutputNames(Ort::Session *sess,
                    std::vector<std::string> *output_names,
                    std::vector<const char *> *output_names_ptr);
```

**Implementation:**
```cpp
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

**Usage:**
```cpp
std::vector<std::string> input_names;
std::vector<const char*> input_names_ptr;
GetInputNames(session.get(), &input_names, &input_names_ptr);

// Use in Run()
outputs = session->Run({}, input_names_ptr.data(), ...);
```

---

## 3. Model Class Template

```cpp
class ParakeetTDTModel {
public:
    explicit ParakeetTDTModel(const ModelConfig &config);
    ~ParakeetTDTModel();

    std::vector<Ort::Value> Forward(Ort::Value features,
                                     Ort::Value features_length);

    int32_t VocabSize() const;
    OrtAllocator* Allocator() const;

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

// Implementation (PIMPL pattern)
class ParakeetTDTModel::Impl {
public:
    explicit Impl(const ModelConfig &config)
        : config_(config),
          env_(ORT_LOGGING_LEVEL_WARNING),
          sess_opts_(GetSessionOptions(config)),
          allocator_{} {

        auto buf = ReadFile(config.model_path);
        InitModel(buf.data(), buf.size());
    }

    std::vector<Ort::Value> Forward(Ort::Value features,
                                     Ort::Value features_length) {
        std::array<Ort::Value, 2> inputs = {
            std::move(features), std::move(features_length)
        };

        auto outputs = session_->Run(
            {},
            input_names_ptr_.data(),
            inputs.data(),
            inputs.size(),
            output_names_ptr_.data(),
            output_names_ptr_.size()
        );

        return std::vector<Ort::Value>(
            std::make_move_iterator(outputs.begin()),
            std::make_move_iterator(outputs.end())
        );
    }

private:
    void InitModel(void *model_data, size_t model_data_length) {
        session_ = std::make_unique<Ort::Session>(
            env_, model_data, model_data_length, sess_opts_);

        GetInputNames(session_.get(), &input_names_, &input_names_ptr_);
        GetOutputNames(session_.get(), &output_names_, &output_names_ptr_);

        // Read metadata
        Ort::ModelMetadata meta_data = session_->GetModelMetadata();
        Ort::AllocatorWithDefaultOptions allocator;

        auto vocab_str = LookupCustomModelMetaData(
            meta_data, "vocab_size", allocator);
        if (!vocab_str.empty()) {
            vocab_size_ = std::stoi(vocab_str);
        }
    }

    ModelConfig config_;
    Ort::Env env_;
    Ort::SessionOptions sess_opts_;
    Ort::AllocatorWithDefaultOptions allocator_;
    std::unique_ptr<Ort::Session> session_;

    std::vector<std::string> input_names_;
    std::vector<const char*> input_names_ptr_;
    std::vector<std::string> output_names_;
    std::vector<const char*> output_names_ptr_;

    int32_t vocab_size_ = 0;
};
```

---

## 4. Tensor Creation Patterns

### Pattern A: From Existing Data (CPU)
```cpp
auto memory_info = Ort::MemoryInfo::CreateCpu(
    OrtDeviceAllocator, OrtMemTypeDefault);

std::array<int64_t, 3> shape = {batch_size, num_frames, feat_dim};

Ort::Value tensor = Ort::Value::CreateTensor<float>(
    memory_info,
    data_ptr,           // Existing data pointer
    data_size,          // Size in elements
    shape.data(),       // Shape array
    shape.size()        // Rank
);
```

### Pattern B: Allocate New Tensor
```cpp
std::array<int64_t, 2> shape = {batch_size, context_size};

Ort::Value tensor = Ort::Value::CreateTensor<int64_t>(
    allocator,          // OrtAllocator*
    shape.data(),
    shape.size()
);

// Fill the tensor
int64_t *p = tensor.GetTensorMutableData<int64_t>();
std::copy(src_begin, src_end, p);
```

---

## 5. Metadata Reading

### Macro Approach
```cpp
// macros.h
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
    } while (0)
```

### Usage
```cpp
Ort::ModelMetadata meta_data = session_->GetModelMetadata();
Ort::AllocatorWithDefaultOptions allocator;

SHERPA_ONNX_READ_META_DATA(vocab_size_, "vocab_size");
SHERPA_ONNX_READ_META_DATA(context_size_, "context_size");
```

### Direct Approach
```cpp
std::string LookupCustomModelMetaData(
    const Ort::ModelMetadata &meta_data,
    const char *key,
    OrtAllocator *allocator) {

    #if ORT_API_VERSION >= 12
        auto v = meta_data.LookupCustomMetadataMapAllocated(key, allocator);
        return v ? v.get() : "";
    #else
        auto v = meta_data.LookupCustomMetadataMap(key, allocator);
        std::string ans = v ? v : "";
        allocator->Free(allocator, v);
        return ans;
    #endif
}
```

---

## 6. Provider Configuration

### Basic CPU/CUDA Selection
```cpp
Ort::SessionOptions GetSessionOptions(int32_t num_threads,
                                      bool use_cuda,
                                      int32_t device_id = 0) {
    Ort::SessionOptions sess_opts;
    sess_opts.SetIntraOpNumThreads(num_threads);
    sess_opts.SetInterOpNumThreads(num_threads);

    if (use_cuda) {
        std::vector<std::string> providers = Ort::GetAvailableProviders();

        auto it = std::find(providers.begin(), providers.end(),
                           "CUDAExecutionProvider");

        if (it != providers.end()) {
            OrtCUDAProviderOptions cuda_opts;
            cuda_opts.device_id = device_id;
            cuda_opts.cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic;
            sess_opts.AppendExecutionProvider_CUDA(cuda_opts);
        } else {
            SHERPA_ONNX_LOGE("CUDA requested but not available, using CPU");
        }
    }

    return sess_opts;
}
```

---

## 7. Tensor Utility Functions

### Clone (Deep Copy)
```cpp
Ort::Value Clone(OrtAllocator *allocator, const Ort::Value *v) {
    auto type_and_shape = v->GetTensorTypeAndShapeInfo();
    std::vector<int64_t> shape = type_and_shape.GetShape();

    Ort::Value ans = Ort::Value::CreateTensor<float>(
        allocator, shape.data(), shape.size());

    const float *src = v->GetTensorData<float>();
    float *dst = ans.GetTensorMutableData<float>();
    size_t count = type_and_shape.GetElementCount();

    std::copy(src, src + count, dst);
    return ans;
}
```

### View (Shallow Copy)
```cpp
Ort::Value View(Ort::Value *v) {
    auto type_and_shape = v->GetTensorTypeAndShapeInfo();
    std::vector<int64_t> shape = type_and_shape.GetShape();

    auto memory_info = Ort::MemoryInfo::CreateCpu(
        OrtDeviceAllocator, OrtMemTypeDefault);

    return Ort::Value::CreateTensor<float>(
        memory_info,
        v->GetTensorMutableData<float>(),
        type_and_shape.GetElementCount(),
        shape.data(),
        shape.size()
    );
}
```

### Fill
```cpp
template <typename T = float>
void Fill(Ort::Value *tensor, T value) {
    auto n = tensor->GetTypeInfo()
        .GetTensorTypeAndShapeInfo().GetElementCount();
    auto p = tensor->GetTensorMutableData<T>();
    std::fill(p, p + n, value);
}
```

---

## 8. Debugging Helpers

### Print Shape
```cpp
void PrintShape(const Ort::Value *v) {
    std::vector<int64_t> shape =
        v->GetTensorTypeAndShapeInfo().GetShape();

    std::ostringstream os;
    os << "Shape: [";
    for (size_t i = 0; i < shape.size(); ++i) {
        if (i > 0) os << ", ";
        os << shape[i];
    }
    os << "]";

    fprintf(stderr, "%s\n", os.str().c_str());
}
```

### Print Tensor Data
```cpp
template <typename T = float>
void Print2D(const Ort::Value *v) {
    std::vector<int64_t> shape =
        v->GetTensorTypeAndShapeInfo().GetShape();
    const T *data = v->GetTensorData<T>();

    std::ostringstream os;
    for (int32_t r = 0; r < shape[0]; ++r) {
        for (int32_t c = 0; c < shape[1]; ++c, ++data) {
            os << *data << " ";
        }
        os << "\n";
    }
    fprintf(stderr, "%s\n", os.str().c_str());
}
```

---

## 9. Complete Example: Parakeet-TDT Integration

```cpp
#include "onnxruntime_cxx_api.h"
#include <memory>
#include <vector>
#include <array>

class ParakeetTDT {
public:
    explicit ParakeetTDT(const std::string &model_path,
                        int num_threads = 4,
                        bool use_cuda = false) {
        // 1. Create environment
        env_ = std::make_unique<Ort::Env>(ORT_LOGGING_LEVEL_WARNING);

        // 2. Configure session
        sess_opts_ = GetSessionOptions(num_threads, use_cuda);

        // 3. Load model
        session_ = std::make_unique<Ort::Session>(
            *env_, model_path.c_str(), sess_opts_);

        // 4. Extract I/O names
        GetInputNames(session_.get(), &input_names_, &input_names_ptr_);
        GetOutputNames(session_.get(), &output_names_, &output_names_ptr_);

        // 5. Read metadata
        Ort::ModelMetadata meta = session_->GetModelMetadata();
        Ort::AllocatorWithDefaultOptions alloc;
        auto vocab_str = LookupMetadata(meta, "vocab_size", alloc);
        vocab_size_ = vocab_str.empty() ? 1024 : std::stoi(vocab_str);
    }

    std::vector<float> Recognize(const std::vector<float> &audio_features,
                                 int batch_size,
                                 int num_frames,
                                 int feat_dim) {
        // 1. Create input tensor
        auto memory_info = Ort::MemoryInfo::CreateCpu(
            OrtDeviceAllocator, OrtMemTypeDefault);

        std::array<int64_t, 3> shape = {batch_size, num_frames, feat_dim};

        Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
            memory_info,
            const_cast<float*>(audio_features.data()),
            audio_features.size(),
            shape.data(),
            shape.size()
        );

        // 2. Run inference
        std::array<Ort::Value, 1> inputs = {std::move(input_tensor)};

        auto outputs = session_->Run(
            Ort::RunOptions{nullptr},
            input_names_ptr_.data(),
            inputs.data(),
            1,
            output_names_ptr_.data(),
            output_names_ptr_.size()
        );

        // 3. Extract results
        const float *logits = outputs[0].GetTensorData<float>();
        auto shape = outputs[0].GetTensorTypeAndShapeInfo().GetShape();
        size_t total_size = std::accumulate(
            shape.begin(), shape.end(), 1, std::multiplies<int64_t>());

        return std::vector<float>(logits, logits + total_size);
    }

private:
    std::unique_ptr<Ort::Env> env_;
    Ort::SessionOptions sess_opts_;
    std::unique_ptr<Ort::Session> session_;

    std::vector<std::string> input_names_;
    std::vector<const char*> input_names_ptr_;
    std::vector<std::string> output_names_;
    std::vector<const char*> output_names_ptr_;

    int vocab_size_;
};
```

---

## Key Takeaways

1. **Use factory functions** for session options
2. **Extract I/O names** with helper functions
3. **Store dual vectors** (string + char*) for names
4. **Read model metadata** for configuration
5. **Use smart pointers** for ONNX objects
6. **Support multiple providers** with graceful fallback
7. **Add utility functions** for common operations
8. **Implement PIMPL pattern** for clean interfaces

These patterns from Sherpa-ONNX are battle-tested across millions of deployments and should significantly improve our implementation's robustness.
