//! Speech recognition with Parakeet RNN-T model

use crate::error::Result;
use crate::model::ParakeetModel;
use ndarray::{Array2, Array3};
use ort::{inputs, value::Value};
use std::path::Path;

/// Recognition result
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

/// Speech recognizer
pub struct Recognizer {
    /// Parakeet model
    model: ParakeetModel,
}

impl Recognizer {
    /// Create new recognizer from model directory
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model = ParakeetModel::from_directory(model_path)?;
        Ok(Self { model })
    }

    /// Create recognizer from existing model
    pub fn from_model(model: ParakeetModel) -> Self {
        Self { model }
    }

    /// Recognize speech from audio samples
    ///
    /// # Arguments
    ///
    /// * `audio` - Audio samples (16kHz, mono, f32)
    ///
    /// # Returns
    ///
    /// Recognition result with transcribed text
    pub fn recognize(&mut self, audio: &[f32]) -> Result<RecognitionResult> {
        let start = std::time::Instant::now();

        // 1. Extract mel filterbank features
        let feature_extractor = crate::features::FeatureExtractor::new(
            crate::features::FeatureConfig::default()
        );
        let features = feature_extractor.extract(audio)?;

        if features.is_empty() {
            return Err(crate::error::SttError::invalid_input(
                "No features extracted from audio"
            ));
        }

        // 2. Run RNN-T greedy decoder
        let token_ids = self.greedy_decode(&features)?;

        // 3. Decode token IDs to text
        let text = self.model.tokens.decode(&token_ids)?;

        // 4. Calculate confidence (placeholder - would need joiner scores)
        let confidence = if token_ids.is_empty() { 0.0 } else { 1.0 };

        let processing_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(RecognitionResult {
            text,
            confidence,
            processing_time_ms,
        })
    }

    /// RNN-T greedy decoder
    ///
    /// Implements the RNN-T greedy search algorithm:
    /// - For each encoder timestep:
    ///   - Loop until blank token or max symbols per frame:
    ///     - Run decoder with current token + states
    ///     - Run joiner to get logits
    ///     - Greedy select argmax token
    ///     - Update states if non-blank
    ///
    /// # Arguments
    ///
    /// * `features` - Mel filterbank features [num_frames, 128]
    ///
    /// # Returns
    ///
    /// Vector of token IDs (excluding blank tokens)
    fn greedy_decode(&mut self, features: &[Vec<f32>]) -> Result<Vec<usize>> {
        let num_frames = features.len();
        if num_frames == 0 {
            return Ok(Vec::new());
        }

        let feature_dim = features[0].len();

        // 1. Prepare encoder input: [batch=1, feature_dim=128, time_frames]
        let mut encoder_input_data = Vec::with_capacity(num_frames * feature_dim);
        for frame in features {
            encoder_input_data.extend_from_slice(frame);
        }

        let encoder_input = Array3::from_shape_vec(
            (1, feature_dim, num_frames),
            encoder_input_data,
        )?;

        // 2. Run encoder
        // Encoder expects: audio_signal [batch, 128, time] (f32), length [batch] (i64)
        let encoder_input_tensor = Value::from_array(encoder_input)?;
        let length_tensor = Value::from_array(ndarray::arr1(&[num_frames as i64]))?;

        let encoder_out = {
            let encoder_outputs = self.model.encoder().run(inputs![
                "audio_signal" => &encoder_input_tensor,
                "length" => &length_tensor
            ])?;
            let encoder_out_tensor = encoder_outputs["outputs"]
                .try_extract_tensor::<f32>()?;

            // Convert to ndarray for easier manipulation
            let encoder_out_shape = encoder_out_tensor.0;
            let encoder_out_data = encoder_out_tensor.1;
            Array3::from_shape_vec(
                (encoder_out_shape[0] as usize, encoder_out_shape[1] as usize, encoder_out_shape[2] as usize),
                encoder_out_data.to_vec(),
            )?
        };

        // encoder_out shape: [batch=1, encoder_dim=1024, encoded_time]
        let encoded_time = encoder_out.shape()[2];

        // 3. Initialize decoder states
        // Decoder has 2 LSTM layers with hidden size 640
        // states.1: [2, batch=1, 640] - first state (variable batch)
        // onnx::Slice_3: [2, 1, 640] - second state (fixed at 1)
        let mut decoder_state_1 = Array3::<f32>::zeros((2, 1, 640));
        let decoder_state_2 = Array3::<f32>::zeros((2, 1, 640)); // Fixed size, doesn't change

        // 4. RNN-T greedy search
        // For RNN-T, blank is typically at output index 0 in joiner logits
        // Start decoder with <unk> token (ID 0 in vocabulary)
        let blank_output_id = 0; // Blank in joiner output space
        let start_token = 0; // <unk> token to start decoding
        let max_symbols_per_frame = 3;
        let mut hypothesis: Vec<usize> = Vec::new();
        let mut prev_token = start_token;

        for t in 0..encoded_time {
            let mut symbols_emitted = 0;

            loop {
                // Prepare decoder inputs
                // targets: [batch=1, seq=1] with previous token (int32)
                let targets = Array2::from_shape_vec((1, 1), vec![prev_token as i32])?;
                // target_length: [batch=1] (int32)
                let target_length = ndarray::arr1(&[1i32]);

                // Run decoder with current token and states
                let targets_tensor = Value::from_array(targets)?;
                let target_length_tensor = Value::from_array(target_length)?;
                let decoder_state_1_tensor = Value::from_array(decoder_state_1.clone())?;
                let decoder_state_2_tensor = Value::from_array(decoder_state_2.clone())?;

                let (decoder_out, new_decoder_state_1) = {
                    let decoder_outputs = self.model.decoder().run(inputs![
                        "targets" => &targets_tensor,
                        "target_length" => &target_length_tensor,
                        "states.1" => &decoder_state_1_tensor,
                        "onnx::Slice_3" => &decoder_state_2_tensor
                    ])?;

                    // Extract decoder output: [batch=1, 640, seq=1]
                    let decoder_out_result = decoder_outputs["outputs"]
                        .try_extract_tensor::<f32>()?;
                    let decoder_out = Array3::from_shape_vec(
                        (decoder_out_result.0[0] as usize, decoder_out_result.0[1] as usize, decoder_out_result.0[2] as usize),
                        decoder_out_result.1.to_vec(),
                    )?;

                    // Update first decoder state
                    let decoder_state_result = decoder_outputs["states"]
                        .try_extract_tensor::<f32>()?;
                    let new_decoder_state_1 = Array3::from_shape_vec(
                        (decoder_state_result.0[0] as usize, decoder_state_result.0[1] as usize, decoder_state_result.0[2] as usize),
                        decoder_state_result.1.to_vec(),
                    )?;

                    (decoder_out, new_decoder_state_1)
                };
                decoder_state_1 = new_decoder_state_1;

                // Get encoder embedding at time t: [batch=1, 1024, 1]
                let encoder_proj = encoder_out
                    .slice(ndarray::s![0..1, .., t..t+1])
                    .to_owned();

                // decoder_out shape already [batch=1, 640, seq=1] - perfect for joiner

                // Run joiner to get logits
                let encoder_proj_tensor = Value::from_array(encoder_proj)?;
                let decoder_proj_tensor = Value::from_array(decoder_out.clone())?;
                let logits = {
                    let joiner_outputs = self.model.joiner().run(inputs![
                        "encoder_outputs" => &encoder_proj_tensor,
                        "decoder_outputs" => &decoder_proj_tensor
                    ])?;

                    // Output: [batch, time, seq, vocab] = [1, 1, 1, 8198]
                    let logits_result = joiner_outputs["outputs"]
                        .try_extract_tensor::<f32>()?;
                    // We want [1, 8198] - flatten first 3 dims
                    Array2::from_shape_vec(
                        (1, logits_result.0[3] as usize),
                        logits_result.1.to_vec(),
                    )?
                };

                // logits shape: [batch=1, vocab_size=8198]
                // Find argmax token
                let logits_slice = logits.slice(ndarray::s![0, ..]);
                let (token_id, _score) = logits_slice
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .unwrap();

                if token_id == blank_output_id {
                    // Blank token - move to next time step
                    break;
                }

                // Safety check: ensure token_id is valid for vocabulary
                if token_id >= 8193 {
                    // Invalid token from joiner, treat as blank
                    break;
                }

                // Non-blank token - add to hypothesis
                hypothesis.push(token_id);
                prev_token = token_id;
                symbols_emitted += 1;

                // Prevent infinite loops
                if symbols_emitted >= max_symbols_per_frame {
                    break;
                }
            }
        }

        Ok(hypothesis)
    }

    /// Get model information
    pub fn model_info(&self) -> String {
        format!(
            "Parakeet-TDT-0.6B-V3\n{}\n{}\n{}\nVocab size: {}",
            self.model.encoder_info(),
            self.model.decoder_info(),
            self.model.joiner_info(),
            self.model.tokens.vocab_size()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognizer_creation() {
        let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
        if !Path::new(model_path).exists() {
            println!("Skipping recognizer test - model not found");
            return;
        }

        let recognizer = Recognizer::new(model_path);
        assert!(recognizer.is_ok());
    }
}
