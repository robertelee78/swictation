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
    // Chunking constants
    // ---------------------------------------------------------------------------

    /// Encoder's fixed input window: 15 seconds at 16 kHz.
    const MAX_AUDIO_SAMPLES: usize = 15 * 16_000; // 240_000

    /// Chunk size for windowed processing (equals encoder window).
    const CHUNK_SAMPLES: usize = MAX_AUDIO_SAMPLES; // 240_000

    /// Overlap between adjacent chunks (2 seconds) — gives the conformer encoder
    /// acoustic context at chunk boundaries, reducing word-boundary errors.
    const OVERLAP_SAMPLES: usize = 2 * 16_000; // 32_000

    /// Stride between chunk start positions: window minus overlap.
    const STRIDE_SAMPLES: usize = CHUNK_SAMPLES - OVERLAP_SAMPLES; // 208_000

    // ---------------------------------------------------------------------------
    // Decoder state carry-over
    // ---------------------------------------------------------------------------

    /// State carried across chunk boundaries during multi-chunk decoding.
    ///
    /// When audio exceeds the encoder's 15-second window, we split it into
    /// overlapping chunks and decode sequentially. This struct preserves the
    /// decoder LSTM's hidden/cell states and embedding output so that language
    /// model context flows naturally across chunk boundaries.
    struct DecoderCarryState {
        /// LSTM hidden state h: shape [2, 1, hidden_size] flattened.
        /// Length: 2 * hidden_size (1280 for hidden_size=640).
        state_h: Vec<f32>,

        /// LSTM cell state c: shape [2, 1, hidden_size] flattened.
        /// Length: 2 * hidden_size.
        state_c: Vec<f32>,

        /// Decoder embedding output from the last `run_decoder()` call.
        /// Length: hidden_size (640). Empty vec signals "first chunk — bootstrap
        /// from blank_id".
        decoder_out: Vec<f32>,

        /// Last token fed to `run_decoder()`. For the first chunk this is
        /// `blank_id`; for subsequent chunks it is the last emitted non-blank
        /// token (or `blank_id` if the previous chunk emitted nothing).
        last_token: i64,
    }

    impl DecoderCarryState {
        /// Create initial state for the start of a new utterance.
        fn initial(hidden_size: usize, blank_id: i64) -> Self {
            Self {
                state_h: vec![0.0f32; 2 * hidden_size],
                state_c: vec![0.0f32; 2 * hidden_size],
                decoder_out: Vec::new(), // empty = "needs bootstrap"
                last_token: blank_id,
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
    ///
    /// Note: Currently unused — LSTM states appear to already be in the correct
    /// row-major layout without de-interleaving (verified by working single-chunk
    /// decoding). Retained in case a CoreML version change alters the layout.
    #[allow(dead_code)]
    fn deinterleave_lstm_state(data: &[f32], hidden_size: usize) -> Vec<f32> {
        let num_layers = 2;
        let expected = num_layers * hidden_size;
        if data.len() != expected {
            // Not the expected size — return as-is (may already be row-major)
            return data.to_vec();
        }
        let mut result = vec![0.0f32; expected];
        for i in 0..hidden_size {
            result[i] = data[2 * i]; // layer 0
            result[hidden_size + i] = data[2 * i + 1]; // layer 1
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
                    SttError::ModelLoadError("Could not find <blk> token in tokens.txt".to_string())
                })? as i64;

            // Default unk_id=0 is safe: token 0 in Parakeet-TDT is <blk> (blank),
            // already filtered by blank_id. Matches recognizer_ort.rs behavior.
            let unk_id = tokens.iter().position(|t| t == "<unk>").unwrap_or(0) as i64;

            info!(
                "Loaded {} tokens (blank_id={}, unk_id={})",
                tokens.len(),
                blank_id,
                unk_id
            );

            // Load the three CoreML model bundles with All compute units (CPU+GPU+ANE)
            info!("Loading encoder.mlmodelc (compute_units=All)...");
            let encoder = Model::load(model_path.join("encoder.mlmodelc"), ComputeUnits::All)
                .map_err(|e| SttError::ModelLoadError(format!("Failed to load encoder: {}", e)))?;

            info!("Loading decoder.mlmodelc (compute_units=All)...");
            let decoder = Model::load(model_path.join("decoder.mlmodelc"), ComputeUnits::All)
                .map_err(|e| SttError::ModelLoadError(format!("Failed to load decoder: {}", e)))?;

            info!("Loading joiner.mlmodelc (compute_units=All)...");
            let joiner = Model::load(model_path.join("joiner.mlmodelc"), ComputeUnits::All)
                .map_err(|e| SttError::ModelLoadError(format!("Failed to load joiner: {}", e)))?;

            let audio_processor = AudioProcessor::with_mel_features(config.n_mel_features)?;

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
                decoder_state_h: vec![0.0f32; 2 * hidden_size],
                decoder_state_c: vec![0.0f32; 2 * hidden_size],
                config,
            })
        }

        /// Returns `true` -- CoreML always runs on GPU/ANE when available.
        pub fn is_gpu(&self) -> bool {
            true
        }

        /// Recognize speech from raw audio samples (16 kHz, mono, f32).
        ///
        /// Handles audio of any length by splitting into overlapping chunks of
        /// [`CHUNK_SAMPLES`] (15 s) with [`OVERLAP_SAMPLES`] (2 s) overlap.
        /// Decoder LSTM state is carried across chunks so language-model context
        /// is preserved at chunk boundaries.
        pub fn recognize_samples(&mut self, samples: &[f32]) -> Result<String> {
            if samples.is_empty() {
                return Ok(String::new());
            }

            let total_samples = samples.len();
            info!(
                "Processing {} audio samples via CoreML ({:.1}s)",
                total_samples,
                total_samples as f64 / 16_000.0
            );

            // Compute chunk count using stride-based windowing.
            // First chunk starts at 0; subsequent chunks advance by STRIDE_SAMPLES.
            let num_chunks = if total_samples <= CHUNK_SAMPLES {
                1
            } else {
                1 + (total_samples - CHUNK_SAMPLES).div_ceil(STRIDE_SAMPLES)
            };

            info!(
                "Windowed chunking: {} chunk(s), window={}s, overlap={}s, stride={}s",
                num_chunks,
                CHUNK_SAMPLES as f64 / 16_000.0,
                OVERLAP_SAMPLES as f64 / 16_000.0,
                STRIDE_SAMPLES as f64 / 16_000.0
            );

            // Initialize decoder carry state ONCE for the entire utterance.
            let hidden_size = self.config.decoder_hidden_size;
            let mut carry = DecoderCarryState::initial(hidden_size, self.blank_id);
            let mut all_tokens: Vec<i64> = Vec::new();

            for chunk_idx in 0..num_chunks {
                let start = chunk_idx * STRIDE_SAMPLES;
                let end = (start + CHUNK_SAMPLES).min(total_samples);
                let chunk_samples = &samples[start..end];
                let actual_length = chunk_samples.len();

                // Skip tiny last chunks entirely if their audio falls within the
                // overlap region already encoded by the previous chunk.
                if chunk_idx > 0 && actual_length <= OVERLAP_SAMPLES {
                    info!(
                        "Chunk {}/{}: skipping — {} samples within overlap region",
                        chunk_idx + 1,
                        num_chunks,
                        actual_length
                    );
                    continue;
                }

                // Pad chunk to the encoder's fixed window size.
                let audio = if actual_length >= MAX_AUDIO_SAMPLES {
                    chunk_samples[..MAX_AUDIO_SAMPLES].to_vec()
                } else {
                    let mut padded = vec![0.0f32; MAX_AUDIO_SAMPLES];
                    padded[..actual_length].copy_from_slice(chunk_samples);
                    padded
                };

                info!(
                    "Chunk {}/{}: samples[{}..{}] ({} samples, {:.1}s)",
                    chunk_idx + 1,
                    num_chunks,
                    start,
                    end,
                    actual_length,
                    actual_length as f64 / 16_000.0
                );

                // Run the fused preprocessor+encoder on this chunk.
                let (encoder_features, encoder_dim, valid_frames) =
                    self.run_encoder(&audio, actual_length)?;

                if valid_frames == 0 && actual_length > 0 {
                    warn!(
                        "Chunk {}/{}: encoder returned 0 valid frames for {} samples of audio",
                        chunk_idx + 1,
                        num_chunks,
                        actual_length
                    );
                }

                // INVARIANT: Load carry state into self BEFORE any run_decoder() call.
                // This ensures the LSTM states from the previous chunk (or initial zeros
                // for the first chunk) are used. After this function returns, self.decoder_state_h/c
                // will hold the final state of the last chunk — overwritten on the next call
                // to recognize_samples() by a fresh DecoderCarryState::initial().
                self.decoder_state_h = carry.state_h.clone();
                self.decoder_state_c = carry.state_c.clone();

                // Compute how many encoder frames to skip (the overlap region).
                // First chunk: skip nothing. Subsequent chunks: skip frames that
                // correspond to the 2-second overlap already decoded by the prior chunk.
                let skip_frames = if chunk_idx == 0 {
                    0
                } else {
                    // The encoder downsampling ratio is constant (MAX_AUDIO_SAMPLES -> ~188 frames)
                    // regardless of actual audio length, so use the fixed window size as denominator.
                    let skip = (OVERLAP_SAMPLES as f64 / MAX_AUDIO_SAMPLES as f64
                        * valid_frames as f64)
                        .round() as usize;
                    // Ensure we decode at least 1 frame.
                    skip.min(valid_frames.saturating_sub(1))
                };

                // Decode this chunk's encoder output.
                let (chunk_tokens, final_carry) = self.tdt_greedy_decode(
                    &encoder_features,
                    encoder_dim,
                    valid_frames,
                    &carry,
                    skip_frames,
                )?;

                info!(
                    "Chunk {}/{}: {} tokens (skipped {} overlap frames)",
                    chunk_idx + 1,
                    num_chunks,
                    chunk_tokens.len(),
                    skip_frames
                );

                all_tokens.extend(chunk_tokens);
                carry = final_carry;
            }

            let text = self.tokens_to_text(&all_tokens);
            Ok(text)
        }

        /// Recognize speech from an audio file (WAV, MP3, FLAC).
        pub fn recognize_file<P: AsRef<Path>>(&mut self, audio_path: P) -> Result<String> {
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
            let targets_tensor = BorrowedTensor::from_i32(&targets_i32, &[1, 1])
                .map_err(|e| coreml_err("decoder targets tensor", e))?;

            let target_length_i32 = [1i32];
            let target_length_tensor = BorrowedTensor::from_i32(&target_length_i32, &[1])
                .map_err(|e| coreml_err("decoder target_length tensor", e))?;

            let h_tensor = BorrowedTensor::from_f32(&self.decoder_state_h, &[2, 1, hidden_size])
                .map_err(|e| coreml_err("decoder h tensor", e))?;

            let c_tensor = BorrowedTensor::from_f32(&self.decoder_state_c, &[2, 1, hidden_size])
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

            // One-time debug log to verify LSTM state layout (row-major vs column-major).
            // Row-major [2, 1, 640]: flat = [L0[0], L0[1], ..., L0[639], L1[0], ..., L1[639]]
            // Column-major interleaved: flat = [L0[0], L1[0], L0[1], L1[1], ...]
            // Python reference (blank_id=1024): h[0,0,0]=-0.040, h[0,0,1]=0.014, h[1,0,0]=-0.154
            // Row-major:  flat[0]=-0.040, flat[1]=0.014,  flat[640]=-0.154
            // Col-major:  flat[0]=-0.040, flat[1]=-0.154, flat[2]=0.014
            if new_h.len() > hidden_size {
                debug!(
                    "LSTM h layout check: h[0]={:.4}, h[1]={:.4}, h[{}]={:.4} (row-major: h[1] should be ~0.014, col-major: h[1] should be ~-0.154)",
                    new_h[0], new_h[1], hidden_size, new_h[hidden_size]
                );
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
        fn run_joiner(&self, encoder_frame: &[f32], decoder_out: &[f32]) -> Result<Vec<f32>> {
            let enc_dim = encoder_frame.len();
            let dec_dim = decoder_out.len();

            let enc_tensor = BorrowedTensor::from_f32(encoder_frame, &[1, 1, enc_dim])
                .map_err(|e| coreml_err("joiner encoder tensor", e))?;

            let dec_tensor = BorrowedTensor::from_f32(decoder_out, &[1, 1, dec_dim])
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
        /// Supports multi-chunk decoding: `prior_state` carries decoder LSTM
        /// context from the previous chunk, and `skip_frames` skips the overlap
        /// region at the start of subsequent chunks.
        ///
        /// Reference: sherpa-onnx `DecodeOneTDT`
        /// (offline-transducer-greedy-search-nemo-decoder.cc).
        fn tdt_greedy_decode(
            &mut self,
            encoder_features: &[f32],
            encoder_dim: usize,
            num_frames: usize,
            prior_state: &DecoderCarryState,
            skip_frames: usize,
        ) -> Result<(Vec<i64>, DecoderCarryState)> {
            let vocab_size = self.tokens.len();
            let blank_id = self.blank_id;
            let max_tokens_per_frame = 5; // sherpa-onnx TDT default

            let mut tokens = Vec::new();
            let mut blank_count = 0_usize;
            let mut nonblank_count = 0_usize;

            // Bootstrap decoder output: first chunk computes from blank_id;
            // subsequent chunks reuse the embedding from the previous chunk.
            let mut decoder_out = if prior_state.decoder_out.is_empty() {
                self.run_decoder(blank_id)?
            } else {
                prior_state.decoder_out.clone()
            };

            let mut tokens_this_frame = 0;
            let mut t = skip_frames; // skip overlap frames on subsequent chunks
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

                // Extract single encoder frame t from [1, encoder_dim, total_time] in row-major order.
                // Row-major: flat[f * total_time + t] = feature f at time t.
                if encoder_dim == 0 {
                    warn!("encoder_dim is 0 — cannot decode");
                    break;
                }
                let total_time = encoder_features.len() / encoder_dim;
                if t >= total_time {
                    warn!(
                        "Encoder frame {} out of bounds (total_time={})",
                        t, total_time
                    );
                    break;
                }
                let encoder_frame: Vec<f32> = (0..encoder_dim)
                    .map(|f| encoder_features[f * total_time + t])
                    .collect();

                // Run joiner
                let logits = self.run_joiner(&encoder_frame, &decoder_out)?;
                let num_tdt_durations = 5; // TDT duration classes: 0,1,2,3,4
                let token_end = vocab_size.min(logits.len());
                let duration_end = (vocab_size + num_tdt_durations).min(logits.len());
                let num_durations = duration_end.saturating_sub(vocab_size);

                let token_logits = &logits[0..token_end];
                let duration_logits = &logits[vocab_size..duration_end];

                // Greedy token selection (NaN-safe comparison)
                let y = token_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(idx, _)| idx as i64)
                    .unwrap_or(blank_id);

                // Greedy duration selection (NaN-safe comparison)
                let mut skip = if num_durations > 0 {
                    duration_logits
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap_or(0)
                } else {
                    // No duration logits available — advance by 1 frame to prevent
                    // infinite loop (the while loop only advances when skip > 0).
                    1
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
                "TDT decode complete: {} tokens from {} frames (started at frame {}, {} blank, {} non-blank, {} iterations)",
                tokens.len(),
                num_frames,
                skip_frames,
                blank_count,
                nonblank_count,
                iteration_count
            );
            if !tokens.is_empty() {
                let token_strs: Vec<&str> = tokens
                    .iter()
                    .map(|&t| {
                        self.tokens
                            .get(t as usize)
                            .map(|s| s.as_str())
                            .unwrap_or("?")
                    })
                    .collect();
                info!("  Tokens: {:?}", token_strs);
            }

            // Package final state for the next chunk.
            let final_carry = DecoderCarryState {
                state_h: self.decoder_state_h.clone(),
                state_c: self.decoder_state_c.clone(),
                decoder_out,
                last_token: if tokens.is_empty() {
                    prior_state.last_token
                } else {
                    *tokens.last().unwrap()
                },
            };

            Ok((tokens, final_carry))
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
            SttError::ModelLoadError(format!("Failed to read {}: {}", tokens_path.display(), e))
        })?;

        let tokens: Vec<String> = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.split_whitespace().next().unwrap_or("").to_string())
            .collect();

        Ok(tokens)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_decoder_carry_state_initial() {
            let hidden_size = 640;
            let blank_id = 1024_i64;
            let state = DecoderCarryState::initial(hidden_size, blank_id);

            // LSTM states are zero-initialized with correct length
            assert_eq!(state.state_h.len(), 2 * hidden_size);
            assert_eq!(state.state_c.len(), 2 * hidden_size);
            assert!(state.state_h.iter().all(|&v| v == 0.0));
            assert!(state.state_c.iter().all(|&v| v == 0.0));

            // decoder_out is empty (sentinel for "needs bootstrap")
            assert!(state.decoder_out.is_empty());

            // last_token is blank_id
            assert_eq!(state.last_token, blank_id);
        }

        #[test]
        fn test_chunking_constants() {
            assert_eq!(MAX_AUDIO_SAMPLES, 240_000);
            assert_eq!(CHUNK_SAMPLES, 240_000);
            assert_eq!(OVERLAP_SAMPLES, 32_000);
            assert_eq!(STRIDE_SAMPLES, 208_000);
            assert_eq!(CHUNK_SAMPLES - OVERLAP_SAMPLES, STRIDE_SAMPLES);
        }

        /// Helper: compute num_chunks for a given total_samples (mirrors recognize_samples logic).
        fn compute_num_chunks(total_samples: usize) -> usize {
            if total_samples == 0 {
                0
            } else if total_samples <= CHUNK_SAMPLES {
                1
            } else {
                1 + (total_samples - CHUNK_SAMPLES).div_ceil(STRIDE_SAMPLES)
            }
        }

        #[test]
        fn test_chunk_count_calculation() {
            // Empty
            assert_eq!(compute_num_chunks(0), 0);
            // Single sample
            assert_eq!(compute_num_chunks(1), 1);
            // Short audio (5 seconds)
            assert_eq!(compute_num_chunks(80_000), 1);
            // Just under one full window
            assert_eq!(compute_num_chunks(239_999), 1);
            // Exactly one full window
            assert_eq!(compute_num_chunks(240_000), 1);
            // One sample over — needs second chunk
            assert_eq!(compute_num_chunks(240_001), 2);
            // Exactly stride + window (one full stride past first chunk)
            assert_eq!(compute_num_chunks(STRIDE_SAMPLES + CHUNK_SAMPLES), 2);
            // 30 seconds (480,000 samples): chunk0=[0..240k], chunk1=[208k..448k], chunk2=[416k..480k]
            assert_eq!(compute_num_chunks(480_000), 3);
            // 45 seconds (720,000 samples)
            // 1 + ceil((720000-240000)/208000) = 1 + ceil(480000/208000) = 1 + 3 = 4
            assert_eq!(compute_num_chunks(720_000), 4);
            // 60 seconds (960,000 samples)
            // 1 + ceil((960000-240000)/208000) = 1 + ceil(720000/208000) = 1 + 4 = 5
            assert_eq!(compute_num_chunks(960_000), 5);
        }

        #[test]
        fn test_chunk_boundaries() {
            let total_samples = 480_000; // 30 seconds
            let num_chunks = compute_num_chunks(total_samples);
            assert_eq!(num_chunks, 3);

            // Chunk 0: start=0, end=240000
            let start0 = 0; // chunk 0 always starts at 0
            let end0 = (start0 + CHUNK_SAMPLES).min(total_samples);
            assert_eq!(start0, 0);
            assert_eq!(end0, 240_000);

            // Chunk 1: start=208000, end=448000 (with overlap)
            let start1 = STRIDE_SAMPLES;
            let end1 = (start1 + CHUNK_SAMPLES).min(total_samples);
            assert_eq!(start1, 208_000);
            assert_eq!(end1, 448_000);
            // Overlap between chunk 0 and 1
            assert_eq!(end0 - start1, OVERLAP_SAMPLES);

            // Chunk 2: start=416000, end=480000 (partial last chunk)
            let start2 = 2 * STRIDE_SAMPLES;
            let end2 = (start2 + CHUNK_SAMPLES).min(total_samples);
            assert_eq!(start2, 416_000);
            assert_eq!(end2, 480_000);
            // Last chunk has 64,000 samples (4 seconds) — larger than overlap, so decoded
            assert_eq!(end2 - start2, 64_000);
            assert!(end2 - start2 > OVERLAP_SAMPLES);
        }

        #[test]
        fn test_chunk_boundaries_partial_last() {
            // 20 seconds = 320,000 samples
            // 1 + ceil((320000-240000)/208000) = 1 + ceil(80000/208000) = 1 + 1 = 2
            let total_samples = 320_000;
            let num_chunks = compute_num_chunks(total_samples);
            assert_eq!(num_chunks, 2);

            let start1 = STRIDE_SAMPLES;
            let end1 = (start1 + CHUNK_SAMPLES).min(total_samples);
            assert_eq!(start1, 208_000);
            assert_eq!(end1, 320_000);
            // Last chunk has 112,000 samples (7 seconds) — larger than OVERLAP_SAMPLES, so decoded
            assert_eq!(end1 - start1, 112_000);
            assert!(end1 - start1 > OVERLAP_SAMPLES);
        }

        #[test]
        fn test_skip_tiny_last_chunk() {
            // 15.5 seconds = 248,000 samples
            // Chunk 0: [0..240000], Chunk 1: [208000..248000] = 40,000 samples
            // 40,000 > OVERLAP_SAMPLES (32,000), so it's NOT skipped
            let total = 248_000;
            let start1 = STRIDE_SAMPLES;
            let actual1 = total.min(start1 + CHUNK_SAMPLES) - start1;
            assert_eq!(actual1, 40_000);
            assert!(actual1 > OVERLAP_SAMPLES); // not skipped

            // 15.1 seconds = 241,600 samples
            // Chunk 1: [208000..241600] = 33,600 samples > 32,000 — NOT skipped
            let total2 = 241_600;
            let actual2 = total2.min(start1 + CHUNK_SAMPLES) - start1;
            assert_eq!(actual2, 33_600);
            assert!(actual2 > OVERLAP_SAMPLES);

            // Edge: audio where last chunk = exactly overlap size
            // total = 208000 + 32000 = 240000 — that's exactly 1 chunk, no second chunk
            let total3 = STRIDE_SAMPLES + OVERLAP_SAMPLES;
            assert_eq!(total3, CHUNK_SAMPLES);
            assert_eq!(compute_num_chunks(total3), 1);

            // Verify the skip predicate fires: chunk_idx > 0 AND actual_length <= OVERLAP_SAMPLES
            // total = 240001 → 2 chunks. Chunk 1: start=208000, end=240001, len=32001
            // 32001 > 32000 → NOT skipped (barely). This is the tightest non-skip case.
            let total4 = CHUNK_SAMPLES + 1;
            let actual4 = total4.min(STRIDE_SAMPLES + CHUNK_SAMPLES) - STRIDE_SAMPLES;
            assert_eq!(actual4, 32_001);
            assert!(actual4 > OVERLAP_SAMPLES); // not skipped

            // total = 240000 + 0 = 240000 → 1 chunk, skip never tested. But:
            // Simulate: if somehow chunk_idx=1 with actual_length=100, the skip FIRES.
            let tiny_actual = 100_usize;
            let chunk_idx_sim = 1_usize;
            let should_skip = chunk_idx_sim > 0 && tiny_actual <= OVERLAP_SAMPLES;
            assert!(
                should_skip,
                "Skip predicate must fire for tiny chunks on non-first chunk"
            );

            // And the predicate does NOT fire for chunk 0 even with tiny audio
            let should_skip_first = 0 > 0 && tiny_actual <= OVERLAP_SAMPLES;
            assert!(!should_skip_first, "Skip must never fire on chunk 0");
        }

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
