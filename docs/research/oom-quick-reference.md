# OOM Prevention Quick Reference

**For:** Coder/Implementer Agents
**System:** RTX A1000 4GB VRAM
**Issue:** CUDA "unspecified launch failure", VRAM exhaustion

---

## TL;DR - Immediate Fixes

1. **GPU→RAM Spillover for VAD** → Move VAD to CPU when VRAM > 3500MB
2. **Reduce NeMo Context** → Change `total_buffer=10.0` → `5.0` (line 230, swictationd.py)
3. **Add VRAM Guards** → Check VRAM before transcription operations

---

## Critical Code Locations

### 1. VAD Model Loading (swictationd.py:139-171)
```python
# Current: Always on GPU (line 152-153)
if torch.cuda.is_available():
    vad_model = vad_model.cuda()

# Fix: Add adaptive placement
if torch.cuda.is_available():
    current_vram = torch.cuda.memory_allocated() / 1e6
    if current_vram < 3500:  # Only load on GPU if safe
        vad_model = vad_model.cuda()
    else:
        print("⚠️ High VRAM, keeping VAD on CPU")
```

### 2. VAD Inference (swictationd.py:605-635)
```python
# Current: Assumes GPU (line 617-618)
audio_tensor = torch.from_numpy(audio).float()
if torch.cuda.is_available():
    audio_tensor = audio_tensor.cuda()

# Fix: Check model device
audio_tensor = torch.from_numpy(audio).float()
if self.vad_model.device.type == 'cuda':
    audio_tensor = audio_tensor.cuda()
else:
    audio_tensor = audio_tensor.cpu()  # Match model device
```

### 3. NeMo Streaming Buffer (swictationd.py:225-236)
```python
# Current: 10s context (line 230)
total_buffer=10.0,

# Fix: Reduce for 4GB GPUs
total_buffer=5.0,  # Saves ~10MB VRAM
```

### 4. Performance Monitoring Thresholds (performance_monitor.py:115-121)
```python
# Current thresholds
self.thresholds = {
    'gpu_memory_mb': 4000,  # Too late!

# Fix: More aggressive thresholds
self.thresholds = {
    'gpu_memory_mb': 3200,  # 80% warning
    'gpu_memory_critical': 3500,  # 87.5% critical
    'gpu_memory_emergency': 3800,  # 95% emergency
```

---

## Implementation Checklist

### Phase 1: Emergency Fixes (2-3 hours)

- [ ] Add `AdaptiveMemoryManager` class to `swictationd.py`
- [ ] Modify VAD loading to check VRAM before GPU placement
- [ ] Modify VAD inference to respect model device
- [ ] Add VRAM check before transcription operations
- [ ] Change `total_buffer` from 10.0 → 5.0
- [ ] Update thresholds in `performance_monitor.py`
- [ ] Test with sustained load (30 minutes)

### Phase 2: Robust Monitoring (2 hours)

- [ ] Implement `CUDAMemoryGuard` circuit breaker
- [ ] Add pre-transcription VRAM checks
- [ ] Implement automatic VAD CPU/GPU migration
- [ ] Add VRAM fragmentation detection
- [ ] Test CUDA error recovery

### Phase 3: Optimization (4 hours)

- [ ] Enable FP16 mixed precision (`precision=16`)
- [ ] Add memory watchdog thread
- [ ] Implement degraded mode (VAD-only when critical)
- [ ] Long-term: INT8 quantization research

---

## Memory Budget (4GB VRAM Target)

| Component | Current | Optimized | Savings |
|-----------|---------|-----------|---------|
| Canary Model | 3370 MB | 1700 MB (FP16) | 1670 MB |
| NeMo Buffer | 20 MB | 10 MB (5s ctx) | 10 MB |
| VAD Model | 2.2 MB | 0 MB (CPU) | 2.2 MB |
| Inference | 8.5 MB | 8.5 MB | 0 MB |
| **Total** | **3400 MB** | **1719 MB** | **1682 MB** |
| **Margin** | **600 MB (15%)** | **2281 MB (57%)** | **+38%** |

---

## Testing Commands

```bash
# Test sustained load
python3 tests/test_memory_leaks.py --duration 30

# Test CUDA error recovery
python3 tests/test_streaming_e2e.py -k test_gpu_memory_stable

# Monitor VRAM in real-time
watch -n 1 'nvidia-smi --query-gpu=memory.used,memory.free --format=csv'

# Check for fragmentation
python3 -c "
import torch
if torch.cuda.is_available():
    alloc = torch.cuda.memory_allocated() / 1e6
    resv = torch.cuda.memory_reserved() / 1e6
    print(f'Allocated: {alloc:.0f}MB, Reserved: {resv:.0f}MB')
    print(f'Fragmentation ratio: {resv/alloc:.2f}x')
"
```

---

## Expected Results

### Before Fixes
- VRAM: 3400-3600MB (85-90% capacity)
- CUDA errors: Frequent "unspecified launch failure"
- RAM: 6GB peak with 1.3GB swap
- Stability: Crashes every 15-30 minutes

### After Phase 1 Fixes
- VRAM: 3200-3400MB (80-85% capacity)
- CUDA errors: Rare (VAD on CPU prevents most)
- RAM: 4GB peak, minimal swap
- Stability: 2+ hours sustained operation

### After Phase 2+3 (with FP16)
- VRAM: 1700-1900MB (42-47% capacity)
- CUDA errors: None
- RAM: 3-4GB, no swap
- Stability: Indefinite operation

---

## Code Templates

### Template 1: Adaptive VAD Placement
```python
class AdaptiveMemoryManager:
    def __init__(self, vram_threshold_mb=3500):
        self.vram_threshold = vram_threshold_mb
        self.vad_on_cpu = False

    def should_use_cpu_for_vad(self):
        if not torch.cuda.is_available():
            return True
        current_vram = torch.cuda.memory_allocated() / 1e6
        return current_vram > self.vram_threshold

    def place_vad_model(self, vad_model):
        if self.should_use_cpu_for_vad():
            if not self.vad_on_cpu:
                vad_model = vad_model.cpu()
                self.vad_on_cpu = True
                print("⚠️ VAD moved to CPU (high VRAM)")
        else:
            if self.vad_on_cpu:
                vad_model = vad_model.cuda()
                self.vad_on_cpu = False
                print("✓ VAD moved to GPU (VRAM available)")
        return vad_model
```

### Template 2: Pre-Transcription Guard
```python
def safe_transcribe(audio_path, max_vram_mb=3700):
    """Transcribe with VRAM guard"""
    if torch.cuda.is_available():
        current_vram = torch.cuda.memory_allocated() / 1e6
        if current_vram > max_vram_mb:
            # Emergency cleanup
            torch.cuda.empty_cache()
            torch.cuda.synchronize()
            gc.collect()

            new_vram = torch.cuda.memory_allocated() / 1e6
            if new_vram > max_vram_mb:
                raise MemoryError(
                    f"VRAM too high: {new_vram:.0f}MB (threshold: {max_vram_mb}MB)"
                )

    try:
        return stt_model.transcribe([audio_path], batch_size=1)
    finally:
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
```

---

## Coordination Notes

**Research Findings Stored In:**
- `/opt/swictation/docs/research/oom-prevention-strategies.md` (full report)
- Hive Memory: `hive/research/oom-prevention` (via hooks)

**Next Agent:** Coder/Implementer
**Priority:** CRITICAL (system unstable)
**Estimated Time:** 4-6 hours for Phase 1+2

**Dependencies:**
- No new packages required
- All fixes use existing PyTorch APIs
- Testing uses existing test infrastructure

---

**Last Updated:** 2025-10-31 by Research Agent
