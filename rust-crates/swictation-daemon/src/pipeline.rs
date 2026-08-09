//! Audio → VAD → STT → Midstream → Corrections → Text Injection pipeline integration

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use midstreamer_text_transform::transform;
use swictation_audio::AudioCapture;
use swictation_broadcaster::MetricsBroadcaster;
use swictation_metrics::{MetricsCollector, SegmentMetrics};
#[cfg(all(target_os = "macos", feature = "coreml-native"))]
use swictation_stt::CoreMLRecognizer;
use swictation_stt::{OrtRecognizer, SttEngine};
use swictation_vad::{VadConfig, VadDetector, VadResult};

use crate::capitalization::{
    apply_capitalization, normalize_0_6b_punctuation, process_capital_commands,
};
use crate::config::DaemonConfig;
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

    /// Active session's task handles (ADR-035 stop-drain protocol).
    /// Taken by stop_recording() and awaited OUTSIDE the daemon locks.
    vad_task: Option<tokio::task::JoinHandle<()>>,
    stt_task: Option<tokio::task::JoinHandle<()>>,
    backpressure_task: Option<tokio::task::JoinHandle<()>>,
}

/// Task handles detached by `Pipeline::stop_recording`.
///
/// Draining awaits real STT inference and takes the metrics lock, so it MUST
/// happen after the caller releases the daemon state/pipeline locks — awaiting
/// under those locks can deadlock against the metrics updater (lock order:
/// metrics → state). See `Daemon::toggle_inner`.
pub struct DrainHandles {
    vad_task: Option<tokio::task::JoinHandle<()>>,
    stt_task: Option<tokio::task::JoinHandle<()>>,
    backpressure_task: Option<tokio::task::JoinHandle<()>>,
    capture_stopped_at: Instant,
}

impl DrainHandles {
    /// The instant audio capture detached. This — not the moment the drain
    /// finishes — is when the user stopped dictating, so it is what the
    /// session's wall time must be measured against (ADR-035).
    pub fn capture_stopped_at(&self) -> Instant {
        self.capture_stopped_at
    }

    /// Await the VAD task (drains queued audio, flushes the detector, forwards
    /// the flush through the same channel streamed segments used), then the
    /// STT task (drains every remaining segment including that flush), then
    /// aborts the backpressure monitor. Ordering guarantees exactly-once
    /// processing of the final utterance (ADR-035).
    pub async fn drain(mut self) {
        if let Some(task) = self.vad_task.take() {
            if let Err(e) = task.await {
                warn!("VAD task join error during drain: {}", e);
            }
        }
        if let Some(task) = self.stt_task.take() {
            if let Err(e) = task.await {
                warn!("STT task join error during drain: {}", e);
            }
        }
        if let Some(task) = self.backpressure_task.take() {
            task.abort();
        }
    }
}

/// The speech one `process_audio` call produced, oldest first.
///
/// A call spans many windows, so with `max_speech` splitting it can complete
/// more than one segment: it returns the oldest and queues the rest for
/// `drain_pending`. Both have to reach STT. The queue had no consumer at all
/// when it was introduced, so every segment past the first was silently
/// dropped — a speaker who never paused lost all but one slice per call.
fn payloads_from_call(returned: VadResult, queued: Vec<VadResult>) -> Vec<Vec<f32>> {
    speech_only(std::iter::once(returned).chain(queued))
}

/// Everything the detector still owes once audio stops, oldest first.
///
/// The queue drains BEFORE the flush: those segments completed while audio was
/// still arriving, so they precede the tail the flush produces.
fn payloads_at_stop(queued: Vec<VadResult>, flushed: Option<VadResult>) -> Vec<Vec<f32>> {
    speech_only(queued.into_iter().chain(flushed))
}

fn speech_only(results: impl IntoIterator<Item = VadResult>) -> Vec<Vec<f32>> {
    results
        .into_iter()
        .filter_map(|result| match result {
            VadResult::Speech { samples, .. } => Some(samples),
            VadResult::Silence => None,
        })
        .collect()
}

impl Pipeline {
    /// Create new pipeline with GPU acceleration
    /// Returns (Pipeline, transcription_receiver)
    pub async fn new(
        config: DaemonConfig,
        gpu_provider: Option<String>,
    ) -> Result<(Self, mpsc::Receiver<Result<String>>)> {
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
        let audio =
            AudioCapture::new(audio_config).context("Failed to initialize audio capture")?;

        info!(
            "Initializing VAD with {} provider...",
            gpu_provider.as_deref().unwrap_or("CPU")
        );
        let vad_config = VadConfig::with_model(config.vad_model_path.display().to_string())
            .min_silence(config.vad_min_silence)
            .min_speech(config.vad_min_speech)
            .max_speech(config.vad_max_speech)
            .threshold(config.vad_threshold)
            .provider(gpu_provider.clone())
            .num_threads(config.num_threads)
            .debug(); // Enable VAD debug output for troubleshooting

        let vad = VadDetector::new(vad_config).context("Failed to initialize VAD")?;

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
        //   "1.1b-coreml" = Force CoreML 1.1B on macOS (alias: "coreml-native")

        let stt = if config.stt_model_override != "auto" {
            // MANUAL OVERRIDE: User specified exact model
            info!("STT model override active: {}", config.stt_model_override);

            match config.stt_model_override.as_str() {
                "1.1b-gpu" => {
                    info!("  Loading Parakeet-TDT-1.1B-INT8 via ONNX Runtime (forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_1_1b_model_path, true)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to load 1.1B INT8 model from {}. \
                            \nError: {}",
                                config.stt_1_1b_model_path.display(),
                                e
                            )
                        })?;
                    info!("✓ Parakeet-TDT-1.1B-INT8 loaded successfully (GPU, forced)");
                    SttEngine::Parakeet1_1B(ort_recognizer)
                }
                "0.6b-gpu" => {
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (GPU, forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, true)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to load 0.6B GPU model from {}. \
                            \nError: {}",
                                config.stt_0_6b_model_path.display(),
                                e
                            )
                        })?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU, forced)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
                }
                "0.6b-cpu" => {
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (CPU, forced)...");
                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to load 0.6B CPU model from {}. \
                            \nError: {}",
                                config.stt_0_6b_model_path.display(),
                                e
                            )
                        })?;
                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU, forced)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
                }
                #[cfg(all(target_os = "macos", feature = "coreml-native"))]
                "1.1b-coreml" | "coreml-native" => {
                    info!("  Loading Parakeet-TDT-1.1B via native CoreML (forced)...");
                    let recognizer = CoreMLRecognizer::new(&config.stt_coreml_model_path)
                        .map_err(|e| anyhow::anyhow!("Failed to load CoreML model: {}", e))?;
                    info!("✓ Parakeet-TDT-1.1B loaded successfully (CoreML-ANE, forced)");
                    SttEngine::CoreMLNative(recognizer)
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid stt_model_override: '{}'. \
                        Valid options: 'auto', '0.6b-cpu', '0.6b-gpu', '1.1b-gpu'{}",
                        config.stt_model_override,
                        if cfg!(all(target_os = "macos", feature = "coreml-native")) {
                            ", '1.1b-coreml' (or 'coreml-native')"
                        } else {
                            ""
                        }
                    ));
                }
            }
        } else {
            // AUTO MODE: Platform-aware adaptive selection
            info!("STT model selection: auto");

            // macOS: Try native CoreML first (full ANE utilization, much faster than ORT).
            // Build an Option<SttEngine>; if Some, skip the ORT VRAM path entirely.
            #[allow(unused_mut)]
            let mut coreml_engine: Option<SttEngine> = None;

            #[cfg(all(target_os = "macos", feature = "coreml-native"))]
            {
                let coreml_model_path = config.stt_coreml_model_path.clone();
                if coreml_model_path.join("encoder.mlmodelc").exists() {
                    info!("Native CoreML models found — using CoreML with full ANE acceleration");
                    info!("  Loading Parakeet-TDT-1.1B via native CoreML...");
                    match CoreMLRecognizer::new(&coreml_model_path) {
                        Ok(recognizer) => {
                            info!("✓ Parakeet-TDT-1.1B loaded successfully (CoreML-ANE)");
                            coreml_engine = Some(SttEngine::CoreMLNative(recognizer));
                        }
                        Err(e) => {
                            warn!("CoreML native init failed, falling back to ORT: {}", e);
                        }
                    }
                } else {
                    info!(
                        "CoreML models not found at {}, using ORT fallback",
                        coreml_model_path.display()
                    );
                }
            }

            if let Some(engine) = coreml_engine {
                engine
            } else {
                // ORT-based VRAM-adaptive selection
                info!("Detecting GPU memory for adaptive model selection...");
                let vram_mb = get_gpu_memory_mb().map(|(_total, available)| available);

                if let Some(vram) = vram_mb {
                    info!("Detected GPU with {}MB VRAM", vram);

                    if vram >= 6000 {
                        // High VRAM: Use 1.1B INT8 model (best close-mic quality: 1.39% WER LS test-clean)
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
                        // Moderate VRAM: Use 0.6B GPU (1.93% WER LS test-clean)
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
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to load 0.6B CPU model. \
                                    \nTroubleshooting:\
                                    \n  1. Verify model files: ls {}\
                                    \n  2. Check available RAM (need ~1GB free)\
                                    \n  3. Ensure ONNX Runtime CPU EP is available\
                                    \nError: {}",
                                    config.stt_0_6b_model_path.display(),
                                    e
                                )
                            })?;

                        info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                        SttEngine::Parakeet0_6B(ort_recognizer)
                    }
                } else {
                    // No GPU detected: Fall back to CPU
                    warn!("⚠️  No GPU detected (nvidia-smi failed or no NVIDIA GPU)");
                    warn!("  Falling back to CPU mode (slower but functional)");
                    info!("  Loading Parakeet-TDT-0.6B via ONNX Runtime (CPU)...");

                    let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, false)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to load 0.6B CPU model. \
                            \nTroubleshooting:\
                            \n  1. Verify model files: ls {}\
                            \n  2. Check available RAM (need ~1GB free)\
                            \n  3. Ensure ONNX Runtime CPU EP is available\
                            \nError: {}",
                                config.stt_0_6b_model_path.display(),
                                e
                            )
                        })?;

                    info!("✓ Parakeet-TDT-0.6B loaded successfully (CPU)");
                    SttEngine::Parakeet0_6B(ort_recognizer)
                }
            }
        };

        // Log final configuration
        info!(
            "📊 STT Engine: {} ({}, {})",
            stt.model_name(),
            stt.model_size(),
            stt.backend()
        );

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
            std::fs::create_dir_all(parent).context("Failed to create metrics directory")?;
        }

        let metrics = MetricsCollector::new(
            metrics_db_path.to_str().unwrap(),
            40.0,   // typing_baseline_wpm
            false,  // store_transcription_text - keep transcriptions ephemeral
            true,   // warnings_enabled
            1000.0, // high_latency_threshold_ms
            80.0,   // gpu_memory_threshold_percent
        )
        .context("Failed to initialize metrics collector")?;

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
        std::fs::create_dir_all(&corrections_dir).context("Failed to create config directory")?;

        let mut corrections = CorrectionEngine::new(corrections_dir, config.phonetic_threshold);
        if let Err(e) = corrections.start_watching() {
            warn!(
                "Failed to start corrections file watcher: {}. Hot-reload disabled.",
                e
            );
        }
        let corrections = Arc::new(corrections);
        info!("✓ Corrections engine initialized");

        #[allow(clippy::arc_with_non_send_sync)]
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
            vad_task: None,
            stt_task: None,
            backpressure_task: None,
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

        // Start from a clean detector. A segment left queued by the previous
        // session — or audio still buffered mid-utterance in the segmenter —
        // must never surface as the first thing this session transcribes
        // (ADR-035).
        self.vad.lock().reset();

        // Create BOUNDED channel for audio chunks (cpal callback → VAD/STT processing)
        // Capacity: 20 chunks = 10 seconds at 0.5s/chunk
        // This prevents memory exhaustion if processing falls behind
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<f32>>(20);

        // Track dropped chunks for metrics
        let dropped_chunks = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let dropped_chunks_clone = dropped_chunks.clone();

        // Set up audio callback to push chunks via channel
        {
            let mut audio = self.audio.lock();
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
                        eprintln!(
                            "WARNING: Audio chunk dropped (processing too slow). Total dropped: {}",
                            dropped_chunks_clone.load(std::sync::atomic::Ordering::Relaxed)
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Channel closed - recording stopped
                    }
                }
            });

            // Start audio capture (cpal will invoke callback)
            audio.start()?;
        }

        // Log backpressure warning if chunks are being dropped.
        // Runs until aborted by stop_recording — the old `if current == 0 break`
        // exited after 5s in the healthy case and never in the degraded one.
        let dropped_monitor = dropped_chunks.clone();
        self.backpressure_task = Some(tokio::spawn(async move {
            let mut last_count = 0u64;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let current = dropped_monitor.load(std::sync::atomic::Ordering::Relaxed);
                if current > last_count {
                    warn!("⚠️  BACKPRESSURE: Dropped {} audio chunks in last 5s (STT cannot keep up with speaker)",
                              current - last_count);
                    last_count = current;
                }
            }
        }));

        // Clone components for parallel VAD/STT processing
        let vad = self.vad.clone();
        let stt = self.stt.clone();
        let tx = self.tx.clone();
        let tx_for_vad = self.tx.clone();
        let metrics = self.metrics.clone();
        let session_id = self.session_id.clone();
        let broadcaster = self.broadcaster.clone();
        let corrections = self.corrections.clone();

        // Create channel for VAD → STT communication
        // Capacity: 10 speech segments (allows VAD to detect ahead while STT processes)
        let (vad_tx, mut stt_rx) = mpsc::channel::<Vec<f32>>(10);

        // Spawn VAD task (processes audio chunks and detects speech segments)
        let vad_for_task = vad.clone();
        self.vad_task = Some(tokio::spawn(async move {
            let vad = vad_for_task;
            let mut buffer = Vec::with_capacity(16000); // 1 second buffer
            let mut chunk_count = 0;

            while let Some(chunk) = audio_rx.recv().await {
                chunk_count += 1;
                if chunk_count % 10 == 0 {
                    eprintln!(
                        "DEBUG: Received {} chunks, chunk size: {}",
                        chunk_count,
                        chunk.len()
                    );
                }
                buffer.extend_from_slice(&chunk);

                // Process in 0.5 second chunks for VAD
                while buffer.len() >= 8000 {
                    // 0.5 second chunks at 16kHz
                    let vad_chunk: Vec<f32> = buffer.drain(..8000).collect();

                    // Check audio levels
                    let max_amplitude = vad_chunk.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                    let avg_amplitude =
                        vad_chunk.iter().map(|x| x.abs()).sum::<f32>() / vad_chunk.len() as f32;
                    eprintln!("DEBUG: Processing VAD chunk, buffer len: {}, max_amplitude: {:.6}, avg_amplitude: {:.6}",
                              buffer.len(), max_amplitude, avg_amplitude);

                    // Process through VAD (scoped to ensure lock is dropped before any async ops).
                    // The call returns its oldest completed segment and queues
                    // any others it finished, so both are taken here.
                    let vad_result = {
                        let mut vad_lock = vad.lock();
                        match vad_lock.process_audio(&vad_chunk) {
                            Ok(returned) => {
                                Ok(payloads_from_call(returned, vad_lock.drain_pending()))
                            }
                            Err(e) => Err(e),
                        }
                    }; // vad_lock automatically dropped here

                    match vad_result {
                        Ok(segments) if segments.is_empty() => {
                            eprintln!("DEBUG: VAD detected silence");
                            // Skip silence (VAD ensures we only transcribe speech segments)
                        }
                        Ok(segments) => {
                            let mut stt_gone = false;
                            for speech_samples in segments {
                                eprintln!(
                                    "DEBUG: VAD detected speech! {} samples",
                                    speech_samples.len()
                                );

                                // Send speech segment to STT task (non-blocking with backpressure)
                                if let Err(e) = vad_tx.send(speech_samples).await {
                                    // STT task died mid-session (previously this was
                                    // silent: hotkey and metrics kept "working" while
                                    // no text ever appeared again). Surface it through
                                    // the transcription channel so it reaches the logs
                                    // and the daemon consumer (ADR-035).
                                    eprintln!("Failed to send speech segment to STT task: {}", e);
                                    let _ = tx_for_vad
                                        .send(Err(anyhow::anyhow!(
                                        "STT task terminated unexpectedly — transcription halted. \
                                         Toggle recording off and on to restart it."
                                    )))
                                        .await;
                                    stt_gone = true;
                                    break;
                                }
                            }
                            if stt_gone {
                                break; // STT task has terminated
                            }
                        }
                        Err(e) => {
                            eprintln!("VAD error: {}", e);
                        }
                    }
                }
            }

            // Audio channel closed (stop_recording replaced the capture callback,
            // dropping the only sender). Hand over everything the detector
            // still owes — segments queued by the last call, then the flushed
            // tail — through the SAME channel the streamed segments used. The
            // single STT consumer processes each exactly once, so the final
            // ~0.8s of dictation is transcribed instead of discarded, and
            // duplicate injection is structurally impossible (ADR-035;
            // replaces the flush-discard workaround).
            let tail = {
                let mut vad_lock = vad.lock();
                // Drain BEFORE flushing: flush() pushes onto the same queue and
                // returns its front, so flushing first would hand back an older
                // segment and reorder the tail.
                let queued = vad_lock.drain_pending();
                let flushed = vad_lock.flush();
                payloads_at_stop(queued, flushed)
            };
            for speech_samples in tail {
                info!(
                    "Forwarding tail speech to STT: {} samples",
                    speech_samples.len()
                );
                if let Err(e) = vad_tx.send(speech_samples).await {
                    eprintln!("Failed to forward tail speech to STT task: {}", e);
                    break;
                }
            }
            // vad_tx drops here → stt_rx closes after the STT task drains it.
        }));

        // Spawn STT task (processes speech segments from VAD in parallel)
        self.stt_task = Some(tokio::spawn(async move {
            while let Some(speech_samples) = stt_rx.recv().await {
                eprintln!("DEBUG: STT processing {} samples", speech_samples.len());

                // Process through STT (scoped to ensure lock is dropped before any async ops)
                let stt_start = Instant::now();
                let (text, stt_latency, is_0_6b) = {
                    let mut stt_lock = stt.lock();

                    // Use STT engine (OrtRecognizer)
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
                    let is_0_6b = stt_lock.model_size() == "0.6B";
                    (text, stt_latency, is_0_6b)
                }; // stt_lock automatically dropped here

                if !text.is_empty() {
                    // Transform voice commands → symbols (Midstream)
                    // "hello comma world" → "hello, world"
                    let transform_start = Instant::now();

                    // IMPORTANT: 0.6B model has built-in ITN (Inverse Text Normalization) that
                    // INCONSISTENTLY handles punctuation:
                    // - "comma" → "," (word replaced with symbol)
                    // - "period" → "period." (word kept + symbol added at end of sentence)
                    //
                    // Solution: Smart normalization that avoids duplicate punctuation:
                    // - If punctuation WORD exists → remove the symbol (it's redundant)
                    // - If punctuation WORD doesn't exist → convert symbol to word
                    //
                    // This ensures Secretary Mode always sees consistent word-based input.
                    // 1.1B model outputs raw text without ITN - no conversion needed.
                    let text = if is_0_6b {
                        normalize_0_6b_punctuation(&text)
                    } else {
                        text
                    };

                    // Step 1: Process capital commands first ("capital r robert" → "Robert")
                    let with_capitals = process_capital_commands(&text);

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

                    // Privacy: never log dictated content at info — journald persists it (ADR-034).
                    info!(
                        "Transcribed: {} chars in, {} chars out",
                        text.chars().count(),
                        capitalized.chars().count()
                    );
                    debug!("Transcribed content: {} → {}", text, capitalized);

                    // Track segment metrics (ephemeral - no text stored in DB)
                    let word_count = capitalized.split_whitespace().count() as i32;
                    let char_count = capitalized.len() as i32;

                    // Get current session ID (scoped to ensure lock is dropped)
                    let current_session_id = { *session_id.lock() };

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
                            vad_latency_ms: 0.0,       // Not tracked in parallel mode
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
                            if let Err(e) = metrics.lock().add_segment(segment) {
                                eprintln!("Failed to add segment metrics: {}", e);
                            }
                        }

                        // Broadcast transcription to UI clients (scoped to ensure lock is dropped)
                        let broadcaster_clone = { broadcaster.lock().as_ref().map(|b| b.clone()) };

                        if let Some(broadcaster_ref) = broadcaster_clone {
                            let wpm = (word_count as f64 / (duration_s / 60.0)).min(300.0); // Cap at 300 WPM
                            tokio::spawn({
                                let text_clone = capitalized.clone();
                                async move {
                                    broadcaster_ref
                                        .add_transcription(
                                            text_clone,
                                            wpm,
                                            total_latency_ms,
                                            word_count,
                                        )
                                        .await;
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
        }));

        Ok(())
    }

    /// Stop recording: detach the session's tasks WITHOUT awaiting them.
    ///
    /// Returns DrainHandles the caller must `.drain().await` AFTER releasing
    /// the daemon state/pipeline locks — draining runs real STT inference and
    /// takes the metrics lock, and the metrics updater's lock order
    /// (metrics → state) would otherwise deadlock (ADR-035; see also the
    /// lock-ordering history that shaped toggle_inner).
    pub fn stop_recording(&mut self) -> Result<DrainHandles> {
        if !self.is_recording {
            return Ok(DrainHandles {
                vad_task: None,
                stt_task: None,
                backpressure_task: None,
                capture_stopped_at: Instant::now(),
            });
        }

        self.is_recording = false;

        // 1. Stop audio capture (cpal stream stops invoking the callback).
        if let Err(e) = self.audio.lock().stop() {
            warn!("Audio stop error (continuing): {}", e);
        }

        // 2. Close the audio channel: the capture callback owns the ONLY
        //    sender, so replacing it drops that sender. The VAD task then
        //    drains whatever was queued, flushes the detector, forwards any
        //    tail speech through the same channel, and exits — nothing is
        //    discarded and nothing is transcribed twice.
        self.audio.lock().set_chunk_callback(|_| {});

        // 3. The mic is off as of here. Stamp the moment so the drain that
        //    follows — real STT inference on the tail utterance — is not
        //    billed to the user as dictation wall time (ADR-035).
        let capture_stopped_at = Instant::now();

        info!("Recording stopped; session tasks detached for drain");
        Ok(DrainHandles {
            vad_task: self.vad_task.take(),
            stt_task: self.stt_task.take(),
            backpressure_task: self.backpressure_task.take(),
            capture_stopped_at,
        })
    }

    /// Check if currently recording
    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get metrics collector (clone Arc for external use)
    pub fn get_metrics(&self) -> Arc<Mutex<MetricsCollector>> {
        self.metrics.clone()
    }

    /// Get audio sample rate
    #[allow(dead_code)]
    pub fn audio_sample_rate(&self) -> u32 {
        16000
    }

    /// Get audio channels
    #[allow(dead_code)]
    pub fn audio_channels(&self) -> u16 {
        1
    }

    /// Shutdown pipeline
    #[allow(dead_code)]
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.is_recording {
            let drain = self.stop_recording()?;
            drain.drain().await;
        }
        Ok(())
    }

    /// Set the current session ID
    pub fn set_session_id(&self, session_id: i64) {
        *self.session_id.lock() = Some(session_id);
    }

    /// Clear the session ID
    pub fn clear_session_id(&self) {
        *self.session_id.lock() = None;
    }

    /// Set the broadcaster for real-time updates
    pub fn set_broadcaster(&self, broadcaster: Arc<MetricsBroadcaster>) {
        *self.broadcaster.lock() = Some(broadcaster);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A segment whose payload identifies it by content.
    fn segment(marker: f32) -> VadResult {
        VadResult::Speech {
            start_sample: 0,
            samples: vec![marker; 2],
        }
    }

    fn markers(payloads: &[Vec<f32>]) -> Vec<f32> {
        payloads.iter().map(|payload| payload[0]).collect()
    }

    /// Queued-segment fence (ADR-035 review round): the VAD queue exists
    /// because one call can finish several segments, and every one of them is
    /// dictation. Forwarding only the returned value drops the rest.
    #[test]
    fn every_segment_one_vad_call_completed_is_forwarded_in_order() {
        let forwarded = payloads_from_call(segment(1.0), vec![segment(2.0), segment(3.0)]);

        assert_eq!(
            markers(&forwarded),
            vec![1.0, 2.0, 3.0],
            "the returned segment then the queued ones, oldest first"
        );
    }

    #[test]
    fn a_call_that_completed_nothing_forwards_nothing() {
        assert!(payloads_from_call(VadResult::Silence, Vec::new()).is_empty());
    }

    /// Stop ordering fence (ADR-035): segments queued by the last call finished
    /// while audio was still arriving, so they precede the flushed tail. This
    /// is also why the VAD task drains the queue BEFORE calling flush —
    /// `flush()` pushes onto the same queue and returns its front, so flushing
    /// first would hand back an older segment and reorder the tail.
    #[test]
    fn stop_forwards_queued_segments_before_the_flush() {
        let forwarded = payloads_at_stop(vec![segment(1.0), segment(2.0)], Some(segment(9.0)));

        assert_eq!(
            markers(&forwarded),
            vec![1.0, 2.0, 9.0],
            "queued segments then the flushed tail"
        );
    }

    #[test]
    fn stop_with_nothing_buffered_forwards_nothing() {
        assert!(payloads_at_stop(Vec::new(), None).is_empty());
    }

    /// Drain contract (ADR-035): drain() must not return until both session
    /// tasks have fully completed, and must abort the backpressure monitor.
    #[tokio::test]
    async fn drain_awaits_both_tasks_and_aborts_monitor() {
        let vad_done = Arc::new(AtomicBool::new(false));
        let stt_done = Arc::new(AtomicBool::new(false));

        let vad_flag = vad_done.clone();
        let vad_task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            vad_flag.store(true, Ordering::SeqCst);
        });
        let stt_flag = stt_done.clone();
        let stt_task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            stt_flag.store(true, Ordering::SeqCst);
        });
        let monitor = tokio::spawn(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
        let monitor_handle_probe = monitor.abort_handle();

        DrainHandles {
            vad_task: Some(vad_task),
            stt_task: Some(stt_task),
            backpressure_task: Some(monitor),
            capture_stopped_at: Instant::now(),
        }
        .drain()
        .await;

        assert!(
            vad_done.load(Ordering::SeqCst),
            "drain returned before VAD task finished"
        );
        assert!(
            stt_done.load(Ordering::SeqCst),
            "drain returned before STT task finished"
        );
        assert!(
            monitor_handle_probe.is_finished() || {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                monitor_handle_probe.is_finished()
            },
            "backpressure monitor was not aborted"
        );
    }

    /// Duplicate-injection fence (ADR-035): the stop protocol's exactly-once
    /// guarantee rests on this channel shape — the producer drains its input
    /// after the sender is dropped, forwards ONE flush segment through the
    /// SAME channel the streamed segments used, then closes it. The single
    /// consumer therefore sees every streamed segment, then the flush, each
    /// exactly once. The pre-ADR-035 bug was a second, parallel transcription
    /// path for the flush (duplicate injection); the pre-existing "fix" was
    /// discarding the flush (lost final utterance). This encodes why neither
    /// can recur while stop_recording uses this topology.
    #[tokio::test]
    async fn stop_drain_delivers_streamed_segments_then_flush_exactly_once() {
        let (audio_tx, mut audio_rx) = mpsc::channel::<u32>(20);
        let (vad_tx, mut stt_rx) = mpsc::channel::<u32>(10);

        const FLUSH_MARKER: u32 = 999;

        // Mirror of the VAD task: forward until input closes, then flush once.
        let vad_task = tokio::spawn(async move {
            while let Some(segment) = audio_rx.recv().await {
                vad_tx.send(segment).await.expect("consumer alive");
            }
            vad_tx.send(FLUSH_MARKER).await.expect("consumer alive");
            // vad_tx drops here → consumer's channel closes after drain.
        });

        // Mirror of the STT task: single consumer, collects everything.
        let stt_task = tokio::spawn(async move {
            let mut seen = Vec::new();
            while let Some(segment) = stt_rx.recv().await {
                seen.push(segment);
            }
            seen
        });

        audio_tx.send(1).await.unwrap();
        audio_tx.send(2).await.unwrap();
        // stop_recording: drop the only sender (callback replacement).
        drop(audio_tx);

        vad_task.await.unwrap();
        let seen = stt_task.await.unwrap();

        assert_eq!(
            seen,
            vec![1, 2, FLUSH_MARKER],
            "all streamed segments then the flush, in order, exactly once"
        );
    }
}
