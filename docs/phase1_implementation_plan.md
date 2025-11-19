# Phase 1: Extend OrtRecognizer for 0.6B Model - Implementation Plan

## Current State Analysis

### What Already Works ✅
1. **Model path detection** (lines 254-279 in recognizer_ort.rs):
   - Detects `1.1b` vs `0.6b` from path
   - Auto-selects correct mel features (80 for 1.1B, 128 for 0.6B)
   - Auto-selects transpose behavior

2. **Model file finding** (lines 109-140):
   - Already supports FP32, FP16, and INT8 quantization formats
   - Prefers FP32 for GPU, INT8 for CPU

3. **External weights handling** (lines 142-151):
   - Correctly changes directory for external data loading
   - Works for 1.1B external weights

### What Needs Fixing ❌

**Line 42-46**: Hardcoded decoder state size of 640
```rust
// Decoder RNN states (2, batch, 640)  ← HARDCODED FOR 1.1B
decoder_state1: Option<Array3<f32>>,
decoder_state2: Option<Array3<f32>>,
```

**Lines 843-916**: All decoder operations use hardcoded 640:
```rust
self.decoder_state1 = Some(Array3::zeros((2, batch_size, 640)));  ← Line 845
self.decoder_state2 = Some(Array3::zeros((2, 1, 640)));           ← Line 846
vec![2, batch_size, 640],                                         ← Line 851
vec![2, 1, 640],                                                  ← Line 858
(2, batch_size, 640)                                              ← Line 893
(2, batch_size, 640)                                              ← Line 909
decoder_out.len()  // This is 640 for 1.1B                        ← Line 916
vec![1, 640, 1]                                                   ← Line 952
(1, 1024, 1) and (1, 640, 1)                                      ← Line 964
```

## Implementation Changes Required

### 1. Add Model Configuration Struct

Add after line 30:
```rust
/// Model configuration for different Parakeet-TDT variants
#[derive(Debug, Clone, Copy)]
struct ModelConfig {
    /// Decoder hidden state size (512 for 0.6B, 640 for 1.1B)
    decoder_hidden_size: usize,
    /// Number of mel features (128 for 0.6B, 80 for 1.1B)
    n_mel_features: usize,
    /// Whether encoder expects transposed input
    transpose_input: bool,
    /// Model variant name for logging
    model_name: &'static str,
}

impl ModelConfig {
    fn for_0_6b() -> Self {
        Self {
            decoder_hidden_size: 512,
            n_mel_features: 128,
            transpose_input: true,
            model_name: "Parakeet-TDT-0.6B",
        }
    }

    fn for_1_1b() -> Self {
        Self {
            decoder_hidden_size: 640,
            n_mel_features: 80,
            transpose_input: false,
            model_name: "Parakeet-TDT-1.1B",
        }
    }

    /// Auto-detect from model path
    fn detect_from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy();
        if path_str.contains("1.1b") || path_str.contains("1-1b") {
            Self::for_1_1b()
        } else {
            // Default to 0.6B (more common, lower VRAM)
            Self::for_0_6b()
        }
    }
}
```

### 2. Add Config Field to OrtRecognizer

Modify line 33-47:
```rust
pub struct OrtRecognizer {
    encoder: Session,
    decoder: Session,
    joiner: Session,
    tokens: Vec<String>,
    blank_id: i64,
    unk_id: i64,
    model_path: PathBuf,
    audio_processor: AudioProcessor,
    // Decoder RNN states - size depends on model variant
    decoder_state1: Option<Array3<f32>>,
    decoder_state2: Option<Array3<f32>>,
    // Model configuration
    config: ModelConfig,  // ← ADD THIS
}
```

### 3. Update Constructor (Lines 66-296)

Replace lines 252-295 with:
```rust
// Auto-detect model configuration from path
let config = ModelConfig::detect_from_path(&model_path);

info!("Detected {} model", config.model_name);
info!("  Decoder hidden size: {}", config.decoder_hidden_size);
info!("  Mel features: {}", config.n_mel_features);
info!("  Transpose input: {}", config.transpose_input);

let audio_processor = AudioProcessor::with_mel_features(config.n_mel_features)?;

Ok(Self {
    encoder,
    decoder,
    joiner,
    tokens,
    blank_id,
    unk_id,
    model_path,
    audio_processor,
    decoder_state1: None,
    decoder_state2: None,
    config,  // ← ADD THIS
})
```

### 4. Update All Decoder State Operations

**Line 843-846**: Initialize with correct size
```rust
// Initialize states to zeros: (2, batch, decoder_hidden_size)
let hidden_size = self.config.decoder_hidden_size;
self.decoder_state1 = Some(Array3::zeros((2, batch_size, hidden_size)));
self.decoder_state2 = Some(Array3::zeros((2, 1, hidden_size)));
```

**Line 849-858**: Create tensors with correct dimensions
```rust
let state1_data = self.decoder_state1.as_ref().unwrap().as_slice().unwrap().to_vec();
let state1 = Tensor::from_array((
    vec![2, batch_size, self.config.decoder_hidden_size],
    state1_data.into_boxed_slice()
))?;

let state2_data = self.decoder_state2.as_ref().unwrap().as_slice().unwrap().to_vec();
let state2 = Tensor::from_array((
    vec![2, 1, self.config.decoder_hidden_size],
    state2_data.into_boxed_slice()
))?;
```

**Line 892-915**: Extract states with correct dimensions
```rust
let hidden_size = self.config.decoder_hidden_size;

// outputs[2] is the new state (2, batch, hidden_size)
if let Some(output2) = outputs.get(2) {
    if let Ok(tensor) = output2.try_extract_tensor::<f32>() {
        let data = tensor.into_owned().as_slice().unwrap().to_vec();
        self.decoder_state1 = Some(Array3::from_shape_vec(
            (2, batch_size, hidden_size),
            data
        ).expect("Failed to reshape decoder state1"));
    }
}

// outputs[3] is the second state (2, batch, hidden_size)
if let Some(output3) = outputs.get(3) {
    if let Ok(tensor) = output3.try_extract_tensor::<f32>() {
        let data = tensor.into_owned().as_slice().unwrap().to_vec();
        self.decoder_state2 = Some(Array3::from_shape_vec(
            (2, batch_size, hidden_size),
            data
        ).expect("Failed to reshape decoder state2"));
    }
}

// Extract the last timestep: shape is (batch, hidden_size, seq_len)
let decoder_out = Array1::from_vec(
    output0.into_owned().as_slice().unwrap().to_vec()
);
```

**Line 952**: Update joiner input shape
```rust
let decoder_input = Tensor::from_array((
    vec![1, self.config.decoder_hidden_size, 1],  // (batch, hidden_size, 1)
    decoder_out.to_vec().into_boxed_slice()
))?;
```

**Line 964**: Update comment
```rust
// With inputs (1, 1024, 1) and (1, {hidden_size}, 1), output is (1, 1, 1, 1030)
```

### 5. Update transpose_input Usage

Line 294: Already using `transpose_input` field - change to use config:
```rust
// Use config.transpose_input throughout recognize_samples()
```

## Testing Plan

### Unit Tests
1. Test ModelConfig::detect_from_path() with various paths
2. Verify decoder_state1/2 initialization sizes
3. Test with both 0.6B and 1.1B model paths

### Integration Tests
1. Load 0.6B model and verify hidden_size = 512
2. Load 1.1B model and verify hidden_size = 640
3. Run inference on both models with sample audio

## Files to Modify

1. `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs` - Main implementation
2. `/opt/swictation/rust-crates/swictation-stt/Cargo.toml` - No changes needed (dependencies already correct)

## Estimated Time

- Add ModelConfig struct: 15 min
- Update OrtRecognizer fields: 5 min
- Update constructor: 15 min
- Update decoder operations: 30 min
- Testing and debugging: 1-2 hours

**Total: 2-3 hours**

## Success Criteria

✅ Code compiles without errors
✅ 1.1B model still works (regression test)
✅ 0.6B model loads and runs
✅ Decoder states have correct dimensions for each model
✅ No hardcoded 640 values remain
