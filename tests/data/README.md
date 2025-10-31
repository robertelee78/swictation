# Test Audio Files

This directory contains audio files used for end-to-end testing of streaming transcription.

## Files

### en-short.mp3 (6 seconds)
- **Expected transcription**: "Hello world. Testing, one, two, three."
- **Purpose**: Short utterance accuracy test (100% target)
- **Source**: Manual recording
- **Use**: Baseline accuracy validation

### en-long.mp3 (1+ minutes)
- **Purpose**: Long-form streaming vs batch WER comparison
- **Target**: WER < 1% difference between streaming and batch
- **Use**: Context preservation and sustained accuracy validation

### silent-10s.mp3 (10 seconds)
- **Purpose**: Hallucination detection test
- **Expected**: No transcription or minimal noise artifacts
- **Use**: Verify model doesn't generate phantom words on silence

## Test Criteria

### Accuracy Targets
- **Short audio**: ≥95% word accuracy (ideally 100%)
- **Long audio**: WER difference ≤1% (streaming vs batch)
- **Silent audio**: ≤2 words output (noise tolerance)

### Performance Targets
- **Latency**: <2000ms total (audio → transcription → injection)
- **RTF**: <1.0 (faster than realtime)
- **GPU Memory**: Stable, no leaks (<100MB increase over 5 runs)

### Quality Checks
- ✅ No missed words
- ✅ No hallucinated words
- ✅ Context preserved across chunks
- ✅ Progressive text injection works
- ✅ Proper punctuation and capitalization

## Generating Additional Test Audio

### Text-to-Speech (for controlled tests)
```bash
# Using espeak (install: sudo apt install espeak)
espeak "The cat sat on the mat. The cat was orange." -w context-test.wav
ffmpeg -i context-test.wav -ar 16000 -ac 1 context-test.mp3
```

### Silent Audio (hallucination test)
```bash
ffmpeg -f lavfi -i anullsrc=r=16000:cl=mono -t 10 -acodec libmp3lame silent-10s.mp3
```

### Recording Real Audio
```bash
# Using sox (install: sudo apt install sox)
rec -r 16000 -c 1 recording.wav

# Convert to mp3
ffmpeg -i recording.wav -ar 16000 -ac 1 recording.mp3
```

## Manual Testing

Run individual tests:
```bash
# Full test suite
pytest tests/test_streaming_e2e.py -v

# Specific test class
pytest tests/test_streaming_e2e.py::TestShortAudioAccuracy -v

# Direct run (without pytest)
python tests/test_streaming_e2e.py

# With detailed output
pytest tests/test_streaming_e2e.py -v -s
```

## Expected Output

### Successful Test Run
```
Test: Short audio batch
  Text: 'Hello World. Testing. One, two, three.'
  Latency: 498ms
  Accuracy: 100.0%
✅ PASSED

Test: Streaming vs Batch WER
  Duration: 60.2s
  Batch RTF: 0.081x
  Streaming RTF: 0.085x
  WER: 0.34%
✅ PASSED
```

### Common Issues

**Empty Transcription**
- Check audio format (should be 16kHz mono)
- Verify GPU/CUDA available
- Check model loaded successfully

**High WER**
- Audio quality issues (noise, compression)
- Wrong sample rate
- Model not in streaming mode

**Hallucinations**
- Disable streaming for comparison
- Check hallucinations_detector=True in config
- Verify VAD working correctly

**Memory Leaks**
- Run `torch.cuda.empty_cache()` between tests
- Check temp files cleaned up
- Monitor with `nvidia-smi`

## Automation

These tests are run automatically:
1. **Pre-commit**: Quick smoke tests
2. **CI/CD**: Full suite on every push
3. **Nightly**: Extended tests with longer audio

## Contributing

When adding new test audio:
1. Include expected transcription in filename or README
2. Keep files <5MB (use compression)
3. Document source and purpose
4. Verify accuracy manually before adding to suite
5. Update test cases to include new audio
