# Memory Protection System

## Overview

Swictation's memory protection system prevents GPU out-of-memory (OOM) crashes through pre-emptive monitoring, progressive degradation, and automatic recovery.

## Features

### 1. Pre-emptive Monitoring
- **Interval**: 2-second checks (non-blocking thread)
- **Zero overhead**: Separate monitoring thread doesn't impact transcription
- **Real-time alerts**: Immediate notification of memory pressure

### 2. Progressive Degradation (4 Levels)

#### Level 1: NORMAL (<80% usage)
- **Action**: None required
- **State**: Optimal performance
- **GPU**: Full acceleration

#### Level 2: WARNING (80-90% usage)
- **Action**: Garbage collection
- **Impact**: Minimal (<10ms)
- **Recovery**: Automatic cleanup

#### Level 3: CRITICAL (90-95% usage)
- **Action**: Aggressive cleanup (3x GC + cache clear)
- **Impact**: Moderate (~50ms)
- **Recovery**: Peak memory reset

#### Level 4: EMERGENCY (>95% usage)
- **Action**: Model offloading to CPU
- **Impact**: High (1-2s, slower transcription on CPU)
- **Recovery**: Prevents kernel crash

### 3. CUDA Error Recovery

```python
# Automatic retry with fallback
try:
    hypothesis = model.transcribe(audio)  # Try GPU
except RuntimeError as e:
    if "out of memory" in str(e):
        model = model.cpu()              # Fallback to CPU
        hypothesis = model.transcribe(audio)  # Retry
```

**Error Tolerance**: 3 CUDA errors before permanent CPU fallback

### 4. Model Offloading

**GPU → CPU (Emergency)**:
- VAD model: 2.2 MB → CPU (maintains accuracy)
- STT model: 1.5 GB → CPU (slower but stable)

**CPU → GPU (Recovery)**:
- Automatic restoration when memory drops below 60%
- Gradual model migration
- No transcription interruption

## Architecture

### Memory Manager (`src/memory_manager.py`)

```python
class MemoryManager:
    def __init__(
        self,
        check_interval=2.0,        # Check every 2 seconds
        warning_threshold=0.80,    # 80% triggers warnings
        critical_threshold=0.90,   # 90% triggers cleanup
        emergency_threshold=0.95,  # 95% triggers offload
        callbacks={}               # Custom callbacks per level
    ):
```

**Key Methods**:
- `start_monitoring()` - Start background thread
- `get_memory_status()` - Current GPU state
- `handle_cuda_error(error)` - Automatic recovery
- `register_model(name, model)` - Track models for offload

### Integration with Daemon

**Initialization**:
```python
# In SwictationDaemon.__init__()
self.memory_manager = MemoryManager(
    check_interval=2.0,
    callbacks={
        'warning': memory_warning,
        'critical': memory_critical,
        'emergency': memory_emergency,
        'emergency_shutdown': emergency_shutdown
    }
)

# Register models for management
memory_manager.register_model('vad_model', vad_model)
memory_manager.register_model('stt_model', stt_model)
```

**Runtime Protection**:
```python
# VAD with error recovery
def _detect_speech_vad(self, audio):
    try:
        audio_tensor = audio_tensor.cuda()
    except RuntimeError as e:
        if not memory_manager.handle_cuda_error(e):
            # Permanent CPU fallback
            vad_model = vad_model.cpu()
            audio_tensor = audio_tensor.cpu()
```

## Performance Impact

### Memory Overhead
- **Monitoring thread**: <1 MB RAM
- **Check frequency**: Every 2 seconds
- **CPU overhead**: <0.1%

### Response Times
- **Warning → Cleanup**: <10ms
- **Critical → Aggressive cleanup**: ~50ms
- **Emergency → Offload**: 1-2 seconds (one-time)

### Accuracy Preservation
- **GPU mode**: Full accuracy (maintained)
- **CPU fallback**: Same accuracy, slower inference
- **VAD on CPU**: No impact on speech detection

## Configuration

### Custom Thresholds

```python
memory_manager = MemoryManager(
    warning_threshold=0.75,    # Earlier warnings
    critical_threshold=0.85,   # Earlier intervention
    emergency_threshold=0.90,  # Earlier offload (more conservative)
)
```

### Custom Callbacks

```python
def custom_warning(status):
    logger.warning(f"GPU at {status.usage_percent*100:.1f}%")
    send_alert_to_monitoring()

callbacks = {
    'warning': custom_warning,
    'critical': trigger_cleanup_jobs,
    'emergency': notify_admin,
}
```

## Monitoring

### Real-time Status

```python
# Get current memory state
status = memory_manager.get_memory_status()

print(f"GPU Memory: {status.allocated_mb:.0f}/{status.total_mb:.0f} MB")
print(f"Usage: {status.usage_percent*100:.1f}%")
print(f"Pressure: {status.pressure_level.value}")
```

### Status Report

```bash
# Daemon logs status every 5 minutes
============================================================
🧠 Memory Manager Status
============================================================
Pressure Level: NORMAL
GPU Memory: 1234.5 / 8000.0 MB (15.4%)
Free: 6765.5 MB
Reserved: 1300.0 MB

Models on GPU: 2
Models offloaded: 0
CUDA errors: 0/3
============================================================
```

## Testing

### Unit Tests (`tests/test_memory_protection.py`)

```bash
pytest tests/test_memory_protection.py -v
```

**Coverage**:
- ✅ Pressure level detection
- ✅ Warning/critical/emergency handling
- ✅ Model offloading/restoration
- ✅ CUDA error recovery
- ✅ Integration scenarios

### Manual Testing

```python
# Simulate memory pressure
import torch

# Allocate until warning
tensors = []
while True:
    tensors.append(torch.randn(1024, 1024, device='cuda'))
    # Watch for warning callbacks
```

## Troubleshooting

### Issue: Frequent Emergency Offloads

**Cause**: Insufficient GPU memory for workload
**Solutions**:
1. Lower `emergency_threshold` to 0.90 (offload earlier)
2. Reduce VAD context window (`total_buffer=5.0`)
3. Use smaller batch sizes
4. Upgrade GPU memory

### Issue: Models Stuck on CPU

**Cause**: Memory never drops below restoration threshold (60%)
**Solutions**:
1. Manually restore: `memory_manager._restore_models_to_gpu()`
2. Lower restoration threshold in code
3. Clear other GPU processes

### Issue: CUDA Errors After Restart

**Cause**: Previous crash left GPU in bad state
**Solutions**:
```bash
# Reset CUDA context
sudo nvidia-smi --gpu-reset

# Clear PyTorch cache
python3 -c "import torch; torch.cuda.empty_cache()"
```

## Best Practices

1. **Monitor logs**: Watch for warning/critical alerts
2. **Adjust thresholds**: Tune based on your GPU size
3. **Test limits**: Run stress tests before production
4. **Plan capacity**: Leave 20% headroom for spikes
5. **Update regularly**: Memory patterns may change with model updates

## Future Enhancements

- [ ] Multi-GPU support (distribute models)
- [ ] Predictive offloading (ML-based forecasting)
- [ ] Dynamic batch sizing (reduce on pressure)
- [ ] Memory pooling (reuse allocations)
- [ ] Quantization (int8 for lower memory)

## References

- **Implementation**: `src/memory_manager.py`
- **Integration**: `src/swictationd.py`
- **Tests**: `tests/test_memory_protection.py`
- **Performance**: `src/performance_monitor.py`
