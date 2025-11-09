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

    // State for streaming (combined RNN state: [2, 1, 128])
    state: Array3<f32>,
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

        // Calculate sample counts from durations
        let min_speech_samples = (min_speech_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
        let min_silence_samples = (min_silence_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;

        // Initialize RNN state (2 layers, 1 batch, 128 hidden units)
        let state = Array3::<f32>::zeros((2, 1, 128));

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            sample_rate,
            window_size,
            threshold,
            min_speech_samples,
            min_silence_samples,
            state,
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
            eprintln!("  state shape: {:?}", self.state.shape());
            eprintln!("  state is C-contiguous: {}", self.state.is_standard_layout());
            eprintln!("  sr value: {}", self.sample_rate);
        }

        // Create tensors from arrays for ort 2.0 - need to clone the data since Tensor takes ownership
        let input_tensor = Tensor::from_array(input_array)
            .map_err(|e| VadError::processing(format!("Failed to create input tensor: {}", e)))?;

        let state_tensor = Tensor::from_array(self.state.clone())
            .map_err(|e| VadError::processing(format!("Failed to create state tensor: {}", e)))?;

        // Create sample rate tensor - sherpa-onnx uses shape [1], not scalar []
        // Even though ONNX model metadata says [], the C++ code uses [1]
        let sr_array = ndarray::Array1::from(vec![self.sample_rate as i64]);
        let sr_tensor = Tensor::from_array(sr_array)
            .map_err(|e| VadError::processing(format!("Failed to create sr tensor: {}", e)))?;

        // Run inference using named inputs (order doesn't matter with named inputs)
        let mut session_guard = self.session
            .lock()
            .map_err(|e| VadError::processing(format!("Failed to lock session: {}", e)))?;

        let outputs = session_guard
            .run(inputs![
                "input" => input_tensor,
                "state" => state_tensor,
                "sr" => sr_tensor
            ])
            .map_err(|e| VadError::processing(format!("Failed to run inference: {}", e)))?;

        if self.debug && self.current_sample == 0 {
            eprintln!("  Model returned {} outputs", outputs.len());
        }

        // Extract speech probability from the output (shape: [batch, 1])
        let output_array: ndarray::ArrayView2<f32> = outputs["output"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract output: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape output: {}", e)))?;
        let speech_prob = output_array[[0, 0]];

        if self.debug && self.current_sample == 0 {
            eprintln!("  Output array shape: {:?}", output_array.shape());
            eprintln!("  Output array: {:?}", output_array);
            eprintln!("  Speech probability extracted: {}", speech_prob);
        }

        // Extract and update state
        let state_n: ArrayView3<f32> = outputs["stateN"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract stateN: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape stateN: {}", e)))?;

        // Copy state data
        if self.debug && self.current_sample == 0 {
            eprintln!("  State before update: sum={}", self.state.sum());
            eprintln!("  State after inference: sum={}", state_n.sum());
        }
        self.state.assign(&state_n);

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
                self.temp_end = self.current_sample;
            }
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
        self.state.fill(0.0);
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