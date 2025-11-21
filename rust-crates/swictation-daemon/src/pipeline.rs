//! Audio → VAD → STT → Midstream → Corrections → Text Injection pipeline integration

use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};
use chrono::Utc;

use swictation_audio::AudioCapture;
use swictation_vad::{VadConfig, VadDetector, VadResult};
use swictation_stt::{SttEngine, OrtRecognizer};
use midstreamer_text_transform::transform;
use swictation_metrics::{MetricsCollector, SegmentMetrics};
use swictation_broadcaster::MetricsBroadcaster;

use crate::config::DaemonConfig;
use crate::capitalization::{apply_capitalization, process_capital_commands};
use crate::corrections::CorrectionEngine;
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

    /// Transcription result channel sender (bounded to prevent OOM)
    tx: mpsc::Sender<Result<String>>,

    /// Learned pattern corrections engine
    corrections: Arc<CorrectionEngine>,
}

impl Pipeline {
    /// Create new pipeline with GPU acceleration
    /// Returns (Pipeline, transcription_receiver)
    pub async fn new(config: DaemonConfig, gpu_provider: Option<String>) -> Result<(Self, mpsc::Receiver<Result<String>>)> {
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
        //   ≥6GB VRAM → 1.1B INT8 GPU (requires ~6GB for safety)
        //   ≥3.5GB VRAM → 0.6B GPU (fits in 4GB with headroom)
        //   <3.5GB or no GPU → 0.6B CPU fallback
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
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (GPU, forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B GPU model from {}. \
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
                }
                "0.6b-cpu" => {
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (CPU, forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B CPU model from {}. \
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
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

                if vram >= 6000 {
                    // High VRAM: Use 1.1B INT8 model for best quality (5.77% WER)
                    info!("✓ Sufficient VRAM for 1.1B INT8 model (requires ≥6GB)");
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

                } else if vram >= 3500 {
                    // Moderate VRAM: Use 0.6B GPU for good quality (7-8% WER)
                    info!("✓ Sufficient VRAM for 0.6B GPU model (requires ≥3.5GB)");
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (GPU)...");

                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, true)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B GPU model despite {}MB VRAM. \
                            \nTroubleshooting:\
                            \n  1. Verify model files: ls {}\
                            \n  2. Check CUDA availability: nvidia-smi\
                            \n  3. Verify ONNX Runtime CUDA support\
                            \n  4. Try CPU fallback by setting stt_model_override=\"0.6b-cpu\" in config\
                            \nError: {}", vram, config.stt_0_6b_model_path.display(), e
                        ))?;

                    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU)");
                    SttEngine::Parakeet0_6B(ort_recognizer)

                } else {
                    // Low VRAM: Fall back to CPU
                    warn!("⚠️  Only {}MB VRAM available (need ≥3.5GB for GPU)", vram);
                    warn!("  Falling back to CPU mode (slower but functional)");
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (CPU)...");

                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| anyhow::anyhow!(
                            "Failed to load 0.6B CPU model. \
                            \nTroubleshooting:\
                            \n  1. Verify model files: ls {}\
                            \n  2. Check available RAM (need ~1GB free)\
                            \n  3. Ensure ONNX Runtime CPU EP is available\
                            \nError: {}", config.stt_0_6b_model_path.display(), e
                        ))?;

                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
                }
            } else {
                // No GPU detected: Fall back to CPU
                warn!("⚠️  No GPU detected (nvidia-smi failed or no NVIDIA GPU)");
                warn!("  Falling back to CPU mode (slower but functional)");
                info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (CPU)...");

                let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, false)
                    .map_err(|e| anyhow::anyhow!(
                        "Failed to load 0.6B CPU model. \
                        \nTroubleshooting:\
                        \n  1. Verify model files: ls {}\
                        \n  2. Check available RAM (need ~1GB free)\
                        \n  3. Ensure ONNX Runtime CPU EP is available\
                        \nError: {}", config.stt_0_6b_model_path.display(), e
                    ))?;

                info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                SttEngine::Parakeet0_6B(ort_recognizer)
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

        // Bounded channel for transcription results (capacity: 100 results)
        // Prevents memory exhaustion if consumer is slow
        let (tx, rx) = mpsc::channel(100);

        // Initialize learned corrections engine with hot-reloading
        info!("Initializing corrections engine...");
        let corrections_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from(".config"))
            .join("swictation");

        // Ensure config directory exists
        std::fs::create_dir_all(&corrections_dir)
            .context("Failed to create config directory")?;

        let mut corrections = CorrectionEngine::new(corrections_dir, 0.3); // 0.3 = phonetic threshold
        if let Err(e) = corrections.start_watching() {
            warn!("Failed to start corrections file watcher: {}. Hot-reload disabled.", e);
        }
        let corrections = Arc::new(corrections);
        info!("✓ Corrections engine initialized");

        let pipeline = Self {
            audio: Arc::new(Mutex::new(audio)),
            vad: Arc::new(Mutex::new(vad)),
            stt: Arc::new(Mutex::new(stt)),
            metrics: Arc::new(Mutex::new(metrics)),
            is_recording: false,
            session_id: Arc::new(Mutex::new(None)),
            broadcaster: Arc::new(Mutex::new(None)),
            tx,
            corrections,
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

        // Create BOUNDED channel for audio chunks (cpal callback → VAD/STT processing)
        // Capacity: 20 chunks = 10 seconds at 0.5s/chunk
        // This prevents memory exhaustion if processing falls behind
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<f32>>(20);

        // Track dropped chunks for metrics
        let dropped_chunks = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped_chunks_clone = dropped_chunks.clone();

        // Set up audio callback to push chunks via channel
        {
            let mut audio = self.audio.lock().unwrap();
            let audio_tx_clone = audio_tx.clone();

            audio.set_chunk_callback(move |chunk| {
                // This runs in cpal's audio thread - must be non-blocking
                match audio_tx_clone.try_send(chunk) {
                    Ok(_) => {
                        // Successfully queued chunk
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Channel full - backpressure activated
                        // Drop this chunk to prevent blocking audio thread
                        dropped_chunks_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        eprintln!("WARNING: Audio chunk dropped (processing too slow). Total dropped: {}",
                                  dropped_chunks_clone.load(std::sync::atomic::Ordering::Relaxed));
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Channel closed - recording stopped
                    }
                }
            });

            // Start audio capture (cpal will invoke callback)
            audio.start()?;
        }

        // Log backpressure warning if chunks are being dropped
        let dropped_monitor = dropped_chunks.clone();
        tokio::spawn(async move {
            let mut last_count = 0u64;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let current = dropped_monitor.load(std::sync::atomic::Ordering::Relaxed);
                if current > last_count {
                    eprintln!("⚠️  BACKPRESSURE: Dropped {} audio chunks in last 5s (STT cannot keep up with speaker)",
                              current - last_count);
                    last_count = current;
                }
                if current == 0 {
                    break; // Recording stopped
                }
            }
        });

        // Clone components for parallel VAD/STT processing
        let vad = self.vad.clone();
        let stt = self.stt.clone();
        let tx = self.tx.clone();
        let metrics = self.metrics.clone();
        let session_id = self.session_id.clone();
        let broadcaster = self.broadcaster.clone();
        let corrections = self.corrections.clone();

        // Create channel for VAD → STT communication
        // Capacity: 10 speech segments (allows VAD to detect ahead while STT processes)
        let (vad_tx, mut stt_rx) = mpsc::channel::<Vec<f32>>(10);

        // Spawn VAD task (processes audio chunks and detects speech segments)
        let vad_task = tokio::spawn(async move {
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

                    // Process through VAD (scoped to ensure lock is dropped before any async ops)
                    let vad_result = {
                        let mut vad_lock = match vad.lock() {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("VAD lock error: {}", e);
                                continue;
                            }
                        };
                        vad_lock.process_audio(&vad_chunk)
                    }; // vad_lock automatically dropped here

                    match vad_result {
                        Ok(VadResult::Speech { samples: speech_samples, .. }) => {
                            eprintln!("DEBUG: VAD detected speech! {} samples", speech_samples.len());

                            // Send speech segment to STT task (non-blocking with backpressure)
                            if let Err(e) = vad_tx.send(speech_samples).await {
                                eprintln!("Failed to send speech segment to STT task: {}", e);
                                break; // STT task has terminated
                            }
                        }
                        Ok(VadResult::Silence) => {
                            eprintln!("DEBUG: VAD detected silence");
                            // Skip silence (VAD ensures we only transcribe speech segments)
                        }
                        Err(e) => {
                            eprintln!("VAD error: {}", e);
                        }
                    }
                }
            }
        });

        // Spawn STT task (processes speech segments from VAD in parallel)
        let stt_task = tokio::spawn(async move {
            while let Some(speech_samples) = stt_rx.recv().await {
                eprintln!("DEBUG: STT processing {} samples", speech_samples.len());

                // Process through STT (scoped to ensure lock is dropped before any async ops)
                let stt_start = Instant::now();
                let (text, stt_latency) = {
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
                    (text, stt_latency)
                }; // stt_lock automatically dropped here

                if !text.is_empty() {
                    // Transform voice commands → symbols (Midstream)
                    // "hello comma world" → "hello, world"
                    let transform_start = Instant::now();

                    // IMPORTANT: 0.6B model via sherpa-rs adds auto-punctuation/capitalization
                    // Strip it before Secretary Mode transformation to prevent double punctuation
                    let cleaned_text = text
                        .to_lowercase()  // Remove auto-capitalization
                        .replace(",", "")  // Remove auto-added commas
                        .replace(".", "")  // Remove auto-added periods
                        .replace("?", "")  // Remove auto-added question marks
                        .replace("!", "")  // Remove auto-added exclamation points
                        .replace(";", "")  // Remove auto-added semicolons
                        .replace(":", "");  // Remove auto-added colons

                    // Step 1: Process capital commands first ("capital r robert" → "Robert")
                    let with_capitals = process_capital_commands(&cleaned_text);

                    // Step 2: Transform punctuation ("comma" → ",")
                    let transformed = transform(&with_capitals);

                    // Step 3: Apply learned corrections ("arkon" → "archon")
                    let corrected = corrections.apply(&transformed, "all");

                    // Flush usage counts if threshold reached
                    if corrections.should_flush() {
                        if let Err(e) = corrections.flush_usage_counts() {
                            warn!("Failed to flush usage counts: {}", e);
                        }
                    }

                    // Step 4: Apply automatic capitalization rules
                    let capitalized = apply_capitalization(&corrected);

                    let transform_latency = transform_start.elapsed().as_micros() as f64;

                    info!("Transcribed: {} → {}", text, capitalized);

                    // Track segment metrics (ephemeral - no text stored in DB)
                    let word_count = capitalized.split_whitespace().count() as i32;
                    let char_count = capitalized.len() as i32;

                    // Get current session ID (scoped to ensure lock is dropped)
                    let current_session_id = {
                        *session_id.lock().unwrap()
                    };

                    if let Some(sid) = current_session_id {
                        let duration_s = (speech_samples.len() as f64) / 16000.0; // samples / sample_rate
                        // Note: VAD latency not tracked in parallel mode (VAD runs independently)
                        let total_latency_ms = stt_latency + (transform_latency / 1000.0);

                        let segment = SegmentMetrics {
                            segment_id: None,
                            session_id: Some(sid),
                            timestamp: Some(Utc::now()),
                            duration_s,
                            words: word_count,
                            characters: char_count,
                            text: capitalized.clone(), // Will be ignored since store_text=false
                            vad_latency_ms: 0.0, // Not tracked in parallel mode
                            audio_save_latency_ms: 0.0,
                            stt_latency_ms: stt_latency,
                            transform_latency_us: transform_latency,
                            injection_latency_ms: 0.0,
                            total_latency_ms,
                            transformations_count: if text != capitalized { 1 } else { 0 },
                            keyboard_actions_count: 0,
                        };

                        // Add segment to metrics (scoped to ensure lock is dropped)
                        {
                            if let Err(e) = metrics.lock().unwrap().add_segment(segment) {
                                eprintln!("Failed to add segment metrics: {}", e);
                            }
                        }

                        // Broadcast transcription to UI clients (scoped to ensure lock is dropped)
                        let broadcaster_clone = {
                            broadcaster.lock().unwrap().as_ref().map(|b| b.clone())
                        };

                        if let Some(broadcaster_ref) = broadcaster_clone {
                            let wpm = (word_count as f64 / (duration_s / 60.0)).min(300.0); // Cap at 300 WPM
                            tokio::spawn({
                                let text_clone = capitalized.clone();
                                async move {
                                    broadcaster_ref.add_transcription(
                                        text_clone,
                                        wpm,
                                        total_latency_ms,
                                        word_count,
                                    ).await;
                                }
                            });
                        }
                    }

                    // Add trailing space between speech segments
                    let final_text = if capitalized.ends_with(char::is_whitespace) {
                        capitalized
                    } else {
                        format!("{} ", capitalized)
                    };

                    // Send transcription (bounded channel - will block if consumer is slow)
                    if let Err(e) = tx.send(Ok(final_text)).await {
                        eprintln!("Failed to send transcription (consumer dropped): {}", e);
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

                    // IMPORTANT: 0.6B model via sherpa-rs adds auto-punctuation/capitalization
                    // Strip it before Secretary Mode transformation to prevent double punctuation
                    let cleaned_text = text
                        .to_lowercase()  // Remove auto-capitalization
                        .replace(",", "")  // Remove auto-added commas
                        .replace(".", "")  // Remove auto-added periods
                        .replace("?", "")  // Remove auto-added question marks
                        .replace("!", "")  // Remove auto-added exclamation points
                        .replace(";", "")  // Remove auto-added semicolons
                        .replace(":", "");  // Remove auto-added colons

                    // Step 1: Process capital commands first
                    let with_capitals = process_capital_commands(&cleaned_text);

                    // Step 2: Transform punctuation
                    let transformed = transform(&with_capitals);

                    // Step 3: Apply learned corrections
                    let corrected = self.corrections.apply(&transformed, "all");

                    // Flush usage counts if threshold reached
                    if self.corrections.should_flush() {
                        if let Err(e) = self.corrections.flush_usage_counts() {
                            warn!("Failed to flush usage counts: {}", e);
                        }
                    }

                    // Step 4: Apply automatic capitalization rules
                    let capitalized = apply_capitalization(&corrected);

                    let transform_latency = transform_start.elapsed().as_micros() as f64;

                    info!("Flushed transcription: {} → {}", text, capitalized);

                    // Track segment metrics
                    let word_count = capitalized.split_whitespace().count() as i32;
                    let char_count = capitalized.len() as i32;

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
                            text: capitalized.clone(),
                            vad_latency_ms: vad_latency,
                            audio_save_latency_ms: 0.0,
                            stt_latency_ms: stt_latency,
                            transform_latency_us: transform_latency,
                            injection_latency_ms: 0.0,
                            total_latency_ms,
                            transformations_count: if text != capitalized { 1 } else { 0 },
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
                                let text_clone = capitalized.clone();
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

                    // Send through transcription channel (bounded - provides backpressure)
                    if let Err(e) = self.tx.send(Ok(capitalized)).await {
                        eprintln!("Failed to send flushed transcription: {}", e);
                    }
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

