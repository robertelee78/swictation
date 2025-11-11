//! Direct ONNX Runtime implementation of Silero VAD
//! Replaces sherpa-rs dependency with modern ort crate

use crate::{Result, VadError};
use ort::{
    execution_providers::{CUDAExecutionProvider, CPUExecutionProvider},
    session::Session,
    inputs,
    value::Tensor,
};
use std::sync::{Arc, Mutex};
use ndarray::{Array2, Array3, ArrayView3};

/// Silero VAD model using direct ONNX Runtime
pub struct SileroVadOrt {
    session: Arc<Mutex<Session>>,
    sample_rate: i32,
    window_size: usize,
    threshold: f32,
    min_speech_samples: usize,
    min_silence_samples: usize,

    // State for streaming - Silero VAD v6 uses LSTM with separate h and c states
    // Each state is [2 layers, 1 batch, 64 hidden units]
    h_state: Array3<f32>,  // LSTM hidden state [2, 1, 64]
    c_state: Array3<f32>,  // LSTM cell state [2, 1, 64]
    triggered: bool,
    temp_end: usize,
    current_sample: usize,

    // Speech segment buffering
    speech_buffer: Vec<f32>,

    // Debug mode
    debug: bool,
}

impl SileroVadOrt {
    /// Create new Silero VAD with ONNX Runtime
    pub fn new(
        model_path: &str,
        threshold: f32,
        sample_rate: i32,
        window_size: usize,
        min_speech_duration_ms: i32,
        min_silence_duration_ms: i32,
        provider: Option<String>,
        debug: bool,
    ) -> Result<Self> {
        // Build session with appropriate provider
        let session = if let Some(ref prov) = provider {
            if prov.contains("cuda") || prov.contains("CUDA") {
                // Try CUDA provider
                match Session::builder()
                    .map_err(|e| VadError::initialization(format!("Failed to create session builder: {}", e)))?
                    .with_execution_providers([CUDAExecutionProvider::default().build()])
                    .map_err(|e| VadError::initialization(format!("Failed to set CUDA provider: {}", e)))?
                    .commit_from_file(model_path) {
                    Ok(s) => {
                        println!("Silero VAD: Using CUDA provider");
                        s
                    }
                    Err(e) => {
                        println!("Silero VAD: CUDA not available ({}), falling back to CPU", e);
                        Session::builder()
                            .map_err(|e| VadError::initialization(format!("Failed to create session builder: {}", e)))?
                            .with_execution_providers([CPUExecutionProvider::default().build()])
                            .map_err(|e| VadError::initialization(format!("Failed to set CPU provider: {}", e)))?
                            .commit_from_file(model_path)
                            .map_err(|e| VadError::initialization(format!("Failed to load model with CPU: {}", e)))?
                    }
                }
            } else {
                Session::builder()
                    .map_err(|e| VadError::initialization(format!("Failed to create session builder: {}", e)))?
                    .with_execution_providers([CPUExecutionProvider::default().build()])
                    .map_err(|e| VadError::initialization(format!("Failed to set CPU provider: {}", e)))?
                    .commit_from_file(model_path)
                    .map_err(|e| VadError::initialization(format!("Failed to load model: {}", e)))?
            }
        } else {
            Session::builder()
                .map_err(|e| VadError::initialization(format!("Failed to create session builder: {}", e)))?
                .with_execution_providers([CPUExecutionProvider::default().build()])
                .map_err(|e| VadError::initialization(format!("Failed to set CPU provider: {}", e)))?
                .commit_from_file(model_path)
                .map_err(|e| VadError::initialization(format!("Failed to load model: {}", e)))?
        };

        // Print model input/output names for debugging
        println!("=== ONNX Model Metadata ===");
        println!("Model inputs:");
        for input in session.inputs.iter() {
            println!("  - name: '{}' (type: {:?})", input.name, input.input_type);
        }
        println!("Model outputs:");
        for output in session.outputs.iter() {
            println!("  - name: '{}' (type: {:?})", output.name, output.output_type);
        }
        println!("===========================");

        // Calculate sample counts from durations
        let min_speech_samples = (min_speech_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
        let min_silence_samples = (min_silence_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;

        // Initialize LSTM states (2 layers, 1 batch, 64 hidden units each)
        let h_state = Array3::<f32>::zeros((2, 1, 64));
        let c_state = Array3::<f32>::zeros((2, 1, 64));

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            sample_rate,
            window_size,
            threshold,
            min_speech_samples,
            min_silence_samples,
            h_state,
            c_state,
            triggered: false,
            temp_end: 0,
            current_sample: 0,
            speech_buffer: Vec::new(),
            debug,
        })
    }

    /// Process audio chunk and detect speech
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<Option<Vec<f32>>> {
        if audio_chunk.len() != self.window_size {
            return Err(VadError::processing(format!(
                "Expected {} samples, got {}",
                self.window_size,
                audio_chunk.len()
            )));
        }

        // Convert audio to ndarray with proper shape (batch_size=1, sequence_len)
        // CRITICAL: Must use standard (C-contiguous) layout, not Fortran layout
        let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())
            .map_err(|e| VadError::processing(format!("Failed to reshape input: {}", e)))?;

        if self.debug && self.current_sample == 0 {
            eprintln!("VAD Debug:");
            eprintln!("  input shape: {:?}", input_array.shape());
            eprintln!("  input is C-contiguous: {}", input_array.is_standard_layout());
            eprintln!("  input range: [{:.6}, {:.6}]",
                     input_array.iter().copied().fold(f32::INFINITY, f32::min),
                     input_array.iter().copied().fold(f32::NEG_INFINITY, f32::max));
            eprintln!("  h_state shape: {:?}", self.h_state.shape());
            eprintln!("  c_state shape: {:?}", self.c_state.shape());
        }

        // Create tensors from OWNED arrays - ort 2.0.0-rc.10 requires owned, not views
        // Model expects inputs: "x", "h", "c" (NOT "input", "state", "sr")
        let input_value = Tensor::from_array(input_array)
            .map_err(|e| VadError::processing(format!("Failed to create input: {}", e)))?;
        let h_value = Tensor::from_array(self.h_state.clone())
            .map_err(|e| VadError::processing(format!("Failed to create h state: {}", e)))?;
        let c_value = Tensor::from_array(self.c_state.clone())
            .map_err(|e| VadError::processing(format!("Failed to create c state: {}", e)))?;

        // Run inference with correct tensor names
        let mut session_guard = self.session
            .lock()
            .map_err(|e| VadError::processing(format!("Failed to lock session: {}", e)))?;

        let outputs = session_guard
            .run(inputs![
                "x" => input_value,
                "h" => h_value,
                "c" => c_value
            ])
            .map_err(|e| VadError::processing(format!("Failed to run inference: {}", e)))?;

        if self.debug && self.current_sample == 0 {
            eprintln!("  Model returned {} outputs", outputs.len());
        }

        // Extract speech probability from the output (shape: [batch, 1])
        // Model outputs: "prob", "new_h", "new_c"
        let output_array: ndarray::ArrayView2<f32> = outputs["prob"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract prob: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape prob: {}", e)))?;
        let speech_prob = output_array[[0, 0]];

        if self.debug && self.current_sample == 0 {
            eprintln!("  Output array shape: {:?}", output_array.shape());
            eprintln!("  Speech probability: {}", speech_prob);
        }

        // Extract and update LSTM states
        let new_h: ArrayView3<f32> = outputs["new_h"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract new_h: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape new_h: {}", e)))?;

        let new_c: ArrayView3<f32> = outputs["new_c"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract new_c: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape new_c: {}", e)))?;

        // Copy state data
        if self.debug && self.current_sample == 0 {
            eprintln!("  h_state before: sum={}", self.h_state.sum());
            eprintln!("  c_state before: sum={}", self.c_state.sum());
            eprintln!("  new_h sum: {}", new_h.sum());
            eprintln!("  new_c sum: {}", new_c.sum());
        }
        self.h_state.assign(&new_h);
        self.c_state.assign(&new_c);

        self.current_sample += audio_chunk.len();

        if self.debug {
            // Print every chunk between 1-2 seconds where we expect speech (RMS=0.087746 in second 1)
            let time_s = self.current_sample as f32 / self.sample_rate as f32;
            if time_s >= 1.0 && time_s <= 2.0 {
                eprintln!("VAD: t={:.2}s, prob={:.6}, threshold={:.3}", time_s, speech_prob, self.threshold);
            } else if time_s >= 3.0 && time_s <= 4.5 {
                eprintln!("VAD: t={:.2}s, prob={:.6}, threshold={:.3}", time_s, speech_prob, self.threshold);
            } else if self.current_sample % (self.sample_rate as usize) == 0 {
                eprintln!("VAD: t={:.2}s, prob={:.6}, threshold={:.3}", time_s, speech_prob, self.threshold);
            }
        }

        // Improved speech detection with buffering
        if speech_prob >= self.threshold {
            // Speech detected
            if !self.triggered {
                self.triggered = true;
            }
            // Update temp_end to track the END of speech (last sample where speech was detected)
            self.temp_end = self.current_sample;
            // Add samples to buffer
            self.speech_buffer.extend_from_slice(audio_chunk);
        } else if self.triggered {
            // Was speaking, now silence
            if self.current_sample - self.temp_end > self.min_silence_samples {
                // Silence duration exceeded threshold - speech segment complete
                if self.speech_buffer.len() >= self.min_speech_samples {
                    // Have enough speech samples to return
                    let speech = self.speech_buffer.clone();
                    self.speech_buffer.clear();
                    self.triggered = false;
                    return Ok(Some(speech));
                } else {
                    // Not enough speech, discard (was noise)
                    self.speech_buffer.clear();
                    self.triggered = false;
                }
            } else {
                // Still within silence tolerance, keep buffering
                self.speech_buffer.extend_from_slice(audio_chunk);
            }
        }

        Ok(None)
    }

    /// Reset the VAD state
    pub fn reset(&mut self) {
        self.h_state.fill(0.0);
        self.c_state.fill(0.0);
        self.triggered = false;
        self.temp_end = 0;
        self.current_sample = 0;
        self.speech_buffer.clear();
    }

    /// Flush any remaining buffered speech (call at end of stream)
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if !self.speech_buffer.is_empty() && self.speech_buffer.len() >= self.min_speech_samples {
            let speech = self.speech_buffer.clone();
            self.speech_buffer.clear();
            self.triggered = false;
            Some(speech)
        } else {
            self.speech_buffer.clear();
            self.triggered = false;
            None
        }
    }
}