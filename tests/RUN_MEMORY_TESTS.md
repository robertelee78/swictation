# Memory & OOM Prevention Test Suite

Comprehensive test suite for validating OOM (Out-Of-Memory) prevention mechanisms in Swictation.

## 📋 Test Overview

### Test Files

1. **`test_memory_protection.py`** - Memory protection mechanisms
   - Memory stress tests
   - GPU/CPU fallback transitions
   - CUDA error recovery
   - Emergency shutdown triggers
   - Data preservation

2. **`test_oom_recovery.py`** - OOM recovery mechanisms
   - OOM detection
   - Graceful degradation
   - Data preservation during OOM
   - Multiple OOM handling
   - Recovery metrics

## 🚀 Quick Start

### Run All Memory Tests

```bash
# All tests (includes slow tests)
pytest tests/test_memory_protection.py tests/test_oom_recovery.py -v

# Quick tests only (skip 1-hour tests)
pytest tests/test_memory_protection.py tests/test_oom_recovery.py -v -m "not slow"
```

### Run Specific Test Categories

```bash
# Memory stress tests
pytest tests/test_memory_protection.py::TestMemoryStress -v

# GPU fallback tests
pytest tests/test_memory_protection.py::TestGPUFallback -v

# OOM detection tests
pytest tests/test_oom_recovery.py::TestOOMDetection -v

# Recovery mechanism tests
pytest tests/test_oom_recovery.py::TestGracefulDegradation -v
```

### Run Individual Tests

```bash
# Sustained memory usage test (60 seconds)
pytest tests/test_memory_protection.py::TestMemoryStress::test_sustained_high_memory_usage -v

# GPU→CPU fallback test
pytest tests/test_memory_protection.py::TestGPUFallback::test_gpu_to_cpu_fallback_on_oom -v

# OOM detection test
pytest tests/test_oom_recovery.py::TestOOMDetection::test_oom_detection -v

# Data preservation test
pytest tests/test_oom_recovery.py::TestDataPreservation::test_audio_buffer_preservation -v
```

## ⏱️ Test Durations

| Test | Duration | Marker |
|------|----------|--------|
| Sustained high memory usage | 60 seconds | - |
| Memory allocation stress | ~30 seconds | - |
| **1-hour continuous recording** | **1 hour** | `@pytest.mark.slow` |
| GPU→CPU fallback | ~10 seconds | - |
| Rapid toggle cycles | ~20 seconds | - |
| CUDA error recovery | ~5 seconds | - |
| OOM detection | ~5 seconds | - |
| Data preservation | ~5 seconds | - |

**Total runtime (all tests):** ~2 hours
**Quick tests only:** ~5 minutes

## 🎯 Test Scenarios

### 1. Memory Stress Tests

#### Sustained High Memory Usage (60s)
```bash
pytest tests/test_memory_protection.py::TestMemoryStress::test_sustained_high_memory_usage -v
```
**Tests:** Memory leak detection under sustained 500MB load
**Pass criteria:**
- Memory growth rate < 1 MB/s
- No leak detected after 60s
- Stable memory within ±10%

#### Allocation/Deallocation Stress (100 cycles)
```bash
pytest tests/test_memory_protection.py::TestMemoryStress::test_memory_allocation_stress -v
```
**Tests:** Rapid allocation/deallocation cycles (50MB × 100)
**Pass criteria:**
- All allocations complete
- Memory returns to baseline
- No fragmentation

#### 1-Hour Continuous Recording ⚠️ SLOW
```bash
pytest tests/test_memory_protection.py::TestMemoryStress::test_continuous_recording_memory_leak -v
```
**Tests:** Long-duration memory leak detection
**Pass criteria:**
- No leak detected
- Total growth < 100 MB
- No crash or OOM

### 2. GPU→CPU Fallback Tests

#### OOM Fallback Transition
```bash
pytest tests/test_memory_protection.py::TestGPUFallback::test_gpu_to_cpu_fallback_on_oom -v
```
**Tests:** Graceful fallback when GPU OOMs
**Pass criteria:**
- OOM detected correctly
- CPU fallback succeeds
- Processing continues
- User notified

#### Rapid Toggle with Fallback (50 cycles)
```bash
pytest tests/test_memory_protection.py::TestGPUFallback::test_rapid_toggle_with_fallback -v
```
**Tests:** Recording toggles with GPU/CPU transitions
**Pass criteria:**
- All toggles complete
- Memory cleaned between toggles
- No resource leaks

### 3. CUDA Error Recovery

#### CUDA Error Recovery
```bash
pytest tests/test_memory_protection.py::TestCUDAErrorRecovery::test_cuda_error_recovery -v
```
**Tests:** Recovery from CUDA errors
**Pass criteria:**
- Error detected
- Recovery attempted
- GPU returns to operational state

#### Repeated CUDA Errors (3 errors)
```bash
pytest tests/test_memory_protection.py::TestCUDAErrorRecovery::test_repeated_cuda_errors -v
```
**Tests:** Multiple CUDA error handling
**Pass criteria:**
- All errors handled
- GPU remains available
- No permanent damage

### 4. Emergency Shutdown Tests

#### Shutdown Trigger
```bash
pytest tests/test_memory_protection.py::TestEmergencyShutdown::test_emergency_shutdown_trigger -v
```
**Tests:** Emergency shutdown on critical memory
**Pass criteria:**
- Shutdown triggered at threshold
- Resources cleaned up
- Clean process exit

#### Data Preservation on Shutdown
```bash
pytest tests/test_memory_protection.py::TestEmergencyShutdown::test_data_preservation_on_shutdown -v
```
**Tests:** Data saving during emergency shutdown
**Pass criteria:**
- All buffered audio saved
- No sample loss
- Recovery possible

### 5. OOM Recovery Tests

#### OOM Detection
```bash
pytest tests/test_oom_recovery.py::TestOOMDetection::test_oom_detection -v
```
**Tests:** OOM condition detection
**Pass criteria:**
- OOM detected on impossible allocation
- Appropriate error raised
- Counter incremented

#### Progressive Recovery Strategies
```bash
pytest tests/test_oom_recovery.py::TestGracefulDegradation::test_progressive_recovery_strategies -v
```
**Tests:** Recovery strategy progression
**Pass criteria:**
- Strategies tried in order
- Recovery succeeds
- System operational

#### Audio Buffer Preservation
```bash
pytest tests/test_oom_recovery.py::TestDataPreservation::test_audio_buffer_preservation -v
```
**Tests:** Audio data preservation during OOM
**Pass criteria:**
- All samples saved
- No data loss
- Audio quality maintained

#### Consecutive OOM Recovery (5 OOMs)
```bash
pytest tests/test_oom_recovery.py::TestMultipleOOMHandling::test_consecutive_oom_recovery -v
```
**Tests:** Multiple OOM event handling
**Pass criteria:**
- All OOMs detected
- Recovery attempted for each
- Eventually falls back to CPU

## 🧪 Memory Stress Harness

For extended stress testing beyond pytest:

### Run Custom Stress Test

```bash
# 1-hour stress test
python tests/test_memory_protection.py --duration 1.0

# 4-hour stress test with custom pressure
python tests/test_memory_protection.py \
    --duration 4.0 \
    --gpu-pressure 2000 \
    --cpu-pressure 1000 \
    --toggle-interval 60.0
```

### Parameters

- `--duration`: Test duration in hours (default: 1.0)
- `--gpu-pressure`: GPU memory pressure in MB (default: 1000)
- `--cpu-pressure`: CPU memory pressure in MB (default: 500)
- `--toggle-interval`: Recording toggle interval in seconds (default: 30.0)

## 📊 Pass/Fail Criteria

### Memory Leak Detection

✅ **PASS:** Memory growth rate < 1 MB/s
❌ **FAIL:** Sustained growth > 1 MB/s or total growth > 100 MB

### GPU Fallback

✅ **PASS:** Graceful transition to CPU, no crash, user notified
❌ **FAIL:** Crash on OOM, or fallback fails

### Data Preservation

✅ **PASS:** All data saved, bit-perfect recovery
❌ **FAIL:** Any data loss or corruption

### Recovery Success

✅ **PASS:** System returns to operational state
❌ **FAIL:** Permanent failure or hung state

### CUDA Error Recovery

✅ **PASS:** GPU operational after recovery
❌ **FAIL:** GPU permanently damaged or unavailable

## 🔧 Test Configuration

Tests use `pytest_memory.ini` for configuration:

- **Timeout:** 2 hours per test
- **Markers:** slow, gpu, stress, oom, recovery
- **Logging:** INFO level with timestamps

## 📈 Expected Output

### Successful Test Run

```
==================== test session starts ====================
tests/test_memory_protection.py::TestMemoryStress::test_sustained_high_memory_usage PASSED
tests/test_memory_protection.py::TestGPUFallback::test_gpu_to_cpu_fallback_on_oom PASSED
tests/test_oom_recovery.py::TestOOMDetection::test_oom_detection PASSED
...
==================== X passed in Y.YYs ====================
```

### Failed Test (Memory Leak)

```
tests/test_memory_protection.py::TestMemoryStress::test_sustained_high_memory_usage FAILED

📊 Results:
  Leak detected: True
  Growth rate: 1.234 MB/s
  Total growth: 74.0 MB

AssertionError: Memory leak detected!
```

## 🐛 Debugging Failed Tests

### Enable Verbose Logging

```bash
pytest tests/test_memory_protection.py -v -s --log-cli-level=DEBUG
```

### Check GPU Status

```bash
nvidia-smi
```

### Monitor Memory During Test

```bash
# Terminal 1: Run test
pytest tests/test_memory_protection.py::TestMemoryStress::test_sustained_high_memory_usage -v

# Terminal 2: Monitor GPU
watch -n 1 nvidia-smi
```

## 🚨 Known Issues

1. **GPU-only tests skip on CPU systems:** Expected behavior
2. **1-hour test takes full hour:** Use `-m "not slow"` to skip
3. **OOM tests may trigger system alerts:** Expected, tests trigger OOMs intentionally

## 📝 Test Maintenance

### Adding New Tests

1. Add test to appropriate test class
2. Document pass/fail criteria in docstring
3. Add to this README with command and description
4. Tag with appropriate markers (@pytest.mark.slow, etc.)

### Updating Pass Criteria

Thresholds defined in test files:
- `test_memory_protection.py`: Line references in docstrings
- `test_oom_recovery.py`: Documented in each test

## 🎯 CI/CD Integration

### GitHub Actions Example

```yaml
name: Memory Tests

on: [push, pull_request]

jobs:
  memory-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v2
      - uses: actions/setup-python@v2
        with:
          python-version: '3.9'

      - name: Install dependencies
        run: |
          pip install -r requirements.txt
          pip install pytest pytest-timeout

      - name: Run quick memory tests
        run: pytest tests/test_memory_protection.py tests/test_oom_recovery.py -v -m "not slow"

      # Optional: Run slow tests nightly
      - name: Run full memory tests (nightly)
        if: github.event_name == 'schedule'
        run: pytest tests/test_memory_protection.py tests/test_oom_recovery.py -v
```

## 📚 Related Documentation

- [Architecture: Memory Management](../docs/architecture.md#memory-management)
- [Performance Monitoring](../src/performance_monitor.py)
- [Audio Capture](../src/audio_capture.py)

## ✅ Validation Checklist

Before deploying OOM protection:

- [ ] All quick tests pass (`-m "not slow"`)
- [ ] 1-hour continuous test passes
- [ ] GPU→CPU fallback tested
- [ ] CUDA error recovery verified
- [ ] Emergency shutdown tested
- [ ] Data preservation validated
- [ ] Multiple OOM scenarios handled
- [ ] Recovery metrics tracking works

## 🔗 Hive Mind Coordination

Test results stored in hive memory:
- **Key:** `hive/tests/oom-prevention`
- **Status updates:** Post-edit hooks after each test run
- **Integration:** Tester coordinates with Coder via memory
