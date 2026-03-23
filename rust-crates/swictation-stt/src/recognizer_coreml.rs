//! Native CoreML recognizer for Parakeet-TDT models on macOS
//!
//! Uses compiled .mlmodelc bundles loaded via Objective-C CoreML API
//! through a C bridge (coreml_bridge.m). This achieves full ANE utilization
//! unlike the ONNX Runtime CoreML EP which only dispatches ~32% of nodes.
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
use ndarray::{s, Array1, Array2, Array3};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// FFI declarations matching coreml_bridge.h
// ---------------------------------------------------------------------------
extern "C" {
    fn coreml_load_model(
        path: *const c_char,
        compute_units: i32,
        error_out: *mut *mut c_char,
    ) -> *mut std::ffi::c_void;

    fn coreml_predict(
        model: *mut std::ffi::c_void,
        input_name: *const c_char,
        input_data: *const f32,
        input_shape: *const i64,
        input_ndims: i32,
        output_name: *const c_char,
        output_data: *mut f32,
        output_size: i64,
        error_out: *mut *mut c_char,
    ) -> i32;

    fn coreml_predict_multi(
        model: *mut std::ffi::c_void,
        input_names: *const *const c_char,
        input_datas: *const *const f32,
        input_shapes: *const *const i64,
        input_ndims_arr: *const i32,
        num_inputs: i32,
        output_name: *const c_char,
        output_data: *mut f32,
        output_size: i64,
        error_out: *mut *mut c_char,
    ) -> i32;

    fn coreml_free_model(model: *mut std::ffi::c_void);

    fn coreml_free_string(str_ptr: *mut c_char);
}

// ---------------------------------------------------------------------------
// CoreML compute-unit constants (matching coreml_bridge.m switch)
// ---------------------------------------------------------------------------

/// CPU only
#[allow(dead_code)]
const COMPUTE_UNITS_CPU: i32 = 0;
/// CPU + GPU
#[allow(dead_code)]
const COMPUTE_UNITS_CPU_GPU: i32 = 1;
/// CPU + GPU + ANE (all)
const COMPUTE_UNITS_ALL: i32 = 2;
/// CPU + ANE
#[allow(dead_code)]
const COMPUTE_UNITS_CPU_ANE: i32 = 3;

// ---------------------------------------------------------------------------
// Safe wrapper around a CoreML model handle
// ---------------------------------------------------------------------------

/// RAII wrapper around an opaque CoreML model pointer.
///
/// All unsafe FFI interaction is confined to this struct so the rest of the
/// recognizer can be written in safe Rust.
struct CoreMLModel {
    handle: *mut std::ffi::c_void,
    /// Path used for error messages
    path: PathBuf,
}

// Apple documents that MLModel.predictionFromFeatures is thread-safe for
// concurrent read-only predictions on the same model instance.
unsafe impl Send for CoreMLModel {}
unsafe impl Sync for CoreMLModel {}

impl CoreMLModel {
    /// Load a compiled `.mlmodelc` bundle.
    ///
    /// `compute_units` selects the execution target (see constants above).
    fn load(path: &Path, compute_units: i32) -> Result<Self> {
        let path_cstr = path_to_cstring(path)?;
        let mut error_ptr: *mut c_char = std::ptr::null_mut();

        let handle = unsafe {
            coreml_load_model(path_cstr.as_ptr(), compute_units, &mut error_ptr)
        };

        if handle.is_null() {
            let msg = consume_error_string(error_ptr)
                .unwrap_or_else(|| "unknown error".to_string());
            return Err(SttError::ModelLoadError(format!(
                "Failed to load CoreML model at {}: {}",
                path.display(),
                msg
            )));
        }

        info!("Loaded CoreML model: {}", path.display());
        Ok(Self {
            handle,
            path: path.to_path_buf(),
        })
    }

    /// Run a single-input / single-output prediction.
    ///
    /// Returns a `Vec<f32>` of length `output_size`.
    fn predict_single(
        &self,
        input_name: &str,
        input_data: &[f32],
        input_shape: &[i64],
        output_name: &str,
        output_size: usize,
    ) -> Result<Vec<f32>> {
        let in_name = CString::new(input_name).map_err(|e| {
            SttError::InferenceError(format!("Invalid input name '{}': {}", input_name, e))
        })?;
        let out_name = CString::new(output_name).map_err(|e| {
            SttError::InferenceError(format!("Invalid output name '{}': {}", output_name, e))
        })?;

        let mut output_buf = vec![0.0f32; output_size];
        let mut error_ptr: *mut c_char = std::ptr::null_mut();

        let rc = unsafe {
            coreml_predict(
                self.handle,
                in_name.as_ptr(),
                input_data.as_ptr(),
                input_shape.as_ptr(),
                input_shape.len() as i32,
                out_name.as_ptr(),
                output_buf.as_mut_ptr(),
                output_size as i64,
                &mut error_ptr,
            )
        };

        if rc != 0 {
            let msg = consume_error_string(error_ptr)
                .unwrap_or_else(|| "unknown error".to_string());
            return Err(SttError::InferenceError(format!(
                "CoreML predict failed on {} (input='{}', output='{}'): {}",
                self.path.display(),
                input_name,
                output_name,
                msg
            )));
        }

        Ok(output_buf)
    }

    /// Run a multi-input / single-output prediction.
    ///
    /// Each entry in `inputs` is `(name, data, shape)`.
    /// Returns a `Vec<f32>` of length `output_size`.
    fn predict_multi(
        &self,
        inputs: &[(&str, &[f32], &[i64])],
        output_name: &str,
        output_size: usize,
    ) -> Result<Vec<f32>> {
        // Prepare C-compatible arrays. Keep CStrings alive across the FFI call.
        let c_names: Vec<CString> = inputs
            .iter()
            .map(|(n, _, _)| {
                CString::new(*n).map_err(|e| {
                    SttError::InferenceError(format!("Invalid input name '{}': {}", n, e))
                })
            })
            .collect::<Result<_>>()?;
        let name_ptrs: Vec<*const c_char> = c_names.iter().map(|n| n.as_ptr()).collect();
        let data_ptrs: Vec<*const f32> = inputs.iter().map(|(_, d, _)| d.as_ptr()).collect();
        let shape_ptrs: Vec<*const i64> = inputs.iter().map(|(_, _, s)| s.as_ptr()).collect();
        let ndims: Vec<i32> = inputs.iter().map(|(_, _, s)| s.len() as i32).collect();

        let out_name = CString::new(output_name).map_err(|e| {
            SttError::InferenceError(format!("Invalid output name '{}': {}", output_name, e))
        })?;

        let mut output_buf = vec![0.0f32; output_size];
        let mut error_ptr: *mut c_char = std::ptr::null_mut();

        let rc = unsafe {
            coreml_predict_multi(
                self.handle,
                name_ptrs.as_ptr(),
                data_ptrs.as_ptr(),
                shape_ptrs.as_ptr(),
                ndims.as_ptr(),
                inputs.len() as i32,
                out_name.as_ptr(),
                output_buf.as_mut_ptr(),
                output_size as i64,
                &mut error_ptr,
            )
        };

        if rc != 0 {
            let msg = consume_error_string(error_ptr)
                .unwrap_or_else(|| "unknown error".to_string());
            let in_names: Vec<&str> = inputs.iter().map(|(n, _, _)| *n).collect();
            return Err(SttError::InferenceError(format!(
                "CoreML predict_multi failed on {} (inputs={:?}, output='{}'): {}",
                self.path.display(),
                in_names,
                output_name,
                msg
            )));
        }

        Ok(output_buf)
    }
}

impl Drop for CoreMLModel {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { coreml_free_model(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

// ---------------------------------------------------------------------------
// FFI helpers
// ---------------------------------------------------------------------------

/// Convert a `Path` to a nul-terminated CString for the C bridge.
fn path_to_cstring(path: &Path) -> Result<CString> {
    let s = path
        .to_str()
        .ok_or_else(|| SttError::ModelLoadError(format!(
            "Path contains non-UTF8 characters: {}",
            path.display()
        )))?;
    CString::new(s).map_err(|e| {
        SttError::ModelLoadError(format!("Path contains interior nul: {}", e))
    })
}

/// Consume an error string returned by the C bridge.
/// Returns `None` if the pointer is null.
fn consume_error_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let msg = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { coreml_free_string(ptr) };
    Some(msg)
}

// ---------------------------------------------------------------------------
// Model configuration (mirrors recognizer_ort.rs ModelConfig)
// ---------------------------------------------------------------------------

/// Model configuration for different Parakeet-TDT variants.
///
/// This is a local copy of the struct in `recognizer_ort.rs`.  If that struct
/// is made `pub` in a future refactor, this duplicate can be removed.
#[derive(Debug, Clone, Copy)]
struct ModelConfig {
    /// Decoder hidden state size (640 for both 0.6B and 1.1B)
    decoder_hidden_size: usize,
    /// Number of mel features (128 for 0.6B, 80 for 1.1B)
    n_mel_features: usize,
    /// Whether encoder expects transposed input (currently both use transposed)
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
// Decoder state type alias (matches recognizer_ort.rs)
// ---------------------------------------------------------------------------

/// (tokens, final_decoder_token, final_decoder_out, (blank_count, nonblank_count))
type DecoderState = (Vec<i64>, i64, Array1<f32>, (usize, usize));

// ---------------------------------------------------------------------------
// CoreMLRecognizer
// ---------------------------------------------------------------------------

/// Native CoreML recognizer for Parakeet-TDT speech recognition.
///
/// Loads three compiled CoreML model bundles (encoder, decoder, joiner) and
/// performs TDT greedy search decoding identically to `OrtRecognizer`, but
/// using the Apple Neural Engine for maximum on-device throughput.
pub struct CoreMLRecognizer {
    encoder: CoreMLModel,
    decoder: CoreMLModel,
    joiner: CoreMLModel,
    tokens: Vec<String>,
    blank_id: i64,
    unk_id: i64,
    model_path: PathBuf,
    audio_processor: AudioProcessor,
    /// Decoder LSTM hidden state: shape (2, 1, hidden_size)
    decoder_state1: Option<Array3<f32>>,
    /// Decoder LSTM cell state: shape (2, 1, hidden_size)
    decoder_state2: Option<Array3<f32>>,
    config: ModelConfig,
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
    /// # Arguments
    ///
    /// * `model_dir` - Path to the model directory.
    ///
    /// # Errors
    ///
    /// Returns `SttError::ModelLoadError` if any model bundle or `tokens.txt`
    /// cannot be loaded.
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
        let compute_units = COMPUTE_UNITS_ALL;
        info!("Loading encoder.mlmodelc (compute_units=All)...");
        let encoder = CoreMLModel::load(
            &model_path.join("encoder.mlmodelc"),
            compute_units,
        )?;

        info!("Loading decoder.mlmodelc (compute_units=All)...");
        let decoder = CoreMLModel::load(
            &model_path.join("decoder.mlmodelc"),
            compute_units,
        )?;

        info!("Loading joiner.mlmodelc (compute_units=All)...");
        let joiner = CoreMLModel::load(
            &model_path.join("joiner.mlmodelc"),
            compute_units,
        )?;

        let audio_processor = AudioProcessor::with_mel_features(config.n_mel_features)?;

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
            decoder_state1: None,
            decoder_state2: None,
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

        // Extract mel-spectrogram features
        let features = self.audio_processor.extract_mel_features(samples)?;
        info!("Extracted mel features: {:?}", features.shape());

        // Chunk and decode
        let text = if features.nrows() <= 80 {
            let chunks = self.audio_processor.chunk_features(&features);
            info!("Small audio: {} chunks", chunks.len());
            self.greedy_search_decode(&chunks)?
        } else {
            info!(
                "Large audio: {} frames -- chunking",
                features.nrows()
            );
            let padded_rows = features.nrows().div_ceil(80) * 80;
            let mut padded = Array2::zeros((padded_rows, features.ncols()));
            padded
                .slice_mut(s![..features.nrows(), ..])
                .assign(&features);
            let chunks = self.audio_processor.chunk_features(&padded);
            info!("Processing {} encoder chunks", chunks.len());
            self.greedy_search_decode(&chunks)?
        };

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
    // Internal: greedy search decode (mirrors OrtRecognizer exactly)
    // -----------------------------------------------------------------------

    /// Top-level greedy search over a sequence of feature chunks.
    fn greedy_search_decode(&mut self, chunks: &[Array2<f32>]) -> Result<String> {
        let mut all_tokens = Vec::new();

        // Reset decoder states at the start
        self.decoder_state1 = None;
        self.decoder_state2 = None;

        let mut decoder_out_opt: Option<Array1<f32>> = None;
        let mut last_decoder_token = self.blank_id;

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            debug!(
                "Processing chunk {}/{}, shape {:?}",
                chunk_idx + 1,
                chunks.len(),
                chunk.shape()
            );

            let encoder_out = self.run_encoder(chunk)?;
            debug!("Encoder output shape: {:?}", encoder_out.shape());

            let (chunk_tokens, final_token, final_decoder_out, (bc, nbc)) = self
                .decode_frames_with_state(
                    &encoder_out,
                    decoder_out_opt.take(),
                    last_decoder_token,
                )?;
            debug!(
                "Chunk {}: {} tokens, {} blank / {} non-blank",
                chunk_idx + 1,
                chunk_tokens.len(),
                bc,
                nbc
            );

            all_tokens.extend(chunk_tokens);
            last_decoder_token = final_token;
            decoder_out_opt = Some(final_decoder_out);
        }

        let text = self.tokens_to_text(&all_tokens);
        Ok(text)
    }

    // -----------------------------------------------------------------------
    // Encoder
    // -----------------------------------------------------------------------

    /// Run the encoder on a single feature chunk.
    ///
    /// Input layout (matching ONNX model):
    /// - `audio_signal`: shape `[1, n_mel_features, num_frames]` (transposed)
    /// - `length`: shape `[1]` (i64 cast to f32 for CoreML MLMultiArray)
    ///
    /// Output:
    /// - Encoded features as `Array3<f32>` with shape `[1, encoder_dim, T']`
    fn run_encoder(&self, features: &Array2<f32>) -> Result<Array3<f32>> {
        let num_frames = features.nrows();
        let num_features = features.ncols();

        debug!(
            "Encoder: {} frames x {} features",
            num_frames, num_features
        );

        // Transpose to (1, num_features, num_frames) -- same layout as OrtRecognizer
        let mut audio_data = Vec::with_capacity(num_frames * num_features);
        for col_idx in 0..num_features {
            for row in features.outer_iter() {
                audio_data.push(row[col_idx]);
            }
        }
        let audio_shape: [i64; 3] = [1, num_features as i64, num_frames as i64];

        // Length tensor -- CoreML MLMultiArray is always Float32, so we pass
        // the frame count as f32 (the compiled model should accept this).
        let length_data = [num_frames as f32];
        let length_shape: [i64; 1] = [1];

        // Determine expected encoder output size.  The Parakeet encoder
        // produces (1, encoder_dim, T') where T' <= num_frames.  We
        // allocate conservatively: encoder_dim is typically 1024 for 1.1B
        // and 640 for 0.6B.  Since we do not know the exact subsampling
        // factor until we run, we allocate for the worst case (no subsampling).
        let max_encoder_dim = 1024;
        let max_output_size = 1 * max_encoder_dim * num_frames;
        let mut output_buf = vec![0.0f32; max_output_size];

        // Build multi-input call
        let inputs: Vec<(&str, &[f32], &[i64])> = vec![
            ("audio_signal", &audio_data, &audio_shape),
            ("length", &length_data, &length_shape),
        ];

        let result = self.encoder.predict_multi(&inputs, "output", max_output_size);

        // If the default output name fails, try common alternatives
        let output_data = match result {
            Ok(data) => data,
            Err(ref _e) => {
                debug!(
                    "Output name 'output' failed, trying 'encoder_output'. \
                     If this also fails, inspect the .mlmodelc for the correct \
                     output feature name."
                );
                let alt = self
                    .encoder
                    .predict_multi(&inputs, "encoder_output", max_output_size);
                match alt {
                    Ok(data) => data,
                    Err(_) => {
                        // Return original error with guidance
                        return Err(SttError::InferenceError(format!(
                            "Encoder prediction failed. Tried output names 'output' and \
                             'encoder_output'. Check the encoder.mlmodelc model description \
                             for the correct output feature name. Original error: {}",
                            _e
                        )));
                    }
                }
            }
        };

        // Discover the actual output shape.  The encoder subsamples time by
        // a factor of 4 (two conv layers with stride 2).  encoder_dim is
        // typically 1024 (1.1B) or 640 (0.6B).
        //
        // We infer the shape from the total element count:
        //   total = 1 * encoder_dim * T'
        // where T' = ceil(num_frames / 4) for Conformer subsampling.
        //
        // We try known encoder_dim values in order of likelihood.
        let total = output_data.iter().take_while(|&&v| v != 0.0 || true).count();
        // Actually use the full buffer length that was returned
        let total = output_data.len();

        let (encoder_dim, time_out) = infer_encoder_shape(total, num_frames)?;

        // Trim to actual size and reshape
        let actual_size = 1 * encoder_dim * time_out;
        let trimmed = &output_data[..actual_size];

        let encoder_out =
            Array3::from_shape_vec((1, encoder_dim, time_out), trimmed.to_vec()).map_err(
                |e| {
                    SttError::InferenceError(format!(
                        "Failed to reshape encoder output (total={}, encoder_dim={}, time={}): {}",
                        actual_size, encoder_dim, time_out, e
                    ))
                },
            )?;

        Ok(encoder_out)
    }

    // -----------------------------------------------------------------------
    // Decoder
    // -----------------------------------------------------------------------

    /// Run the decoder with a single token and LSTM states.
    ///
    /// Mirrors `OrtRecognizer::run_decoder` exactly:
    /// - inputs: `targets` (i32), `target_length` (i32), `states.1`, `onnx::Slice_3`
    /// - outputs: decoder_out `[1, hidden, 1]`, unused, new_state1, new_state2
    ///
    /// NOTE: The CoreML compiled model may rename inputs/outputs.  The names
    /// used here match the ONNX originals.  If prediction fails due to name
    /// mismatches, the error message will guide debugging.
    ///
    /// IMPORTANT: The current C bridge (`coreml_predict_multi`) supports only
    /// a single output extraction per call.  The decoder produces 4 outputs.
    /// To work around this limitation, this method runs the decoder model
    /// separately for each output that needs extraction (decoder_out, state1,
    /// state2).  This is ~3x slower than a single call with multi-output
    /// extraction.  A future enhancement to `coreml_bridge.m` adding
    /// `coreml_predict_multi_output()` would eliminate this overhead.
    fn run_decoder(&mut self, tokens: &[i64]) -> Result<Array1<f32>> {
        let hidden_size = self.config.decoder_hidden_size;
        let seq_len = tokens.len();

        // CoreML MLMultiArray is typed Float32 -- we pass int values as f32.
        // The compiled model's input type must match; if the original ONNX
        // used int32, coremltools should have inserted a cast.
        let targets_f32: Vec<f32> = tokens.iter().map(|&t| t as f32).collect();
        let targets_shape: [i64; 2] = [1, seq_len as i64];

        let target_length_f32 = [seq_len as f32];
        let target_length_shape: [i64; 1] = [1];

        // Initialize LSTM states if needed
        if self.decoder_state1.is_none() {
            self.decoder_state1 = Some(Array3::zeros((2, 1, hidden_size)));
            self.decoder_state2 = Some(Array3::zeros((2, 1, hidden_size)));
        }

        let state1_data: Vec<f32> = self
            .decoder_state1
            .as_ref()
            .unwrap()
            .iter()
            .copied()
            .collect();
        let state1_shape: [i64; 3] = [2, 1, hidden_size as i64];

        let state2_data: Vec<f32> = self
            .decoder_state2
            .as_ref()
            .unwrap()
            .iter()
            .copied()
            .collect();
        let state2_shape: [i64; 3] = [2, 1, hidden_size as i64];

        let inputs: Vec<(&str, &[f32], &[i64])> = vec![
            ("targets", &targets_f32, &targets_shape),
            ("target_length", &target_length_f32, &target_length_shape),
            ("states.1", &state1_data, &state1_shape),
            ("onnx::Slice_3", &state2_data, &state2_shape),
        ];

        // Extract decoder output (output index 0): shape (1, hidden_size, seq_len)
        let decoder_out_size = 1 * hidden_size * seq_len;
        let decoder_out_data = self.run_decoder_for_output(&inputs, "output", decoder_out_size)?;

        // Extract new state1 (output index 2): shape (2, 1, hidden_size)
        let state_size = 2 * 1 * hidden_size;
        let new_state1_data =
            self.run_decoder_for_output(&inputs, "new_state1", state_size);
        // Extract new state2 (output index 3): shape (2, 1, hidden_size)
        let new_state2_data =
            self.run_decoder_for_output(&inputs, "new_state2", state_size);

        // Update LSTM states if extraction succeeded.
        // If the output names do not match, log a warning and keep old states.
        match new_state1_data {
            Ok(data) => {
                self.decoder_state1 = Some(
                    Array3::from_shape_vec((2, 1, hidden_size), data)
                        .map_err(|e| {
                            SttError::InferenceError(format!(
                                "Failed to reshape decoder state1: {}",
                                e
                            ))
                        })?,
                );
            }
            Err(e) => {
                warn!(
                    "Could not extract decoder state1 (tried 'new_state1'). \
                     LSTM states will not update. Error: {}. \
                     Check decoder.mlmodelc output feature names.",
                    e
                );
            }
        }

        match new_state2_data {
            Ok(data) => {
                self.decoder_state2 = Some(
                    Array3::from_shape_vec((2, 1, hidden_size), data)
                        .map_err(|e| {
                            SttError::InferenceError(format!(
                                "Failed to reshape decoder state2: {}",
                                e
                            ))
                        })?,
                );
            }
            Err(e) => {
                warn!(
                    "Could not extract decoder state2 (tried 'new_state2'). \
                     LSTM states will not update. Error: {}. \
                     Check decoder.mlmodelc output feature names.",
                    e
                );
            }
        }

        // Reshape decoder output: (1, hidden_size, seq_len) -> take last timestep
        let decoder_out_3d = Array3::from_shape_vec(
            (1, hidden_size, seq_len),
            decoder_out_data,
        )
        .map_err(|e| {
            SttError::InferenceError(format!(
                "Failed to reshape decoder output: {}",
                e
            ))
        })?;

        let last_frame = decoder_out_3d.slice(s![0, .., seq_len - 1]).to_owned();
        Ok(last_frame)
    }

    /// Helper: run the decoder model and extract a single named output.
    ///
    /// Tries the given `output_name` first, then falls back to common
    /// alternatives derived from ONNX conversion patterns.
    fn run_decoder_for_output(
        &self,
        inputs: &[(&str, &[f32], &[i64])],
        output_name: &str,
        output_size: usize,
    ) -> Result<Vec<f32>> {
        let result = self.decoder.predict_multi(inputs, output_name, output_size);
        if result.is_ok() {
            return result;
        }

        // Try alternative names that coremltools may generate
        let alternatives: &[&str] = match output_name {
            "output" => &["decoder_output", "output_0"],
            "new_state1" => &["states.1_output", "output_2", "onnx::LSTM_output_1"],
            "new_state2" => &["onnx::Slice_3_output", "output_3", "onnx::LSTM_output_2"],
            _ => &[],
        };

        for alt in alternatives {
            if let Ok(data) = self.decoder.predict_multi(inputs, alt, output_size) {
                debug!(
                    "Decoder output '{}' not found, but '{}' worked",
                    output_name, alt
                );
                return Ok(data);
            }
        }

        // Return the original error
        result
    }

    // -----------------------------------------------------------------------
    // Joiner
    // -----------------------------------------------------------------------

    /// Combine encoder frame and decoder output through the joint network.
    ///
    /// Mirrors `OrtRecognizer::run_joiner`:
    /// - inputs: `encoder_outputs` `[1, enc_dim, 1]`, `decoder_outputs` `[1, hidden, 1]`
    /// - output: logits `[1, 1, 1, vocab_size + num_durations]`
    fn run_joiner(
        &self,
        encoder_frame: &Array1<f32>,
        decoder_out: &Array1<f32>,
    ) -> Result<Array1<f32>> {
        let enc_data: Vec<f32> = encoder_frame.to_vec();
        let enc_shape: [i64; 3] = [1, encoder_frame.len() as i64, 1];

        let dec_data: Vec<f32> = decoder_out.to_vec();
        let dec_shape: [i64; 3] = [1, decoder_out.len() as i64, 1];

        // Output size: vocab_size + duration_logits
        // For TDT models, there are typically 5 duration bins appended.
        // Total = vocab_size + num_durations.  We know vocab_size from tokens.
        // Allocate generously (tokens.len() + 16 for durations).
        let output_size = self.tokens.len() + 16;

        let inputs: Vec<(&str, &[f32], &[i64])> = vec![
            ("encoder_outputs", &enc_data, &enc_shape),
            ("decoder_outputs", &dec_data, &dec_shape),
        ];

        let result = self.joiner.predict_multi(&inputs, "output", output_size);
        let output_data = match result {
            Ok(d) => d,
            Err(ref _e) => {
                // Try alternative output name
                let alt = self
                    .joiner
                    .predict_multi(&inputs, "logits", output_size);
                match alt {
                    Ok(d) => d,
                    Err(_) => {
                        return Err(SttError::InferenceError(format!(
                            "Joiner prediction failed. Tried output names 'output' and \
                             'logits'. Check joiner.mlmodelc output feature names. \
                             Original error: {}",
                            _e
                        )));
                    }
                }
            }
        };

        Ok(Array1::from_vec(output_data))
    }

    // -----------------------------------------------------------------------
    // TDT greedy search (identical logic to OrtRecognizer)
    // -----------------------------------------------------------------------

    /// Decode encoder output frames using TDT greedy search with cross-chunk
    /// state persistence.
    ///
    /// This is a direct port of `OrtRecognizer::decode_frames_with_state`.
    /// Reference: sherpa-onnx `DecodeOneTDT` (offline-transducer-greedy-search-nemo-decoder.cc).
    fn decode_frames_with_state(
        &mut self,
        encoder_out: &Array3<f32>,
        prev_decoder_out: Option<Array1<f32>>,
        initial_token: i64,
    ) -> Result<DecoderState> {
        let num_frames = encoder_out.shape()[2];
        let vocab_size = self.tokens.len();
        let blank_id = self.blank_id;
        let max_tokens_per_frame = 5; // sherpa-onnx TDT default

        let mut tokens = Vec::new();
        let mut blank_count = 0_usize;
        let mut nonblank_count = 0_usize;

        // Initialize decoder output: reuse from previous chunk or compute fresh
        let mut decoder_out = if let Some(prev_out) = prev_decoder_out {
            debug!("Reusing decoder_out from previous chunk (token={})", initial_token);
            prev_out
        } else {
            debug!("Computing initial decoder_out (token={})", initial_token);
            self.run_decoder(&[initial_token])?
        };
        let mut last_emitted_token = initial_token;

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

            // Extract single encoder frame: (encoder_dim,)
            let encoder_frame = encoder_out.slice(s![0, .., t]).to_owned();

            // Run joiner
            let logits = self.run_joiner(&encoder_frame, &decoder_out)?;
            let logits_slice = logits.as_slice().unwrap();
            let output_size = logits_slice.len();
            let num_durations = output_size - vocab_size;

            let token_logits = &logits_slice[0..vocab_size];
            let duration_logits = &logits_slice[vocab_size..];

            // Greedy token selection
            let (y, _) = token_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();
            let y = y as i64;

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
                decoder_out = self.run_decoder(&[y])?;
                last_emitted_token = y;
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

        debug!(
            "Decoded {} tokens from {} frames ({} blank, {} non-blank)",
            tokens.len(),
            num_frames,
            blank_count,
            nonblank_count
        );

        Ok((tokens, last_emitted_token, decoder_out, (blank_count, nonblank_count)))
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

/// Infer encoder output shape from total element count and input frame count.
///
/// The Parakeet Conformer encoder subsamples time by 4x (two conv layers
/// with stride 2).  Known encoder dimensions:
/// - 1024 for 1.1B
/// - 640  for 0.6B
/// - 512  for smaller variants
///
/// Returns `(encoder_dim, time_out)`.
fn infer_encoder_shape(total_elements: usize, input_frames: usize) -> Result<(usize, usize)> {
    // Try known encoder dimensions in order of likelihood
    let candidates = [1024_usize, 640, 512, 768, 256];
    // Possible subsampling factors
    let subsample_factors = [4_usize, 2, 8, 1];

    for &dim in &candidates {
        if total_elements % dim == 0 {
            let time_out = total_elements / dim;
            // Sanity check: time_out should be roughly input_frames / subsample
            for &factor in &subsample_factors {
                let expected = input_frames.div_ceil(factor);
                // Allow some slack (encoder may pad or trim slightly)
                if time_out <= expected + 2 && time_out + 2 >= expected.saturating_sub(2) {
                    debug!(
                        "Inferred encoder shape: (1, {}, {}) from {} elements \
                         (subsample factor ~{})",
                        dim, time_out, total_elements, factor
                    );
                    return Ok((dim, time_out));
                }
            }
        }
    }

    // Fallback: assume encoder_dim = total / ceil(input_frames/4) if it divides evenly
    let time_guess = input_frames.div_ceil(4);
    if time_guess > 0 && total_elements % time_guess == 0 {
        let dim = total_elements / time_guess;
        warn!(
            "Using fallback encoder shape inference: (1, {}, {}) from {} elements",
            dim, time_guess, total_elements
        );
        return Ok((dim, time_guess));
    }

    Err(SttError::InferenceError(format!(
        "Cannot infer encoder output shape from {} total elements \
         (input_frames={}). Expected encoder_dim in {:?} with ~4x subsampling.",
        total_elements, input_frames, candidates
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_encoder_shape_1_1b() {
        // 1.1B model: 1024 dim, 4x subsample, 10000 input frames -> 2500 output
        let (dim, time) = infer_encoder_shape(1024 * 2500, 10000).unwrap();
        assert_eq!(dim, 1024);
        assert_eq!(time, 2500);
    }

    #[test]
    fn test_infer_encoder_shape_0_6b() {
        // 0.6B model: 640 dim, 4x subsample, 10000 input frames -> 2500 output
        let (dim, time) = infer_encoder_shape(640 * 2500, 10000).unwrap();
        assert_eq!(dim, 640);
        assert_eq!(time, 2500);
    }

    #[test]
    fn test_model_config_detection() {
        let cfg_1_1b = ModelConfig::detect_from_path(Path::new("/models/parakeet-tdt-1.1b-coreml"));
        assert_eq!(cfg_1_1b.n_mel_features, 80);
        assert_eq!(cfg_1_1b.decoder_hidden_size, 640);

        let cfg_0_6b = ModelConfig::detect_from_path(Path::new("/models/parakeet-tdt-0.6b-coreml"));
        assert_eq!(cfg_0_6b.n_mel_features, 128);
        assert_eq!(cfg_0_6b.decoder_hidden_size, 640);
    }

    #[test]
    fn test_path_to_cstring() {
        let p = Path::new("/tmp/model.mlmodelc");
        let c = path_to_cstring(p).unwrap();
        assert_eq!(c.to_str().unwrap(), "/tmp/model.mlmodelc");
    }
}

} // mod inner

// Re-export the public type at crate level when the feature is active.
#[cfg(all(target_os = "macos", feature = "coreml-native"))]
pub use inner::CoreMLRecognizer;
