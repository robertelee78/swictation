//! Audio → VAD → STT → Midstream → Text Injection pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

use swictation_audio::AudioCapture;
use swictation_vad::{VadConfig, VadDetector, VadResult};
use swictation_stt::Recognizer;
use midstreamer_text_transform::transform;

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

    /// Transcription result channel sender
    tx: mpsc::UnboundedSender<Result<String>>,
}

impl Pipeline {
    /// Create new pipeline with GPU acceleration
    /// Returns (Pipeline, transcription_receiver)
    pub async fn new(config: DaemonConfig, gpu_provider: Option<String>) -> Result<(Self, mpsc::UnboundedReceiver<Result<String>>)> {
        info!("Initializing Audio capture...");
        let audio_config = swictation_audio::AudioConfig {
            sample_rate: 16000,
            channels: 1,
            blocksize: 1024,
            buffer_duration: 10.0,
            device_index: config.audio_device_index,
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

        let pipeline = Self {
            audio: Arc::new(Mutex::new(audio)),
            vad: Arc::new(Mutex::new(vad)),
            stt: Arc::new(Mutex::new(stt)),
            is_recording: false,
            tx,
        };

        Ok((pipeline, rx))
    }

    /// Start recording and processing
    pub async fn start_recording(&mut self) -> Result<()> {
        if self.is_recording {
            return Ok(());
        }

        self.is_recording = true;
        info!("Recording started");

        // Create channel for audio chunks (cpal callback → VAD/STT processing)
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();

        // Set up audio callback to push chunks via channel
        {
            let mut audio = self.audio.lock().unwrap();
            let audio_tx_clone = audio_tx.clone();

            audio.set_chunk_callback(move |chunk| {
                // This runs in cpal's audio thread
                let _ = audio_tx_clone.send(chunk);
            });

            // Start audio capture (cpal will invoke callback)
            audio.start()?;
        }

        // Clone components for VAD/STT processing thread
        let vad = self.vad.clone();
        let stt = self.stt.clone();
        let tx = self.tx.clone();

        // Spawn VAD→STT processing thread (receives audio chunks via channel)
        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(16000); // 1 second buffer
            let mut chunk_count = 0;

            while let Some(chunk) = audio_rx.recv().await {
                chunk_count += 1;
                if chunk_count % 10 == 0 {
                    eprintln!("DEBUG: Received {} chunks, chunk size: {}", chunk_count, chunk.len());
                }
                buffer.extend_from_slice(&chunk);

                // Process in 0.5 second chunks for VAD
                while buffer.len() >= 8000 { // 0.5 second chunks at 16kHz
                    let vad_chunk: Vec<f32> = buffer.drain(..8000).collect();
                    eprintln!("DEBUG: Processing VAD chunk, buffer len: {}", buffer.len());

                    // Process through VAD
                    let mut vad_lock = match vad.lock() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("VAD lock error: {}", e);
                            continue;
                        }
                    };

                    match vad_lock.process_audio(&vad_chunk) {
                        Ok(VadResult::Speech { samples: speech_samples, .. }) => {
                            eprintln!("DEBUG: VAD detected speech! {} samples", speech_samples.len());
                            drop(vad_lock); // Release VAD lock before STT

                            // Process through STT (full segment with context for accuracy)
                            let mut stt_lock = match stt.lock() {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("STT lock error: {}", e);
                                    continue;
                                }
                            };

                            match stt_lock.recognize(&speech_samples) {
                                Ok(result) => {
                                    // Transform voice commands → symbols (Midstream)
                                    // "hello comma world" → "hello, world"
                                    let transformed = transform(&result.text);
                                    info!("Transcribed: {} → {}", result.text, transformed);
                                    let _ = tx.send(Ok(transformed));
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e.into()));
                                }
                            }
                        }
                        Ok(VadResult::Silence) => {
                            eprintln!("DEBUG: VAD detected silence");
                            // Skip silence (VAD ensures we only transcribe speech segments)
                            // This preserves context for better STT accuracy
                        }
                        Err(e) => {
                            eprintln!("VAD error: {}", e);
                        }
                    }
                }
            }
        });

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

