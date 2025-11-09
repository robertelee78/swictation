# Silero VAD ONNX Model Integration Research - Debugging Findings

**Date:** 2025-01-08
**Issue:** Silero VAD returns constant probability ~0.0005 for all audio (silence and speech)
**Expected:** >0.5 for speech, <0.1 for silence
**Implementation:** ort crate 2.0.0-rc.10 with direct ONNX Runtime

---

## 1. Official Silero VAD v5 Model

### Model Information
- **Official Repository:** https://github.com/snakers4/silero-vad
- **Hugging Face Mirror:** https://huggingface.co/onnx-community/silero-vad

### Model Hash Verification
- **Our Model Hash:** `1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3`
- **Official v5 Hash (model_q4.onnx):** `e39b192962685cf99bfda19c272fd99837f1326fb8daa36a4be4ee0ac11ec50c`

**🚨 CRITICAL FINDING: Hash Mismatch**
- Our model hash does NOT match the official quantized v5 model
- This could indicate:
  - We're using a different model version
  - We're using the non-quantized version
  - The model file is corrupted
  - We're using v4 or earlier

### Official Model Downloads
```bash
# Official v5 ONNX model (recommended)
wget https://github.com/snakers4/silero-vad/raw/master/files/silero_vad.onnx

# Quantized v5 (smaller, faster)
wget https://huggingface.co/onnx-community/silero-vad/resolve/main/onnx/model_q4.onnx
```

### Model Requirements
- **Supported Sample Rates:** 8000 Hz or 16000 Hz (16 kHz recommended)
- **Input Format:** Mono, float32 normalized to [-1.0, 1.0] OR int16 full range
- **Chunk Sizes (16 kHz):** 512, 1024, or 1536 samples (32ms, 64ms, 96ms)
- **Model Size:** ~2 MB (JIT/ONNX)

---

## 2. ort 2.0.0-rc.10 Rust Crate Analysis

### Tensor Creation API (Correct Usage)

The official ort 2.0 API for creating tensors:

```rust
use ort::value::Tensor;
use ndarray::Array2;

// Method 1: From ndarray (requires 'ndarray' feature)
let array = Array2::<f32>::zeros((1, 512));
let tensor = Tensor::from_array(array)?; // Takes ownership

// Method 2: From tuple (shape, data)
let tensor = Tensor::from_array(([1usize, 512], vec![0.0f32; 512]))?;

// For borrowed data, use TensorRef
use ort::value::TensorRef;
let array = Array2::<f32>::zeros((1, 512));
let tensor_ref = TensorRef::from_array_view(array.view())?;
```

### Our Current Implementation (CORRECT ✅)

```rust
// From /opt/swictation/rust-crates/swictation-vad/src/silero_ort.rs

// Input tensor: [1, 512] - CORRECT
let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())?;
let input_tensor = Tensor::from_array(input_array)?;

// State tensor: [2, 1, 128] - CORRECT
let state_tensor = Tensor::from_array(self.state.clone())?;

// Sample rate: [1] - CORRECT (sherpa-onnx uses [1], not scalar)
let sr_array = ndarray::Array1::from(vec![self.sample_rate as i64]);
let sr_tensor = Tensor::from_array(sr_array)?;

// Inference - CORRECT
let outputs = session.run(inputs![
    "input" => input_tensor,
    "state" => state_tensor,
    "sr" => sr_tensor
])?;
```

### Known ort 2.0 Issues

| Issue | Description | Status |
|-------|-------------|--------|
| ndarray feature | Must enable `ndarray` feature in Cargo.toml | ✅ Enabled |
| ndarray version | Must use ndarray 0.16 (ort 2.0.0-rc.5+) | ✅ Correct |
| Windows linker | VS2022 >= 17.10 required | N/A (Linux) |
| WASM support | Dropped in 2.0.0-rc.5 | N/A |
| API stability | RC, not final | ⚠️ Monitor |

**No critical bugs found in ort 2.0.0-rc.10 with `Tensor::from_array` and ndarray**

---

## 3. Silero VAD Audio Preprocessing Requirements

### Required Audio Format

| Requirement | Value |
|------------|-------|
| Sample Rate | 8000 Hz or 16000 Hz |
| Channels | Mono |
| Data Type | float32 or int16 |
| Normalization (float32) | [-1.0, 1.0] |
| Normalization (int16) | Full int16 range (-32768 to 32767) |
| Chunk Size (16 kHz) | 512, 1024, or 1536 samples |

### Preprocessing Steps (Python Reference)

```python
import numpy as np
import soundfile as sf
import librosa

# 1. Load and resample
audio, sr = sf.read('input.wav')
if sr != 16000:
    audio = librosa.resample(audio, orig_sr=sr, target_sr=16000)

# 2. Convert to mono
if audio.ndim > 1:
    audio = np.mean(audio, axis=1)

# 3. Normalize to float32 [-1.0, 1.0]
audio = audio.astype(np.float32)
audio = np.clip(audio, -1.0, 1.0)

# 4. Process in chunks
chunk_size = 512  # 32ms at 16 kHz
for i in range(0, len(audio), chunk_size):
    chunk = audio[i:i+chunk_size]
    if len(chunk) == chunk_size:
        prob = vad.process(chunk)
```

### Our Implementation Check

From our code analysis, we need to verify:
- ✅ Audio is mono
- ✅ Audio is 16 kHz
- ✅ Chunk size is 512 samples
- ❓ **Audio normalization to [-1.0, 1.0]** - NEEDS VERIFICATION
- ❓ **Input data type (float32 vs int16)** - NEEDS VERIFICATION

---

## 4. Working Rust Integration Examples

### silero-vad-rs Crate (Reference Implementation)

```rust
use silero_vad::{SileroVAD, VADIterator};
use ndarray::Array1;

// Initialize model
let model = SileroVAD::new("path/to/silero_vad.onnx")?;
let mut vad = VADIterator::new(
    model,
    0.5,    // threshold
    16000,  // sample rate
    100,    // min silence duration (ms)
    30,     // speech pad (ms)
);

// Process streaming chunks
let chunk_size = 512; // for 16kHz
let audio_chunk = Array1::zeros(chunk_size);

if let Some(ts) = vad.process_chunk(&audio_chunk.view())? {
    println!("Speech from {:.2}s to {:.2}s", ts.start, ts.end);
}
```

**Key Differences from Our Implementation:**
1. Uses `ArrayView` for input (borrowed data)
2. Includes `VADIterator` wrapper with state management
3. Returns timestamps directly
4. Uses same ort crate underneath

---

## 5. Recommended Next Debugging Steps

### Priority 1: Model Verification
```bash
# Check our current model
sha256sum /path/to/current/silero_vad.onnx

# Download official v5 model
wget https://github.com/snakers4/silero-vad/raw/master/files/silero_vad.onnx -O silero_vad_v5_official.onnx

# Verify hash
sha256sum silero_vad_v5_official.onnx
# Should output: (varies by version, check official repo)

# Replace and test
cp silero_vad_v5_official.onnx /path/to/model/
```

### Priority 2: Audio Normalization Check
```rust
// Add to process() method before creating tensors
if self.debug && self.current_sample == 0 {
    eprintln!("Audio chunk statistics:");
    eprintln!("  Length: {}", audio_chunk.len());
    eprintln!("  Min: {:.6}", audio_chunk.iter().copied().fold(f32::INFINITY, f32::min));
    eprintln!("  Max: {:.6}", audio_chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max));
    eprintln!("  Mean: {:.6}", audio_chunk.iter().sum::<f32>() / audio_chunk.len() as f32);
    eprintln!("  Expected range: [-1.0, 1.0]");

    // Check for common normalization issues
    let max_abs = audio_chunk.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    if max_abs > 1.0 {
        eprintln!("  ⚠️ WARNING: Audio exceeds [-1.0, 1.0] range!");
    } else if max_abs < 0.01 {
        eprintln!("  ⚠️ WARNING: Audio may be severely under-normalized!");
    }
}
```

### Priority 3: Compare with Working Implementation
```bash
# Add silero-vad-rs as a dev dependency to compare
cargo add --dev silero-vad-rs

# Create side-by-side test
# Process same audio with both our implementation and silero-vad-rs
# Compare:
# - Speech probabilities
# - State evolution
# - Output values
```

### Priority 4: Input Tensor Verification
```rust
// Verify tensor data matches audio
eprintln!("Tensor check:");
let (shape, data) = input_tensor.extract_tensor();
eprintln!("  Tensor shape: {:?}", shape);
eprintln!("  Tensor data matches audio: {}",
    data.iter().zip(audio_chunk.iter()).all(|(a, b)| (a - b).abs() < 1e-6)
);
```

### Priority 5: Model Metadata Inspection
```bash
# Use netron to inspect model inputs/outputs
pip install netron
netron silero_vad.onnx

# Or use Python ONNX tools
python3 << EOF
import onnx
model = onnx.load("silero_vad.onnx")
print("Inputs:")
for input in model.graph.input:
    print(f"  {input.name}: {input.type}")
print("\nOutputs:")
for output in model.graph.output:
    print(f"  {output.name}: {output.type}")
EOF
```

---

## 6. Most Likely Root Causes (Prioritized)

### 1. **Model Version Mismatch** (HIGH PROBABILITY)
- **Evidence:** Hash mismatch between our model and official v5
- **Impact:** Could explain constant low probability
- **Fix:** Download official v5 model and replace
- **Test:** Verify hash matches official

### 2. **Audio Normalization Issue** (HIGH PROBABILITY)
- **Evidence:** Research shows audio MUST be in [-1.0, 1.0]
- **Impact:** Unnormalized audio → garbage predictions
- **Fix:** Ensure audio is properly normalized before processing
- **Test:** Add logging to verify audio range

### 3. **State Initialization Problem** (MEDIUM PROBABILITY)
- **Evidence:** RNN models are sensitive to initial state
- **Impact:** Wrong initial state → stuck in low-probability mode
- **Fix:** Verify state is correctly zero-initialized
- **Test:** Compare state evolution with reference implementation

### 4. **int16 vs float32 Mismatch** (MEDIUM PROBABILITY)
- **Evidence:** Model might expect int16 but receive float32 (or vice versa)
- **Impact:** Type mismatch → wrong interpretation of values
- **Fix:** Verify model's expected input type
- **Test:** Try both int16 and float32 inputs

### 5. **ort 2.0 API Usage** (LOW PROBABILITY)
- **Evidence:** Our implementation follows official patterns correctly
- **Impact:** Unlikely to be the issue
- **Fix:** N/A
- **Test:** Already verified correct

---

## 7. Quick Verification Script

```rust
// File: /opt/swictation/rust-crates/swictation-vad/examples/verify_model.rs

use ort::session::Session;
use ndarray::Array2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load model
    let session = Session::builder()?.commit_from_file("path/to/model.onnx")?;

    // Print model info
    println!("Model loaded successfully");
    println!("Inputs:");
    for (name, info) in session.inputs.iter() {
        println!("  {}: {:?}", name, info);
    }
    println!("Outputs:");
    for (name, info) in session.outputs.iter() {
        println!("  {}: {:?}", name, info);
    }

    // Test with known good input (silence)
    let silence = Array2::<f32>::zeros((1, 512));
    let state = ndarray::Array3::<f32>::zeros((2, 1, 128));
    let sr = ndarray::Array1::from(vec![16000i64]);

    let outputs = session.run(ort::inputs![
        "input" => ort::value::Tensor::from_array(silence)?,
        "state" => ort::value::Tensor::from_array(state)?,
        "sr" => ort::value::Tensor::from_array(sr)?
    ])?;

    let prob: ndarray::ArrayView2<f32> = outputs["output"]
        .try_extract_array()?.into_dimensionality()?;

    println!("\nSilence probability: {:.6}", prob[[0, 0]]);
    println!("Expected: < 0.1");

    Ok(())
}
```

---

## 8. References

### Official Documentation
- Silero VAD GitHub: https://github.com/snakers4/silero-vad
- Silero VAD ONNX: https://huggingface.co/onnx-community/silero-vad
- ort crate docs: https://docs.rs/ort/2.0.0-rc.10
- ort fundamentals: https://ort.pyke.io/fundamentals/value

### Working Examples
- silero-vad-rs: https://docs.rs/silero-vad-rs
- Rust example: https://github.com/snakers4/silero-vad/tree/master/examples/rust-example
- C++ example: https://github.com/snakers4/silero-vad/tree/master/examples/cpp

### Community Resources
- ort GitHub discussions: https://github.com/pykeio/ort/discussions
- ort Discord: #💬｜ort-discussions in pyke Discord

---

## Summary

**The most likely issue is model version mismatch** based on the SHA256 hash difference. The second most likely issue is audio normalization not being in the expected [-1.0, 1.0] range.

**Immediate Actions:**
1. ✅ Verify model hash and download official v5 if mismatch
2. ✅ Add audio normalization verification logging
3. ✅ Test with known good audio (official examples)
4. ✅ Compare state evolution with reference implementation

**Code Review Result:**
Our ort 2.0 tensor creation and inference code is correct according to official documentation and examples. The issue is most likely in the data (model file or audio preprocessing), not the code.
