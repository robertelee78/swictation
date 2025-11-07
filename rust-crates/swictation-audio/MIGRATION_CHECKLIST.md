# Audio Capture Rust Migration - API Compatibility Checklist

## Source: `/opt/swictation/src/audio_capture.py` (474 lines)

### ✅ = Implemented | ⏳ = In Progress | ❌ = Not Started

---

## 1. AudioCapture Class

### Constructor (`__init__`) - Lines 22-75
- ❌ `sample_rate: int = 16000` - Target sample rate
- ❌ `channels: int = 1` - Number of channels (mono/stereo)
- ❌ `dtype: str = 'float32'` - Audio data type
- ❌ `blocksize: int = 1024` - Samples per callback
- ❌ `device: Optional[int] = None` - Device index or string name
- ❌ `buffer_duration: float = 10.0` - Max buffer duration in seconds
- ❌ `chunk_duration: float = 1.0` - Chunk duration for streaming mode
- ❌ `streaming_mode: bool = False` - Enable streaming with chunk callbacks

### Instance Variables to Track
- ❌ `self.sample_rate`
- ❌ `self.channels`
- ❌ `self.dtype`
- ❌ `self.blocksize`
- ❌ `self.device`
- ❌ `self.max_buffer_samples` - Calculated from buffer_duration
- ✅ `self.buffer` - Circular buffer (deque → ringbuf)
- ❌ `self.buffer_lock` - Thread safety (threading.Lock → parking_lot::Mutex)
- ❌ `self.streaming_mode`
- ❌ `self.chunk_duration`
- ❌ `self.chunk_frames` - Calculated from chunk_duration
- ❌ `self._chunk_buffer` - Accumulator for streaming chunks
- ❌ `self._chunk_buffer_lock`
- ❌ `self.stream` - sounddevice stream handle
- ❌ `self.parec_process` - subprocess.Popen for parec
- ❌ `self.parec_thread` - Thread for reading parec output
- ❌ `self.use_parec` - Flag for backend selection
- ❌ `self.is_recording` - Recording state flag
- ❌ `self.total_frames` - Total frames captured counter
- ❌ `self.on_audio_callback` - Legacy callback hook
- ❌ `self.on_chunk_ready` - Streaming mode callback

---

## 2. Public Methods

### `list_devices()` - Lines 76-104
**Purpose**: Enumerate and display available audio devices

**Requirements**:
- ❌ Query all audio devices (cpal host.devices())
- ❌ Display device index, name, type (INPUT/OUTPUT)
- ❌ Show max input/output channels
- ❌ Show default sample rate
- ❌ Mark default input/output devices
- ❌ Format output matching Python version
- ❌ Return devices list/iterator

**Output Format**:
```
Available Audio Devices:
==============================================================================

  0: Built-in Audio Analog Stereo
     Type: INPUT/OUTPUT [DEFAULT INPUT]
     Channels: IN=2, OUT=2
     Sample Rate: 48000 Hz
```

### `start()` - Lines 201-277
**Purpose**: Start audio capture

**Requirements**:
- ❌ Check if already recording (print warning if true)
- ❌ Print startup info (sample rate, channels, device, blocksize)
- ❌ Print streaming mode info if enabled
- ❌ Clear both buffers (main buffer + chunk buffer)
- ❌ Detect PipeWire monitor sources (device name starts with 'alsa_' or contains '.')
- ❌ **Dual backend support**:
  - ❌ **parec backend**: For PipeWire/PulseAudio string device names
    - ❌ Spawn subprocess: `parec --device X --rate Y --channels Z --format s16le --latency-msec 50`
    - ❌ Start reader thread (`_parec_reader`)
    - ❌ Set `use_parec = True`
  - ❌ **sounddevice/cpal backend**: For regular device indices
    - ❌ Create input stream with callback
    - ❌ Set `use_parec = False`
- ❌ Set `is_recording = True`
- ❌ Print success message with backend name
- ❌ Handle errors (print error, clean up, raise exception)

### `stop()` - Lines 279-320
**Purpose**: Stop audio capture and return buffered audio

**Requirements**:
- ❌ Check if recording (return empty array if not)
- ❌ Print stop message
- ❌ Set `is_recording = False`
- ❌ **parec cleanup**:
  - ❌ Terminate subprocess
  - ❌ Wait with 2-second timeout
  - ❌ Kill if timeout expires
  - ❌ Join reader thread with timeout
- ❌ **sounddevice cleanup**:
  - ❌ Stop stream
  - ❌ Close stream
- ❌ Get buffered audio (thread-safe)
- ❌ Calculate duration
- ❌ Print capture summary (frames, duration)
- ❌ Return numpy array (or Rust Vec<f32> → Python via PyO3)

**Return Type**: `np.ndarray` (dtype=self.dtype)

### `get_buffer()` - Lines 322-325
**Purpose**: Get current buffer contents without stopping

**Requirements**:
- ❌ Thread-safe buffer read
- ❌ Return copy as numpy array
- ❌ Don't modify buffer state

### `get_buffer_duration()` - Lines 327-330
**Purpose**: Get buffer duration in seconds

**Requirements**:
- ❌ Thread-safe buffer length query
- ❌ Calculate: `len(buffer) / sample_rate`
- ❌ Return float

### `is_active()` - Lines 332-334
**Purpose**: Check if recording

**Requirements**:
- ❌ Return `self.is_recording` boolean

### `get_chunk_buffer_size()` - Lines 336-339
**Purpose**: Get streaming chunk buffer size

**Requirements**:
- ❌ Thread-safe chunk buffer length query
- ❌ Return int (number of samples accumulated)

### `get_chunk_buffer_progress()` - Lines 341-346
**Purpose**: Get chunk buffer progress percentage

**Requirements**:
- ❌ Thread-safe chunk buffer query
- ❌ Calculate: `min(len(chunk_buffer) / chunk_frames, 1.0)`
- ❌ Handle division by zero (return 0.0)
- ❌ Return float (0.0 to 1.0)

### Context Manager Protocol
#### `__enter__()` - Lines 348-351
- ❌ Call `start()`
- ❌ Return self

#### `__exit__()` - Lines 353-356
- ❌ Call `stop()` if recording

---

## 3. Private Methods (Internal Implementation)

### `_audio_callback(indata, frames, time_info, status)` - Lines 106-149
**Purpose**: Real-time audio callback (called by sounddevice/cpal)

**Requirements**:
- ❌ Print status if error/warning
- ❌ Convert stereo to mono if needed (np.mean across channels)
- ❌ **Main buffer** (thread-safe):
  - ❌ Extend buffer with audio samples
  - ❌ Increment `total_frames`
- ❌ **Streaming mode** (if enabled):
  - ❌ Extend chunk buffer
  - ❌ Loop while chunk buffer >= chunk_frames:
    - ❌ Extract exactly chunk_frames samples
    - ❌ Remove processed samples from buffer
    - ❌ Call `on_chunk_ready(chunk)` if set
- ❌ Call `on_audio_callback(audio, frames)` if set (legacy)

**Critical**: This runs in real-time audio thread - must be lock-free and fast!

### `_parec_reader()` - Lines 151-199
**Purpose**: Background thread to read from parec subprocess

**Requirements**:
- ❌ Run while `is_recording` and `parec_process` alive
- ❌ Read raw audio data from subprocess stdout
- ❌ Convert bytes → numpy array:
  - ❌ Format: s16le (signed 16-bit little-endian)
  - ❌ Normalize to float32: `audio / 32768.0`
- ❌ Convert stereo to mono if needed
- ❌ **Same buffer logic as `_audio_callback`**:
  - ❌ Extend main buffer
  - ❌ Increment total_frames
  - ❌ Handle streaming mode chunks
  - ❌ Call callbacks
- ❌ Handle exceptions (print error if recording)
- ❌ Exit gracefully on EOF or error

**Thread Safety**: Uses same locks as `_audio_callback`

---

## 4. Module-Level Functions

### `find_loopback_device()` - Lines 359-399
**Purpose**: Find system audio loopback/monitor device for testing

**Requirements**:
- ❌ Try `pactl list short sources` first
- ❌ Parse output for `.monitor` devices
- ❌ Extract device name (second field)
- ❌ Fallback to sounddevice/cpal enumeration
- ❌ Search for keywords: 'monitor', 'loopback', 'what-u-hear', 'stereo mix'
- ❌ Return device name/index or None

**Return Type**: `Optional[Union[str, int]]`

### `test_audio_capture(duration, device)` - Lines 402-457
**Purpose**: Standalone test function

**Requirements**:
- ❌ Create AudioCapture instance
- ❌ List devices
- ❌ Record for specified duration
- ❌ Analyze audio:
  - ❌ Calculate RMS
  - ❌ Calculate dB level
  - ❌ Find peak amplitude
  - ❌ Print analysis
  - ❌ Warn about audio levels
- ❌ Handle Ctrl+C gracefully
- ❌ Return audio array or None

---

## 5. Critical Features NOT to Lose

### Dual Backend Support
- ✅ **Why Critical**: PipeWire monitor sources require parec, regular devices use sounddevice
- ❌ **Implementation**:
  - Detect device name format (string with 'alsa_' or '.')
  - Spawn parec subprocess for PipeWire sources
  - Use cpal for regular devices

### Streaming Mode
- ✅ **Why Critical**: Used for VAD-triggered transcription
- ❌ **Implementation**:
  - Accumulate samples in separate chunk buffer
  - Emit fixed-size chunks (chunk_frames)
  - Call Python callback: `on_chunk_ready(chunk)`
  - Must work from both backends (parec + cpal)

### Thread Safety
- ✅ **Why Critical**: Audio callback runs in separate thread
- ✅ **Implementation**:
  - Main buffer: parking_lot::Mutex or lock-free ringbuf
  - Chunk buffer: parking_lot::Mutex
  - Atomic `is_recording` flag

### Legacy Callback Support
- ✅ **Why Critical**: Backward compatibility with existing code
- ❌ **Implementation**: `on_audio_callback(audio, frames)` hook

### Device Detection
- ✅ **Why Critical**: Users need to see available devices
- ❌ **Implementation**: `list_devices()` with formatted output

### Error Handling
- ✅ **Why Critical**: Must match Python exception behavior
- ❌ **Implementation**: PyO3 exception types

### Mono Conversion
- ✅ **Why Critical**: STT models expect mono audio
- ❌ **Implementation**: Average across channels if input is stereo

---

## 6. PyO3 Bindings Required

### Class: `PyAudioCapture`
- ❌ Match all public methods
- ❌ Numpy array returns (via `numpy` crate)
- ❌ Python exception mapping
- ❌ Context manager protocol (`__enter__`, `__exit__`)
- ❌ Callbacks to Python (use `pyo3::PyObject`)

### Callback Signatures
```python
# Legacy callback
def on_audio_callback(audio: np.ndarray, frames: int) -> None: ...

# Streaming callback
def on_chunk_ready(chunk: np.ndarray) -> None: ...
```

---

## 7. Testing Requirements

### Unit Tests (Rust)
- ❌ CircularBuffer operations
- ❌ Stereo → mono conversion
- ❌ Chunk accumulation logic
- ❌ Thread safety (concurrent access)

### Integration Tests (Python)
- ❌ Device enumeration
- ❌ Recording start/stop
- ❌ Buffer retrieval
- ❌ Streaming mode chunks
- ❌ parec backend (if available)
- ❌ Callback invocation
- ❌ Context manager

### Compatibility Tests
- ❌ Drop-in replacement test (import Rust version, run existing tests)
- ❌ API signature matching
- ❌ Return type matching (numpy arrays)
- ❌ Error message matching

### Performance Tests
- ❌ Callback latency (<50μs)
- ❌ Zero-copy verification
- ❌ CPU usage during capture
- ❌ Memory stability (1-hour session)

---

## 8. Known Edge Cases

### Parec-Specific
- ❌ Device name detection (`alsa_` prefix, contains '.')
- ❌ Subprocess cleanup on error
- ❌ Thread join with timeout
- ❌ Kill vs terminate

### Audio Callback
- ❌ Status warnings (print but continue)
- ❌ Empty indata handling
- ❌ Multi-channel to mono averaging

### Buffer Management
- ❌ Overflow handling (deque maxlen)
- ❌ Concurrent read/write
- ❌ Clear while recording

### Streaming Mode
- ❌ Partial chunks (don't emit until full)
- ❌ Chunk boundary alignment
- ❌ Callback errors don't crash capture

---

## 9. Performance Targets

| Metric | Python | Rust Target | Status |
|--------|--------|-------------|--------|
| Callback latency | ~1-2ms | <50μs | ❌ |
| Buffer copy overhead | High (deque extend) | Zero-copy | ✅ |
| CPU usage (active) | ~5-10% | <2% | ❌ |
| Memory overhead | ~50-100MB | <20MB | ❌ |
| Thread sync overhead | ~100-500μs | <10μs | ❌ |

---

## 10. Migration Strategy

### Phase 1: Core Rust Implementation ⏳
- ✅ Circular buffer (lock-free ringbuf)
- ❌ Audio capture (cpal)
- ❌ Mono conversion
- ❌ Streaming chunks
- ❌ Error types

### Phase 2: PyO3 Bindings
- ❌ Python class wrapper
- ❌ Numpy integration
- ❌ Callback hooks
- ❌ Exception mapping

### Phase 3: Backend Parity
- ❌ cpal backend (default)
- ❌ parec backend (PipeWire monitors)
- ❌ Device detection logic

### Phase 4: Testing & Validation
- ❌ Unit tests
- ❌ Integration tests
- ❌ Performance benchmarks
- ❌ API compatibility verification

### Phase 5: Feature Flag Rollout
- ❌ `USE_RUST_AUDIO=1` environment variable
- ❌ Fallback to Python on error
- ❌ Side-by-side testing

---

## 11. Open Questions

1. **Parec subprocess in Rust**: Use `std::process::Command` or find native PipeWire bindings?
   - ✅ Decision: Start with subprocess (exact parity), migrate to native later

2. **Resampling**: Do we need it? Python version doesn't resample.
   - ⏳ Decision: Add resampler module but make it optional

3. **Device strings vs indices**: How does cpal handle PipeWire device names?
   - ⏳ Need to test: cpal host.input_devices() on PipeWire

4. **Callback threading**: Does cpal guarantee single-threaded callbacks?
   - ⏳ Need to verify: cpal callback thread safety guarantees

---

## Next Steps

1. ✅ Create Cargo.toml and project structure
2. ✅ Implement CircularBuffer (lock-free)
3. ⏳ **NOW**: Implement AudioCapture with cpal
4. ⏳ Add parec subprocess backend
5. ⏳ PyO3 bindings
6. ⏳ Testing suite

---

## Notes

- **Zero tolerance for API changes**: Every Python method must have exact Rust equivalent
- **Behavioral parity**: Edge cases, error messages, print statements should match
- **Performance gains secondary to correctness**: Get it working first, optimize second
- **Feature flags for rollout**: Must be able to switch back to Python instantly if issues arise
