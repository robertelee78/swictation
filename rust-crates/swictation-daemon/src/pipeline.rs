//! Audio → VAD → STT pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use swictation_audio::AudioCapture;
use swictation_vad::{VadConfig, VadDetector, VadResult};
use swictation_stt::Recognizer;

use crate::config::DaemonConfig;

/// Pipeline state
pub struct Pipeline {
    /// Audio capture
    audio: Arc<Mutex<AudioCapture>>,

    /// Voice Activity Detection
    vad: Arc<Mutex<VadDetector>>,

    /// Speech-to-Text recognizer
    stt: Arc<Mutex<Recognizer>>,

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
        let audio_config = swictation_audio::AudioConfig {
            sample_rate: 16000,
            channels: 1,
            blocksize: 1024,
            buffer_duration: 10.0,
            device_index: None,
            streaming_mode: true,
            chunk_duration: 0.5,
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
        // Note: GPU provider and num_threads will be configured via ORT env vars
        // when ParakeetModel supports them
        let stt = Recognizer::new(&config.stt_model_path)
            .context("Failed to initialize STT recognizer")?;

        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            audio: Arc::new(Mutex::new(audio)),
            vad: Arc::new(Mutex::new(vad)),
            stt: Arc::new(Mutex::new(stt)),
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

        // Start audio capture with thread-safe pipeline
        let _vad = self.vad.clone();
        let _stt = self.stt.clone();
        let _tx = self.tx.clone();

        // Note: Full audio → VAD → STT pipeline implementation pending
        // AudioCapture callback needs to be Send + Sync safe
        // This will be implemented when we make Pipeline fully thread-safe
        self.audio.lock().unwrap().start()?;

        Ok(())
    }

    /// Stop recording
    pub async fn stop_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Ok(());
        }

        self.is_recording = false;
        self.audio.lock().unwrap().stop()?;

        // Flush remaining audio through VAD
        self.vad.lock().unwrap().flush();

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

