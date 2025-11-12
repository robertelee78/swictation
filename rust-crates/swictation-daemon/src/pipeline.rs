//! Audio → VAD → STT → Midstream → Text Injection pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};
use chrono::Utc;

use swictation_audio::AudioCapture;
use swictation_vad::{VadConfig, VadDetector, VadResult};
use swictation_stt::{SttEngine, Recognizer, OrtRecognizer};
use midstreamer_text_transform::transform;
use swictation_metrics::{MetricsCollector, SegmentMetrics};
use swictation_broadcaster::MetricsBroadcaster;

use crate::config::DaemonConfig;
use crate::gpu::get_gpu_memory_mb;

/// Pipeline state
pub struct Pipeline {
    /// Audio capture
    audio: Arc<Mutex<AudioCapture>>,

    /// Voice Activity Detection
    vad: Arc<Mutex<VadDetector>>,

    /// Speech-to-Text engine (adaptive: 1.1B GPU / 0.6B GPU / 0.6B CPU)
    stt: Arc<Mutex<SttEngine>>,

    /// Metrics collector
    metrics: Arc<Mutex<MetricsCollector>>,

    /// Recording state
    is_recording: bool,

    /// Current session ID (set when recording starts)
    session_id: Arc<Mutex<Option<i64>>>,

    /// Metrics broadcaster for real-time updates
    broadcaster: Arc<Mutex<Option<Arc<MetricsBroadcaster>>>>,

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
        let vad_config = VadConfig::with_model(config.vad_model_path.display().to_string())
            .min_silence(config.vad_min_silence)
            .min_speech(config.vad_min_speech)
            .max_speech(config.vad_max_speech)
            .threshold(config.vad_threshold)
            .provider(gpu_provider.clone())
            .num_threads(config.num_threads)
            .debug();  // Enable VAD debug output for troubleshooting

        let vad = VadDetector::new(vad_config)
            .context("Failed to initialize VAD")?;

        // ADAPTIVE MODEL SELECTION based on GPU VRAM availability
        // Decision tree:
        //   ≥4GB VRAM → 1.1B INT8 GPU (peak 3.5GB + 596MB headroom)
        //   ≥1.5GB VRAM → 0.6B GPU (peak 1.2GB + 336MB headroom)
        //   <1.5GB or no GPU → 0.6B CPU fallback
        //
        // Config override: stt_model_override can force a specific model:
        //   "auto" = VRAM-based selection (default)
        //   "0.6b-cpu" = Force 0.6B CPU
        //   "0.6b-gpu" = Force 0.6B GPU
        //   "1.1b-gpu" = Force 1.1B GPU

        let stt = if config.stt_model_override != "auto" {
            // MANUAL OVERRIDE: User specified exact model
            info!("STT model override active: {}", config.stt_model_override);

            match config.stt_model_override.as_str() {
                "1.1b-gpu" => {
                    info!("  Loading Parakeet-TDT-1.1B-INT8 via ONNX Runtime (forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_1_1b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 1.1B INT8 model from {}. \
                            \nError: {}", config.stt_1_1b_model_path.display(), e
                        ))?;
                    info!("✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU, forced)");
                    SttEngine::Parakeet1_1B(ort_recognizer)
                }
                "0.6b-gpu" => {
                    info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (GPU, forced)...");
                    let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B GPU model from {}. \
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)");
                    SttEngine::Parakeet0_6B(recognizer)
                }
                "0.6b-cpu" => {
                    info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (CPU, forced)...");
                    let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B CPU model from {}. \
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)");
                    SttEngine::Parakeet0_6B(recognizer)
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid stt_model_override: '{}'. \
                        Valid options: 'auto', '0.6b-cpu', '0.6b-gpu', '1.1b-gpu'",
                        config.stt_model_override
                    ));
                }
            }
        } else {
            // AUTO MODE: VRAM-based adaptive selection
            info!("STT model selection: auto (VRAM-based)");
            info!("Detecting GPU memory for adaptive model selection...");
            let vram_mb = get_gpu_memory_mb().map(|(total, _free)| total);

            if let Some(vram) = vram_mb {
            info!("Detected GPU with {}MB VRAM", vram);

                if vram >= 4096 {
                    // High VRAM: Use 1.1B INT8 model for best quality (5.77% WER)
                    info!("✓ Sufficient VRAM for 1.1B INT8 model (requires ≥4GB)");
                    info!("  Loading Parakeet-TDT-1.1B-INT8 via ONNX Runtime...");

                    let ort_recognizer = OrtRecognizer::new(&config.stt_1_1b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                        "Failed to load 1.1B INT8 model despite {}MB VRAM. \
                        \nTroubleshooting:\
                        \n  1. Verify model files exist: ls {}\
                        \n  2. Check CUDA/cuDNN installation: nvidia-smi\
                        \n  3. Ensure ONNX Runtime CUDA EP is available\
                        \n  4. Try 0.6B fallback by setting stt_model_override=\"0.6b-gpu\" in config\
                        \nError: {}", vram, config.stt_1_1b_model_path.display(), e
                    ))?;

                    info!("✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU)");
                    SttEngine::Parakeet1_1B(ort_recognizer)

                } else if vram >= 1536 {
                    // Moderate VRAM: Use 0.6B GPU for good quality (7-8% WER)
                    info!("✓ Sufficient VRAM for 0.6B GPU model (requires ≥1.5GB)");
                    info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (GPU)...");

                    let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B GPU model despite {}MB VRAM. \
                            \nTroubleshooting:\
                            \n  1. Verify model files: ls {}\
                            \n  2. Check CUDA availability: nvidia-smi\
                            \n  3. Verify sherpa-rs CUDA support\
                            \n  4. Try CPU fallback by setting stt_model_override=\"0.6b-cpu\" in config\
                            \nError: {}", vram, config.stt_0_6b_model_path.display(), e
                        ))?;

                    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU)");
                    SttEngine::Parakeet0_6B(recognizer)

                } else {
                    // Low VRAM: Fall back to CPU
                    warn!("⚠️  Only {}MB VRAM available (need ≥1.5GB for GPU)", vram);
                    warn!("  Falling back to CPU mode (slower but functional)");
                    info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (CPU)...");

                    let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B CPU model. \
                            \nTroubleshooting:\
                            \n  1. Verify model files: ls {}\
                            \n  2. Check available RAM (need ~1GB free)\
                            \n  3. Ensure ONNX Runtime CPU EP is available\
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;

                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                    SttEngine::Parakeet0_6B(recognizer)
                }
            } else {
                // No GPU detected: Fall back to CPU
                warn!("⚠️  No GPU detected (nvidia-smi failed or no NVIDIA GPU)");
                warn!("  Falling back to CPU mode (slower but functional)");
                info!("  Loading Parakeet-TDT-0.6B via sherpa-rs (CPU)...");

                let recognizer = Recognizer::new(&config.stt_0_6b_model_path, false)
                    .map_err(|e| anyhow::anyhow!(
                        "Failed to load 0.6B CPU model. \
                        \nTroubleshooting:\
                        \n  1. Verify model files: ls {}\
                        \n  2. Check available RAM (need ~1GB free)\
                        \n  3. Ensure ONNX Runtime CPU EP is available\
                        \nError: {}", config.stt_0_6b_model_path.display(), e
                    ))?;

                info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                SttEngine::Parakeet0_6B(recognizer)
            }
        };

        // Log final configuration
        info!("📊 STT Engine: {} ({}, {})",
              stt.model_name(),
              stt.model_size(),
              stt.backend());

        if stt.vram_required_mb() > 0 {
            info!("   Minimum VRAM: {}MB", stt.vram_required_mb());
        }

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
            false, // store_transcription_text - keep transcriptions ephemeral
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
            session_id: Arc::new(Mutex::new(None)),
            broadcaster: Arc::new(Mutex::new(None)),
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
        let metrics = self.metrics.clone();
        let session_id = self.session_id.clone();
        let broadcaster = self.broadcaster.clone();

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

                            // Track timing for metrics
                            let segment_start = Instant::now();
                            let vad_latency = segment_start.elapsed().as_millis() as f64;

                            drop(vad_lock); // Release VAD lock before STT

                            // Process through STT (full segment with context for accuracy)
                            let stt_start = Instant::now();
                            let mut stt_lock = match stt.lock() {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("STT lock error: {}", e);
                                    continue;
                                }
                            };

                            // Use Sherpa-RS recognizer
                            let result = stt_lock.recognize(&speech_samples).unwrap_or_else(|e| {
                                eprintln!("STT transcribe error: {}", e);
                                swictation_stt::RecognitionResult {
                                    text: String::new(),
                                    confidence: 0.0,
                                    processing_time_ms: 0.0,
                                }
                            });
                            let text = result.text;
                            let stt_latency = stt_start.elapsed().as_millis() as f64;

                            if !text.is_empty() {
                                // Transform voice commands → symbols (Midstream)
                                // "hello comma world" → "hello, world"
                                let transform_start = Instant::now();
                                let transformed = transform(&text);
                                let transform_latency = transform_start.elapsed().as_micros() as f64;

                                info!("Transcribed: {} → {}", text, transformed);

                                // Track segment metrics (ephemeral - no text stored in DB)
                                let word_count = transformed.split_whitespace().count() as i32;
                                let char_count = transformed.len() as i32;

                                // Get current session ID
                                let current_session_id = *session_id.lock().unwrap();

                                if let Some(sid) = current_session_id {
                                    let duration_s = (speech_samples.len() as f64) / 16000.0; // samples / sample_rate
                                    let total_latency_ms = vad_latency + stt_latency + (transform_latency / 1000.0);

                                    let segment = SegmentMetrics {
                                        segment_id: None,
                                        session_id: Some(sid),
                                        timestamp: Some(Utc::now()),
                                        duration_s,
                                        words: word_count,
                                        characters: char_count,
                                        text: transformed.clone(), // Will be ignored since store_text=false
                                        vad_latency_ms: vad_latency,
                                        audio_save_latency_ms: 0.0, // Not tracking this yet
                                        stt_latency_ms: stt_latency,
                                        transform_latency_us: transform_latency,
                                        injection_latency_ms: 0.0, // Will be tracked later when injecting
                                        total_latency_ms,
                                        transformations_count: if text != transformed { 1 } else { 0 },
                                        keyboard_actions_count: 0, // Could be tracked in text injection
                                    };

                                    // Add segment to metrics
                                    if let Err(e) = metrics.lock().unwrap().add_segment(segment) {
                                        eprintln!("Failed to add segment metrics: {}", e);
                                    }

                                    // Broadcast transcription to UI clients
                                    if let Some(ref broadcaster_ref) = *broadcaster.lock().unwrap() {
                                        let wpm = (word_count as f64 / (duration_s / 60.0)).min(300.0); // Cap at 300 WPM
                                        tokio::spawn({
                                            let broadcaster = broadcaster_ref.clone();
                                            let text_clone = transformed.clone();
                                            async move {
                                                broadcaster.add_transcription(
                                                    text_clone,
                                                    wpm,
                                                    total_latency_ms,
                                                    word_count,
                                                ).await;
                                            }
                                        });
                                    }
                                }

                                // Add trailing space between VAD chunks (always, unless already has whitespace)
                                // This fixes: "hello world" + "testing" → "hello worldtesting" (NO SPACE)
                                //        AND: "hello world." + "testing" → "hello world.testing" (NO SPACE AFTER PUNCTUATION)
                                // Solution: ALWAYS add trailing space (proper secretary would leave space after punctuation)
                                let final_text = if transformed.ends_with(char::is_whitespace) {
                                    transformed  // Already has trailing whitespace
                                } else {
                                    format!("{} ", transformed)  // Add space for next chunk (even after punctuation)
                                };

                                let _ = tx.send(Ok(final_text));
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

        // Flush remaining audio through VAD and process any final speech
        if let Some(vad_result) = self.vad.lock().unwrap().flush() {
            if let swictation_vad::VadResult::Speech { samples: speech_samples, .. } = vad_result {
                info!("Processing flushed speech segment: {} samples", speech_samples.len());

                // DEBUG: Save flushed audio to file for analysis
                match save_audio_debug(&speech_samples, "/tmp/swictation_flushed_audio.wav") {
                    Ok(()) => eprintln!("DEBUG: Saved flushed audio to /tmp/swictation_flushed_audio.wav"),
                    Err(e) => eprintln!("DEBUG: Failed to save audio: {}", e),
                }

                // Track timing for metrics
                let segment_start = Instant::now();
                let vad_latency = segment_start.elapsed().as_millis() as f64;

                // Process through STT (same pattern as start_recording)
                let stt_start = Instant::now();
                let mut stt_lock = match self.stt.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("STT lock error during flush: {}", e);
                        info!("Recording stopped");
                        return Ok(());
                    }
                };

                let result = stt_lock.recognize(&speech_samples).unwrap_or_else(|e| {
                    eprintln!("STT transcribe error during flush: {}", e);
                    swictation_stt::RecognitionResult {
                        text: String::new(),
                        confidence: 0.0,
                        processing_time_ms: 0.0,
                    }
                });
                let text = result.text;
                let stt_latency = stt_start.elapsed().as_millis() as f64;

                if !text.is_empty() {
                    // Transform voice commands → symbols (Midstream)
                    let transform_start = Instant::now();
                    let transformed = transform(&text);
                    let transform_latency = transform_start.elapsed().as_micros() as f64;

                    info!("Flushed transcription: {} → {}", text, transformed);

                    // Track segment metrics
                    let word_count = transformed.split_whitespace().count() as i32;
                    let char_count = transformed.len() as i32;

                    let current_session_id = *self.session_id.lock().unwrap();

                    if let Some(sid) = current_session_id {
                        let duration_s = (speech_samples.len() as f64) / 16000.0;
                        let total_latency_ms = vad_latency + stt_latency + (transform_latency / 1000.0);

                        let segment = SegmentMetrics {
                            segment_id: None,
                            session_id: Some(sid),
                            timestamp: Some(Utc::now()),
                            duration_s,
                            words: word_count,
                            characters: char_count,
                            text: transformed.clone(),
                            vad_latency_ms: vad_latency,
                            audio_save_latency_ms: 0.0,
                            stt_latency_ms: stt_latency,
                            transform_latency_us: transform_latency,
                            injection_latency_ms: 0.0,
                            total_latency_ms,
                            transformations_count: if text != transformed { 1 } else { 0 },
                            keyboard_actions_count: 0,
                        };

                        if let Err(e) = self.metrics.lock().unwrap().add_segment(segment) {
                            eprintln!("Failed to add flushed segment metrics: {}", e);
                        }

                        // Broadcast transcription to UI clients
                        if let Some(ref broadcaster_ref) = *self.broadcaster.lock().unwrap() {
                            let wpm = (word_count as f64 / (duration_s / 60.0)).min(300.0);
                            tokio::spawn({
                                let broadcaster = broadcaster_ref.clone();
                                let text_clone = transformed.clone();
                                async move {
                                    broadcaster.add_transcription(
                                        text_clone,
                                        wpm,
                                        total_latency_ms,
                                        word_count,
                                    ).await;
                                }
                            });
                        }
                    }

                    // Send through transcription channel
                    let _ = self.tx.send(Ok(transformed));
                }
            }
        }

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

    /// Set the current session ID
    pub fn set_session_id(&self, session_id: i64) {
        *self.session_id.lock().unwrap() = Some(session_id);
    }

    /// Clear the session ID
    pub fn clear_session_id(&self) {
        *self.session_id.lock().unwrap() = None;
    }

    /// Set the broadcaster for real-time updates
    pub fn set_broadcaster(&self, broadcaster: Arc<MetricsBroadcaster>) {
        *self.broadcaster.lock().unwrap() = Some(broadcaster);
    }
}

/// DEBUG: Save audio samples to WAV file for analysis
fn save_audio_debug(samples: &[f32], path: &str) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| anyhow::anyhow!("Failed to create WAV file: {}", e))?;

    for &sample in samples {
        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(sample_i16)
            .map_err(|e| anyhow::anyhow!("Failed to write sample: {}", e))?;
    }

    writer.finalize()
        .map_err(|e| anyhow::anyhow!("Failed to finalize WAV: {}", e))?;
    Ok(())
}

