//! Direct ONNX Runtime implementation of Silero VAD
//! Replaces sherpa-rs dependency with modern ort crate

use crate::{Result, VadError};
use ort::{
    execution_providers::{CUDAExecutionProvider, CPUExecutionProvider},
    session::Session,
    inputs,
};
use std::sync::Arc;
use ndarray::{Array1, Array3, ArrayView1};

/// Silero VAD model using direct ONNX Runtime
pub struct SileroVadOrt {
    session: Arc<Session>,
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
        let builder = if let Some(ref prov) = provider {
            if prov.contains("cuda") || prov.contains("CUDA") {
                // Try CUDA provider
                match Session::builder()?.with_execution_providers([CUDAExecutionProvider::default().build()]) {
                    Ok(b) => {
                        println!("Silero VAD: Using CUDA provider");
                        b
                    }
                    Err(e) => {
                        println!("Silero VAD: CUDA not available ({}), falling back to CPU", e);
                        Session::builder()?.with_execution_providers([CPUExecutionProvider::default().build()])
                            .map_err(|e| VadError::initialization(format!("Failed to create CPU session: {}", e)))?
                    }
                }
            } else {
                Session::builder()?.with_execution_providers([CPUExecutionProvider::default().build()])
                    .map_err(|e| VadError::initialization(format!("Failed to create CPU session: {}", e)))?
            }
        } else {
            Session::builder()?.with_execution_providers([CPUExecutionProvider::default().build()])
                .map_err(|e| VadError::initialization(format!("Failed to create CPU session: {}", e)))?
        };

        let session = builder
            .with_model_from_file(model_path)
            .map_err(|e| VadError::initialization(format!("Failed to load Silero VAD model: {}", e)))?;

        // Calculate sample counts from durations
        let min_speech_samples = (min_speech_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
        let min_silence_samples = (min_silence_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;

        // Initialize RNN hidden states (2 layers, 1 batch, 64 hidden units)
        let h = Array3::<f32>::zeros((2, 1, 64));
        let c = Array3::<f32>::zeros((2, 1, 64));

        Ok(Self {
            session: Arc::new(session),
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

        // Convert audio to ndarray
        let input_array = Array1::from_vec(audio_chunk.to_vec())
            .into_shape((1, audio_chunk.len()))
            .map_err(|e| VadError::processing(format!("Failed to reshape input: {}", e)))?;

        // Prepare sample rate as scalar
        let sr_array = Array1::from_vec(vec![self.sample_rate as i64]);

        // Run inference
        let outputs = self.session.run(inputs![
            "input" => input_array.view(),
            "h" => self.h.view(),
            "c" => self.c.view(),
            "sr" => sr_array.view()
        ].map_err(|e| VadError::processing(format!("Failed to prepare inputs: {}", e)))?)?;

        // Extract outputs
        let output = outputs["output"]
            .try_extract_tensor::<f32>()
            .map_err(|e| VadError::processing(format!("Failed to extract output tensor: {}", e)))?;
        let speech_prob = output.view().first().copied().unwrap_or(0.0);

        // Update hidden states
        let hn = outputs["hn"]
            .try_extract_tensor::<f32>()
            .map_err(|e| VadError::processing(format!("Failed to extract hn tensor: {}", e)))?;
        let cn = outputs["cn"]
            .try_extract_tensor::<f32>()
            .map_err(|e| VadError::processing(format!("Failed to extract cn tensor: {}", e)))?;

        // Copy hidden state data
        self.h.assign(&hn.view().into_dimensionality().unwrap());
        self.c.assign(&cn.view().into_dimensionality().unwrap());

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