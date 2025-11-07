//! Audio → VAD → STT pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use swictation_audio::{AudioCapture, AudioCaptureConfig};
use swictation_vad::{VadConfig, VadDetector, VadResult};
use swictation_stt::{Recognizer, RecognizerConfig};

use crate::config::DaemonConfig;

/// Pipeline state
pub struct Pipeline {
    /// Audio capture
    audio: AudioCapture,

    /// Voice Activity Detection
    vad: VadDetector,

    /// Speech-to-Text recognizer
    stt: Recognizer,

    /// Recording state
    is_recording: bool,

    /// Transcription result channel
    tx: mpsc::UnboundedSender<Result<String>>,
    rx: mpsc::UnboundedReceiver<Result<String>>,
}

impl Pipeline {
    /// Create new pipeline with GPU acceleration
    pub async fn new(config: DaemonConfig, gpu_provider: Option<String>) -> Result<Self> {
        info!("Initializing Audio capture...");
        let audio_config = AudioCaptureConfig {
            sample_rate: 16000,
            channels: 1,
            buffer_size: 8000, // 0.5 seconds
        };
        let audio = AudioCapture::new(audio_config)
            .context("Failed to initialize audio capture")?;

        info!("Initializing VAD with {} provider...",
              gpu_provider.as_deref().unwrap_or("CPU"));
        let vad_config = VadConfig::with_model(&config.vad_model_path)
            .min_silence(config.vad_min_silence)
            .min_speech(config.vad_min_speech)
            .max_speech(config.vad_max_speech)
            .threshold(config.vad_threshold)
            .provider(gpu_provider.clone())
            .num_threads(config.num_threads);

        let vad = VadDetector::new(vad_config)
            .context("Failed to initialize VAD")?;

        info!("Initializing STT with {} provider...",
              gpu_provider.as_deref().unwrap_or("CPU"));
        let stt_config = RecognizerConfig {
            model_path: config.stt_model_path.clone(),
            tokens_path: config.stt_tokens_path.clone(),
            provider: gpu_provider,
            num_threads: config.num_threads,
            sample_rate: 16000,
            enable_endpoint: true,
            decoding_method: "greedy_search".to_string(),
            max_active_paths: 4,
            hotwords_file: None,
            hotwords_score: 1.5,
        };

        let stt = Recognizer::new(stt_config)
            .context("Failed to initialize STT recognizer")?;

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            audio,
            vad,
            stt,
            is_recording: false,
            tx,
            rx,
        })
    }

    /// Start recording and processing
    pub async fn start_recording(&mut self) -> Result<()> {
        if self.is_recording {
            return Ok(());
        }

        self.is_recording = true;
        info!("Recording started");

        // Start audio capture
        // Note: For now, we'll collect audio and process in batches
        // A more sophisticated implementation would use Arc<Mutex<>> for thread-safe sharing
        // TODO: Implement proper streaming with thread-safe VAD/STT

        // For now, just start audio capture - processing will be implemented next
        self.audio.start(move |_samples: &[f32]| {
            // TODO: Implement pipeline processing
            // This requires thread-safe wrappers around VAD and STT
        })?;

        Ok(())
    }

    /// Stop recording
    pub async fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        self.is_recording = false;
        self.audio.stop()?;

        // Flush remaining audio through VAD
        self.vad.flush();

        info!("Recording stopped");
        Ok(())
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get next transcription result
    pub async fn next_transcription(&mut self) -> Option<Result<String>> {
        self.rx.recv().await
    }

    /// Get audio sample rate
    pub fn audio_sample_rate(&self) -> u32 {
        16000
    }

    /// Get audio channels
    pub fn audio_channels(&self) -> u16 {
        1
    }

    /// Shutdown pipeline
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.is_recording {
            self.stop_recording().await?;
        }
        Ok(())
    }
}

