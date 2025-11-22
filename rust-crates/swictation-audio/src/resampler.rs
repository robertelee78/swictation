//! Audio resampling with rubato
//!
//! Converts audio from any sample rate to 16kHz mono for STT models.

use rubato::{
    Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

use crate::error::{AudioError, Result};

/// Resampler for converting audio to target sample rate
pub struct Resampler {
    source_rate: u32,
    target_rate: u32,
    channels: u16,
    resampler: Option<SincFixedIn<f32>>,
}

impl Resampler {
    /// Create new resampler
    ///
    /// # Arguments
    ///
    /// * `source_rate` - Source sample rate (e.g., 48000)
    /// * `target_rate` - Target sample rate (typically 16000)
    /// * `channels` - Number of channels (1 = mono, 2 = stereo)
    pub fn new(source_rate: u32, target_rate: u32, channels: u16) -> Result<Self> {
        if source_rate == 0 || target_rate == 0 {
            return Err(AudioError::invalid_config("Sample rate cannot be zero"));
        }

        if channels == 0 {
            return Err(AudioError::invalid_config("Channel count cannot be zero"));
        }

        // If rates are the same, no resampling needed
        let resampler = if source_rate != target_rate {
            Some(Self::create_resampler(source_rate, target_rate, channels)?)
        } else {
            None
        };

        Ok(Self {
            source_rate,
            target_rate,
            channels,
            resampler,
        })
    }

    /// Create rubato resampler instance
    fn create_resampler(
        source_rate: u32,
        target_rate: u32,
        channels: u16,
    ) -> Result<SincFixedIn<f32>> {
        // Use high-quality sinc interpolation for best audio quality
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        // Calculate chunk size (process 100ms at a time)
        let chunk_size = (source_rate as f32 * 0.1) as usize;

        let resampler = SincFixedIn::<f32>::new(
            target_rate as f64 / source_rate as f64,
            2.0, // max_resample_ratio_relative
            params,
            chunk_size,
            channels as usize,
        ).map_err(|e| AudioError::ResampleError(format!("Failed to create resampler: {:?}", e)))?;

        Ok(resampler)
    }

    /// Resample audio data
    ///
    /// # Arguments
    ///
    /// * `input` - Input audio samples (interleaved if multi-channel)
    ///
    /// # Returns
    ///
    /// Resampled audio at target sample rate
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        // If no resampling needed, return input as-is
        if self.resampler.is_none() {
            return Ok(input.to_vec());
        }

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let resampler = self.resampler.as_mut().unwrap();

        // Convert interleaved to planar format (rubato expects Vec<Vec<f32>>)
        let frames = input.len() / self.channels as usize;
        let mut planar_input = vec![vec![0.0f32; frames]; self.channels as usize];

        for (frame_idx, frame) in input.chunks(self.channels as usize).enumerate() {
            for (ch_idx, &sample) in frame.iter().enumerate() {
                planar_input[ch_idx][frame_idx] = sample;
            }
        }

        // Process with resampler
        let planar_output = resampler.process(&planar_input, None)
            .map_err(|e| AudioError::ResampleError(format!("Resampling failed: {:?}", e)))?;

        // Convert planar back to interleaved
        let output_frames = planar_output[0].len();
        let mut interleaved_output = Vec::with_capacity(output_frames * self.channels as usize);

        for frame_idx in 0..output_frames {
            for channel_data in planar_output.iter().take(self.channels as usize) {
                interleaved_output.push(channel_data[frame_idx]);
            }
        }

        Ok(interleaved_output)
    }

    /// Convert stereo to mono by averaging channels
    pub fn stereo_to_mono(stereo: &[f32]) -> Vec<f32> {
        if !stereo.len().is_multiple_of(2) {
            // Odd length, just take every other sample
            return stereo.iter().step_by(2).copied().collect();
        }

        stereo.chunks(2)
            .map(|frame| (frame[0] + frame[1]) / 2.0)
            .collect()
    }

    /// Get expected output length for given input length
    pub fn expected_output_len(&self, input_len: usize) -> usize {
        if self.resampler.is_none() {
            return input_len;
        }

        let frames = input_len / self.channels as usize;
        let output_frames = (frames as f64 * self.target_rate as f64 / self.source_rate as f64) as usize;
        output_frames * self.channels as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_resampling_needed() {
        let mut resampler = Resampler::new(16000, 16000, 1).unwrap();
        let input = vec![0.5, 0.3, 0.1, -0.2];
        let output = resampler.process(&input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_resampling_48k_to_16k() {
        let mut resampler = Resampler::new(48000, 16000, 1).unwrap();

        // Generate 100ms of audio at 48kHz (smaller chunk for fixed-in resampler)
        let input: Vec<f32> = (0..4800)
            .map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5)
            .collect();

        let output = resampler.process(&input).unwrap();

        // Output should be approximately 1/3 the length (48kHz → 16kHz)
        // Expected: 4800 / 3 = 1600 samples
        assert!(output.len() > 1500 && output.len() < 1700,
                "Output length {} not in expected range (expected ~1600)", output.len());
    }

    #[test]
    fn test_stereo_to_mono() {
        let stereo = vec![0.5, 0.3, 0.1, -0.1, 0.2, 0.4];
        let mono = Resampler::stereo_to_mono(&stereo);

        assert_eq!(mono.len(), 3);
        assert_eq!(mono[0], 0.4); // (0.5 + 0.3) / 2
        assert_eq!(mono[1], 0.0); // (0.1 + -0.1) / 2
        assert_eq!(mono[2], 0.3); // (0.2 + 0.4) / 2
    }

    #[test]
    fn test_invalid_config() {
        assert!(Resampler::new(0, 16000, 1).is_err());
        assert!(Resampler::new(48000, 0, 1).is_err());
        assert!(Resampler::new(48000, 16000, 0).is_err());
    }
}
