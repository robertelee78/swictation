# Silero VAD ONNX Threshold Configuration Guide

## ⚠️ IMPORTANT: Empirically Validated Configuration

This guide reflects the **actual working configuration** validated through real-world testing and production use.

## Quick Start: Daemon Default (Recommended)

The daemon uses empirically optimized defaults that work for real-time transcription:

```rust
// rust-crates/swictation-daemon/src/config.rs
vad_threshold: 0.25              // Empirically optimized (NOT 0.003!)
vad_min_silence: 0.8             // Seconds (NOT 0.5!)
vad_min_speech: 0.25             // Seconds
```

**Performance**: Captures 12/12 words in test scenarios with background noise present.

## Threshold Range: Standard 0.0-1.0 Probability

The Silero VAD ONNX model outputs speech probabilities in the **standard 0.0-1.0 range**.

| Threshold | Behavior | Use Case |
|-----------|----------|----------|
| **0.25** | **Recommended default** | Real-time transcription with background noise |
| 0.20 | More sensitive | Quiet environments |
| 0.30 | Less sensitive | Noisy environments |
| 0.15 | Very sensitive | May trigger on background noise |
| 0.40 | Very conservative | Risk missing quiet speech |

**Valid range**: `0.0 to 1.0` (validated in `lib.rs:211-213`)

## Why 0.25 Works (Empirical Evidence)

### Timeline of Discovery

**November 11, 2025 - Real-world Testing**

| Configuration | Result | Analysis |
|---------------|--------|----------|
| **Threshold: 0.003** | ❌ **0/12 words captured** | Too sensitive → background noise prevents silence detection |
| **Threshold: 0.25** | ✅ **12/12 words captured** | Optimal → properly distinguishes speech from background |

**Source**: Git commit `d1110c3b` - "Captures 12/12 words in test scenarios (was 0 with old settings)"

### Technical Explanation

**Why Low Thresholds (0.003) Failed:**
1. Too sensitive to background noise (AC, keyboard, etc.)
2. Background noise registered as "speech"
3. **No silence gaps detected** → transcription never triggered
4. Result: Recording continued indefinitely, 0 words captured

**Why 0.25 Works:**
1. Ignores background noise effectively
2. Detects actual silence gaps between speech
3. **Triggers real-time transcription** during natural pauses
4. Result: All words captured with proper segmentation

### Code Evidence

```rust
// rust-crates/swictation-daemon/src/config.rs:117
vad_threshold: 0.25, // Optimized for real-time transcription
                     // (original 0.003 prevented silence detection)
```

## Library vs Daemon Defaults

### Library Default (swictation-vad crate)

```rust
// rust-crates/swictation-vad/src/lib.rs:119
threshold: 0.003  // Conservative default for library users
```

**When to use**: Testing, research, or custom applications with controlled audio

### Daemon Default (swictation-daemon)

```rust
// rust-crates/swictation-daemon/src/config.rs:117
vad_threshold: 0.25  // Production-tested default
```

**When to use**: Real-time dictation, typical office/home environments

## Configuration Examples

### For Real-Time Dictation (Recommended)

```toml
# ~/.config/swictation/config.toml
[vad]
threshold = 0.25           # Optimal for real-time with background noise
min_silence_duration = 0.8 # Allow complete phrases
min_speech_duration = 0.25 # Filter out clicks/noise
```

### For Quiet Environments

```toml
[vad]
threshold = 0.20           # More sensitive
min_silence_duration = 0.6 # Shorter pauses OK
min_speech_duration = 0.25 # Same filtering
```

### For Noisy Environments

```toml
[vad]
threshold = 0.30           # Less sensitive to noise
min_silence_duration = 1.0 # Longer pauses required
min_speech_duration = 0.30 # Slightly longer minimum
```

## Common Issues and Solutions

### Issue: No Speech Detected

**Symptom**: VAD never triggers transcription, recordings are silent

**Likely cause**: Threshold too HIGH

```toml
# Try lowering the threshold
[vad]
threshold = 0.20  # Down from 0.25
```

### Issue: Background Noise Triggers Constantly

**Symptom**: Transcription triggers on ambient noise, fan sounds, etc.

**Likely cause**: Threshold too LOW

```toml
# Try raising the threshold
[vad]
threshold = 0.30  # Up from 0.25
```

### Issue: Speech Fragments Cut Off

**Symptom**: Words at start/end of phrases are missing

**Likely cause**: `min_silence_duration` too short

```toml
[vad]
threshold = 0.25
min_silence_duration = 1.0  # Up from 0.8
```

## Testing Your Configuration

### Method 1: Real-time Monitoring

```bash
# Monitor VAD detection in real-time
journalctl --user -u swictation-daemon -f | grep "VAD:"
```

**Look for**:
- Speech probability values during speech
- Whether silence gaps are detected properly
- Timing of transcription triggers

### Method 2: Test Audio

```bash
# Record test audio
parecord --channels=1 --rate=16000 --format=float32le test.raw

# Process with different thresholds
# Edit config.toml, restart daemon, test
```

### Method 3: Synthetic Test

```rust
use swictation_vad::{VadDetector, VadConfig};

let config = VadConfig::with_model("path/to/silero_vad.onnx")
    .threshold(0.25)  // Test different values
    .debug();         // Enable probability logging

let mut vad = VadDetector::new(config)?;
```

## Understanding Speech Probabilities

The ONNX model outputs a probability value for each audio window:
- **Near 0.0**: Silence or background noise
- **0.1-0.2**: Uncertain (could be noise or quiet speech)
- **0.3-0.5**: Likely speech
- **0.6-1.0**: High confidence speech

**Threshold of 0.25** means:
- Probabilities < 0.25 → Classified as silence
- Probabilities ≥ 0.25 → Classified as speech

## Validation Logic

```rust
// rust-crates/swictation-vad/src/lib.rs:211-213
if !(0.0..=1.0).contains(&self.threshold) {
    return Err(VadError::config("Threshold must be between 0.0 and 1.0"));
}
```

**Valid range**: `0.0` to `1.0` (standard probability range)

## Detection Algorithm

```rust
// rust-crates/swictation-vad/src/silero_ort.rs:231
if speech_prob >= self.threshold {
    // Speech detected
    self.triggered = true;
    self.speech_buffer.extend(audio_chunk);
} else if self.triggered {
    // Was speaking, now silence
    if silence_duration > min_silence_duration {
        // Return complete speech segment
        return Some(speech_buffer);
    }
}
```

## References

- **Production config**: `rust-crates/swictation-daemon/src/config.rs:117`
- **Library defaults**: `rust-crates/swictation-vad/src/lib.rs:119`
- **Validation logic**: `rust-crates/swictation-vad/src/lib.rs:211-213`
- **Detection algorithm**: `rust-crates/swictation-vad/src/silero_ort.rs:231`
- **Empirical testing**: Git commit `d1110c3b` (Nov 11, 2025)

## Model Information

- **Model**: Silero VAD v6 (ONNX format)
- **Repository**: https://github.com/onnx-community/silero-vad
- **Input**: 16kHz mono audio, window size 512 samples
- **Output**: Speech probability (0.0-1.0 range)
- **Runtime**: ONNX Runtime with CPU or CUDA acceleration

## Migration Notes

### From Previous Guide

**OLD guidance (INCORRECT)**:
- Threshold range: 0.001-0.005
- Claim: "ONNX outputs 100-200x lower probabilities"
- Default: 0.003

**NEW guidance (VALIDATED)**:
- Threshold range: 0.0-1.0 (standard probability)
- Reality: Empirically tested with real speech
- Default: **0.25** for production use

### Updating Existing Code

```rust
// OLD (will fail in real-world use)
let config = VadConfig::with_model("model.onnx")
    .threshold(0.003);  // ❌ Too sensitive

// NEW (empirically validated)
let config = VadConfig::with_model("model.onnx")
    .threshold(0.25);   // ✅ Works in production
```

## Summary

✅ **Use threshold 0.25** for real-time dictation (empirically validated)
✅ **Threshold range is 0.0-1.0** (standard probability range)
✅ **Higher threshold = better silence detection** in noisy environments
✅ **Lower threshold = more sensitive** but may trigger on background noise
✅ **Test with your specific audio environment** and adjust as needed

**Source of truth**: `rust-crates/swictation-daemon/src/config.rs:117`
