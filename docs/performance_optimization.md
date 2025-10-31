# Performance Optimization and Monitoring

This document describes the performance optimization features and monitoring capabilities in Swictation.

## Overview

Swictation includes comprehensive performance monitoring to ensure efficient operation without memory leaks or performance degradation. The system tracks:

- **GPU Memory Usage**: Monitor CUDA memory allocation and prevent OOM
- **CPU Usage**: Track CPU utilization to prevent blocking
- **Latency**: Measure end-to-end transcription latency
- **Memory Leaks**: Detect sustained memory growth
- **Resource Bottlenecks**: Identify performance constraints

## Performance Targets

| Metric | Target | Warning Threshold |
|--------|--------|-------------------|
| GPU Memory | < 4GB | 4000 MB |
| CPU Usage (mean) | < 50% | 50% |
| Latency (P95) | < 2s | 2000 ms |
| Memory Growth | Stable | 1.0 MB/s sustained |

## Performance Monitoring

### Automatic Monitoring

The daemon automatically monitors performance when `enable_performance_monitoring=True` (default):

```python
daemon = SwictationDaemon(enable_performance_monitoring=True)
```

Features:
- **Background Monitoring**: Samples metrics every 5 seconds
- **Automatic Warnings**: Alerts when thresholds exceeded
- **Periodic Reports**: Status reports every 5 minutes
- **Final Summary**: Performance summary on shutdown

### Manual Monitoring

For custom monitoring:

```python
from performance_monitor import PerformanceMonitor

monitor = PerformanceMonitor(history_size=1000)
monitor.start_background_monitoring(interval=1.0)

# Capture snapshot
metrics = monitor.capture_metrics({'custom_metric': 123})

# Check GPU
gpu_stats = monitor.get_gpu_memory_stats()
print(f"GPU Memory: {gpu_stats['current_mb']:.1f} MB")

# Check CPU
cpu_stats = monitor.get_cpu_stats(window_seconds=60)
print(f"CPU Mean: {cpu_stats['mean']:.1f}%")

monitor.stop_background_monitoring()
```

## Latency Measurement

### End-to-End Latency

The daemon automatically measures chunk processing latency with phase breakdown:

**Phases:**
1. **Audio Capture**: Time to capture audio chunk
2. **VAD**: Voice Activity Detection
3. **STT**: Speech-to-Text transcription
4. **Injection**: Text injection to active window

**Example Output:**
```
⏱️  Chunk Processing Latency:
  Mean: 523.4ms
  P95: 687.2ms
  Count: 150
```

### Custom Latency Measurement

```python
# Start measurement
measurement = monitor.start_latency_measurement('my_operation')

# ... do work ...
monitor.measure_phase(measurement, 'phase1')

# ... more work ...
monitor.measure_phase(measurement, 'phase2')

# Complete
monitor.end_latency_measurement('my_operation')

# Get stats
stats = monitor.get_latency_stats('my_operation')
print(f"Mean: {stats['mean']:.1f}ms")
print(f"P95: {stats['p95']:.1f}ms")
```

## Memory Leak Detection

### Automatic Detection

The daemon automatically checks for memory leaks every 5 minutes:

```
💾 Memory:
  Growth rate: 0.0023 MB/s
  ✅ No leak detected
```

If a leak is detected:
```
💾 Memory:
  Growth rate: 1.2345 MB/s
  ⚠️  LEAK DETECTED!
```

### Manual Leak Detection

```python
# Analyze last 60 seconds
leak_result = monitor.detect_memory_leak(window_seconds=60)

if leak_result['leak_detected']:
    print(f"Leak! Growth: {leak_result['growth_rate_mb_s']:.4f} MB/s")
    print(f"Projected hourly: {leak_result['growth_rate_mb_s'] * 3600:.1f} MB/hour")
```

**Detection Method:**
- Linear regression on process memory over time window
- Leak flagged if slope > 1.0 MB/s sustained
- Accounts for normal fluctuations

## GPU Memory Management

### Automatic Cache Clearing

The daemon automatically clears GPU cache between chunks in batch mode:

```python
if torch.cuda.is_available():
    torch.cuda.empty_cache()
    gc.collect()
```

### Monitoring GPU Usage

```python
gpu_stats = monitor.get_gpu_memory_stats()

if gpu_stats['available']:
    print(f"Current: {gpu_stats['current_mb']:.1f} MB")
    print(f"Peak: {gpu_stats['peak_mb']:.1f} MB")
    print(f"Reserved: {gpu_stats['reserved_mb']:.1f} MB")
```

### OOM Recovery

The daemon implements graceful OOM handling:

1. Catch `torch.cuda.OutOfMemoryError`
2. Clear cache and retry with smaller chunk
3. Log error and continue with next chunk
4. Maintain daemon stability

## CPU Optimization

### Non-Blocking Architecture

- **Audio Capture**: Separate thread with callback
- **Chunk Processing**: Background thread pool
- **IPC Server**: Dedicated thread with timeout
- **Main Loop**: Minimal overhead

### Thread Management

```python
# Streaming chunk processing (non-blocking)
self._streaming_thread = threading.Thread(
    target=self._process_streaming_chunk,
    args=(chunk.copy(),),
    daemon=True
)
self._streaming_thread.start()
```

## Testing Performance

### Memory Leak Test

Run for 10 minutes to detect memory leaks:

```bash
cd tests
python test_memory_leaks.py --duration 10
```

Options:
- `--duration N`: Test duration in minutes (default: 10)
- `--interval SECS`: Sampling interval (default: 1.0s)
- `--simulate`: Quick simulation test (1 minute)

**Output:**
```
🔍 Memory Leak Analysis (5-minute window):
  Leak detected: False
  Growth rate: 0.0012 MB/s
  Projected hourly: 4.3 MB/hour
  ✅ Growth rate within acceptable range
```

### Performance Profiling

Profile streaming performance:

```bash
cd tests
python test_performance_profiling.py --chunks 100
```

Options:
- `--chunks N`: Number of chunks to process (default: 100)
- `--chunk-size MS`: Chunk size in ms (default: 400)
- `--test-measurement`: Test latency measurement only

**Output:**
```
⏱️  End-to-End Latency:
  Mean: 523.4ms
  P95: 687.2ms
  ✅ P95 latency within target (<2000ms)

🔍 Phase Breakdown:
  audio_capture    :   400.00ms ( 76.5%)
  vad              :     2.00ms (  0.4%)
  stt              :   100.00ms ( 19.1%)
  injection        :     1.00ms (  0.2%)

🎯 Bottleneck: audio_capture (400.00ms)
```

## Optimization Tips

### Reduce GPU Memory

1. **Use Chunking**: Process long audio in 10s chunks
2. **Clear Cache**: Call `torch.cuda.empty_cache()` between chunks
3. **Reduce Batch Size**: Use `batch_size=1` for streaming
4. **Model Quantization**: Consider int8/fp16 quantization

### Reduce Latency

1. **Smaller Chunks**: Use 0.4s chunks for streaming
2. **GPU Acceleration**: Ensure CUDA is available
3. **Optimize VAD**: Skip VAD for continuous speech
4. **Fast Injection**: Use wtype for minimal overhead

### Reduce CPU Usage

1. **Background Processing**: Use threading for non-blocking
2. **Efficient Callbacks**: Keep audio callbacks minimal
3. **Batch Operations**: Process multiple chunks together
4. **Sleep Intervals**: Add small delays in busy loops

### Prevent Memory Leaks

1. **Clean Temp Files**: Delete audio files after processing
2. **Clear Buffers**: Reset buffers after recordings
3. **Limit History**: Cap metrics history size (default: 1000)
4. **Close Resources**: Use context managers for cleanup

## Performance Metrics API

### PerformanceMetrics

```python
@dataclass
class PerformanceMetrics:
    timestamp: float
    gpu_memory_allocated: float  # MB
    gpu_memory_reserved: float   # MB
    gpu_memory_peak: float       # MB
    cpu_percent: float
    ram_used: float              # MB
    ram_percent: float
    process_memory: float        # MB
    process_cpu: float
    num_threads: int
    custom: Dict[str, float]
```

### LatencyMeasurement

```python
@dataclass
class LatencyMeasurement:
    name: str
    start_time: float
    end_time: Optional[float]
    phase_times: Dict[str, float]

    @property
    def total_ms(self) -> float
```

## Warning Callbacks

Register custom warning handlers:

```python
def handle_high_gpu(message: str):
    # Send alert, log, etc.
    logging.warning(f"GPU: {message}")

def handle_leak(message: str):
    # Critical alert
    logging.critical(f"LEAK: {message}")

callbacks = {
    'high_gpu_memory': handle_high_gpu,
    'memory_leak': handle_leak,
    'high_cpu': lambda msg: print(f"CPU: {msg}"),
    'high_latency': lambda msg: print(f"Latency: {msg}"),
}

monitor = PerformanceMonitor(warning_callbacks=callbacks)
```

## Troubleshooting

### High GPU Memory

**Symptoms:**
- GPU OOM errors
- `gpu_memory_allocated > 4000 MB`

**Solutions:**
1. Reduce chunk duration: `chunk_duration=5.0`
2. Enable cache clearing in batch mode
3. Check for memory leaks with test suite
4. Monitor with: `monitor.get_gpu_memory_stats()`

### High CPU Usage

**Symptoms:**
- `cpu_percent > 50%`
- Audio capture lagging
- System unresponsive

**Solutions:**
1. Increase streaming chunk size: `streaming_chunk_size=1.0`
2. Reduce monitoring frequency: `interval=10.0`
3. Check thread count: `metrics.num_threads`
4. Profile with: `monitor.get_cpu_stats()`

### High Latency

**Symptoms:**
- `latency_p95 > 2000ms`
- Delayed transcription
- Text injection lag

**Solutions:**
1. Check bottleneck with phase breakdown
2. Ensure GPU acceleration enabled
3. Reduce audio capture buffer: `buffer_duration=10.0`
4. Profile with: `test_performance_profiling.py`

### Memory Leak

**Symptoms:**
- `growth_rate_mb_s > 1.0`
- Increasing RAM over time
- Process killed by OS

**Solutions:**
1. Run leak test: `test_memory_leaks.py`
2. Check temp file cleanup in `/tmp/swictation/`
3. Verify buffer clearing: `_streaming_buffer = []`
4. Monitor with: `monitor.detect_memory_leak()`

## Best Practices

1. **Always Enable Monitoring**: Use `enable_performance_monitoring=True`
2. **Regular Testing**: Run leak tests after code changes
3. **Set Thresholds**: Adjust thresholds based on hardware
4. **Monitor Production**: Keep monitoring enabled in production
5. **Log Metrics**: Persist metrics for trend analysis
6. **Alert on Warnings**: Set up alerts for performance warnings
7. **Profile Regularly**: Use profiling tests to catch regressions
8. **Clean Resources**: Always clean up temp files and buffers
9. **Test Long-Running**: Test 10+ minute sessions for leaks
10. **Document Changes**: Update performance docs with optimizations
