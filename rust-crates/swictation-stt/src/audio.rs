//! Audio processing for Parakeet-TDT models
//!
//! Handles audio file loading, resampling, and mel-spectrogram feature extraction
//! compatible with NVIDIA NeMo Parakeet-TDT models.

use crate::error::{Result, SttError};
use hound::WavReader;
use ndarray::{s, Array2};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{debug, info};

/// Target sample rate for Parakeet-TDT models
pub const SAMPLE_RATE: u32 = 16000;

/// Mel-spectrogram parameters for Parakeet-TDT models
pub const N_MEL_FEATURES: usize = 128;  // Number of mel filters (0.6B model)
pub const N_MEL_FEATURES_1_1B: usize = 80; // Number of mel filters (1.1B model)
pub const N_FFT: usize = 512;            // FFT size
pub const HOP_LENGTH: usize = 160;       // 10ms hop at 16kHz
pub const WIN_LENGTH: usize = 400;       // 25ms window at 16kHz
pub const CHUNK_FRAMES: usize = 80;      // Frames per encoder chunk

/// Audio processor for Parakeet-TDT models
pub struct AudioProcessor {
    mel_filters: Array2<f32>,
    fft_planner: FftPlanner<f32>,
    n_mel_features: usize,
}

impl AudioProcessor {
    /// Create new audio processor with Parakeet-TDT parameters
    pub fn new() -> Result<Self> {
        Self::with_mel_features(N_MEL_FEATURES)
    }

    /// Create audio processor with custom mel feature count
    ///
    /// # Arguments
    ///
    /// * `n_mel_features` - Number of mel filterbank features (80 for 1.1B, 128 for 0.6B)
    pub fn with_mel_features(n_mel_features: usize) -> Result<Self> {
        // Create mel filterbank
        // NeMo models use: low_freq=0, high_freq=8000 (or default SR/2)
        // See sherpa-onnx/csrc/offline-recognizer-transducer-nemo-impl.h:164-165
        let mel_filters = create_mel_filterbank(
            n_mel_features,
            N_FFT,
            SAMPLE_RATE as f32,
            0.0,     // NeMo models use low_freq=0
            8000.0,  // NeMo models use high_freq=8000
        );

        Ok(Self {
            mel_filters,
            fft_planner: FftPlanner::new(),
            n_mel_features,
        })
    }

    /// Load audio from file (WAV or MP3)
    pub fn load_audio<P: AsRef<Path>>(&self, path: P) -> Result<Vec<f32>> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| SttError::AudioLoadError(
                "Could not determine file extension".to_string()
            ))?;

        match extension.to_lowercase().as_str() {
            "wav" => self.load_wav(path),
            "mp3" | "flac" | "ogg" => self.load_with_symphonia(path),
            _ => Err(SttError::AudioLoadError(format!(
                "Unsupported audio format: {}",
                extension
            ))),
        }
    }

    /// Load WAV file
    fn load_wav<P: AsRef<Path>>(&self, path: P) -> Result<Vec<f32>> {
        let mut reader = WavReader::open(path.as_ref())
            .map_err(|e| SttError::AudioLoadError(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        info!(
            "Loaded WAV: {} Hz, {} channels, {} bits",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        );

        // Read all samples and convert to f32 mono
        let samples: Vec<f32> = if spec.bits_per_sample == 16 {
            reader
                .samples::<i16>()
                .map(|s| s.map(|sample| sample as f32 / 32768.0))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| SttError::AudioLoadError(format!("Failed to read samples: {}", e)))?
        } else if spec.bits_per_sample == 32 {
            reader
                .samples::<i32>()
                .map(|s| s.map(|sample| sample as f32 / 2147483648.0))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| SttError::AudioLoadError(format!("Failed to read samples: {}", e)))?
        } else {
            return Err(SttError::AudioLoadError(format!(
                "Unsupported bit depth: {}",
                spec.bits_per_sample
            )));
        };

        // Convert stereo to mono if needed
        let mono_samples = if spec.channels == 1 {
            samples
        } else {
            samples
                .chunks(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect()
        };

        // Resample if needed
        if spec.sample_rate != SAMPLE_RATE {
            self.resample(&mono_samples, spec.sample_rate, SAMPLE_RATE)
        } else {
            Ok(mono_samples)
        }
    }

    /// Load audio file using Symphonia (MP3, FLAC, etc.)
    fn load_with_symphonia<P: AsRef<Path>>(&self, path: P) -> Result<Vec<f32>> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| SttError::AudioLoadError(format!("Failed to open file: {}", e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();
        let decoder_opts = DecoderOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| SttError::AudioLoadError(format!("Failed to probe format: {}", e)))?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SttError::AudioLoadError("No audio tracks found".to_string()))?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.ok_or_else(|| {
            SttError::AudioLoadError("Could not determine sample rate".to_string())
        })?;
        let channels_count = track.codec_params.channels.map(|c| c.count()).unwrap_or(0);
        let codec_params = track.codec_params.clone();

        info!(
            "Loaded audio via Symphonia: {} Hz, {} channels",
            sample_rate, channels_count
        );

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &decoder_opts)
            .map_err(|e| SttError::AudioLoadError(format!("Failed to create decoder: {}", e)))?;

        let mut samples = Vec::new();
        let mut sample_buf = None;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    return Err(SttError::AudioLoadError(format!(
                        "Failed to read packet: {}",
                        e
                    )))
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = decoder
                .decode(&packet)
                .map_err(|e| SttError::AudioLoadError(format!("Failed to decode: {}", e)))?;

            if sample_buf.is_none() {
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
            }

            if let Some(ref mut buf) = sample_buf {
                buf.copy_interleaved_ref(decoded);
                samples.extend_from_slice(buf.samples());
            }
        }

        // Convert to mono
        let mono_samples: Vec<f32> = if channels_count == 1 {
            samples
        } else {
            samples
                .chunks(channels_count)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect()
        };

        // Resample if needed
        if sample_rate != SAMPLE_RATE {
            self.resample(&mono_samples, sample_rate, SAMPLE_RATE)
        } else {
            Ok(mono_samples)
        }
    }

    /// Simple linear resampling (for basic resampling needs)
    fn resample(&self, samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
        if from_rate == to_rate {
            return Ok(samples.to_vec());
        }

        info!("Resampling from {} Hz to {} Hz", from_rate, to_rate);

        let ratio = from_rate as f64 / to_rate as f64;
        let new_len = (samples.len() as f64 / ratio).ceil() as usize;
        let mut resampled = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let src_idx = (i as f64 * ratio) as usize;
            if src_idx < samples.len() {
                resampled.push(samples[src_idx]);
            }
        }

        Ok(resampled)
    }

    /// Extract mel-spectrogram features from audio samples
    ///
    /// Returns a 2D array of shape (num_frames, N_MEL_FEATURES)
    ///
    /// Features are normalized to mean=0, std=1 as expected by NVIDIA NeMo models
    pub fn extract_mel_features(&mut self, samples: &[f32]) -> Result<Array2<f32>> {
        debug!("Extracting mel-spectrogram from {} samples", samples.len());

        // Apply preemphasis filter (standard for speech recognition)
        let preemphasized = apply_preemphasis(samples, 0.97);

        // Compute STFT
        let stft = self.compute_stft(&preemphasized)?;
        debug!("STFT shape: {:?}", stft.shape());

        // Compute power spectrogram
        let power_spec = stft.mapv(|c| c.re * c.re + c.im * c.im);

        // Apply mel filterbank
        // power_spec is (frames, freqs) = (num_frames, 257)
        // mel_filters is (n_mels, freqs) = (128, 257)
        // Result should be (frames, n_mels) = (num_frames, 128)
        let mel_spec = power_spec.dot(&self.mel_filters.t());
        debug!("Mel spectrogram shape: {:?}", mel_spec.shape());

        // Apply log scaling (add small epsilon to avoid log(0))
        let log_mel = mel_spec.mapv(|x| (x + 1e-10).ln());

        // DEBUG: Check log_mel values before normalization
        if log_mel.nrows() > 0 {
            debug!("BEFORE normalization - first frame (first 10): {:?}",
                   &log_mel.row(0).as_slice().unwrap()[..10.min(log_mel.ncols())]);
            if log_mel.nrows() > 10 {
                debug!("BEFORE normalization - middle frame {} (first 10): {:?}",
                       log_mel.nrows() / 2,
                       &log_mel.row(log_mel.nrows() / 2).as_slice().unwrap()[..10.min(log_mel.ncols())]);
            }
            debug!("BEFORE normalization - stats: min={:.6}, max={:.6}, mean={:.6}",
                   log_mel.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
                   log_mel.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
                   log_mel.mean().unwrap_or(0.0));

            // DEBUG: Check mel filterbank to ensure it's not all zeros
            debug!("Mel filterbank sum: {:.6}, shape: {:?}",
                   self.mel_filters.sum(),
                   self.mel_filters.shape());

            // Check which mel bins have non-zero sum (should be ALL of them!)
            for mel_idx in 0..8 {
                let row_sum: f32 = self.mel_filters.row(mel_idx).sum();
                debug!("Mel bin {}: sum={:.6}", mel_idx, row_sum);
            }
        }

        // Normalize features per mel-bin (across time) as expected by NVIDIA NeMo models
        // Each mel feature gets normalized independently across all frames
        let mut normalized = log_mel.clone();

        for mel_idx in 0..self.n_mel_features {
            // Get all values for this mel bin across all frames
            let mel_column = log_mel.column(mel_idx);
            let mean = mel_column.mean().unwrap_or(0.0);
            let std = mel_column.std(0.0);  // ddof=0 for population std

            // Normalize this mel bin
            if std > 1e-8 {
                for frame_idx in 0..log_mel.nrows() {
                    normalized[[frame_idx, mel_idx]] = (log_mel[[frame_idx, mel_idx]] - mean) / std;
                }
            } else {
                // If std is too small, just subtract mean
                for frame_idx in 0..log_mel.nrows() {
                    normalized[[frame_idx, mel_idx]] = log_mel[[frame_idx, mel_idx]] - mean;
                }
            }
        }

        debug!("Extracted features: shape {:?}, normalized per-feature",
               normalized.shape());
        Ok(normalized)
    }

    /// Compute Short-Time Fourier Transform (STFT)
    fn compute_stft(&mut self, samples: &[f32]) -> Result<Array2<Complex<f32>>> {
        let num_frames = (samples.len() - WIN_LENGTH) / HOP_LENGTH + 1;
        let mut stft = Array2::zeros((num_frames, N_FFT / 2 + 1));

        // Create Povey window (NeMo models use Povey, not Hann)
        // See sherpa-onnx/csrc/features.h:61 - default is "povey"
        let window = povey_window(WIN_LENGTH);

        // Create FFT plan
        let fft = self.fft_planner.plan_fft_forward(N_FFT);

        for frame_idx in 0..num_frames {
            let start = frame_idx * HOP_LENGTH;
            let end = start + WIN_LENGTH;

            if end > samples.len() {
                break;
            }

            // NOTE: NeMo models use remove_dc_offset = FALSE
            // See sherpa-onnx/csrc/offline-recognizer-transducer-nemo-impl.h:167
            // GigaAM uses remove_dc_offset = false
            // Standard NeMo models (Parakeet-TDT) also use remove_dc_offset = false

            // Apply window and zero-pad to N_FFT
            let mut buffer: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); N_FFT];
            for i in 0..WIN_LENGTH {
                // NO DC offset removal for NeMo models!
                buffer[i] = Complex::new(samples[start + i] * window[i], 0.0);
            }

            // Compute FFT
            fft.process(&mut buffer);

            // Store magnitude spectrum (first half + DC and Nyquist)
            for i in 0..=N_FFT / 2 {
                stft[[frame_idx, i]] = buffer[i];
            }
        }

        Ok(stft)
    }

    /// Split features into chunks of 80 frames for encoder
    ///
    /// Returns a Vec of 3D arrays with shape (1, 80, 128) suitable for encoder input
    pub fn chunk_features(&self, features: &Array2<f32>) -> Vec<Array2<f32>> {
        let total_frames = features.nrows();
        let mut chunks = Vec::new();

        for start in (0..total_frames).step_by(CHUNK_FRAMES) {
            let end = (start + CHUNK_FRAMES).min(total_frames);
            let chunk_size = end - start;

            if chunk_size < CHUNK_FRAMES {
                // Pad last chunk if needed
                let mut padded = Array2::zeros((CHUNK_FRAMES, N_MEL_FEATURES));
                padded
                    .slice_mut(s![..chunk_size, ..])
                    .assign(&features.slice(s![start..end, ..]));
                chunks.push(padded);
            } else {
                chunks.push(features.slice(s![start..end, ..]).to_owned());
            }
        }

        chunks
    }
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioProcessor")
    }
}

/// Create Hann window for STFT
/// Povey window (Kaldi-style, used by NeMo models)
/// Similar to Hamming but with pow(0.5 - 0.5*cos, 0.85) for better frequency resolution
fn povey_window(window_length: usize) -> Vec<f32> {
    (0..window_length)
        .map(|n| {
            let factor = 2.0 * PI * n as f32 / (window_length - 1) as f32;
            let base = 0.5 - 0.5 * factor.cos();
            base.powf(0.85)  // Kaldi's Povey window exponent
        })
        .collect()
}

fn hann_window(window_length: usize) -> Vec<f32> {
    (0..window_length)
        .map(|n| {
            let factor = 2.0 * PI * n as f32 / (window_length - 1) as f32;
            0.5 * (1.0 - factor.cos())
        })
        .collect()
}

/// Apply preemphasis filter to audio signal
///
/// Preemphasis emphasizes high-frequency components which improves
/// speech recognition accuracy. Standard coefficient is ~0.97.
///
/// Formula: y[n] = x[n] - coef * x[n-1]
fn apply_preemphasis(audio: &[f32], coef: f32) -> Vec<f32> {
    let mut result = Vec::with_capacity(audio.len());

    // First sample remains unchanged
    result.push(audio[0]);

    // Apply filter to remaining samples
    for i in 1..audio.len() {
        result.push(audio[i] - coef * audio[i - 1]);
    }

    result
}

/// Convert Hz to mel scale
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale to Hz
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Create mel filterbank matrix
///
/// Returns a matrix of shape (n_mels, n_fft/2 + 1) where each row is a triangular filter
/// This matches the parakeet-rs implementation which works in frequency space
fn create_mel_filterbank(
    n_mels: usize,
    n_fft: usize,
    sample_rate: f32,
    fmin: f32,
    fmax: f32,
) -> Array2<f32> {
    let freq_bins = n_fft / 2 + 1;

    // Create mel-spaced frequency points (in Hz)
    let mel_min = hz_to_mel(fmin);
    let mel_max = hz_to_mel(fmax);
    let mel_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| {
            let mel = mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32;
            mel_to_hz(mel)
        })
        .collect();

    // Frequency bin width in Hz
    let freq_bin_width = sample_rate / n_fft as f32;

    // Create filterbank: shape (n_mels, freq_bins)
    let mut filterbank = Array2::zeros((n_mels, freq_bins));

    for mel_idx in 0..n_mels {
        let left = mel_points[mel_idx];
        let center = mel_points[mel_idx + 1];
        let right = mel_points[mel_idx + 2];

        for freq_idx in 0..freq_bins {
            let freq = freq_idx as f32 * freq_bin_width;

            // Rising slope: left to center
            if freq >= left && freq <= center {
                if center != left {
                    filterbank[[mel_idx, freq_idx]] = (freq - left) / (center - left);
                }
            }
            // Falling slope: center to right
            else if freq > center && freq <= right {
                if right != center {
                    filterbank[[mel_idx, freq_idx]] = (right - freq) / (right - center);
                }
            }
        }
    }

    filterbank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_processor_creation() {
        let processor = AudioProcessor::new();
        assert!(processor.is_ok());
    }

    #[test]
    fn test_mel_filterbank() {
        let processor = AudioProcessor::new().unwrap();
        // Verify mel filterbank has correct shape (n_mels, n_fft/2 + 1)
        assert_eq!(processor.mel_filters.shape(), &[N_MEL_FEATURES, N_FFT / 2 + 1]);
    }
}
