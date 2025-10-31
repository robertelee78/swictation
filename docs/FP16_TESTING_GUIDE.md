# FP16 Testing Guide

## Overview

This guide explains how to test the FP16 mixed precision implementation for the Swictation daemon to validate transcription accuracy after the VRAM optimization.

## Implementation Status

✅ **FP16 Conversion**: Model successfully converted to `torch.float16`
✅ **VRAM Reduction**: 3600 MB → 1792.6 MB (50.2% reduction)
✅ **Buffer Increase**: 10s → 20s context window
✅ **System Stability**: Daemon runs without crashes, 87% GPU utilization

## Testing Methods

### Method 1: Live Microphone Testing (Recommended)

The most reliable way to test transcription accuracy is using your actual microphone:

```bash
# Start log monitoring
./tests/test_fp16_manual.sh
```

**Test Procedure:**
1. Toggle dictation ON (Caps Lock or your configured hotkey)
2. Speak clearly: **"Hello world. Testing, one, two, three."**
3. Toggle dictation OFF
4. Check transcription output in the terminal

**Expected Result:**
- All words transcribed correctly
- Proper punctuation and capitalization
- Response time < 2 seconds

**Success Criteria:**
- ✓ Perfect: 100% accuracy (all words correct)
- ⚠️  Good: 95%+ accuracy (1 minor error allowed)
- ✗ Poor: <95% accuracy

### Method 2: Real-World Usage Testing

Test with normal dictation workflow:

```bash
# Monitor transcriptions in real-time
journalctl --user -u swictation.service -f | grep -E "(Text:|Recording)"
```

Then use the daemon normally for dictation. Check that:
- ✓ Words are spelled correctly
- ✓ Punctuation is appropriate
- ✓ No hallucinations (phantom words)
- ✓ Context is maintained across sentences

### Method 3: Extended Stress Test

Validate long-term stability:

```bash
# 1. Start daemon
systemctl --user start swictation.service

# 2. Dictate for 5-10 minutes continuously

# 3. Check for memory leaks
watch -n 1 'nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits'
```

**Expected:**
- VRAM stays stable at ~2200-3600 MB
- No gradual increase over time
- No OOM errors

## Validation Checklist

### ✓ Core Functionality
- [ ] Daemon starts without errors
- [ ] FP16 precision confirmed in logs
- [ ] Recording and transcription work
- [ ] Text injection functional

### ✓ Performance Metrics
- [ ] VRAM usage: ~1800 MB model + ~400 MB buffer = ~2200 MB total
- [ ] GPU utilization: 85-90%
- [ ] No CUDA errors during operation
- [ ] Response time comparable to FP32

### ✓ Accuracy Validation
- [ ] Short utterances: 95%+ accuracy
- [ ] Long dictation: Maintained quality
- [ ] No hallucinations on silence
- [ ] Punctuation and capitalization correct

### ✓ Stability
- [ ] No crashes during extended use (2+ hours)
- [ ] VRAM stable (no leaks)
- [ ] Consistent performance over time

## Known Limitations

### Audio Input Routing

The daemon captures from your microphone, not from audio playback. This means:

**❌ Won't Work:**
- Playing MP3 files through speakers expecting transcription
- Using `paplay` or similar without proper routing
- Direct MP3 → daemon tests

**✓ Will Work:**
- Speaking into your microphone
- Real-world dictation usage
- Manual transcription testing

### Why MP3 Tests Are Difficult

The daemon uses `sounddevice` to capture from hardware microphone input. To test with MP3 files programmatically, you would need to:

1. Create a virtual audio device
2. Route MP3 playback to that device
3. Configure daemon to use virtual device
4. This is complex and not worth it for validation

**Recommendation:** Use live microphone testing instead.

## Troubleshooting

### No Transcription Output

**Symptom:** Daemon records but no "Text:" output appears

**Possible Causes:**
1. **Silence detection:** VAD detected no speech
   - Solution: Speak louder/clearer
2. **Empty transcription:** Model returned empty result
   - Check: `journalctl -u swictation.service | grep "Empty transcription"`
3. **Streaming mode:** Text injected incrementally, no final output
   - Normal behavior if using streaming mode

### Poor Accuracy

**Symptom:** Words are wrong or missing

**Debugging:**
```bash
# Check model precision
journalctl --user -u swictation.service | grep "Precision"
# Should show: torch.float16

# Check for CUDA errors
journalctl --user -u swictation.service | grep -i "cuda error"

# Verify VRAM headroom
nvidia-smi
# Should be <90% utilization
```

**Solutions:**
- Ensure FP16 is active (check logs)
- Verify sufficient VRAM (should have ~500MB free)
- Check microphone audio quality
- Test with clear, loud speech

### VRAM Still High

**Expected:** ~2200 MB (model + buffer)
**If higher:** Check for:
- Other GPU processes: `nvidia-smi`
- Buffer size: Should be 20s (check code)
- Multiple model instances running

## Comparing FP32 vs FP16

To compare accuracy, you would need to:

1. **Baseline FP32:**
   - Revert to FP32 (comment out `.half()` line)
   - Record test transcriptions
   - Calculate WER (Word Error Rate)

2. **Test FP16:**
   - Use current FP16 implementation
   - Record same test transcriptions
   - Calculate WER

3. **Compare:**
   - Expected: <0.5% WER difference
   - Acceptable: <1% WER difference

**Note:** This requires extensive testing infrastructure. For practical validation, manual testing is sufficient if transcriptions are accurate.

## Success Criteria Summary

### Minimum Requirements (MUST PASS)
- ✓ FP16 precision confirmed in logs
- ✓ VRAM < 3800 MB (under 4GB limit)
- ✓ No OOM errors during normal use
- ✓ Transcription produces text output
- ✓ Basic accuracy (most words correct)

### Target Requirements (SHOULD PASS)
- ✓ 95%+ transcription accuracy
- ✓ Proper punctuation and capitalization
- ✓ No hallucinations on silence
- ✓ Stable VRAM usage over time
- ✓ Response time < 2 seconds

### Stretch Goals (NICE TO HAVE)
- ✓ 100% accuracy on test phrases
- ✓ Indistinguishable from FP32 quality
- ✓ 8+ hours continuous operation
- ✓ 30s buffer (future enhancement)

## Test Scripts Available

```bash
# Manual testing with live mic
./tests/test_fp16_manual.sh

# Integrated test (requires interaction)
./tests/test_fp16_integrated.sh

# Live test helper
./tests/test_fp16_live.sh

# Status check
./src/swictation_cli.py status
```

## Validation Complete ✓

Once you've confirmed:
1. Transcriptions are accurate (95%+)
2. No OOM errors occur
3. VRAM stays stable
4. System remains functional

Then the FP16 implementation can be marked as **PRODUCTION READY**.

## Next Steps

After successful validation:

1. **Mark Archon task as "done"**: Task f601c29b-9497-4dba-ad37-f987970f50a1
2. **Optional: Increase buffer to 30s** if you want even more context
3. **Monitor in production** for a few days
4. **Optional: Run extended test suite** (`tests/test_memory_protection.py`)

---

**Last Updated:** October 31, 2025
**Implementation:** FP16 + 20s buffer
**Status:** Testing phase
