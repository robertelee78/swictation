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
use ndarray::{Array2, Array3, ArrayView1, ArrayView3};

/// Silero VAD model using direct ONNX Runtime
pub struct SileroVadOrt {
    session: Arc<Mutex<Session>>,
    sample_rate: i32,
    window_size: usize,
    threshold: f32,
    min_speech_samples: usize,
    min_silence_samples: usize,

    // State for streaming
    h: Array3<f32>,
    c: Array3<f32>,
    last_sr: i32,
    triggered: bool,
    temp_end: usize,
    current_sample: usize,
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

        // Initialize RNN hidden states (2 layers, 1 batch, 64 hidden units)
        let h = Array3::<f32>::zeros((2, 1, 64));
        let c = Array3::<f32>::zeros((2, 1, 64));

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            sample_rate,
            window_size,
            threshold,
            min_speech_samples,
            min_silence_samples,
            h,
            c,
            last_sr: 0,
            triggered: false,
            temp_end: 0,
            current_sample: 0,
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
        let input_array = Array2::from_shape_vec((1, audio_chunk.len()), audio_chunk.to_vec())
            .map_err(|e| VadError::processing(format!("Failed to reshape input: {}", e)))?;

        // Create tensors from arrays for ort 2.0 - need to clone the data since Tensor takes ownership
        let input_tensor = Tensor::from_array(input_array)
            .map_err(|e| VadError::processing(format!("Failed to create input tensor: {}", e)))?;

        let h_tensor = Tensor::from_array(self.h.clone())
            .map_err(|e| VadError::processing(format!("Failed to create h tensor: {}", e)))?;

        let c_tensor = Tensor::from_array(self.c.clone())
            .map_err(|e| VadError::processing(format!("Failed to create c tensor: {}", e)))?;

        let sr_tensor = Tensor::from_array(([1], vec![self.sample_rate as i64]))
            .map_err(|e| VadError::processing(format!("Failed to create sr tensor: {}", e)))?;

        // Run inference using the inputs! macro with named inputs
        let mut session_guard = self.session
            .lock()
            .map_err(|e| VadError::processing(format!("Failed to lock session: {}", e)))?;

        let outputs = session_guard
            .run(inputs![
                "input" => input_tensor,
                "h" => h_tensor,
                "c" => c_tensor,
                "sr" => sr_tensor
            ])
            .map_err(|e| VadError::processing(format!("Failed to run inference: {}", e)))?;

        // Extract speech probability from the first output
        let output_array: ArrayView1<f32> = outputs["output"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract output: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape output: {}", e)))?;
        let speech_prob = output_array[0];

        // Extract and update hidden states
        let hn_array: ArrayView3<f32> = outputs["hn"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract hn: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape hn: {}", e)))?;

        let cn_array: ArrayView3<f32> = outputs["cn"]
            .try_extract_array()
            .map_err(|e| VadError::processing(format!("Failed to extract cn: {}", e)))?
            .into_dimensionality()
            .map_err(|e| VadError::processing(format!("Failed to reshape cn: {}", e)))?;

        // Copy hidden state data
        self.h.assign(&hn_array);
        self.c.assign(&cn_array);

        self.current_sample += audio_chunk.len();

        // Simple speech detection logic
        if speech_prob >= self.threshold {
            if !self.triggered {
                self.triggered = true;
                self.temp_end = self.current_sample;
            }
        } else if self.triggered {
            if self.current_sample - self.temp_end > self.min_silence_samples {
                // Speech ended
                self.triggered = false;
                return Ok(Some(audio_chunk.to_vec())); // Simplified: return current chunk
            }
        }

        Ok(None)
    }

    /// Reset the VAD state
    pub fn reset(&mut self) {
        self.h.fill(0.0);
        self.c.fill(0.0);
        self.last_sr = 0;
        self.triggered = false;
        self.temp_end = 0;
        self.current_sample = 0;
    }
}