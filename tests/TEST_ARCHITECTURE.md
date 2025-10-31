# OOM Prevention Test Architecture

## 🏗️ Test Suite Structure

```
tests/
├── test_memory_protection.py      # Memory protection mechanisms
│   ├── TestMemoryStress           # Stress testing
│   ├── TestGPUFallback           # GPU→CPU fallback
│   ├── TestCUDAErrorRecovery     # CUDA error handling
│   ├── TestEmergencyShutdown     # Emergency shutdown
│   └── TestMemoryStressHarness   # Extended stress testing
│
├── test_oom_recovery.py           # OOM recovery mechanisms
│   ├── TestOOMDetection          # OOM detection
│   ├── TestGracefulDegradation   # Graceful degradation
│   ├── TestDataPreservation      # Data preservation
│   ├── TestMultipleOOMHandling   # Multiple OOM events
│   └── TestRecoveryMetrics       # Recovery tracking
│
├── pytest_memory.ini              # Pytest configuration
├── RUN_MEMORY_TESTS.md           # Complete documentation
├── TEST_SUITE_SUMMARY.md         # Summary report
├── TEST_ARCHITECTURE.md          # This file
└── validate_test_suite.py        # Validation script
```

## 🔄 Test Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    OOM Prevention Testing                    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────┐
        │   Memory Stress Tests (60s-1h)    │
        └───────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                ▼                       ▼
    ┌──────────────────┐    ┌──────────────────┐
    │  Normal Operation │    │  Memory Pressure  │
    │   (<75% usage)    │    │   (75-95% usage)  │
    └──────────────────┘    └──────────────────┘
                                        │
                ┌───────────────────────┼───────────────────────┐
                ▼                       ▼                       ▼
        ┌──────────┐          ┌──────────────┐       ┌──────────────┐
        │ WARNING  │          │   CRITICAL   │       │  EMERGENCY   │
        │ (75-85%) │          │  (85-95%)    │       │   (>95%)     │
        └──────────┘          └──────────────┘       └──────────────┘
             │                        │                       │
             ▼                        ▼                       ▼
        ┌──────────┐          ┌──────────────┐       ┌──────────────┐
        │ GC + Cache│         │ Aggressive   │       │Model Offload │
        │   Clear   │         │   Cleanup    │       │   to CPU     │
        └──────────┘          └──────────────┘       └──────────────┘
                                                              │
                                    ┌─────────────────────────┘
                                    ▼
                            ┌───────────────┐
                            │  OOM Detected │
                            └───────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │  Strategy 1  │ │  Strategy 2  │ │  Strategy 3  │
            │ Cache Clear  │ │ GC Collect   │ │Reduce Buffer │
            └─────────────┘ └─────────────┘ └─────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    ▼
                            ┌───────────────┐
                            │   Recovery?   │
                            └───────────────┘
                            ┌───┴───┐
                            ▼       ▼
                        ┌─────┐  ┌──────────────┐
                        │ YES │  │  NO - Try     │
                        │     │  │  Strategy 4+5 │
                        └─────┘  └──────────────┘
                            │           │
                            ▼           ▼
                    ┌──────────┐  ┌────────────┐
                    │ Continue │  │ CPU Fallback│
                    │   GPU    │  │   or Exit   │
                    └──────────┘  └────────────┘
```

## 🧪 Test Scenario Matrix

| Scenario | Duration | Memory Load | GPU | Pass Criteria |
|----------|----------|-------------|-----|---------------|
| **Sustained Load** | 60s | 500MB | ✓ | <1 MB/s growth |
| **Allocation Stress** | 30s | 50MB×100 | ✓ | No leaks |
| **1-Hour Recording** | 1h | Variable | ✓ | <100MB total growth |
| **GPU OOM** | 10s | Max+100MB | ✓ | Graceful fallback |
| **Rapid Toggle** | 20s | 100MB×50 | ✓ | Clean resources |
| **CUDA Error** | 5s | Error inject | ✓ | Recovery success |
| **Repeated CUDA** | 15s | 3× errors | ✓ | GPU operational |
| **Emergency Shutdown** | 5s | >95% | ✓ | Data preserved |
| **OOM Detection** | 5s | Impossible alloc | ✓ | Error caught |
| **Progressive Recovery** | 10s | 5 strategies | ✓ | Success within 5 |
| **Buffer Reduction** | 15s | 4× pressure | ✓ | Buffer scaled down |
| **CPU Fallback** | 10s | 3× OOM | ✓ | CPU mode active |
| **Audio Preservation** | 5s | OOM + 10s audio | ✓ | No sample loss |
| **State Preservation** | 5s | OOM + partial text | ✓ | State saved |
| **Consecutive OOM** | 20s | 5× OOM | ✓ | All handled |
| **Nested OOM** | 15s | OOM in recovery | ✓ | No infinite loop |

## 🎯 Test Categories

### Category 1: Memory Leak Detection
**Purpose:** Ensure no memory leaks during normal operation

**Tests:**
- `test_sustained_high_memory_usage` - 60s constant load
- `test_continuous_recording_memory_leak` - 1-hour test

**Methodology:**
1. Establish baseline memory usage
2. Run workload for duration
3. Sample memory every 1s
4. Calculate growth rate via linear regression
5. Assert growth rate <1 MB/s

**Pass Criteria:** Growth rate <1 MB/s AND total growth <100MB

### Category 2: Allocation Stress
**Purpose:** Validate cleanup in allocation/deallocation cycles

**Tests:**
- `test_memory_allocation_stress` - 100 cycles

**Methodology:**
1. Record baseline GPU memory
2. Allocate 50MB
3. Deallocate
4. Repeat 100 times
5. Check final memory vs baseline

**Pass Criteria:** Final memory within 10MB of baseline

### Category 3: GPU Fallback
**Purpose:** Test graceful GPU→CPU transition on OOM

**Tests:**
- `test_gpu_to_cpu_fallback_on_oom` - Single OOM
- `test_rapid_toggle_with_fallback` - 50 toggles

**Methodology:**
1. Trigger GPU OOM (impossible allocation)
2. Catch RuntimeError
3. Fall back to CPU
4. Verify processing continues
5. Check user notification

**Pass Criteria:** No crash, CPU mode active, user notified

### Category 4: CUDA Error Recovery
**Purpose:** Validate recovery from CUDA errors

**Tests:**
- `test_cuda_error_recovery` - Single error
- `test_repeated_cuda_errors` - 3 consecutive errors

**Methodology:**
1. Trigger CUDA error
2. Clear cache
3. Attempt small test allocation
4. Verify GPU still operational

**Pass Criteria:** GPU operational after recovery

### Category 5: Emergency Shutdown
**Purpose:** Test emergency procedures under critical memory

**Tests:**
- `test_emergency_shutdown_trigger` - Threshold trigger
- `test_data_preservation_on_shutdown` - Data save

**Methodology:**
1. Simulate >95% memory usage
2. Trigger emergency handler
3. Save all buffers to disk
4. Clean shutdown

**Pass Criteria:** All data saved, clean exit

### Category 6: OOM Recovery
**Purpose:** Test OOM detection and recovery strategies

**Tests:**
- `test_oom_detection` - Detection accuracy
- `test_progressive_recovery_strategies` - 5 strategies
- `test_consecutive_oom_recovery` - 5 OOMs

**Methodology:**
1. Detect OOM condition
2. Try recovery strategies in order:
   - Cache clear
   - Garbage collect
   - Reduce buffer
   - CPU fallback
   - Emergency shutdown
3. Verify system returns to operational state

**Pass Criteria:** Recovery within 5 strategies

### Category 7: Data Preservation
**Purpose:** Ensure no data loss during OOM

**Tests:**
- `test_audio_buffer_preservation` - Audio samples
- `test_transcription_state_preservation` - Partial text

**Methodology:**
1. Simulate OOM during processing
2. Emergency save all buffers
3. Verify bit-perfect preservation
4. Test recovery from saved state

**Pass Criteria:** Zero data loss

## 🔧 Test Fixtures

### MemoryPressureSimulator
**Purpose:** Simulate memory pressure conditions

**Methods:**
- `allocate_gpu_memory(mb)` - Allocate GPU memory
- `allocate_cpu_memory(mb)` - Allocate CPU memory
- `release_all()` - Free all allocated memory

### OOMRecoveryManager
**Purpose:** Manage OOM recovery testing

**Methods:**
- `trigger_oom()` - Trigger OOM condition
- `attempt_recovery(strategy)` - Try recovery strategy
- `verify_recovery()` - Check if system operational

### PerformanceMonitor (from src/)
**Purpose:** Monitor system performance

**Methods:**
- `capture_metrics()` - Snapshot current metrics
- `detect_memory_leak()` - Analyze memory growth
- `get_gpu_memory_stats()` - GPU memory info

## 📊 Test Metrics

### Memory Growth Rate
```
Growth Rate = (Final Memory - Initial Memory) / Duration

Acceptable: <1 MB/s
Warning: 1-5 MB/s
Critical: >5 MB/s
```

### Recovery Success Rate
```
Success Rate = Successful Recoveries / Total OOM Events

Target: >90%
Minimum: >75%
```

### Data Preservation Rate
```
Preservation = Saved Samples / Total Samples

Required: 100% (bit-perfect)
```

## 🚀 Execution Modes

### 1. Quick Validation (5 min)
```bash
pytest tests/ -v -m "not slow"
```
**Runs:** 16/19 tests (excludes 1-hour test)
**Use:** Development, pre-commit

### 2. Full Suite (2 hours)
```bash
pytest tests/ -v
```
**Runs:** All 19 tests
**Use:** Pre-release, nightly CI

### 3. Stress Harness (Custom)
```bash
python tests/test_memory_protection.py --duration 4.0
```
**Runs:** Extended stress test
**Use:** Soak testing, stability validation

### 4. Category-Specific
```bash
pytest tests/ -v -k "memory or oom"
```
**Runs:** Filtered by keyword
**Use:** Debugging specific issues

## 🔍 Test Markers

```python
@pytest.mark.slow        # Long-running tests (>1 hour)
@pytest.mark.gpu         # Requires GPU
@pytest.mark.stress      # Stress tests
@pytest.mark.oom         # OOM-specific tests
@pytest.mark.recovery    # Recovery mechanism tests
```

## 📈 Success Metrics

### Code Coverage
- **Target:** >80% line coverage
- **Critical paths:** 100% coverage
- **Error handlers:** 100% coverage

### Test Reliability
- **Target:** 0% flaky tests
- **Repeatability:** 100% deterministic results

### Performance
- **Quick tests:** <5 minutes total
- **Full suite:** <2 hours total
- **No false positives:** 100%

## 🔗 Integration Points

### With MemoryManager (src/memory_manager.py)
```python
# Tests validate:
- Memory status detection
- Pressure level thresholds
- Callback invocation
- Model offloading
- CUDA error handling
```

### With SwictationDaemon (src/swictationd.py)
```python
# Tests validate:
- Real recording sessions
- STT model memory usage
- Audio buffer management
- Streaming mode stability
```

### With PerformanceMonitor (src/performance_monitor.py)
```python
# Tests validate:
- Memory leak detection
- GPU stats accuracy
- Latency measurements
```

## 🎓 Best Practices

### Writing New Tests
1. **Isolate:** Each test independent
2. **Clean up:** Always release resources
3. **Document:** Clear pass/fail criteria
4. **Realistic:** Mirror actual usage
5. **Deterministic:** No random behavior

### Debugging Failures
1. Run with `-v -s` for verbose output
2. Check GPU with `nvidia-smi`
3. Monitor system with `htop`
4. Review logs in `--log-cli-level=DEBUG`
5. Isolate failing test with `-k`

### Maintaining Tests
1. Update thresholds based on hardware
2. Adjust timings for CI environment
3. Keep documentation in sync
4. Review flaky tests monthly
5. Update pass criteria as needed

---

**Test Architecture Complete** ✅
**Ready for Implementation Testing** 🚀
