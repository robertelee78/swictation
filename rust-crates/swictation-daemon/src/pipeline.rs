//! Audio → VAD → STT → Midstream → Text Injection pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

use swictation_audio::AudioCapture;
use swictation_vad::{VadConfig, VadDetector, VadResult};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};
use midstreamer_text_transform::transform;
use swictation_metrics::MetricsCollector;

use crate::config::DaemonConfig;

/// Pipeline state
pub struct Pipeline {
    /// Audio capture
    audio: Arc<Mutex<AudioCapture>>,

    /// Voice Activity Detection
    vad: Arc<Mutex<VadDetector>>,

    /// Speech-to-Text recognizer
    stt: Arc<Mutex<TransducerRecognizer>>,

    /// Metrics collector
    metrics: Arc<Mutex<MetricsCollector>>,

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

        // Use sherpa-rs (working implementation)
        let stt_config = TransducerConfig {
            encoder: format!("{}/encoder.int8.onnx", config.stt_model_path),
            decoder: format!("{}/decoder.int8.onnx", config.stt_model_path),
            joiner: format!("{}/joiner.int8.onnx", config.stt_model_path),
            tokens: format!("{}/tokens.txt", config.stt_model_path),
            num_threads: config.num_threads.unwrap_or(4),
            sample_rate: 16000,
            feature_dim: 128,
            model_type: "nemo_transducer".to_string(),
            decoding_method: "greedy_search".to_string(),
            hotwords_file: String::new(),
            hotwords_score: 1.5,
            modeling_unit: String::new(),
            bpe_vocab: String::new(),
            blank_penalty: 0.0,
            debug: false,
            provider: gpu_provider.clone(),
        };

        let stt = TransducerRecognizer::new(stt_config)
            .map_err(|e| anyhow::anyhow!("Failed to initialize STT recognizer: {}", e))?;

        info!("Initializing metrics collector...");

        // Initialize metrics collector with database
        let metrics_db_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("swictation")
            .join("metrics.db");

        // Ensure directory exists
        if let Some(parent) = metrics_db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create metrics directory")?;
        }

        let metrics = MetricsCollector::new(
            metrics_db_path.to_str().unwrap(),
            40.0,  // typing_baseline_wpm
            true,  // store_transcription_text
            true,  // warnings_enabled
            1000.0, // high_latency_threshold_ms
            80.0,   // gpu_memory_threshold_percent
        ).context("Failed to initialize metrics collector")?;

        // Enable GPU monitoring if provider is available
        if let Some(ref provider) = gpu_provider {
            metrics.enable_gpu_monitoring(provider);
        }

        let (tx, rx) = mpsc::unbounded_channel();

        let pipeline = Self {
            audio: Arc::new(Mutex::new(audio)),
            vad: Arc::new(Mutex::new(vad)),
            stt: Arc::new(Mutex::new(stt)),
            metrics: Arc::new(Mutex::new(metrics)),
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

                    // Check audio levels
                    let max_amplitude = vad_chunk.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                    let avg_amplitude = vad_chunk.iter().map(|x| x.abs()).sum::<f32>() / vad_chunk.len() as f32;
                    eprintln!("DEBUG: Processing VAD chunk, buffer len: {}, max_amplitude: {:.6}, avg_amplitude: {:.6}",
                              buffer.len(), max_amplitude, avg_amplitude);

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

                            // Use sherpa-rs transcribe method
                            let text = stt_lock.transcribe(16000, &speech_samples);

                            if !text.is_empty() {
                                // Transform voice commands → symbols (Midstream)
                                // "hello comma world" → "hello, world"
                                let transformed = transform(&text);
                                info!("Transcribed: {} → {}", text, transformed);
                                let _ = tx.send(Ok(transformed));
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

    /// Get metrics collector (clone Arc for external use)
    pub fn get_metrics(&self) -> Arc<Mutex<MetricsCollector>> {
        self.metrics.clone()
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

