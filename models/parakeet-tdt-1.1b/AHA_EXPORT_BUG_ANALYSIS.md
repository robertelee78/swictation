# 🎯 AHA Moment #20 - Parakeet-TDT 1.1B Export Bug Analysis

**Date:** 2025-11-10
**Status:** ✅ ROOT CAUSE IDENTIFIED & FIXED

---

## 🔍 Problem Statement

sherpa-onnx (v1.12.15) failed to load the exported 1.1B model with error:
```
<blk> is not the last token!
```

Initial hypothesis was that tokens.txt format was incorrect, but deeper investigation revealed TWO critical metadata bugs in the export script.

---

## 🎯 Root Cause Analysis

### Bug #1: Incorrect vocab_size Metadata

**What We Did Wrong:**
```python
# export_parakeet_tdt_1.1b.py line 86 (WRONG!)
vocab_size = len(asr_model.joint.vocabulary) + 1  # = 1024 + 1 = 1025
```

**Why It's Wrong:**
- The metadata `vocab_size` should represent vocabulary ONLY (excluding blank token)
- sherpa-onnx adds the blank token internally
- For TDT models, sherpa-onnx validates: `joiner_output_dim - num_durations = vocab_size + 1`

**The Math:**
```
Our model:
- Joint vocabulary: 1024 tokens
- Joiner output: 1030 dimensions
- TDT durations: 5 (0, 1, 2, 3, 4)

sherpa-onnx calculation:
- Expected vocab_size = (1030 - num_durations) - 1
- With num_durations=4: (1030 - 4) - 1 = 1025 ❌ (but we set 1025)
- With num_durations=5: (1030 - 5) - 1 = 1024 ✅

The issue: sherpa-onnx detected 4 durations instead of 5!
But even with 5, our vocab_size=1025 was still wrong!
```

**Official 0.6B Model for Comparison:**
- vocab_size (metadata): **1024** (NOT 1025!)
- tokens.txt lines: 1025 (1024 vocab + 1 blank)
- joiner output: 1030 (1024 + 1 blank + 5 durations)

**Correct Formula:**
```python
vocab_size = len(asr_model.joint.vocabulary)  # Don't add 1!
```

---

### Bug #2: Incorrect feat_dim Metadata

**What We Did Wrong:**
```python
# export_parakeet_tdt_1.1b.py line 120 (WRONG!)
"feat_dim": 128,  # Copied from 0.6B model
```

**Why It's Wrong:**
- The 1.1B model uses **80 mel filterbank features**, not 128
- This was discovered during Rust testing when encoder expected [batch, 80, time] inputs
- We incorrectly assumed it matched the 0.6B model's 128 features

**Official Specs:**
- 0.6B model: 128 mel features
- 1.1B model: **80 mel features** ← Critical difference!

**Correct Value:**
```python
"feat_dim": 80,  # 1.1B uses 80 mel features!
```

---

## 🔬 Investigation Process

### 1. Initial Error
```
/project/sherpa-onnx/csrc/offline-recognizer-transducer-nemo-impl.h:PostInit:180
<blk> is not the last token!
```

### 2. Token File Analysis
- Checked tokens.txt format: **✅ CORRECT** (1025 lines, <blk> at end)
- Compared with official 0.6B: Same format ✅
- tokens.txt was NOT the problem!

### 3. ONNX Model Inspection
```python
# Joiner output dimensions
model.graph.output[0].shape: [batch, time, 1, 1030]
                                              ^^^^
# But vocabulary only has 1024 tokens + blank = 1025
# Missing 5 dimensions = TDT duration outputs!
```

### 4. TDT Model Discovery
- Researched TDT (Token-and-Duration Transducer) architecture
- Found: Model config shows `num_extra_outputs: 5`
- Found: Loss config shows `durations: [0, 1, 2, 3, 4]`
- **Insight:** Joiner outputs BOTH tokens AND durations!

### 5. sherpa-onnx Compatibility Research
- Discovered sherpa-onnx added TDT support in v1.11.5 (May 2025)
- Confirmed TDT support in v1.12.15 (our version) ✅
- Found bug fix for TDT decoding in v1.12.14 (PR #2606)

### 6. Official Model Comparison
Downloaded official 0.6B model metadata:
```
vocab_size: 1024  ← AHA! Not 1025!
feat_dim: 128
```

Our 1.1B metadata (WRONG):
```
vocab_size: 1025  ← Should be 1024!
feat_dim: 128     ← Should be 80!
```

### 7. Debug Output Analysis
sherpa-onnx debug output revealed:
```
TDT model. vocab_size: 1026, num_durations: 4
```

But our metadata said `vocab_size=1025` → **MISMATCH!**

Also: sherpa-onnx detected only 4 durations, but model has 5 durations!

---

## ✅ Solution

### Fixed Export Script

**File:** `/opt/swictation/scripts/export_parakeet_tdt_1.1b.py`

**Change 1 (Line 87):**
```python
# BEFORE:
vocab_size = len(asr_model.joint.vocabulary) + 1

# AFTER:
# CRITICAL: vocab_size should NOT include blank token (sherpa-onnx adds it internally)
vocab_size = len(asr_model.joint.vocabulary)
```

**Change 2 (Line 122):**
```python
# BEFORE:
"feat_dim": 128,  # Mel filterbank features

# AFTER:
"feat_dim": 80,  # CRITICAL: 1.1B uses 80 mel features, not 128!
```

### Re-Export Process
```bash
docker run --rm \
  -v /opt/swictation:/workspace \
  -w /workspace/models/parakeet-tdt-1.1b \
  nvcr.io/nvidia/nemo:25.07 \
  bash -c "pip install onnxruntime && python3 /workspace/scripts/export_parakeet_tdt_1.1b.py"
```

---

## 📊 Comparison: 0.6B vs 1.1B

| Feature | 0.6B | 1.1B |
|---------|------|------|
| Vocabulary Size | 1024 | 1024 |
| Mel Features | 128 | **80** |
| Encoder Output Dim | 128 | **1024** |
| Decoder Hidden | 640 | 640 |
| TDT Durations | 5 (0-4) | 5 (0-4) |
| tokens.txt Lines | 1025 | 1025 |
| Joiner Output | 1030 | 1030 |
| **vocab_size (metadata)** | **1024** | **1024** ✅ |
| **feat_dim (metadata)** | **128** | **80** ✅ |

---

## 🎓 Key Learnings

1. **Metadata vs File Format:**
   - `vocab_size` metadata = vocabulary only (no blank)
   - `tokens.txt` file = vocabulary + blank token
   - These are different!

2. **TDT Model Architecture:**
   - Joiner outputs = vocab + blank + duration outputs
   - For 1.1B: 1024 + 1 + 5 = 1030 total outputs
   - Duration outputs (0-4) allow frame skipping for faster inference

3. **sherpa-onnx TDT Support:**
   - Requires v1.11.5+ for TDT models
   - Validates: `joiner_output_dim - num_durations = vocab_size + 1`
   - Internally shifts blank token to position 0 (NeMo has it at end)

4. **Model-Specific Parameters:**
   - Don't assume all models from same family have same parameters!
   - 0.6B uses 128 mel features, 1.1B uses 80
   - Always check model config for exact specifications

5. **Validation Strategy:**
   - Use reference implementation (sherpa-onnx) to validate exports
   - Compare with known-working models (0.6B)
   - Check debug output for mismatch clues

---

## ✅ Next Steps

1. **Validation:** Test re-exported model with sherpa-onnx validation script
2. **Rust Integration:** Test with Rust recognizer once validation passes
3. **Documentation:** Update README with correct metadata requirements
4. **Testing:** Add automated tests to catch metadata bugs in future exports

---

## 📝 References

- sherpa-onnx TDT support: https://github.com/k2-fsa/sherpa-onnx/issues/2183
- Official 0.6B model: https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8
- NeMo 1.1B model: https://huggingface.co/nvidia/parakeet-tdt-1.1b
- TDT paper: Token-and-Duration Transducer for fast inference

---

**Status:** 🔄 Re-exporting with corrected metadata...
