//! Direct ONNX Runtime recognizer for Parakeet-TDT 1.1B model
//!
//! Bypasses sherpa-rs to work around the SessionOptions bug with external weights.
//! This implementation uses the `ort` crate directly for full control over ONNX Runtime.
//!
//! ## Environment Setup
//!
//! The `ort` crate requires ONNX Runtime 1.22+ for version 2.0.0-rc.10.
//! Set the `ORT_DYLIB_PATH` environment variable to point to a compatible library:
//!
//! ```bash
//! export ORT_DYLIB_PATH=/path/to/libonnxruntime.so.1.23.2
//! ```
//!
//! For example, with onnxruntime-gpu installed via pip:
//! ```bash
//! export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
//! ```

use crate::audio::AudioProcessor;
use crate::error::{Result, SttError};
use ndarray::{s, Array1, Array2, Array3};
use ort::{
    execution_providers as ep,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Direct ONNX Runtime recognizer for Parakeet-TDT models (0.6B and 1.1B)
pub struct OrtRecognizer {
    encoder: Session,
    decoder: Session,
    joiner: Session,
    tokens: Vec<String>,
    blank_id: i64,
    unk_id: i64,
    model_path: PathBuf,
    audio_processor: AudioProcessor,
    // Decoder RNN states (2, batch, 640)
    decoder_state1: Option<Array3<f32>>,
    decoder_state2: Option<Array3<f32>>,
    // Whether encoder expects transposed input (batch, features, time) vs (batch, time, features)
    transpose_input: bool,
}

impl OrtRecognizer {
    /// Create new recognizer from model directory
    ///
    /// # Arguments
    /// * `model_dir` - Path to directory containing encoder.onnx, decoder.onnx, joiner.onnx, tokens.txt
    /// * `use_gpu` - Enable CUDA execution provider
    ///
    /// # Example
    /// ```no_run
    /// use swictation_stt::recognizer_ort::OrtRecognizer;
    ///
    /// let recognizer = OrtRecognizer::new(
    ///     "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted",
    ///     true
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new<P: AsRef<Path>>(model_dir: P, use_gpu: bool) -> Result<Self> {
        let model_path = model_dir.as_ref().to_path_buf();

        info!("Loading 1.1B Parakeet-TDT model with direct ONNX Runtime");
        info!("Model directory: {}", model_path.display());

        // Load tokens and find special token IDs
        let tokens = Self::load_tokens(&model_path)?;

        // Find blank token (usually "<blk>")
        let blank_id = tokens.iter()
            .position(|t| t == "<blk>" || t == "<blank>")
            .ok_or_else(|| SttError::ModelLoadError("Could not find <blk> token".to_string()))? as i64;

        // Find unk token (usually "<unk>")
        let unk_id = tokens.iter()
            .position(|t| t == "<unk>")
            .unwrap_or(0) as i64;

        info!("Loaded {} tokens (blank_id={}, unk_id={})", tokens.len(), blank_id, unk_id);

        // Configure ONNX Runtime session options
        let mut session_builder = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set optimization level: {}", e)))?
            .with_intra_threads(4)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set intra threads: {}", e)))?;

        if use_gpu {
            info!("Enabling CUDA execution provider");
            session_builder = session_builder
                .with_execution_providers([
                    ep::CUDAExecutionProvider::default().build(),
                    ep::CPUExecutionProvider::default().build(),
                ])
                .map_err(|e| SttError::ModelLoadError(format!("Failed to set execution providers: {}", e)))?;
        } else {
            info!("Using CPU execution provider");
        }

        // Helper function to find model file (try .int8.onnx first, then .onnx)
        let find_model_file = |name: &str| -> std::result::Result<PathBuf, SttError> {
            let int8_path = model_path.join(format!("{}.int8.onnx", name));
            if int8_path.exists() {
                info!("Using INT8 quantized model: {}.int8.onnx", name);
                return Ok(int8_path);
            }
            let onnx_path = model_path.join(format!("{}.onnx", name));
            if onnx_path.exists() {
                info!("Using FP32 model: {}.onnx", name);
                return Ok(onnx_path);
            }
            Err(SttError::ModelLoadError(format!(
                "Could not find {}.onnx or {}.int8.onnx in {:?}",
                name, name, model_path
            )))
        };

        // Load the three ONNX models (external weights load automatically!)
        info!("Loading encoder...");
        let encoder_path = find_model_file("encoder")?;
        let encoder = session_builder
            .commit_from_file(&encoder_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load encoder: {}", e)))?;
        info!("✓ Encoder loaded (external weights automatically loaded)");

        info!("Loading decoder...");
        let decoder_path = find_model_file("decoder")?;
        let decoder = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create decoder session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set decoder optimization: {}", e)))?
            .commit_from_file(&decoder_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load decoder: {}", e)))?;
        info!("✓ Decoder loaded");

        info!("Loading joiner...");
        let joiner_path = find_model_file("joiner")?;
        let joiner = Session::builder()
            .map_err(|e| SttError::ModelLoadError(format!("Failed to create joiner session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to set joiner optimization: {}", e)))?
            .commit_from_file(&joiner_path)
            .map_err(|e| SttError::ModelLoadError(format!("Failed to load joiner: {}", e)))?;
        info!("✓ Joiner loaded");

        // Detect encoder input format by inspecting input shape
        // 0.6B: (batch, 128 features, time) - NEEDS transpose
        // 1.1B: (batch, 80 time, features) - NO transpose
        let encoder_inputs = encoder.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>();
        info!("Encoder inputs: {:?}", encoder_inputs);

        let audio_signal_input = encoder.inputs.iter()
            .find(|i| i.name == "audio_signal")
            .ok_or_else(|| SttError::ModelLoadError("Encoder missing audio_signal input".to_string()))?;

        // Extract tensor dimensions from the input metadata
        // ort 2.0.0-rc.10 API: input.input_type contains shape information
        info!("Audio signal input type: {:?}", audio_signal_input.input_type);

        // For now, use a simple heuristic: check if tokens.txt exists to determine model size
        // 1.1B models have 1025 tokens, 0.6B models have 1025 tokens too, so check model dir name
        let transpose_input = if model_path.to_string_lossy().contains("1.1b") ||
                                  model_path.to_string_lossy().contains("1-1b") {
            info!("Detected 1.1B model from path - using natural format (no transpose)");
            false
        } else if model_path.to_string_lossy().contains("0.6b") ||
                   model_path.to_string_lossy().contains("0-6b") {
            info!("Detected 0.6B model from path - using transposed format");
            true
        } else {
            // Fallback: default to transpose for safety (0.6B is more common)
            info!("Could not determine model version from path, defaulting to transpose");
            true
        };

        let audio_processor = AudioProcessor::new()?;

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
            transpose_input,
        })
    }

    /// Load tokens from tokens.txt
    ///
    /// Format: "<token_text> <token_id>" per line
    /// Example: "<blk> 1024"
    fn load_tokens(model_dir: &Path) -> Result<Vec<String>> {
        let tokens_path = model_dir.join("tokens.txt");
        let contents = fs::read_to_string(&tokens_path)
            .map_err(|e| SttError::ModelLoadError(
                format!("Failed to read tokens.txt: {}", e)
            ))?;

        // Parse each line as "<token_text> <token_id>" and extract token_text
        let tokens: Vec<String> = contents
            .lines()
            .map(|line| {
                // Split on whitespace and take first part (token text)
                line.split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .collect();

        Ok(tokens)
    }

    /// Test encoder inference with dummy input
    ///
    /// This method is for validation purposes only - to prove the 1.1B model
    /// loads and runs successfully with the ort crate.
    ///
    /// # Returns
    /// Confirmation that inference completed
    pub fn test_encoder_inference(&mut self) -> Result<String> {
        info!("Running test inference with dummy input...");

        // Create dummy input using tuple form (shape, data)
        use ort::value::Tensor;

        // audio_signal: (batch=1, num_frames=80, num_features=128)
        let audio_signal_data: Vec<f32> = vec![0.0; 1 * 80 * 128];
        let audio_signal = Tensor::from_array((vec![1usize, 80, 128], audio_signal_data.into_boxed_slice()))
            .map_err(|e| SttError::InferenceError(format!("Failed to create audio_signal tensor: {}", e)))?;

        // length: (batch=1,)
        let length_data: Vec<i64> = vec![80];
        let length_tensor = Tensor::from_array((vec![1usize], length_data.into_boxed_slice()))
            .map_err(|e| SttError::InferenceError(format!("Failed to create length tensor: {}", e)))?;

        // Get output info before running
        let output_names: Vec<_> = self.encoder.outputs.iter().map(|o| o.name.clone()).collect();

        // Run encoder
        let _outputs = self.encoder.run(ort::inputs!["audio_signal" => audio_signal, "length" => length_tensor])
            .map_err(|e| SttError::InferenceError(format!("Encoder inference failed: {}", e)))?;

        Ok(format!("✅ Encoder inference successful! Outputs: {}", output_names.join(", ")))
    }

    /// Recognize speech from audio file
    ///
    /// Full implementation with mel-spectrogram extraction and greedy search decoding
    ///
    /// # Arguments
    /// * `audio_path` - Path to audio file (WAV, MP3, FLAC)
    ///
    /// # Returns
    /// Transcribed text
    pub fn recognize_file<P: AsRef<Path>>(&mut self, audio_path: P) -> Result<String> {
        info!("Loading audio file: {}", audio_path.as_ref().display());

        // Load and process audio
        let samples = self.audio_processor.load_audio(&audio_path)?;
        info!("Loaded {} audio samples", samples.len());

        // Debug: Audio statistics
        let audio_min = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let audio_max = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let audio_mean = samples.iter().sum::<f32>() / samples.len() as f32;
        debug!("Audio stats: min={:.6}, max={:.6}, mean={:.6}", audio_min, audio_max, audio_mean);

        // Extract mel-spectrogram features
        let features = self.audio_processor.extract_mel_features(&samples)?;
        info!("Extracted features: {:?}", features.shape());

        // Debug: Mel-spectrogram statistics
        let features_flat: Vec<f32> = features.iter().copied().collect();
        let mel_min = features_flat.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let mel_max = features_flat.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let mel_mean = features_flat.iter().sum::<f32>() / features_flat.len() as f32;
        debug!("Mel-spectrogram stats: min={:.6}, max={:.6}, mean={:.6}", mel_min, mel_max, mel_mean);

        // Show first and middle frames of mel-spectrogram
        if features.nrows() > 0 {
            debug!("First frame (first 10 values): {:?}", &features.row(0).as_slice().unwrap()[..10.min(features.ncols())]);

            // Show middle frame to see if it's different
            let mid_frame = features.nrows() / 2;
            if mid_frame > 0 {
                debug!("Middle frame {} (first 10 values): {:?}", mid_frame, &features.row(mid_frame).as_slice().unwrap()[..10.min(features.ncols())]);
            }

            // Check if features need normalization (typical for NeMo models)
            debug!("Note: NVIDIA NeMo models typically expect normalized features (mean=0, std=1)");
            debug!("Current features are log-mel without normalization");
        }

        // Chunk features into 80-frame segments
        let chunks = self.audio_processor.chunk_features(&features);
        info!("Split into {} chunks of 80 frames", chunks.len());

        // Run greedy search decoder
        let text = self.greedy_search_decode(&chunks)?;

        Ok(text)
    }

    /// Greedy search decoder implementation
    ///
    /// Implements the transducer decoding loop:
    /// 1. Encoder processes acoustic features
    /// 2. Decoder maintains token history state
    /// 3. Joiner combines encoder/decoder outputs
    /// 4. Greedy selection picks highest probability token
    /// 5. Loop until blank or end-of-sequence
    fn greedy_search_decode(&mut self, chunks: &[Array2<f32>]) -> Result<String> {
        let mut all_tokens = Vec::new();

        // Reset decoder states at the start
        self.decoder_state1 = None;
        self.decoder_state2 = None;

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            debug!("Processing chunk {}/{}", chunk_idx + 1, chunks.len());

            // Run encoder
            let encoder_out = self.run_encoder(chunk)?;
            debug!("Encoder output shape: {:?}", encoder_out.shape());

            // Decode each frame with greedy search
            let chunk_tokens = self.decode_frames(&encoder_out)?;
            all_tokens.extend(chunk_tokens);
        }

        // Convert tokens to text
        let text = self.tokens_to_text(&all_tokens);

        Ok(text)
    }

    /// Run encoder on feature chunk
    ///
    /// Automatically detects and applies correct input format based on model:
    /// - 0.6B models: (batch, 128 features, 80 time) - TRANSPOSED
    /// - 1.1B models: (batch, 80 time, features) - NOT TRANSPOSED
    /// Detection happens in constructor based on encoder input shape.
    fn run_encoder(&mut self, features: &Array2<f32>) -> Result<Array3<f32>> {
        // Prepare input tensors
        let batch_size = 1;
        let num_frames = features.nrows();
        let num_features = features.ncols();

        // CRITICAL: Encoder chunk size is FIXED at 80 frames
        if num_frames != 80 {
            return Err(SttError::InferenceError(format!(
                "Encoder expects exactly 80 frames per chunk, got {}",
                num_frames
            )));
        }

        // Conditionally apply transpose based on detected model format
        let (shape, audio_data) = if self.transpose_input {
            // TRANSPOSE FOR 0.6B: (batch, num_features=128, num_frames=80)
            // Official sherpa-onnx 0.6B models expect features FIRST, then time
            // Layout: Row-major transposed - all frames for feature 0, then feature 1, etc.
            debug!("Using TRANSPOSED input format (0.6B): (batch, features={}, time={})", num_features, num_frames);
            let mut data = Vec::with_capacity(batch_size * num_frames * num_features);
            for col_idx in 0..num_features {
                for row in features.outer_iter() {
                    data.push(row[col_idx]);
                }
            }
            (vec![batch_size, num_features, num_frames], data)
        } else {
            // NO TRANSPOSE FOR 1.1B: (batch, num_frames=80, num_features)
            // 1.1B exported models expect time FIRST, then features
            // Layout: Row-major natural order
            debug!("Using NATURAL input format (1.1B): (batch, time={}, features={})", num_frames, num_features);
            let data: Vec<f32> = features.iter().copied().collect();
            (vec![batch_size, num_frames, num_features], data)
        };

        let audio_signal = Tensor::from_array((shape, audio_data.into_boxed_slice()))
        .map_err(|e| SttError::InferenceError(format!("Failed to create audio tensor: {}", e)))?;

        // length: (batch=1,)
        let length_data = vec![num_frames as i64];
        let length_tensor = Tensor::from_array((vec![batch_size], length_data.into_boxed_slice()))
            .map_err(|e| SttError::InferenceError(format!("Failed to create length tensor: {}", e)))?;

        // Run encoder
        let outputs = self
            .encoder
            .run(ort::inputs!["audio_signal" => audio_signal, "length" => length_tensor])
            .map_err(|e| SttError::InferenceError(format!("Encoder inference failed: {}", e)))?;

        // Extract encoder output (first output is the encoded features)
        let encoder_out_tensor = &outputs[0];
        let (shape, data) = encoder_out_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| SttError::InferenceError(format!("Failed to extract encoder output: {}", e)))?;

        // Convert to ndarray - shape should be (batch, encoder_dim, num_frames)
        let encoder_out = Array3::from_shape_vec(
            (shape[0] as usize, shape[1] as usize, shape[2] as usize),
            data.to_vec(),
        )
        .map_err(|e| SttError::InferenceError(format!("Failed to reshape encoder output: {}", e)))?;

        // Debug: Encoder output statistics
        let enc_min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let enc_max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let enc_mean = data.iter().sum::<f32>() / data.len() as f32;
        debug!("Encoder output stats: min={:.6}, max={:.6}, mean={:.6}", enc_min, enc_max, enc_mean);

        Ok(encoder_out)
    }

    /// Decode frames using greedy search with TDT duration prediction
    ///
    /// Following sherpa-onnx C++ TDT implementation:
    /// - Process encoder output ONE frame at a time
    /// - Joiner outputs token logits AND duration logits
    /// - Use duration prediction for frame skipping
    /// - Loop until blank or max symbols per frame
    fn decode_frames(&mut self, encoder_out: &Array3<f32>) -> Result<Vec<i64>> {
        // Encoder output shape: (batch, encoder_dim, num_frames)
        let encoder_dim = encoder_out.shape()[1];
        let num_frames = encoder_out.shape()[2];
        let vocab_size = self.tokens.len();
        let blank_id = self.blank_id;

        let mut tokens = Vec::new();
        let mut frame_indices = Vec::new();
        let mut durations = Vec::new();

        // Initialize decoder with blank token
        let mut decoder_state: Vec<i64> = vec![blank_id];

        let max_symbols_per_frame = 5;  // sherpa-onnx uses 5
        let mut t = 0;  // Current frame index

        while t < num_frames {
            let mut symbols_this_frame = 0;
            let mut skip = 0;

            // Extract single encoder frame: (encoder_dim, 1) reshaped to (1, encoder_dim, 1)
            let encoder_frame = encoder_out.slice(s![0, .., t]).to_owned();

            loop {
                // Run decoder with current state
                let decoder_out = self.run_decoder(&decoder_state)?;

                // Run joiner to get logits (token + duration)
                let logits = self.run_joiner(&encoder_frame, &decoder_out)?;

                // Split logits into token logits and duration logits
                // sherpa-onnx C++: const float *token_logits = p_logit;
                //                 const float *duration_logits = p_logit + vocab_size;
                let token_logits = &logits.as_slice().unwrap()[0..vocab_size];
                let duration_logits = &logits.as_slice().unwrap()[vocab_size..];

                // Greedy selection for token
                let (y, y_logit) = token_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .unwrap();
                let y = y as i64;

                // Greedy selection for duration (can be 0)
                let duration = if !duration_logits.is_empty() {
                    duration_logits
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(idx, _)| idx)
                        .unwrap_or(0)
                } else {
                    0
                };

                debug!("Frame {}: token={} ('{}'), logit={:.4}, blank={}, duration={}",
                    t, y,
                    if y < self.tokens.len() as i64 { &self.tokens[y as usize] } else { "???" },
                    y_logit, blank_id, duration);

                if y != blank_id {
                    // Non-blank token - emit it
                    tokens.push(y);
                    frame_indices.push(t);
                    durations.push(duration);

                    // Update decoder state to this token
                    decoder_state = vec![y];

                    symbols_this_frame += 1;

                    // Don't advance frame yet - continue emitting from same frame
                } else {
                    // Blank token - advance frame
                    // Use duration prediction if we emitted tokens
                    if duration > 0 && !tokens.is_empty() {
                        skip = duration;
                    } else {
                        skip = 1;
                    }
                    break;
                }

                // Safety: max symbols per frame reached
                if symbols_this_frame >= max_symbols_per_frame {
                    debug!("Max symbols per frame ({}) reached at frame {}", max_symbols_per_frame, t);
                    skip = 1;
                    break;
                }
            }

            // Advance frame by duration skip
            t += skip.max(1);  // Always advance at least 1 frame
        }

        debug!("Decoded {} tokens with {} frame indices", tokens.len(), frame_indices.len());

        Ok(tokens)
    }

    /// Run decoder with token history and RNN states
    ///
    /// Decoder inputs:
    /// - targets: (batch, seq_len) - token history (int32)
    /// - target_length: (batch,) - length of token history (int32)
    /// - states.1: (2, batch, 640) - RNN state (float32)
    /// - onnx::Slice_3: (2, 1, 640) - additional state (float32)
    fn run_decoder(&mut self, tokens: &[i64]) -> Result<Array1<f32>> {
        let batch_size = 1;
        let seq_len = tokens.len();

        // Prepare targets tensor: (batch, seq_len) - convert i64 to i32
        let targets_i32: Vec<i32> = tokens.iter().map(|&t| t as i32).collect();
        let targets = Tensor::from_array((
            vec![batch_size, seq_len],
            targets_i32.into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create targets tensor: {}", e)))?;

        // Prepare target_length tensor: (batch,)
        let target_length = Tensor::from_array((
            vec![batch_size],
            vec![seq_len as i32].into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create target_length tensor: {}", e)))?;

        // Initialize or reuse decoder states
        if self.decoder_state1.is_none() {
            // Initialize states to zeros: (2, batch, 640)
            self.decoder_state1 = Some(Array3::zeros((2, batch_size, 640)));
            self.decoder_state2 = Some(Array3::zeros((2, 1, 640)));
        }

        let state1_data = self.decoder_state1.as_ref().unwrap().as_slice().unwrap().to_vec();
        let state1 = Tensor::from_array((
            vec![2, batch_size, 640],
            state1_data.into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create state1 tensor: {}", e)))?;

        let state2_data = self.decoder_state2.as_ref().unwrap().as_slice().unwrap().to_vec();
        let state2 = Tensor::from_array((
            vec![2, 1, 640],
            state2_data.into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create state2 tensor: {}", e)))?;

        // Run decoder with all 4 inputs
        let outputs = self
            .decoder
            .run(ort::inputs![
                "targets" => targets,
                "target_length" => target_length,
                "states.1" => state1,
                "onnx::Slice_3" => state2
            ])
            .map_err(|e| SttError::InferenceError(format!("Decoder inference failed: {}", e)))?;

        // Extract decoder output: outputs[0] is the decoder output (batch, 640, seq_len)
        let decoder_out_tensor = &outputs[0];
        let (shape, data) = decoder_out_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| SttError::InferenceError(format!("Failed to extract decoder output: {}", e)))?;

        // Update states for next iteration
        // outputs[2] is the new state (2, batch, 640)
        if let Ok((state_shape, state_data)) = outputs[2].try_extract_tensor::<f32>() {
            self.decoder_state1 = Some(Array3::from_shape_vec(
                (state_shape[0] as usize, state_shape[1] as usize, state_shape[2] as usize),
                state_data.to_vec(),
            ).unwrap());
        }
        // outputs[3] is the second state (2, batch, 640)
        if let Ok((state_shape, state_data)) = outputs[3].try_extract_tensor::<f32>() {
            self.decoder_state2 = Some(Array3::from_shape_vec(
                (state_shape[0] as usize, state_shape[1] as usize, state_shape[2] as usize),
                state_data.to_vec(),
            ).unwrap());
        }

        // Extract the last timestep: shape is (batch, 640, seq_len), we want (640,)
        let batch = shape[0] as usize;
        let hidden_size = shape[1] as usize;
        let seq = shape[2] as usize;

        // Reshape and extract last frame
        let decoder_out_3d = Array3::from_shape_vec((batch, hidden_size, seq), data.to_vec())
            .map_err(|e| SttError::InferenceError(format!("Failed to reshape decoder output: {}", e)))?;

        let last_frame = decoder_out_3d.slice(s![0, .., seq - 1]).to_owned();

        Ok(last_frame)
    }

    /// Run joiner to combine encoder and decoder outputs
    fn run_joiner(&mut self, encoder_out: &Array1<f32>, decoder_out: &Array1<f32>) -> Result<Array1<f32>> {
        // Prepare joiner inputs
        let encoder_input = Tensor::from_array((
            vec![1, encoder_out.len(), 1],  // (batch, 1024, 1)
            encoder_out.to_vec().into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create encoder input for joiner: {}", e)))?;

        let decoder_input = Tensor::from_array((
            vec![1, decoder_out.len(), 1],  // (batch, 640, 1)
            decoder_out.to_vec().into_boxed_slice(),
        ))
        .map_err(|e| SttError::InferenceError(format!("Failed to create decoder input for joiner: {}", e)))?;

        // Run joiner with correct input names
        let outputs = self
            .joiner
            .run(ort::inputs!["encoder_outputs" => encoder_input, "decoder_outputs" => decoder_input])
            .map_err(|e| SttError::InferenceError(format!("Joiner inference failed: {}", e)))?;

        // Extract logits from 4D tensor (batch, frames, frames, vocab_size)
        // With inputs (1, 1024, 1) and (1, 640, 1), output is (1, 1, 1, 1030)
        let logits_tensor = &outputs[0];
        let (shape, data) = logits_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| SttError::InferenceError(format!("Failed to extract joiner output: {}", e)))?;

        debug!("Joiner output shape: {:?}, data.len()={}", shape, data.len());

        // Extract the actual logits: [0, 0, 0, :] gives us the vocab_size dimension
        let vocab_size = shape[3] as usize;

        // The logits are ALL the data (since batch=1, frames=1, frames=1, total = vocab_size)
        let logits = Array1::from_vec(data.to_vec());

        // Debug: show first 10 and last 10 logits, plus blank_id logit
        if data.len() >= 10 {
            debug!("First 10 logits: {:?}", &data[..10]);
            debug!("Last 10 logits: {:?}", &data[data.len()-10..]);
            debug!("Logit at blank_id({}): {}", self.blank_id, data[self.blank_id as usize]);

            // Find max and top-5 tokens
            let mut indexed_logits: Vec<(usize, f32)> = data.iter().enumerate()
                .map(|(i, &v)| (i, v))
                .collect();
            indexed_logits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            debug!("Top 5 logits:");
            for (i, &(token_id, logit_val)) in indexed_logits[..5.min(indexed_logits.len())].iter().enumerate() {
                let token_text = if token_id < self.tokens.len() {
                    &self.tokens[token_id]
                } else {
                    "???"
                };
                debug!("  #{}: token={} ('{}'), logit={:.6}", i+1, token_id, token_text, logit_val);
            }
        }

        Ok(logits)
    }

    /// Convert token IDs to text
    fn tokens_to_text(&self, tokens: &[i64]) -> String {
        tokens
            .iter()
            .filter_map(|&token_id| {
                let idx = token_id as usize;
                if idx < self.tokens.len() && token_id != self.blank_id && token_id != self.unk_id {
                    Some(self.tokens[idx].as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .replace("▁", " ")  // Replace BPE underscores with spaces
            .trim()
            .to_string()
    }

    /// Get model information
    pub fn model_info(&self) -> String {
        format!(
            "OrtRecognizer:\n  Model: {}\n  Tokens: {}\n  Blank ID: {}\n  UNK ID: {}",
            self.model_path.display(),
            self.tokens.len(),
            self.blank_id,
            self.unk_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model files
    fn test_ort_recognizer_init() {
        let model_dir = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
        let recognizer = OrtRecognizer::new(model_dir, false);
        if let Err(e) = &recognizer {
            eprintln!("ERROR: {}", e);
        }
        assert!(recognizer.is_ok());
    }
}
