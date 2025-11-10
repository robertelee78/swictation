# Sherpa-ONNX Quick Reference

**Research Completed:** 2025-11-09
**Hive Swarm:** swarm-1762744363981-g4ic134p6

## 📁 Documentation Index

1. **sherpa-onnx-analysis.md** (635 lines)
   - Complete technical analysis
   - Architecture deep-dive
   - 10 sections covering all aspects

2. **sherpa-onnx-code-patterns.md** (484 lines)
   - 9 ready-to-use code patterns
   - Complete working examples
   - Copy-paste implementations

3. **sherpa-onnx-quick-reference.md** (this file)
   - Quick lookup reference
   - Key insights summary

---

## 🎯 Top 5 Immediate Improvements

| Priority | Pattern | Impact | Effort |
|----------|---------|--------|--------|
| 🔴 **HIGH** | Session factory with providers | High | Low |
| 🔴 **HIGH** | GetInputNames/GetOutputNames helpers | High | Low |
| 🔴 **HIGH** | Dual vector I/O name management | High | Medium |
| 🟡 **MEDIUM** | Read model metadata | Medium | Low |
| 🟡 **MEDIUM** | CUDA provider support | High | Medium |

---

## 📊 Architecture Comparison

| Component | Sherpa-ONNX | Our Current | Gap |
|-----------|-------------|-------------|-----|
| Session Factory | ✅ Centralized | ❌ Ad-hoc | **HIGH** |
| Provider Support | ✅ 7 providers | ⚠️ CPU only | **HIGH** |
| I/O Name Extraction | ✅ Helper functions | ❌ Manual | **HIGH** |
| Metadata Reading | ✅ Macros + utils | ❌ None | **MEDIUM** |
| Tensor Utilities | ✅ 20+ functions | ⚠️ Basic | **MEDIUM** |
| Error Handling | ✅ Extensive | ⚠️ Basic | **LOW** |

---

## 🔧 Essential Code Snippets

### 1. Session Initialization (Complete)
```cpp
Ort::Env env(ORT_LOGGING_LEVEL_WARNING);
Ort::SessionOptions opts = GetSessionOptions(config);
auto session = std::make_unique<Ort::Session>(env, model_data, size, opts);

GetInputNames(session.get(), &input_names, &input_names_ptr);
GetOutputNames(session.get(), &output_names, &output_names_ptr);
```

### 2. Run Inference (Complete)
```cpp
std::array<Ort::Value, 1> inputs = {std::move(input_tensor)};
auto outputs = session->Run(
    {}, input_names_ptr.data(), inputs.data(), 1,
    output_names_ptr.data(), output_names_ptr.size());
```

### 3. Provider Selection (Complete)
```cpp
if (use_cuda) {
    OrtCUDAProviderOptions cuda_opts;
    cuda_opts.device_id = 0;
    cuda_opts.cudnn_conv_algo_search = OrtCudnnConvAlgoSearchHeuristic;
    sess_opts.AppendExecutionProvider_CUDA(cuda_opts);
}
```

---

## 📚 Key Files from Sherpa-ONNX

### Must Read
- `session.{h,cc}` - Session factory pattern ⭐⭐⭐⭐⭐
- `onnx-utils.{h,cc}` - Tensor utilities ⭐⭐⭐⭐⭐
- `offline-transducer-model.cc` - Multi-session pattern ⭐⭐⭐⭐

### Good to Know
- `provider-config.h` - Provider configs ⭐⭐⭐
- `features.{h,cc}` - Audio preprocessing ⭐⭐⭐
- `macros.h` - Metadata helpers ⭐⭐

---

## 🎓 Key Learnings

### Pattern 1: Dual Vector for Names
**Why:** ONNX Runtime needs `const char**`, but we need owned strings
```cpp
std::vector<std::string> input_names;        // Owns strings
std::vector<const char*> input_names_ptr;    // Points to c_str()
```

### Pattern 2: Session Options Factory
**Why:** Centralize provider logic and reuse configuration
```cpp
Ort::SessionOptions GetSessionOptions(const Config &config) {
    // Single place for all provider setup
}
```

### Pattern 3: PIMPL for Clean API
**Why:** Hide implementation details, faster compilation
```cpp
class Model {
    class Impl;  // Forward declare
    std::unique_ptr<Impl> impl_;
};
```

---

## 🚀 Migration Path

### Phase 1: Foundation (Week 1)
- [ ] Implement session factory
- [ ] Add GetInputNames/GetOutputNames
- [ ] Setup dual vector I/O names
- [ ] Test with current CPU implementation

### Phase 2: Enhancement (Week 2)  
- [ ] Add metadata reading
- [ ] Implement CUDA provider support
- [ ] Add basic tensor utilities (Clone, View, Fill)
- [ ] Test with GPU

### Phase 3: Polish (Week 3)
- [ ] Add debugging utilities
- [ ] Implement multi-provider fallback
- [ ] Performance optimization
- [ ] Documentation

---

## 📈 Expected Benefits

| Improvement | Benefit | Effort |
|-------------|---------|--------|
| Session factory | Better maintainability | 2-4 hours |
| I/O name helpers | Fewer bugs, cleaner code | 1-2 hours |
| CUDA support | 10-100x faster inference | 4-8 hours |
| Metadata reading | Auto-configuration | 1-2 hours |
| Tensor utilities | Development speed | 2-4 hours |

**Total Time:** ~10-20 hours for complete migration
**Total Impact:** 🚀 Production-ready implementation

---

## 💡 Pro Tips

1. **Start with session factory** - Foundation for everything else
2. **Copy helper functions directly** - They're battle-tested
3. **Test incrementally** - Verify each pattern works
4. **Use smart pointers** - Avoid manual memory management
5. **Read metadata** - Models contain config you need

---

## 🔗 References

- **Analysis:** `/opt/swictation/docs/sherpa-onnx-analysis.md`
- **Patterns:** `/opt/swictation/docs/sherpa-onnx-code-patterns.md`
- **Source:** https://github.com/k2-fsa/sherpa-onnx
- **Memory:** `.swarm/memory.db` (key: `swarm/researcher/sherpa-findings`)

---

## ✅ Checklist for Implementation

- [ ] Read full analysis document
- [ ] Copy session factory pattern
- [ ] Implement I/O name helpers
- [ ] Test with existing model
- [ ] Add CUDA support
- [ ] Verify metadata reading
- [ ] Add tensor utilities
- [ ] Update documentation
- [ ] Performance testing
- [ ] Code review

---

**Status:** ✅ Research Complete | 📝 Documentation Ready | 🚀 Ready for Implementation
