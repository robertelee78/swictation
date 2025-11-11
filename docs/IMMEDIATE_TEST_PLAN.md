# ⚡ Immediate Test Execution Plan
## Next Steps to Isolate the 6 dB Mel Feature Offset

**Created:** 2025-11-10
**Agent:** Tester (Hive Mind)
**Priority:** CRITICAL
**Time Estimate:** 2-4 hours

---

## 🎯 Goal

**Narrow down the 6 dB offset to a specific processing step:**
- Is it in audio loading? (unlikely)
- Is it in FFT/power spectrum? (possible)
- Is it in mel filterbank application? (likely)
- Is it in normalization? (possible)

---

## 📋 Execution Sequence

### Step 1: Test Raw Audio Loading (30 minutes)
**Purpose:** Establish that audio loading is NOT the issue

#### Create Raw Audio Export Script
```bash
# File: rust-crates/swictation-stt/examples/export_raw_audio.rs
```

**Required additions to audio.rs:**
```rust
impl AudioProcessor {
    pub fn export_raw_samples(&self, audio: &[f32], path: &str, count: usize) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        writeln!(file, "sample_index,amplitude")?;

        for (i, &sample) in audio.iter().take(count).enumerate() {
            writeln!(file, "{},{:.10}", i, sample)?;
        }

        // Compute and log statistics
        let rms = (audio.iter().take(count).map(|&x| x * x).sum::<f32>() / count as f32).sqrt();
        let mean = audio.iter().take(count).sum::<f32>() / count as f32;
        let max_amp = audio.iter().take(count).map(|&x| x.abs()).fold(0.0f32, f32::max);

        eprintln!("Rust audio statistics (first {} samples):", count);
        eprintln!("  RMS: {:.6}", rms);
        eprintln!("  Mean: {:.6}", mean);
        eprintln!("  Max amplitude: {:.6}", max_amp);

        Ok(())
    }
}
```

**Test execution:**
```bash
cd /opt/swictation/rust-crates/swictation-stt

# Create example
cat > examples/export_raw_audio.rs << 'EOF'
use swictation_stt::audio::AudioProcessor;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <audio_file> <output_csv>", args[0]);
        std::process::exit(1);
    }

    let audio_path = &args[1];
    let output_csv = &args[2];

    let processor = AudioProcessor::new()?;
    let audio = processor.load_audio(audio_path)?;

    eprintln!("Loaded {} samples", audio.len());
    processor.export_raw_samples(&audio, output_csv, 1000)?;

    Ok(())
}
EOF

# Build and run
cargo build --example export_raw_audio --release
cargo run --example export_raw_audio --release examples/en-short.mp3 rust_raw_audio.csv
```

**Create Python comparison:**
```python
# File: scripts/compare_raw_audio.py
import sys
import numpy as np
import torchaudio

def main():
    if len(sys.argv) < 3:
        print("Usage: python compare_raw_audio.py <rust_csv> <audio_file>")
        sys.exit(1)

    rust_csv = sys.argv[1]
    audio_file = sys.argv[2]

    # Load Rust samples
    rust_data = np.loadtxt(rust_csv, delimiter=',', skiprows=1)
    rust_samples = rust_data[:, 1]

    # Load Python samples
    waveform, sr = torchaudio.load(audio_file)
    if sr != 16000:
        waveform = torchaudio.transforms.Resample(sr, 16000)(waveform)

    # Convert to mono
    if waveform.shape[0] > 1:
        waveform = waveform.mean(dim=0)
    else:
        waveform = waveform[0]

    python_samples = waveform.numpy()[:1000]

    # Compute statistics
    python_rms = np.sqrt(np.mean(python_samples**2))
    python_mean = np.mean(python_samples)
    python_max = np.max(np.abs(python_samples))

    rust_rms = np.sqrt(np.mean(rust_samples**2))
    rust_mean = np.mean(rust_samples)
    rust_max = np.max(np.abs(rust_samples))

    print("Python audio statistics (first 1000 samples):")
    print(f"  RMS: {python_rms:.6f}")
    print(f"  Mean: {python_mean:.6f}")
    print(f"  Max amplitude: {python_max:.6f}")
    print()

    # Compare
    rms_diff = abs(rust_rms - python_rms) / python_rms * 100
    mean_diff = abs(rust_mean - python_mean)
    max_diff = abs(rust_max - python_max)

    print("Comparison:")
    print(f"  RMS difference: {rms_diff:.3f}%")
    print(f"  Mean difference: {mean_diff:.6f}")
    print(f"  Max amplitude difference: {max_diff:.6f}")
    print()

    # Element-by-element comparison
    sample_diff = np.abs(rust_samples - python_samples)
    print(f"  Max sample difference: {sample_diff.max():.6e}")
    print(f"  Mean sample difference: {sample_diff.mean():.6e}")
    print(f"  Correlation: {np.corrcoef(rust_samples, python_samples)[0,1]:.6f}")
    print()

    # Pass/fail
    if rms_diff < 0.1 and sample_diff.max() < 1e-6:
        print("✅ PASS: Audio loading matches Python within tolerance")
        return 0
    else:
        print("❌ FAIL: Audio loading differs from Python")
        return 1

if __name__ == "__main__":
    sys.exit(main())
```

**Run comparison:**
```bash
cd /opt/swictation
python scripts/compare_raw_audio.py rust_raw_audio.csv examples/en-short.mp3
```

**Expected Result:** ✅ PASS (audio loading is fine)
**If FAIL:** Investigate resampling or bit-depth conversion

---

### Step 2: Test Power Spectrum Before Mel (1 hour)
**Purpose:** Check if FFT or power computation has the offset

#### Modify audio.rs to export power spectrum
```rust
impl AudioProcessor {
    pub fn export_power_spectrum_frame(
        &mut self,
        audio: &[f32],
        frame_idx: usize,
        path: &str,
    ) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        // Extract single frame
        let start = frame_idx * HOP_LENGTH;
        if start + WIN_LENGTH > audio.len() {
            return Err(SttError::AudioProcessingError(
                "Frame index out of bounds".to_string()
            ));
        }

        let frame = &audio[start..start + WIN_LENGTH];

        // Apply Povey window
        let povey_window = self.create_povey_window();
        let windowed: Vec<f32> = frame.iter()
            .zip(povey_window.iter())
            .map(|(&s, &w)| s * w)
            .collect();

        // Compute FFT
        let mut fft = self.fft_planner.plan_fft_forward(N_FFT);
        let mut buffer: Vec<Complex<f32>> = windowed.iter()
            .map(|&x| Complex { re: x, im: 0.0 })
            .chain(std::iter::repeat(Complex { re: 0.0, im: 0.0 }))
            .take(N_FFT)
            .collect();

        fft.process(&mut buffer);

        // Compute power spectrum (keep only positive frequencies)
        let power_spec: Vec<f32> = buffer[..N_FFT/2 + 1]
            .iter()
            .map(|c| c.re * c.re + c.im * c.im)
            .collect();

        // Export to CSV
        let mut file = File::create(path)?;
        writeln!(file, "frequency_bin,power,log_power_db")?;

        for (i, &power) in power_spec.iter().enumerate() {
            let log_power = 10.0 * (power + 1e-10).log10();
            writeln!(file, "{},{:.10},{:.6}", i, power, log_power)?;
        }

        eprintln!("Rust power spectrum statistics:");
        eprintln!("  Num bins: {}", power_spec.len());
        eprintln!("  Power range: [{:.6e}, {:.6e}]",
            power_spec.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            power_spec.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap());

        Ok(())
    }

    fn create_povey_window(&self) -> Vec<f32> {
        // Povey window: w[i] = (0.5 - 0.5*cos(2πi/N))^0.85
        (0..WIN_LENGTH)
            .map(|i| {
                let x = 0.5 - 0.5 * (2.0 * PI * i as f32 / WIN_LENGTH as f32).cos();
                x.powf(0.85)
            })
            .collect()
    }
}
```

**Create export example:**
```rust
// File: examples/export_power_spectrum.rs
use swictation_stt::audio::AudioProcessor;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <audio_file> <frame_index> <output_csv>", args[0]);
        std::process::exit(1);
    }

    let audio_path = &args[1];
    let frame_idx: usize = args[2].parse()?;
    let output_csv = &args[3];

    let mut processor = AudioProcessor::new()?;
    let audio = processor.load_audio(audio_path)?;

    eprintln!("Exporting power spectrum for frame {}", frame_idx);
    processor.export_power_spectrum_frame(&audio, frame_idx, output_csv)?;

    Ok(())
}
```

**Create Python comparison:**
```python
# File: scripts/compare_power_spectrum.py
import sys
import numpy as np
import torchaudio
import torch

def compute_power_spectrum(audio, frame_idx, n_fft=512, hop_length=160, win_length=400):
    """Compute power spectrum for a single frame matching Rust implementation"""

    # Extract frame
    start = frame_idx * hop_length
    frame = audio[start:start + win_length]

    # Apply Povey window
    hanning = torch.hann_window(win_length)
    povey_window = torch.pow(hanning, 0.85)
    windowed = frame * povey_window

    # Zero-pad to n_fft
    padded = torch.nn.functional.pad(windowed, (0, n_fft - win_length))

    # Compute FFT
    fft_out = torch.fft.rfft(padded)

    # Compute power (magnitude squared)
    power = fft_out.real**2 + fft_out.imag**2

    return power.numpy()

def main():
    if len(sys.argv) < 4:
        print("Usage: python compare_power_spectrum.py <rust_csv> <audio_file> <frame_idx>")
        sys.exit(1)

    rust_csv = sys.argv[1]
    audio_file = sys.argv[2]
    frame_idx = int(sys.argv[3])

    # Load Rust power spectrum
    rust_data = np.loadtxt(rust_csv, delimiter=',', skiprows=1)
    rust_power = rust_data[:, 1]
    rust_log_power = rust_data[:, 2]

    # Load audio and compute Python power spectrum
    waveform, sr = torchaudio.load(audio_file)
    if sr != 16000:
        waveform = torchaudio.transforms.Resample(sr, 16000)(waveform)
    if waveform.shape[0] > 1:
        waveform = waveform.mean(dim=0)
    else:
        waveform = waveform[0]

    python_power = compute_power_spectrum(waveform, frame_idx)
    python_log_power = 10 * np.log10(python_power + 1e-10)

    print(f"Python power spectrum statistics:")
    print(f"  Num bins: {len(python_power)}")
    print(f"  Power range: [{python_power.min():.6e}, {python_power.max():.6e}]")
    print()

    # Compare
    power_diff = np.abs(rust_power - python_power)
    log_diff = rust_log_power - python_log_power

    print("Comparison (linear power):")
    print(f"  Max difference: {power_diff.max():.6e}")
    print(f"  Mean difference: {power_diff.mean():.6e}")
    print(f"  Correlation: {np.corrcoef(rust_power, python_power)[0,1]:.6f}")
    print()

    print("Comparison (log power dB):")
    print(f"  Max difference: {log_diff.max():.4f} dB")
    print(f"  Mean difference: {log_diff.mean():.4f} dB")
    print(f"  Std difference: {log_diff.std():.4f} dB")
    print(f"  Correlation: {np.corrcoef(rust_log_power, python_log_power)[0,1]:.6f}")
    print()

    # Check for constant offset
    if abs(log_diff.std()) < 0.5:
        print(f"⚠️  CONSTANT OFFSET DETECTED: {log_diff.mean():.4f} dB")
        print("    This suggests systematic scaling difference")

    # Pass/fail
    if abs(log_diff.mean()) < 0.1 and np.corrcoef(rust_power, python_power)[0,1] > 0.99:
        print("✅ PASS: Power spectrum matches Python")
        return 0
    else:
        print("❌ FAIL: Power spectrum differs from Python")
        if abs(log_diff.mean()) > 5.0:
            print("    Likely cause of 6 dB mel offset!")
        return 1

if __name__ == "__main__":
    sys.exit(main())
```

**Run test:**
```bash
cd /opt/swictation/rust-crates/swictation-stt
cargo build --example export_power_spectrum --release
cargo run --example export_power_spectrum --release examples/en-short.mp3 0 rust_power_spectrum.csv

cd /opt/swictation
python scripts/compare_power_spectrum.py rust_power_spectrum.csv examples/en-short.mp3 0
```

**Possible Results:**

**Case A: Power spectrum matches (✅)**
→ Issue is in mel filterbank application (proceed to Step 3)

**Case B: Power spectrum has offset (❌)**
→ Issue is in FFT or windowing:
- Check Povey window normalization
- Check FFT scaling
- Check power vs magnitude computation

---

### Step 3: Test Mel Filterbank Application (1 hour)
**Purpose:** Verify mel filterbank weights and application

#### Export mel filterbank weights
```rust
impl AudioProcessor {
    pub fn export_mel_filterbank(&self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Header
        writeln!(file, "mel_bin,freq_bin,weight")?;

        // Export all non-zero weights
        for mel_idx in 0..self.mel_filters.nrows() {
            for freq_idx in 0..self.mel_filters.ncols() {
                let weight = self.mel_filters[[mel_idx, freq_idx]];
                if weight > 1e-10 {
                    writeln!(file, "{},{},{:.10}", mel_idx, freq_idx, weight)?;
                }
            }
        }

        eprintln!("Mel filterbank exported:");
        eprintln!("  Num filters: {}", self.mel_filters.nrows());
        eprintln!("  Num frequency bins: {}", self.mel_filters.ncols());

        Ok(())
    }
}
```

**Create Python verification:**
```python
# File: scripts/verify_mel_filterbank.py
import sys
import numpy as np
import librosa

def create_mel_filterbank_htk(n_mels, n_fft, sr, fmin, fmax):
    """Create mel filterbank using HTK formula (matching Rust)"""

    # HTK mel scale
    def hz_to_mel_htk(hz):
        return 2595.0 * np.log10(1.0 + hz / 700.0)

    def mel_to_hz_htk(mel):
        return 700.0 * (10.0**(mel / 2595.0) - 1.0)

    # Create mel points
    mel_min = hz_to_mel_htk(fmin)
    mel_max = hz_to_mel_htk(fmax)
    mel_points = np.linspace(mel_min, mel_max, n_mels + 2)
    hz_points = mel_to_hz_htk(mel_points)

    # Convert to FFT bin numbers
    bin_points = np.floor((n_fft + 1) * hz_points / sr).astype(int)

    # Create filterbank matrix
    filterbank = np.zeros((n_mels, n_fft // 2 + 1))

    for i in range(n_mels):
        left = bin_points[i]
        center = bin_points[i + 1]
        right = bin_points[i + 2]

        # Left slope
        for j in range(left, center):
            filterbank[i, j] = (j - left) / (center - left)

        # Right slope
        for j in range(center, right):
            filterbank[i, j] = (right - j) / (right - center)

        # Peak normalization (max = 1.0)
        if filterbank[i].max() > 0:
            filterbank[i] /= filterbank[i].max()

    return filterbank

def main():
    if len(sys.argv) < 2:
        print("Usage: python verify_mel_filterbank.py <rust_csv>")
        sys.exit(1)

    rust_csv = sys.argv[1]

    # Load Rust filterbank
    rust_data = np.loadtxt(rust_csv, delimiter=',', skiprows=1)
    mel_bins = rust_data[:, 0].astype(int)
    freq_bins = rust_data[:, 1].astype(int)
    weights = rust_data[:, 2]

    # Reconstruct sparse matrix
    n_mels = mel_bins.max() + 1
    n_freq = freq_bins.max() + 1
    rust_filterbank = np.zeros((n_mels, n_freq))

    for mel_idx, freq_idx, weight in zip(mel_bins, freq_bins, weights):
        rust_filterbank[mel_idx, freq_idx] = weight

    print(f"Rust filterbank: {n_mels} mels × {n_freq} frequency bins")

    # Create Python reference
    python_filterbank = create_mel_filterbank_htk(
        n_mels=80,
        n_fft=512,
        sr=16000,
        fmin=20.0,
        fmax=7600.0
    )

    print(f"Python filterbank: {python_filterbank.shape[0]} mels × {python_filterbank.shape[1]} bins")

    # Compare
    diff = np.abs(rust_filterbank - python_filterbank)

    print("\nComparison:")
    print(f"  Max weight difference: {diff.max():.6e}")
    print(f"  Mean weight difference: {diff.mean():.6e}")
    print(f"  Num significant differences (>1e-4): {(diff > 1e-4).sum()}")

    # Check peak normalization
    rust_peaks = rust_filterbank.max(axis=1)
    python_peaks = python_filterbank.max(axis=1)

    print(f"\nPeak normalization:")
    print(f"  Rust peaks all ~1.0: {np.allclose(rust_peaks, 1.0, atol=1e-6)}")
    print(f"  Python peaks all ~1.0: {np.allclose(python_peaks, 1.0, atol=1e-6)}")

    # Pass/fail
    if diff.max() < 1e-4:
        print("\n✅ PASS: Mel filterbank matches Python")
        return 0
    else:
        print("\n❌ FAIL: Mel filterbank differs from Python")

        # Find problematic bins
        problem_mels = np.where(diff.max(axis=1) > 1e-4)[0]
        print(f"  Problem mel bins: {problem_mels[:10]}")
        return 1

if __name__ == "__main__":
    sys.exit(main())
```

**Run test:**
```bash
cd /opt/swictation/rust-crates/swictation-stt

# Add method to export filterbank (modify audio.rs or create example)
# Then run:
cargo run --example export_mel_filterbank rust_mel_filterbank.csv

cd /opt/swictation
python scripts/verify_mel_filterbank.py rust_mel_filterbank.csv
```

---

### Step 4: Unified Diagnostic Run (30 minutes)
**Purpose:** Run all tests together and generate comprehensive report

**Create master diagnostic:**
```bash
# File: scripts/diagnose_root_cause.sh
#!/bin/bash

set -e

echo "========================================="
echo "Root Cause Diagnostic for 6 dB Offset"
echo "========================================="
echo ""

cd /opt/swictation

# Step 1: Raw audio
echo "Step 1: Testing raw audio loading..."
cargo run --example export_raw_audio --release examples/en-short.mp3 rust_raw_audio.csv 2>&1 | tail -5
python scripts/compare_raw_audio.py rust_raw_audio.csv examples/en-short.mp3
AUDIO_RESULT=$?
echo ""

# Step 2: Power spectrum
echo "Step 2: Testing power spectrum (frame 0)..."
cargo run --example export_power_spectrum --release examples/en-short.mp3 0 rust_power_spectrum.csv 2>&1 | tail -5
python scripts/compare_power_spectrum.py rust_power_spectrum.csv examples/en-short.mp3 0
POWER_RESULT=$?
echo ""

# Step 3: Mel filterbank
echo "Step 3: Verifying mel filterbank weights..."
cargo run --example export_mel_filterbank rust_mel_filterbank.csv
python scripts/verify_mel_filterbank.py rust_mel_filterbank.csv
MEL_FB_RESULT=$?
echo ""

# Step 4: Full mel features (already working)
echo "Step 4: Testing full mel feature pipeline..."
./scripts/diagnose_feature_mismatch.sh 2>&1 | tail -10
MEL_FULL_RESULT=$?
echo ""

# Summary
echo "========================================="
echo "DIAGNOSTIC SUMMARY"
echo "========================================="
echo ""
echo "Step 1 - Raw audio loading:     $([ $AUDIO_RESULT -eq 0 ] && echo '✅ PASS' || echo '❌ FAIL')"
echo "Step 2 - Power spectrum:         $([ $POWER_RESULT -eq 0 ] && echo '✅ PASS' || echo '❌ FAIL')"
echo "Step 3 - Mel filterbank:         $([ $MEL_FB_RESULT -eq 0 ] && echo '✅ PASS' || echo '❌ FAIL')"
echo "Step 4 - Full mel pipeline:      $([ $MEL_FULL_RESULT -eq 0 ] && echo '✅ PASS' || echo '❌ FAIL')"
echo ""

# Diagnosis
if [ $AUDIO_RESULT -ne 0 ]; then
    echo "🔴 ROOT CAUSE: Audio loading or resampling"
    echo "   Action: Check sample rate conversion and bit-depth normalization"
elif [ $POWER_RESULT -ne 0 ]; then
    echo "🔴 ROOT CAUSE: FFT or power spectrum computation"
    echo "   Action: Check Povey window normalization, FFT scaling, power vs magnitude"
elif [ $MEL_FB_RESULT -ne 0 ]; then
    echo "🔴 ROOT CAUSE: Mel filterbank construction"
    echo "   Action: Check HTK formula, triangular weights, peak normalization"
else
    echo "🔴 ROOT CAUSE: Unknown (possibly normalization or epsilon in log)"
    echo "   Action: Check mel feature normalization, log computation, epsilon values"
fi

echo ""
echo "Next: Implement targeted fix for identified root cause"
```

**Make executable and run:**
```bash
chmod +x scripts/diagnose_root_cause.sh
./scripts/diagnose_root_cause.sh | tee diagnostic_results.txt
```

---

## 📊 Expected Timeline

| Step | Task | Time | Status |
|------|------|------|--------|
| 1 | Raw audio test | 30 min | Pending |
| 2 | Power spectrum test | 1 hour | Pending |
| 3 | Mel filterbank test | 1 hour | Pending |
| 4 | Unified diagnostic | 30 min | Pending |
| **Total** | | **3 hours** | |

---

## 🎯 Success Criteria

**After running all tests, we will know EXACTLY which step causes the 6 dB offset:**

1. ✅ Audio + ❌ Power → FFT/windowing issue
2. ✅ Audio + ✅ Power + ❌ Filterbank → Mel weights issue
3. ✅ Audio + ✅ Power + ✅ Filterbank + ❌ Full → Normalization issue

**Once identified:**
- Implement targeted fix
- Re-run full diagnostic
- Verify "en-short.mp3" transcribes correctly
- Run comprehensive test suite

---

## 🚀 Next Actions (Priority Order)

### Immediate (Next 30 minutes)
1. ✅ Create `scripts/compare_raw_audio.py`
2. ✅ Add `export_raw_samples()` to `audio.rs`
3. ✅ Create `examples/export_raw_audio.rs`
4. Run Test 1 (raw audio)

### Short-term (Next 2 hours)
5. Add `export_power_spectrum_frame()` to `audio.rs`
6. Create `examples/export_power_spectrum.rs`
7. Create `scripts/compare_power_spectrum.py`
8. Run Test 2 (power spectrum)
9. Add `export_mel_filterbank()` to `audio.rs`
10. Create `scripts/verify_mel_filterbank.py`
11. Run Test 3 (mel filterbank)

### Medium-term (Next hour)
12. Create `scripts/diagnose_root_cause.sh`
13. Run unified diagnostic
14. Analyze results and identify root cause
15. Implement fix
16. Validate with full test suite

---

## 📝 Notes for Implementation Team

### Code Locations
- Add methods to: `/opt/swictation/rust-crates/swictation-stt/src/audio.rs`
- Create examples in: `/opt/swictation/rust-crates/swictation-stt/examples/`
- Create scripts in: `/opt/swictation/scripts/`

### Dependencies
- Rust examples need: `swictation_stt` crate
- Python scripts need: `numpy`, `torchaudio`, `torch`, `librosa`

### Test Data
- Primary test file: `/opt/swictation/examples/en-short.mp3`
- Expected transcription: "hey there how are you doing today"

### Coordination
- Store results in: `.swarm/memory.db`
- Memory key: `hive/tester/diagnostic-results`
- Notify hive after each step completion

---

**Status:** Ready to execute
**Blocker:** None (all dependencies exist)
**Next:** Start with Step 1 (raw audio test)
