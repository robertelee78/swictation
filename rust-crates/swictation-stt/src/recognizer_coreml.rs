//! Native CoreML recognizer for Parakeet-TDT models on macOS
//!
//! Uses the `coreml-native` crate for safe, ergonomic CoreML inference with full
//! ANE (Apple Neural Engine) utilization. Replaces the previous hand-written
//! C/Obj-C bridge.
//!
//! ## Model Format
//!
//! Expects pre-compiled CoreML model bundles:
//! - `encoder.mlmodelc/` - Encoder network
//! - `decoder.mlmodelc/` - Decoder LSTM network
//! - `joiner.mlmodelc/`  - Joint network
//! - `tokens.txt`        - Token vocabulary
//!
//! ## Compute Units
//!
//! By default loads with `All` (CPU+GPU+ANE) for maximum throughput.
//! The ANE is especially effective for the encoder's convolution layers.

#[cfg(all(target_os = "macos", feature = "coreml-native"))]
mod inner {

use crate::audio::AudioProcessor;
use crate::error::{Result, SttError};
use coreml_native::{BorrowedTensor, ComputeUnits, Model};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Model configuration (mirrors recognizer_ort.rs ModelConfig)
// ---------------------------------------------------------------------------

/// Model configuration for different Parakeet-TDT variants.
#[derive(Debug, Clone, Copy)]
struct ModelConfig {
    /// Decoder hidden state size (640 for both 0.6B and 1.1B)
    decoder_hidden_size: usize,
    /// Number of mel features (128 for 0.6B, 80 for 1.1B)
    n_mel_features: usize,
    /// Whether encoder expects transposed input
    #[allow(dead_code)]
    transpose_input: bool,
    /// Model variant name for logging
    model_name: &'static str,
}

impl ModelConfig {
    fn for_0_6b() -> Self {
        Self {
            decoder_hidden_size: 640,
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

    /// Auto-detect model variant from directory path.
    fn detect_from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy();
        if path_str.contains("1.1b") || path_str.contains("1-1b") {
            Self::for_1_1b()
        } else {
            Self::for_0_6b()
        }
    }
}

// ---------------------------------------------------------------------------
// CoreMLRecognizer
// ---------------------------------------------------------------------------

/// Native CoreML recognizer for Parakeet-TDT speech recognition.
///
/// Loads three compiled CoreML model bundles (encoder, decoder, joiner) and
/// performs TDT greedy search decoding identically to `OrtRecognizer`, but
/// using the Apple Neural Engine for maximum on-device throughput.
pub struct CoreMLRecognizer {
    encoder: Model,
    decoder: Model,
    joiner: Model,
    tokens: Vec<String>,
    blank_id: i64,
    unk_id: i64,
    model_path: PathBuf,
    audio_processor: AudioProcessor,
    /// Decoder LSTM hidden state: [2, 1, hidden_size] flattened
    decoder_state_h: Vec<f32>,
    /// Decoder LSTM cell state: [2, 1, hidden_size] flattened
    decoder_state_c: Vec<f32>,
    config: ModelConfig,
}

/// Helper to map `coreml_native::Error` into `SttError`.
fn coreml_err(context: &str, e: coreml_native::Error) -> SttError {
    SttError::InferenceError(format!("{}: {}", context, e))
}

/// De-interleave LSTM state from column-major [2, 1, hidden] to row-major.
///
/// CoreML MLMultiArray may return [2, 1, 640] in column-major order where
/// the two LSTM layers are interleaved: [L0[0], L1[0], L0[1], L1[1], ...].
/// This function re-orders to row-major: [L0[0..640], L1[0..640]].
fn deinterleave_lstm_state(data: &[f32], hidden_size: usize) -> Vec<f32> {
    let num_layers = 2;
    let expected = num_layers * hidden_size;
    if data.len() != expected {
        // Not the expected size — return as-is (may already be row-major)
        return data.to_vec();
    }
    let mut result = vec![0.0f32; expected];
    for i in 0..hidden_size {
        result[i] = data[2 * i];                     // layer 0
        result[hidden_size + i] = data[2 * i + 1];   // layer 1
    }
    result
}

impl CoreMLRecognizer {
    /// Create a new recognizer from a model directory.
    ///
    /// The directory must contain:
    /// - `encoder.mlmodelc/`
    /// - `decoder.mlmodelc/`
    /// - `joiner.mlmodelc/`
    /// - `tokens.txt`
    ///
    /// Note: CoreML MLMultiArray may return data in column-major order.
    /// For shapes like `[2, 1, 640]` (LSTM states), this means the two layers
    /// are interleaved element-by-element. We de-interleave after extraction.
    pub fn new<P: AsRef<Path>>(model_dir: P) -> Result<Self> {
        let model_path = model_dir.as_ref().to_path_buf();

        info!(
            "Loading Parakeet-TDT model with native CoreML from {}",
            model_path.display()
        );

        // Auto-detect model configuration from path
        let config = ModelConfig::detect_from_path(&model_path);
        info!("Detected {} model", config.model_name);
        info!("  Decoder hidden size: {}", config.decoder_hidden_size);
        info!("  Mel features: {}", config.n_mel_features);

        // Load tokens
        let tokens = load_tokens(&model_path)?;
        let blank_id = tokens
            .iter()
            .position(|t| t == "<blk>" || t == "<blank>")
            .ok_or_else(|| {
                SttError::ModelLoadError(
                    "Could not find <blk> token in tokens.txt".to_string(),
                )
            })? as i64;

        let unk_id = tokens
            .iter()
            .position(|t| t == "<unk>")
            .unwrap_or(0) as i64;

        info!(
            "Loaded {} tokens (blank_id={}, unk_id={})",
            tokens.len(),
            blank_id,
            unk_id
        );

        // Load the three CoreML model bundles with All compute units (CPU+GPU+ANE)
        info!("Loading encoder.mlmodelc (compute_units=All)...");
        let encoder = Model::load(
            model_path.join("encoder.mlmodelc"),
            ComputeUnits::All,
        )
        .map_err(|e| {
            SttError::ModelLoadError(format!(
                "Failed to load encoder: {}",
                e
            ))
        })?;

        info!("Loading decoder.mlmodelc (compute_units=All)...");
        let decoder = Model::load(
            model_path.join("decoder.mlmodelc"),
            ComputeUnits::All,
        )
        .map_err(|e| {
            SttError::ModelLoadError(format!(
                "Failed to load decoder: {}",
                e
            ))
        })?;

        info!("Loading joiner.mlmodelc (compute_units=All)...");
        let joiner = Model::load(
            model_path.join("joiner.mlmodelc"),
            ComputeUnits::All,
        )
        .map_err(|e| {
            SttError::ModelLoadError(format!(
                "Failed to load joiner: {}",
                e
            ))
        })?;

        let audio_processor =
            AudioProcessor::with_mel_features(config.n_mel_features)?;

        let hidden_size = config.decoder_hidden_size;

        info!("All CoreML models loaded successfully");

        Ok(Self {
            encoder,
            decoder,
            joiner,
            tokens,
            blank_id,
            unk_id,
            model_path,
            audio_processor,
            decoder_state_h: vec![0.0f32; 2 * 1 * hidden_size],
            decoder_state_c: vec![0.0f32; 2 * 1 * hidden_size],
            config,
        })
    }

    /// Returns `true` -- CoreML always runs on GPU/ANE when available.
    pub fn is_gpu(&self) -> bool {
        true
    }

    /// Recognize speech from raw audio samples (16 kHz, mono, f32).
    ///
    /// Returns the transcribed text.
    pub fn recognize_samples(&mut self, samples: &[f32]) -> Result<String> {
        info!("Processing {} audio samples via CoreML", samples.len());

        // The CoreML encoder model includes the preprocessor (mel spectrogram).
        // We pass raw 16kHz audio directly -- no mel extraction needed in Rust.
        // The encoder was traced with a fixed 15-second window (240000 samples).
        const MAX_AUDIO_SAMPLES: usize = 15 * 16000; // 240000

        // Pad or truncate audio to the fixed window size
        let audio: Vec<f32> = if samples.len() >= MAX_AUDIO_SAMPLES {
            samples[..MAX_AUDIO_SAMPLES].to_vec()
        } else {
            let mut padded = vec![0.0f32; MAX_AUDIO_SAMPLES];
            padded[..samples.len()].copy_from_slice(samples);
            padded
        };

        let actual_length = samples.len().min(MAX_AUDIO_SAMPLES);
        info!(
            "Audio: {} samples (padded to {})",
            actual_length, MAX_AUDIO_SAMPLES
        );

        // Run the fused preprocessor+encoder
        let (encoder_features, encoder_dim, valid_frames) =
            self.run_encoder(&audio, actual_length)?;
        info!(
            "Encoder output: {} features, dim={}, valid_frames={}",
            encoder_features.len(),
            encoder_dim,
            valid_frames
        );

        // Debug: compare encoder features with Python reference
        let total_time = encoder_features.len() / encoder_dim;
        info!("DEBUG total_time={}, encoder_dim={}", total_time, encoder_dim);

        // Row-major stride extraction: feature f at time t = flat[f * total_time + t]
        let frame10_stride: Vec<f32> = (0..5).map(|f| encoder_features[f * total_time + 10]).collect();
        info!("DEBUG frame 10 via stride: {:?}", frame10_stride);
        // Python reference: [0.281, 1.029, 0.868, 0.417, -0.937]
        info!("DEBUG Python frame 10:     [0.281, 1.029, 0.868, 0.417, -0.937]");

        // Reset decoder states
        let hidden_size = self.config.decoder_hidden_size;
        self.decoder_state_h = vec![0.0f32; 2 * 1 * hidden_size];
        self.decoder_state_c = vec![0.0f32; 2 * 1 * hidden_size];

        // Decode using TDT greedy search
        let all_tokens = self.tdt_greedy_decode(
            &encoder_features,
            encoder_dim,
            valid_frames,
        )?;

        let text = self.tokens_to_text(&all_tokens);
        Ok(text)
    }

    /// Recognize speech from an audio file (WAV, MP3, FLAC).
    pub fn recognize_file<P: AsRef<Path>>(
        &mut self,
        audio_path: P,
    ) -> Result<String> {
        info!("Loading audio: {}", audio_path.as_ref().display());
        let samples = self.audio_processor.load_audio(&audio_path)?;
        info!("Loaded {} samples", samples.len());
        self.recognize_samples(&samples)
    }

    /// Get model information string.
    pub fn model_info(&self) -> String {
        format!(
            "CoreMLRecognizer:\n  Model: {}\n  Variant: {}\n  Tokens: {}\n  Blank ID: {}\n  UNK ID: {}",
            self.model_path.display(),
            self.config.model_name,
            self.tokens.len(),
            self.blank_id,
            self.unk_id
        )
    }

    // -----------------------------------------------------------------------
    // Encoder
    // -----------------------------------------------------------------------

    /// Run the fused preprocessor+encoder on raw audio.
    ///
    /// Inputs (verified model spec):
    /// - `audio_signal`: [1, 240000] FLOAT32 -- raw 16kHz samples
    /// - `audio_length`: [1] INT32 -- actual sample count
    ///
    /// Outputs (single inference call):
    /// - `obj_3`: [1, 1024, 188] FLOAT16->f32 -- encoded features
    /// - `obj`:   [1] INT32->f32 -- valid frame count
    ///
    /// Returns `(features_flat, encoder_dim, valid_frames)`.
    fn run_encoder(
        &self,
        audio: &[f32],
        actual_length: usize,
    ) -> Result<(Vec<f32>, usize, usize)> {
        debug!("Encoder: {} raw audio samples", audio.len());

        let audio_tensor = BorrowedTensor::from_f32(audio, &[1, audio.len()])
            .map_err(|e| coreml_err("encoder audio tensor", e))?;

        let length_data = [actual_length as i32];
        let length_tensor = BorrowedTensor::from_i32(&length_data, &[1])
            .map_err(|e| coreml_err("encoder length tensor", e))?;

        let prediction = self
            .encoder
            .predict(&[
                ("audio_signal", &audio_tensor),
                ("audio_length", &length_tensor),
            ])
            .map_err(|e| coreml_err("encoder predict", e))?;

        // Extract features: [1, 1024, T] FLOAT16->f32
        let (features, feat_shape) = prediction
            .get_f32("obj_3")
            .map_err(|e| coreml_err("encoder get obj_3", e))?;

        // Extract valid frame count: [1] INT32->f32
        let (valid_len_f32, _) = prediction
            .get_f32("obj")
            .map_err(|e| coreml_err("encoder get obj", e))?;

        let valid_frames = if !valid_len_f32.is_empty() {
            let frames = valid_len_f32[0] as usize;
            info!("Encoder reported {} valid frames", frames);
            frames
        } else {
            // Fallback: derive from feature shape
            if feat_shape.len() >= 3 {
                feat_shape[2]
            } else {
                features.len() / 1024
            }
        };

        // encoder_dim is the second dimension of [1, encoder_dim, T]
        let encoder_dim = if feat_shape.len() >= 3 {
            feat_shape[1]
        } else {
            1024
        };

        let total_time = if feat_shape.len() >= 3 {
            feat_shape[2]
        } else {
            features.len() / encoder_dim
        };

        info!(
            "Encoder output: {} elements -> [1, {}, {}], valid_frames={}",
            features.len(),
            encoder_dim,
            total_time,
            valid_frames
        );

        // Clamp valid_frames to the actual time dimension
        let decode_frames = if valid_frames > 0 && valid_frames <= total_time {
            valid_frames
        } else {
            total_time
        };

        Ok((features, encoder_dim, decode_frames))
    }

    // -----------------------------------------------------------------------
    // Decoder
    // -----------------------------------------------------------------------

    /// Run the decoder with a single token and LSTM states.
    ///
    /// Inputs (verified model spec):
    /// - `targets`: [1, 1] INT32
    /// - `target_length`: [1] INT32
    /// - `h`: [2, 1, 640] FLOAT32
    /// - `c`: [2, 1, 640] FLOAT32
    ///
    /// Outputs (all extracted in ONE call):
    /// - `var_47`: [1, 1, 640] FLOAT16->f32 -- decoder embedding
    /// - `var_34`: [2, 1, 640] FLOAT16->f32 -- new h
    /// - `var_35`: [2, 1, 640] FLOAT16->f32 -- new c
    fn run_decoder(&mut self, token: i64) -> Result<Vec<f32>> {
        let hidden_size = self.config.decoder_hidden_size;

        let targets_i32 = [token as i32];
        let targets_tensor =
            BorrowedTensor::from_i32(&targets_i32, &[1, 1])
                .map_err(|e| coreml_err("decoder targets tensor", e))?;

        let target_length_i32 = [1i32];
        let target_length_tensor =
            BorrowedTensor::from_i32(&target_length_i32, &[1])
                .map_err(|e| coreml_err("decoder target_length tensor", e))?;

        let h_tensor = BorrowedTensor::from_f32(
            &self.decoder_state_h,
            &[2, 1, hidden_size],
        )
        .map_err(|e| coreml_err("decoder h tensor", e))?;

        let c_tensor = BorrowedTensor::from_f32(
            &self.decoder_state_c,
            &[2, 1, hidden_size],
        )
        .map_err(|e| coreml_err("decoder c tensor", e))?;

        let prediction = self
            .decoder
            .predict(&[
                ("targets", &targets_tensor),
                ("target_length", &target_length_tensor),
                ("h", &h_tensor),
                ("c", &c_tensor),
            ])
            .map_err(|e| coreml_err("decoder predict", e))?;

        // Decoder embedding: [1, 1, 640]
        let (dec_out, _) = prediction
            .get_f32("var_47")
            .map_err(|e| coreml_err("decoder get var_47", e))?;

        // New LSTM states: [2, 1, 640] each
        let (new_h, _) = prediction
            .get_f32("var_34")
            .map_err(|e| coreml_err("decoder get var_34", e))?;

        let (new_c, _) = prediction
            .get_f32("var_35")
            .map_err(|e| coreml_err("decoder get var_35", e))?;

        // Log first values for debugging stride order
        if new_h.len() >= 10 {
            info!("DEBUG h[0..10]: {:?}", &new_h[..10]);
            info!("DEBUG h[640..650]: {:?}", &new_h[640..650]);
            // Python reference (from blank_id=1024 initial call):
            // h[0,0,0]=-0.040161, h[0,0,1]=0.013962, h[1,0,0]=-0.154297
            // Row-major: flat[0]=-0.040161, flat[1]=0.013962, flat[640]=-0.154297
            // Col-major: flat[0]=-0.040161, flat[1]=-0.154297, flat[2]=0.013962
        }

        self.decoder_state_h = new_h;
        self.decoder_state_c = new_c;

        Ok(dec_out)
    }

    // -----------------------------------------------------------------------
    // Joiner
    // -----------------------------------------------------------------------

    /// Combine encoder frame and decoder output through the joint network.
    ///
    /// Inputs (verified model spec):
    /// - `encoder_output`: [1, 1, 1024] FLOAT32
    /// - `decoder_output`: [1, 1, 640] FLOAT32
    ///
    /// Output:
    /// - `var_31`: [1, 1, 1, 1030] FLOAT16->f32 -- logits
    fn run_joiner(
        &self,
        encoder_frame: &[f32],
        decoder_out: &[f32],
    ) -> Result<Vec<f32>> {
        let enc_dim = encoder_frame.len();
        let dec_dim = decoder_out.len();

        let enc_tensor =
            BorrowedTensor::from_f32(encoder_frame, &[1, 1, enc_dim])
                .map_err(|e| coreml_err("joiner encoder tensor", e))?;

        let dec_tensor =
            BorrowedTensor::from_f32(decoder_out, &[1, 1, dec_dim])
                .map_err(|e| coreml_err("joiner decoder tensor", e))?;

        let prediction = self
            .joiner
            .predict(&[
                ("encoder_output", &enc_tensor),
                ("decoder_output", &dec_tensor),
            ])
            .map_err(|e| coreml_err("joiner predict", e))?;

        // Logits: [1, 1, 1, 1030]
        let (logits, _) = prediction
            .get_f32("var_31")
            .map_err(|e| coreml_err("joiner get var_31", e))?;

        Ok(logits)
    }

    // -----------------------------------------------------------------------
    // TDT greedy search decode
    // -----------------------------------------------------------------------

    /// Decode encoder output frames using TDT greedy search.
    ///
    /// Reference: sherpa-onnx `DecodeOneTDT`
    /// (offline-transducer-greedy-search-nemo-decoder.cc).
    fn tdt_greedy_decode(
        &mut self,
        encoder_features: &[f32],
        encoder_dim: usize,
        num_frames: usize,
    ) -> Result<Vec<i64>> {
        let vocab_size = self.tokens.len();
        let blank_id = self.blank_id;
        let max_tokens_per_frame = 5; // sherpa-onnx TDT default

        let mut tokens = Vec::new();
        let mut blank_count = 0_usize;
        let mut nonblank_count = 0_usize;

        // Compute initial decoder output from blank token
        let mut decoder_out = self.run_decoder(blank_id)?;

        let mut tokens_this_frame = 0;
        let mut t = 0_usize;
        let mut iteration_count = 0_u64;

        while t < num_frames {
            iteration_count += 1;
            if iteration_count > 100_000 {
                warn!(
                    "TDT decode: safety limit reached after {} iterations at frame {}/{}",
                    iteration_count, t, num_frames
                );
                break;
            }

            // Extract single encoder frame t from [1, 1024, total_time] in row-major order.
            // Row-major means time varies fastest: flat[f * total_time + t] = feature f at time t.
            // Verified: flat[0] matches Python but flat[1] = features[0,0,1] (row-major),
            // NOT features[0,1,0] (column-major).
            let total_time = encoder_features.len() / encoder_dim;
            if t >= total_time {
                warn!("Encoder frame {} out of bounds (total_time={})", t, total_time);
                break;
            }
            let encoder_frame: Vec<f32> = (0..encoder_dim)
                .map(|f| encoder_features[f * total_time + t])
                .collect();

            // Run joiner
            let logits = self.run_joiner(&encoder_frame, &decoder_out)?;
            let actual_logit_count =
                logits.len().min(vocab_size + 5); // vocab + 5 TDT durations
            let num_durations = actual_logit_count.saturating_sub(vocab_size);

            let token_logits = &logits[0..vocab_size];
            let duration_logits =
                &logits[vocab_size..vocab_size + num_durations];

            // Debug: log first frame's logits
            if t == 0 && iteration_count == 1 {
                let (max_tok_idx, max_tok_val) = token_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| {
                        a.partial_cmp(b).unwrap()
                    })
                    .unwrap();
                info!(
                    "Frame 0: total_logits={}, vocab={}, durations={}, best_token={}(val={:.3}), blank_val={:.3}, dur={:?}",
                    logits.len(),
                    vocab_size,
                    num_durations,
                    max_tok_idx,
                    max_tok_val,
                    token_logits[blank_id as usize],
                    duration_logits
                );
            }

            // Greedy token selection
            let y = token_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx as i64)
                .unwrap();

            // Greedy duration selection
            let mut skip = if num_durations > 0 {
                duration_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(idx, _)| idx)
                    .unwrap_or(0)
            } else {
                0
            };

            // Statistics
            if y == blank_id {
                blank_count += 1;
            } else {
                nonblank_count += 1;
            }

            // Emit token and update decoder state
            if y != blank_id {
                tokens.push(y);
                decoder_out = self.run_decoder(y)?;
                tokens_this_frame += 1;
            }

            // Skip logic (matches sherpa-onnx C++ exactly)
            if skip > 0 {
                tokens_this_frame = 0;
            }
            if tokens_this_frame >= max_tokens_per_frame {
                tokens_this_frame = 0;
                skip = 1;
            }
            if y == blank_id && skip == 0 {
                tokens_this_frame = 0;
                skip = 1;
            }

            if skip > 0 {
                t += skip;
            }
            // else: stay at same frame to potentially emit more tokens
        }

        info!(
            "TDT decode complete: {} tokens from {} frames ({} blank, {} non-blank, {} iterations)",
            tokens.len(),
            num_frames,
            blank_count,
            nonblank_count,
            iteration_count
        );
        if !tokens.is_empty() {
            let token_strs: Vec<&str> = tokens.iter()
                .map(|&t| self.tokens.get(t as usize).map(|s| s.as_str()).unwrap_or("?"))
                .collect();
            info!("  Tokens: {:?}", token_strs);
        }

        Ok(tokens)
    }

    // -----------------------------------------------------------------------
    // Token conversion
    // -----------------------------------------------------------------------

    /// Convert token IDs to text (identical to OrtRecognizer::tokens_to_text).
    fn tokens_to_text(&self, tokens: &[i64]) -> String {
        tokens
            .iter()
            .filter_map(|&token_id| {
                let idx = token_id as usize;
                if idx < self.tokens.len()
                    && token_id != self.blank_id
                    && token_id != self.unk_id
                {
                    Some(self.tokens[idx].as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .replace('\u{2581}', " ") // Replace BPE underscore with space
            .trim()
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// Standalone helpers
// ---------------------------------------------------------------------------

/// Load the `tokens.txt` vocabulary file.
///
/// Format: `<token_text> <token_id>` per line.
fn load_tokens(model_dir: &Path) -> Result<Vec<String>> {
    let tokens_path = model_dir.join("tokens.txt");
    let contents = std::fs::read_to_string(&tokens_path).map_err(|e| {
        SttError::ModelLoadError(format!(
            "Failed to read {}: {}",
            tokens_path.display(),
            e
        ))
    })?;

    let tokens: Vec<String> = contents
        .lines()
        .map(|line| {
            line.split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect();

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_detection() {
        let cfg_1_1b =
            ModelConfig::detect_from_path(Path::new("/models/parakeet-tdt-1.1b-coreml"));
        assert_eq!(cfg_1_1b.n_mel_features, 80);
        assert_eq!(cfg_1_1b.decoder_hidden_size, 640);

        let cfg_0_6b =
            ModelConfig::detect_from_path(Path::new("/models/parakeet-tdt-0.6b-coreml"));
        assert_eq!(cfg_0_6b.n_mel_features, 128);
        assert_eq!(cfg_0_6b.decoder_hidden_size, 640);
    }
}

} // mod inner

// Re-export the public type at crate level when the feature is active.
#[cfg(all(target_os = "macos", feature = "coreml-native"))]
pub use inner::CoreMLRecognizer;
