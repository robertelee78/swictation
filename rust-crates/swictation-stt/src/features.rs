//! Audio feature extraction for Parakeet model
//!
//! Parakeet-TDT expects 128-dimensional mel filterbank features at 16kHz

use crate::error::{Result, SttError};
use std::f32::consts::PI;

/// Feature extractor configuration
#[derive(Debug, Clone)]
pub struct FeatureConfig {
    /// Sample rate in Hz
    pub sample_rate: usize,
    /// Feature dimension (mel bins)
    pub feature_dim: usize,
    /// Frame length in samples
    pub frame_length: usize,
    /// Frame shift in samples
    pub frame_shift: usize,
    /// FFT size
    pub fft_size: usize,
    /// Lower frequency bound
    pub f_min: f32,
    /// Upper frequency bound
    pub f_max: f32,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            feature_dim: 128,
            frame_length: 400,  // 25ms at 16kHz
            frame_shift: 160,   // 10ms at 16kHz
            fft_size: 512,
            f_min: 0.0,
            f_max: 8000.0,
        }
    }
}

/// Feature extractor for mel filterbank features
pub struct FeatureExtractor {
    config: FeatureConfig,
    mel_banks: Vec<Vec<f32>>,
}

impl FeatureExtractor {
    /// Create new feature extractor
    pub fn new(config: FeatureConfig) -> Self {
        let mel_banks = Self::create_mel_filterbanks(&config);
        Self { config, mel_banks }
    }

    /// Extract features from audio samples
    ///
    /// # Arguments
    ///
    /// * `audio` - Audio samples (16kHz, mono, f32)
    ///
    /// # Returns
    ///
    /// Feature matrix [num_frames, feature_dim]
    pub fn extract(&self, audio: &[f32]) -> Result<Vec<Vec<f32>>> {
        if audio.is_empty() {
            return Err(SttError::invalid_input("Audio is empty"));
        }

        let num_frames = (audio.len() - self.config.frame_length) / self.config.frame_shift + 1;
        if num_frames == 0 {
            return Err(SttError::invalid_input("Audio too short for feature extraction"));
        }

        let mut features = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * self.config.frame_shift;
            let end = start + self.config.frame_length;

            if end > audio.len() {
                break;
            }

            let frame = &audio[start..end];
            let mel_features = self.compute_mel_features(frame)?;
            features.push(mel_features);
        }

        Ok(features)
    }

    /// Compute mel features for a single frame
    fn compute_mel_features(&self, frame: &[f32]) -> Result<Vec<f32>> {
        // Apply Hamming window
        let windowed = self.apply_hamming_window(frame);

        // Compute FFT magnitude spectrum
        let spectrum = self.compute_power_spectrum(&windowed)?;

        // Apply mel filterbanks
        let mel_features = self.apply_mel_filterbanks(&spectrum);

        // Apply log
        let log_mel = mel_features
            .iter()
            .map(|&x| (x + 1e-10).ln())
            .collect();

        Ok(log_mel)
    }

    /// Apply Hamming window to frame
    fn apply_hamming_window(&self, frame: &[f32]) -> Vec<f32> {
        let n = frame.len();
        frame
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let window = 0.54 - 0.46 * (2.0 * PI * i as f32 / (n - 1) as f32).cos();
                x * window
            })
            .collect()
    }

    /// Compute power spectrum using FFT
    fn compute_power_spectrum(&self, frame: &[f32]) -> Result<Vec<f32>> {
        // Simple DFT implementation (could be optimized with FFT crate later)
        let n = self.config.fft_size;
        let mut spectrum = vec![0.0; n / 2 + 1];

        for k in 0..=n / 2 {
            let mut real = 0.0;
            let mut imag = 0.0;

            for (i, &x) in frame.iter().enumerate() {
                let angle = -2.0 * PI * k as f32 * i as f32 / n as f32;
                real += x * angle.cos();
                imag += x * angle.sin();
            }

            spectrum[k] = (real * real + imag * imag) / n as f32;
        }

        Ok(spectrum)
    }

    /// Apply mel filterbanks to power spectrum
    fn apply_mel_filterbanks(&self, spectrum: &[f32]) -> Vec<f32> {
        self.mel_banks
            .iter()
            .map(|bank| {
                bank.iter()
                    .zip(spectrum.iter())
                    .map(|(b, s)| b * s)
                    .sum()
            })
            .collect()
    }

    /// Create mel filterbanks
    fn create_mel_filterbanks(config: &FeatureConfig) -> Vec<Vec<f32>> {
        let num_bins = config.feature_dim;
        let fft_bins = config.fft_size / 2 + 1;

        // Convert Hz to Mel scale
        let mel_min = Self::hz_to_mel(config.f_min);
        let mel_max = Self::hz_to_mel(config.f_max);

        // Create equally spaced mel points
        let mel_points: Vec<f32> = (0..=num_bins + 1)
            .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (num_bins + 1) as f32)
            .collect();

        // Convert back to Hz
        let hz_points: Vec<f32> = mel_points.iter().map(|&m| Self::mel_to_hz(m)).collect();

        // Convert Hz to FFT bin indices
        let bin_points: Vec<usize> = hz_points
            .iter()
            .map(|&f| ((fft_bins as f32 * f) / config.sample_rate as f32).floor() as usize)
            .collect();

        // Create triangular filters
        let mut banks = Vec::with_capacity(num_bins);

        for i in 0..num_bins {
            let mut bank = vec![0.0; fft_bins];

            let left = bin_points[i];
            let center = bin_points[i + 1];
            let right = bin_points[i + 2];

            // Rising slope
            for j in left..center {
                if center > left {
                    bank[j] = (j - left) as f32 / (center - left) as f32;
                }
            }

            // Falling slope
            for j in center..right {
                if right > center {
                    bank[j] = (right - j) as f32 / (right - center) as f32;
                }
            }

            banks.push(bank);
        }

        banks
    }

    /// Convert Hz to Mel scale
    fn hz_to_mel(hz: f32) -> f32 {
        2595.0 * (1.0 + hz / 700.0).log10()
    }

    /// Convert Mel to Hz scale
    fn mel_to_hz(mel: f32) -> f32 {
        700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_extraction() {
        let config = FeatureConfig::default();
        let extractor = FeatureExtractor::new(config);

        // Test with 1 second of silence
        let audio = vec![0.0; 16000];
        let features = extractor.extract(&audio).unwrap();

        assert!(!features.is_empty());
        assert_eq!(features[0].len(), 128);
    }

    #[test]
    fn test_mel_conversion() {
        let hz = 1000.0;
        let mel = FeatureExtractor::hz_to_mel(hz);
        let hz_back = FeatureExtractor::mel_to_hz(mel);

        assert!((hz - hz_back).abs() < 0.1);
    }
}
