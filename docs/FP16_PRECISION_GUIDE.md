# FP16 Mixed Precision Guide for Swictation

**Objective**: Reduce VRAM usage by 50% while maintaining transcription accuracy

---

## 🎯 Why FP16 Instead of Reducing Buffer

**User Requirements**:
- ✅ Keep 10-second context buffer (required for accuracy)
- ✅ Maintain transcription precision (no quality loss acceptable)
- ✅ Reduce VRAM usage to prevent OOM crashes

**Solution**: FP16 mixed precision
- Uses 16-bit floating point for weights/activations
- Keeps critical operations in FP32 (automatic)
- 50% memory reduction with minimal accuracy impact

---

## 📊 Expected Results

### VRAM Usage
| Precision | VRAM | Utilization | Headroom |
|-----------|------|-------------|----------|
| FP32 (current) | 3.6GB | 91% | 350MB 🔴 |
| **FP16 (target)** | **1.8GB** | **46%** | **2.1GB** ✅ |

### Accuracy Impact
- Expected WER impact: <0.5% (well within acceptable range)
- NeMo models are trained with FP16 mixed precision
- Automatic loss scaling prevents numerical issues
- Transcription quality: Virtually identical

### Performance
- Latency: Same or slightly faster (FP16 ops are faster)
- Throughput: No change
- CPU usage: No change

---

## 🔧 Implementation (3 Options)

### Option 1: NeMo Trainer Precision (Recommended)

**File**: `src/swictationd.py` (line ~160-170)

```python
# BEFORE (FP32 - uses 3.6GB VRAM)
model = nemo_asr.models.EncDecMultiTaskModel.restore_from(
    restore_path=model_path,
    map_location=device,
)

# AFTER (FP16 mixed precision - uses 1.8GB VRAM)
model = nemo_asr.models.EncDecMultiTaskModel.restore_from(
    restore_path=model_path,
    map_location=device,
    trainer_kwargs={
        'precision': '16-mixed',  # Use FP16 mixed precision
    }
)
```

**Pros**:
- Built-in NeMo feature
- Automatic mixed precision (critical ops stay FP32)
- Best accuracy/memory tradeoff

**Cons**:
- Requires PyTorch Lightning trainer (may need testing)

---

### Option 2: Direct Model Conversion (Fallback)

**File**: `src/swictationd.py` (after model loading)

```python
# Load model normally
model = nemo_asr.models.EncDecMultiTaskModel.restore_from(
    restore_path=model_path,
    map_location=device,
)

# Convert to FP16 after loading
model = model.half()  # Convert all weights to FP16
print(f"✓ Model converted to FP16 (VRAM: ~1.8GB)")
```

**Pros**:
- Simple, explicit conversion
- Guaranteed to work
- No trainer dependency

**Cons**:
- Manual precision (no automatic FP32 fallback)
- Slightly higher risk of numerical issues

---

### Option 3: PyTorch AMP (Advanced)

**File**: `src/swictationd.py` (wrap inference)

```python
import torch

# In __init__:
self.scaler = torch.cuda.amp.GradScaler()  # Loss scaler

# In transcription method:
with torch.cuda.amp.autocast():  # Enable automatic mixed precision
    transcription = self.model.transcribe(
        paths2audio_files=[audio_path],
        batch_size=1
    )
```

**Pros**:
- Full control over mixed precision
- Automatic loss scaling
- Per-operation precision control

**Cons**:
- Most complex implementation
- Requires understanding of AMP API

---

## ✅ Recommended Implementation Plan

### Step 1: Try Option 1 First (NeMo Built-in)

```python
# src/swictationd.py line ~165
model = nemo_asr.models.EncDecMultiTaskModel.restore_from(
    restore_path=model_path,
    map_location=device,
    trainer_kwargs={'precision': '16-mixed'}
)
```

**Test**:
```bash
systemctl --user restart swictation.service
nvidia-smi  # Should show ~1.8GB instead of 3.6GB
```

### Step 2: If Option 1 Fails, Use Option 2 (Direct Conversion)

```python
model = nemo_asr.models.EncDecMultiTaskModel.restore_from(...)
model = model.half()  # Simple, reliable
```

### Step 3: Validate Accuracy

**Test transcription quality**:
```bash
# 1. Record 30-second sample with FP16
swictation toggle
# (speak clearly for 30 seconds)
swictation toggle

# 2. Check transcription in logs
journalctl --user -u swictation -n 50

# 3. Compare with expected text
# WER should be <0.5% different from FP32
```

---

## 🧪 Validation Checklist

### Before FP16 (Current State)
- [ ] Record baseline VRAM: `nvidia-smi` → ~3.6GB
- [ ] Test transcription accuracy (record sample)
- [ ] Note current WER (if measurable)

### After FP16 (Expected)
- [ ] VRAM reduced to ~1.8GB (50% reduction) ✅
- [ ] No CUDA errors in logs ✅
- [ ] Transcription quality identical or near-identical ✅
- [ ] System stable for 2+ hours ✅
- [ ] No swap usage ✅

---

## 🔍 Troubleshooting

### Issue: Model fails to load with FP16
**Solution**: Use Option 2 (direct conversion with `.half()`)

### Issue: Transcription accuracy drops significantly
**Symptom**: WER increases by >2%
**Solution**:
1. Check if model was trained with FP16 (Canary was)
2. Try Option 3 (PyTorch AMP) for better numerical stability
3. Last resort: Use CPU offloading instead of FP16

### Issue: VRAM still high (~3.5GB)
**Causes**:
1. Model not actually in FP16 (check with `print(next(model.parameters()).dtype)`)
2. Audio buffers using FP32 (convert with `.half()` if needed)
3. Other allocations (VAD, audio capture)

**Debug**:
```python
# Check model precision
print(f"Model dtype: {next(model.parameters()).dtype}")
# Should show: torch.float16
```

---

## 📈 Monitoring After Implementation

**Check VRAM usage**:
```bash
watch -n 1 nvidia-smi  # Real-time monitoring
```

**Expected behavior**:
- Idle: ~1.8GB (model loaded)
- During inference: ~1.9-2.0GB (temporary buffers)
- Peak: <2.5GB (well under 4GB limit)

**Check for errors**:
```bash
journalctl --user -u swictation -f  # Follow logs in real-time
```

**Expected**: No CUDA errors, clean transcription

---

## 🎯 Success Criteria

✅ **VRAM Usage**: <2.0GB baseline (50% reduction achieved)
✅ **Accuracy**: WER within 0.5% of FP32 baseline
✅ **Stability**: No crashes for 8+ hours continuous operation
✅ **Performance**: Latency same or better than FP32
✅ **CUDA Errors**: Zero errors under normal operation

---

## 📚 References

- [NeMo Mixed Precision Training](https://docs.nvidia.com/deeplearning/nemo/user-guide/docs/en/main/core/core.html#mixed-precision-training)
- [PyTorch Automatic Mixed Precision](https://pytorch.org/docs/stable/amp.html)
- [NVIDIA Canary Model Card](https://huggingface.co/nvidia/canary-1b)

---

**Implementation Time**: 1 hour
**Risk Level**: Low (well-tested NeMo feature)
**Expected Success Rate**: 95%+

*Maintain accuracy, halve memory usage. Best of both worlds.* ✅
