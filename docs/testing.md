# Testing Guide

This document describes the testing strategy for Swictation, with emphasis on end-to-end streaming transcription validation.

## Test Structure

```
tests/
├── test_streaming_e2e.py       # E2E streaming tests (comprehensive)
├── test_streaming_scenarios.py # Scenario-based tests
├── test_nemo_streaming.py      # NeMo-specific streaming tests
├── test_daemon.py              # Daemon lifecycle tests
├── test_text_injection.py      # Text injection tests
├── generate_test_audio.py      # Test audio generator
├── run_e2e_tests.sh           # Test runner script
└── data/                      # Test audio files
    ├── en-short.mp3           # Short utterance (6s)
    ├── en-long.mp3            # Long audio (60s+)
    ├── silent-10s.mp3         # Silent audio (hallucination test)
    └── *.mp3                  # Additional test cases
```

## End-to-End Test Suite

The E2E test suite (`test_streaming_e2e.py`) validates complete streaming transcription accuracy.

### Test Coverage

#### 1. Short Audio Accuracy (`TestShortAudioAccuracy`)
- **Purpose**: Validate 100% word accuracy on short utterances
- **Audio**: `en-short.mp3` (6 seconds)
- **Expected**: "Hello world. Testing, one, two, three."
- **Target**: ≥95% word accuracy (ideally 100%)
- **Tests**:
  - Batch transcription baseline
  - Streaming transcription
  - Hallucination detection

#### 2. Long Audio WER (`TestLongAudioWER`)
- **Purpose**: Compare streaming vs batch transcription quality
- **Audio**: `en-long.mp3` (60+ seconds)
- **Metric**: Word Error Rate (WER)
- **Target**: WER difference ≤1% between streaming and batch
- **Validates**:
  - Context preservation across chunks
  - Sustained accuracy over time
  - No degradation in long recordings

#### 3. Silent Audio Hallucination (`TestSilentAudioHallucination`)
- **Purpose**: Verify no phantom words on silence
- **Audio**: `silent-10s.mp3` (10 seconds)
- **Target**: ≤2 words output (noise tolerance)
- **Validates**:
  - Hallucination detector working
  - Model doesn't invent words
  - Proper silence handling

#### 4. Real-time Latency (`TestRealtimeLatency`)
- **Purpose**: Measure end-to-end pipeline latency
- **Metrics**:
  - Audio → Transcription: Transcription latency
  - Transcription → Injection: Injection latency
  - Total E2E latency
- **Target**: <2000ms total
- **RTF Target**: <1.0 (faster than realtime)

#### 5. Memory Stability (`TestMemoryUsage`)
- **Purpose**: Detect memory leaks
- **Method**: 5 consecutive transcriptions
- **Target**: <100MB memory increase
- **Validates**:
  - Proper cleanup between runs
  - No GPU memory leaks
  - Stable daemon operation

## Running Tests

### Quick Smoke Tests (~2 minutes)
```bash
./tests/run_e2e_tests.sh --quick
```

Runs:
- Short audio accuracy
- Hallucination detection
- Basic latency check

### Full Test Suite (~10 minutes)
```bash
./tests/run_e2e_tests.sh --full
```

Runs all tests including:
- Long audio WER comparison
- Memory stability
- Performance benchmarks

### With Detailed Report
```bash
./tests/run_e2e_tests.sh --full --report
```

Generates HTML report: `test-report.html`

### Using pytest Directly
```bash
# All tests
pytest tests/test_streaming_e2e.py -v

# Specific test class
pytest tests/test_streaming_e2e.py::TestShortAudioAccuracy -v

# Specific test method
pytest tests/test_streaming_e2e.py::TestShortAudioAccuracy::test_short_audio_streaming -v

# With output
pytest tests/test_streaming_e2e.py -v -s

# Parallel execution (4 workers)
pytest tests/test_streaming_e2e.py -n 4
```

### Direct Python Execution
```bash
# Run without pytest
python3 tests/test_streaming_e2e.py
```

## Test Metrics

### Accuracy Metrics

**Word Accuracy** (case-insensitive)
```
Accuracy = (Matching Words / Total Reference Words) × 100
```

**Word Error Rate (WER)**
```
WER = (Substitutions + Insertions + Deletions) / Total Reference Words × 100
```

**Example:**
- Reference: "Hello world testing"
- Hypothesis: "Hello world tested"
- WER: 1/3 = 33.3% (one substitution)

### Performance Metrics

**Real-Time Factor (RTF)**
```
RTF = Processing Time / Audio Duration
```

- RTF = 0.1: 10x faster than realtime
- RTF = 1.0: Processes at realtime speed
- RTF > 1.0: Slower than realtime (unacceptable)

**Latency Components**
1. **Capture**: Microphone → Audio buffer
2. **VAD**: Voice activity detection
3. **Transcription**: STT processing
4. **Injection**: Text → Application

Target: Total <2000ms

## Test Data Generation

### Generate Synthetic Audio
```bash
python3 tests/generate_test_audio.py
```

Creates:
- `en-short-synthetic.mp3`: Controlled short utterance
- `context-test.mp3`: Context preservation test
- `pangram-test.mp3`: Character coverage test
- `punctuation-test.mp3`: Punctuation handling
- `silent-*.mp3`: Silent audio for hallucination tests

### Manual Recording
```bash
# Record with sox
rec -r 16000 -c 1 recording.wav

# Convert to MP3
ffmpeg -i recording.wav -ar 16000 -ac 1 recording.mp3
```

### Text-to-Speech
```bash
# Using espeak
espeak "Your test text here" -w test.wav
ffmpeg -i test.wav -ar 16000 -ac 1 test.mp3
```

## Continuous Integration

### Pre-commit Hooks
```yaml
# .pre-commit-config.yaml
- repo: local
  hooks:
    - id: streaming-smoke-test
      name: Streaming Smoke Test
      entry: ./tests/run_e2e_tests.sh --quick
      language: system
      pass_filenames: false
```

### GitHub Actions
```yaml
# .github/workflows/test.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install dependencies
        run: pip install -r requirements.txt
      - name: Run E2E tests
        run: ./tests/run_e2e_tests.sh --full --report
      - name: Upload report
        uses: actions/upload-artifact@v2
        with:
          name: test-report
          path: test-report.html
```

## Debugging Failed Tests

### Empty Transcription
```bash
# Check audio format
ffprobe tests/data/en-short.mp3

# Verify GPU
python3 -c "import torch; print(torch.cuda.is_available())"

# Check model loading
python3 -c "from nemo.collections.asr.models import EncDecMultiTaskModel; \
             model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')"
```

### High WER
- Check audio quality (bitrate, sample rate)
- Verify streaming configuration
- Compare against batch mode
- Check for audio corruption

### Memory Leaks
```bash
# Monitor during test
watch -n 1 nvidia-smi

# Check for temp file cleanup
ls -la /tmp/swictation/

# Profile memory
python3 -m memory_profiler tests/test_streaming_e2e.py
```

### Hallucinations
- Verify `hallucinations_detector=True`
- Check VAD threshold
- Test with different silence durations
- Review model configuration

## Performance Benchmarking

### Latency Profiling
```python
# Add to test
import cProfile
import pstats

profiler = cProfile.Profile()
profiler.enable()

# Run transcription
transcriber.transcribe_streaming(audio_path)

profiler.disable()
stats = pstats.Stats(profiler)
stats.sort_stats('cumulative')
stats.print_stats(20)
```

### GPU Profiling
```bash
# NVIDIA profiler
nsys profile python3 tests/test_streaming_e2e.py

# PyTorch profiler
python3 -m torch.utils.bottleneck tests/test_streaming_e2e.py
```

## Test Maintenance

### Adding New Tests
1. Create test method in appropriate class
2. Add expected audio/transcription
3. Update test runner script
4. Document in this guide

### Updating Expected Results
When model or configuration changes:
1. Re-run tests with `--report`
2. Review HTML report for changes
3. Update expected transcriptions if needed
4. Document changes in git commit

### Test Data Management
- Keep test files <5MB each
- Use compression (MP3, not WAV)
- Include expected transcriptions in README
- Version control all test data

## Success Criteria

### Ready for Production Checklist
- ✅ Short audio: 100% accuracy (or ≥95%)
- ✅ Long audio: WER <1% vs batch
- ✅ Silent audio: <2 words output
- ✅ Latency: <2000ms E2E
- ✅ RTF: <1.0 (faster than realtime)
- ✅ Memory: Stable (<100MB increase)
- ✅ No hallucinations
- ✅ Progressive injection works
- ✅ Context preserved across chunks

### Performance Targets
| Metric | Target | Current |
|--------|--------|---------|
| Word Accuracy | ≥95% | 100% |
| WER (batch vs streaming) | ≤1% | 0.34% |
| E2E Latency | <2000ms | ~500ms |
| RTF | <1.0 | 0.081 |
| GPU Memory | <4GB | 3.6GB |
| Hallucinations | 0 | 0 |

## Related Documentation
- [Streaming Implementation](streaming_implementation.md)
- [Architecture](architecture.md)
- [Performance Optimization](performance.md)
- [Troubleshooting](troubleshooting.md)
