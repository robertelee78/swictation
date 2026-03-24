//! Swictation Daemon - Pure Rust Voice-to-Text Pipeline
//!
//! Runs as a background service (systemd), keeping models loaded in memory.
//! Communicates via Unix socket (platform path from swictation_paths) for toggle commands.
//! Sway hotkey → socket toggle → start/stop recording (zero latency)

mod capitalization;
mod config;
mod corrections;
mod display_server;
mod gpu;
mod hotkey;
mod ipc;
mod pipeline;
mod socket_utils;
mod text_injection;
mod version;

// macOS text injection module (conditional compilation)
#[cfg(target_os = "macos")]
mod macos_text_inject;

// macOS microphone permission module (conditional compilation)
#[cfg(target_os = "macos")]
mod macos_audio_permission;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use crate::config::DaemonConfig;

/// Swictation Daemon - Voice-to-Text Pipeline
#[derive(Parser, Debug)]
#[command(name = "swictation-daemon")]
#[command(about = "Voice-to-text dictation daemon with adaptive model selection", long_about = None)]
#[command(version)]
struct CliArgs {
    /// Override STT model selection (bypasses auto-detection)
    #[arg(long, value_name = "MODEL")]
    #[arg(value_parser = ["0.6b-cpu", "0.6b-gpu", "1.1b-gpu", "1.1b-coreml"])]
    test_model: Option<String>,

    /// Dry-run: show model selection without loading models
    #[arg(long)]
    dry_run: bool,

    /// Show detailed version information
    #[arg(long)]
    version_info: bool,
}
use crate::gpu::detect_gpu_provider;
use crate::hotkey::{HotkeyEvent, HotkeyManager};
use crate::ipc::{handle_connection as handle_ipc_connection, IpcServer};
use crate::pipeline::Pipeline;
use swictation_broadcaster::MetricsBroadcaster;
use swictation_context_learning::{
    load_or_train_model, ContextModel, LearningConfig, RetrainingConfig,
};
use swictation_metrics::{MemoryMonitor, MemoryPressure};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DaemonState {
    Idle,
    Recording,
}

struct Daemon {
    pipeline: Arc<RwLock<Pipeline>>,
    state: Arc<RwLock<DaemonState>>,
    broadcaster: Arc<MetricsBroadcaster>,
    session_id: Arc<RwLock<Option<i64>>>,
    /// Prevents concurrent toggle() calls when spawned off the event loop
    toggle_lock: tokio::sync::Mutex<()>,
    /// Atomic flag for fast non-blocking "toggle in progress" check
    toggling: AtomicBool,
}

impl Daemon {
    async fn new(
        config: DaemonConfig,
        gpu_provider: Option<String>,
    ) -> Result<(Self, mpsc::Receiver<Result<String>>)> {
        let (pipeline, transcription_rx) = Pipeline::new(config, gpu_provider).await?;

        // Initialize metrics broadcaster with secure socket path
        let metrics_socket =
            socket_utils::get_metrics_socket_path().context("Failed to get metrics socket path")?;
        let broadcaster = Arc::new(
            MetricsBroadcaster::new(&metrics_socket)
                .await
                .context("Failed to create metrics broadcaster")?,
        );

        // Set broadcaster in pipeline for real-time updates
        pipeline.set_broadcaster(broadcaster.clone());

        #[allow(clippy::arc_with_non_send_sync)]
        let daemon = Self {
            pipeline: Arc::new(RwLock::new(pipeline)),
            state: Arc::new(RwLock::new(DaemonState::Idle)),
            broadcaster: broadcaster.clone(),
            session_id: Arc::new(RwLock::new(None)),
            toggle_lock: tokio::sync::Mutex::new(()),
            toggling: AtomicBool::new(false),
        };

        // Start broadcaster Unix socket server
        daemon
            .broadcaster
            .start()
            .await
            .context("Failed to start metrics broadcaster")?;

        Ok((daemon, transcription_rx))
    }

    /// Toggle recording state with proper lock ordering to prevent deadlocks.
    ///
    /// CRITICAL: Lock order must be: state -> pipeline -> session_id
    /// And we must RELEASE locks before any long-running operations (STT inference).
    /// The metrics updater acquires locks in: metrics -> state (read)
    /// To prevent deadlock, we minimize lock scope and release before await points.
    ///
    /// Serialized via toggle_lock to prevent concurrent toggles when spawned
    /// off the event loop. The AtomicBool provides a fast non-blocking check
    /// for callers that want to skip duplicate toggles.
    async fn toggle(&self) -> Result<String> {
        // Fast non-blocking check: if a toggle is already in progress, skip
        if self.toggling.load(Ordering::SeqCst) {
            info!("Toggle already in progress, skipping");
            return Ok("Toggle already in progress".to_string());
        }

        // Acquire serialization lock (blocks if another toggle is running)
        let _guard = self.toggle_lock.lock().await;
        self.toggling.store(true, Ordering::SeqCst);
        let result = self.toggle_inner().await;
        self.toggling.store(false, Ordering::SeqCst);
        result
    }

    /// Inner toggle implementation — must only be called with toggle_lock held.
    async fn toggle_inner(&self) -> Result<String> {
        // Phase 1: Check current state (minimal lock scope)
        let current_state = {
            let state = self.state.read().await;
            *state
        };

        match current_state {
            DaemonState::Idle => {
                info!("▶️ Starting recording");

                // Phase 2: Start session and get metrics (short lock scope)
                let sid = {
                    let pipeline = self.pipeline.read().await;
                    let metrics = pipeline.get_metrics();
                    let sid = metrics.lock().unwrap().start_session()?;
                    sid
                };

                // Phase 3: Update state and start recording
                {
                    let mut state = self.state.write().await;
                    let mut pipeline = self.pipeline.write().await;
                    let mut session_id = self.session_id.write().await;

                    *session_id = Some(sid);
                    pipeline.set_session_id(sid);
                    pipeline.start_recording().await?;
                    *state = DaemonState::Recording;
                }
                // Locks released here before broadcast

                // Phase 4: Broadcast (no locks held - prevents deadlock with metrics updater)
                // CRITICAL: Spawn broadcasts to prevent blocking IPC responses
                // Broadcasting to UI clients can block if clients are slow/disconnected
                // By spawning, we return immediately and let broadcasts happen async
                {
                    let broadcaster = Arc::clone(&self.broadcaster);
                    tokio::spawn(async move {
                        broadcaster.start_session(sid).await;
                        broadcaster
                            .broadcast_state_change(swictation_metrics::DaemonState::Recording)
                            .await;
                    });
                }

                Ok(format!("Recording started (Session #{})", sid))
            }
            DaemonState::Recording => {
                info!("⏸️ Stopping recording");

                // Phase 2: Stop recording (this does STT inference - can take 50-500ms)
                // We MUST release state lock before this to prevent deadlock
                {
                    let mut pipeline = self.pipeline.write().await;
                    pipeline.stop_recording().await?;
                    pipeline.clear_session_id();
                }
                // Pipeline lock released before we touch state

                // Phase 3: Update state and end session
                let (session_metrics, sid) = {
                    let mut state = self.state.write().await;
                    let pipeline = self.pipeline.read().await;
                    let mut session_id = self.session_id.write().await;

                    *state = DaemonState::Idle;

                    let metrics = pipeline.get_metrics();
                    let session_metrics = metrics.lock().unwrap().end_session()?;
                    let sid = *session_id;
                    *session_id = None;

                    (session_metrics, sid)
                };
                // All locks released before broadcast

                // Phase 4: Broadcast (no locks held)
                // CRITICAL: Spawn broadcasts to prevent blocking IPC responses
                // Same rationale as start_recording - avoid blocking on slow clients
                {
                    let broadcaster = Arc::clone(&self.broadcaster);
                    tokio::spawn(async move {
                        if let Some(sid) = sid {
                            broadcaster.end_session(sid).await;
                        }
                        broadcaster
                            .broadcast_state_change(swictation_metrics::DaemonState::Idle)
                            .await;
                    });
                }

                Ok(format!(
                    "Recording stopped ({} words, {:.1} WPM)",
                    session_metrics.words_dictated, session_metrics.words_per_minute
                ))
            }
        }
    }

    async fn status(&self) -> String {
        let state = self.state.read().await;
        match *state {
            DaemonState::Idle => "idle".to_string(),
            DaemonState::Recording => "recording".to_string(),
        }
    }
}

/// Load or train context-aware learning model
async fn load_context_model(_config: &DaemonConfig) -> Option<ContextModel> {
    let data_dir = match dirs::data_local_dir() {
        Some(dir) => dir.join("swictation"),
        None => {
            warn!("Failed to get data directory for context model");
            return None;
        }
    };

    let model_path = data_dir.join("context-model.json");
    let db_path = data_dir.join("metrics.db");

    let learning_config = LearningConfig::default();
    let retrain_config = RetrainingConfig::default();

    match load_or_train_model(&model_path, &db_path, &learning_config, &retrain_config) {
        Ok(model) => model,
        Err(e) => {
            warn!("Failed to load context model: {}", e);
            None
        }
    }
}

/// On macOS, Carbon's RegisterEventHotKey (used by global-hotkey crate) delivers
/// events through GetApplicationEventTarget(), which requires the application
/// event loop to be running on the main thread. Tokio's executor does NOT pump
/// this event loop, so hotkey callbacks never fire.
///
/// CRITICAL: The global-hotkey crate REQUIRES that GlobalHotKeyManager::new() and
/// register() are called on the main thread (same thread as RunApplicationEventLoop).
/// See: global-hotkey crate docs, line 17-18: "On macOS, an event loop must be
/// running on the main thread so you also need to create the global hotkey manager
/// on the main thread."
///
/// Architecture:
/// 1. Main thread: permissions -> load config -> create HotkeyManager -> RunApplicationEventLoop()
/// 2. Background thread: Tokio runtime -> daemon_main(cli, hotkey_manager)
///
/// On Linux, this is unnecessary (X11 XGrabKey doesn't need a run loop), so we
/// keep #[tokio::main] behavior via block_on.
fn main() -> Result<()> {
    // Parse CLI arguments (sync — fine on main thread)
    let cli = CliArgs::parse();

    if cli.version_info {
        println!("{}", version::version_long());
        return Ok(());
    }

    // Initialize logging (sync)
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("Starting Swictation Daemon v{}", env!("CARGO_PKG_VERSION"));

    // macOS: Request permissions on main thread (system dialogs need it)
    #[cfg(target_os = "macos")]
    {
        use crate::macos_audio_permission::request_microphone_permission;
        use crate::macos_text_inject::MacOSTextInjector;

        info!("Checking macOS permissions...");

        if !request_microphone_permission() {
            warn!("Microphone permission not yet granted");
            warn!("   Please enable in: System Settings -> Privacy & Security -> Microphone");
        } else {
            info!("Microphone permission granted");
        }

        if !MacOSTextInjector::request_accessibility_permissions() {
            warn!("Accessibility permission not yet granted");
            warn!("   Please enable in: System Settings -> Privacy & Security -> Accessibility");
        } else {
            info!("Accessibility permission granted");
        }
    }

    // Load config early — needed on main thread for hotkey setup (macOS)
    let config = DaemonConfig::load().context("Failed to load configuration")?;

    // CRITICAL (macOS): Create hotkey manager on the MAIN THREAD.
    // Carbon's RegisterEventHotKey + InstallEventHandler require the main thread —
    // the same thread that will run RunApplicationEventLoop().
    // On Linux/X11, thread doesn't matter, but we do it here for consistency.
    let hotkey_manager = HotkeyManager::new(config.hotkeys.clone())
        .context("Failed to initialize hotkey manager")?;

    if hotkey_manager.is_some() {
        info!("Hotkeys registered on main thread");
    } else {
        info!("Hotkeys not available - using IPC/CLI control only");
    }

    // Build Tokio runtime manually (instead of #[tokio::main])
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?;

    // On macOS: run async daemon on a background thread via block_on,
    // then run Carbon application event loop on the main thread.
    // On Linux: run async daemon directly on main thread via block_on.
    #[cfg(target_os = "macos")]
    {
        // Move hotkey_manager and config to the background thread.
        // The HotkeyManager is Send (global-hotkey marks it as such) and event
        // delivery works cross-thread via crossbeam channels. Only the initial
        // registration needed the main thread.
        std::thread::spawn(move || {
            if let Err(e) = runtime.block_on(daemon_main(cli, config, hotkey_manager)) {
                error!("Daemon error: {}", e);
                std::process::exit(1);
            }
        });

        // Main thread: run Carbon application event loop so global-hotkey events fire.
        // Carbon's RegisterEventHotKey delivers via GetApplicationEventTarget(),
        // which requires this event loop to be actively pumping on the main thread.
        // RunApplicationEventLoop() blocks until QuitApplicationEventLoop() is called.
        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            fn RunApplicationEventLoop();
        }
        info!("Running macOS application event loop on main thread (hotkeys active)");
        unsafe {
            RunApplicationEventLoop();
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux/other: run async daemon directly — no special event loop needed
        // (X11 XGrabKey doesn't require a run loop on the main thread)
        runtime.block_on(daemon_main(cli, config, hotkey_manager))?;
    }

    Ok(())
}

/// Async daemon main — all the pipeline, IPC, hotkey, and event loop logic.
/// Separated from main() so it can run on Tokio's runtime (background threads on macOS).
///
/// `hotkey_manager` is created on the main thread (required for macOS Carbon API)
/// and moved here. Config is also loaded early for the same reason.
async fn daemon_main(
    cli: CliArgs,
    mut config: DaemonConfig,
    mut hotkey_manager: Option<HotkeyManager>,
) -> Result<()> {
    info!("Configuration loaded from {}", config.config_path.display());

    // Apply CLI overrides
    if let Some(ref model) = cli.test_model {
        info!("🧪 CLI override: forcing model '{}'", model);
        config.stt_model_override = model.clone();
    }

    // Detect GPU provider
    let gpu_provider = detect_gpu_provider();
    match &gpu_provider {
        Some(provider) => info!("🎮 GPU detected: {}", provider),
        None => warn!("⚠️ No GPU detected, using CPU (slower)"),
    }

    // DRY-RUN MODE: Show model selection and exit
    if cli.dry_run {
        info!("🧪 DRY-RUN MODE: Showing model selection without loading");

        let vram_mb = crate::gpu::get_gpu_memory_mb().map(|(total, _free)| total);

        if config.stt_model_override != "auto" {
            info!("  Override active: {}", config.stt_model_override);
            match config.stt_model_override.as_str() {
                "1.1b-gpu" => info!("  Would load: Parakeet-TDT-1.1B-INT8 (GPU, forced)"),
                "0.6b-gpu" => info!("  Would load: Parakeet-TDT-0.6B (GPU, forced)"),
                "0.6b-cpu" => info!("  Would load: Parakeet-TDT-0.6B (CPU, forced)"),
                "1.1b-coreml" => {
                    info!("  Would load: Parakeet-TDT-1.1B (CoreML, forced)");
                    info!("    Path: {}", config.stt_coreml_model_path.display());
                    info!("    Reason: Native Apple Neural Engine acceleration");
                }
                _ => error!("  Invalid override value!"),
            }
        } else {
            info!("  Mode: auto (VRAM-based)");
            if let Some(vram) = vram_mb {
                info!("  Detected: {}MB VRAM", vram);
                if vram >= 6000 {
                    info!("  Would load: Parakeet-TDT-1.1B-INT8 (GPU)");
                    info!("    Path: {}", config.stt_1_1b_model_path.display());
                    info!("    Reason: ≥6GB VRAM available");
                } else if vram >= 3500 {
                    info!("  Would load: Parakeet-TDT-0.6B (GPU)");
                    info!("    Path: {}", config.stt_0_6b_model_path.display());
                    info!("    Reason: ≥3.5GB VRAM available");
                } else {
                    info!("  Would load: Parakeet-TDT-0.6B (CPU)");
                    info!("    Path: {}", config.stt_0_6b_model_path.display());
                    info!("    Reason: <3.5GB VRAM ({}MB), using CPU fallback", vram);
                }
            } else {
                info!("  Detected: No GPU");
                info!("  Would load: Parakeet-TDT-0.6B (CPU)");
                info!("    Path: {}", config.stt_0_6b_model_path.display());
                info!("    Reason: No NVIDIA GPU detected");
            }
        }

        info!("✅ Dry-run complete (no models loaded)");
        return Ok(());
    }

    // Initialize daemon with models loaded
    info!("🔧 Initializing pipeline (this may take a moment)...");
    let (daemon, mut transcription_rx) =
        match Daemon::new(config.clone(), gpu_provider.clone()).await {
            Ok(result) => result,
            Err(e) => {
                let err_msg = format!("{:#}", e);

                // Check if error is about missing model files
                if err_msg.contains("No such file or directory")
                    || err_msg.contains("model") && err_msg.contains("not found")
                    || err_msg.contains("Failed to load")
                {
                    error!("❌ Failed to load AI model");
                    error!("");
                    error!("The required AI model files were not found.");
                    error!("Please download the recommended model for your system:");
                    error!("");
                    error!("  swictation download-model 0.6b-gpu    # For 4GB+ VRAM GPUs");
                    error!("  swictation download-model 1.1b-gpu    # For 6GB+ VRAM GPUs");
                    error!("  swictation download-model 0.6b        # For CPU-only systems");
                    error!("");
                    error!("Or download all models:");
                    error!("  swictation download-models");
                    error!("");

                    return Err(
                        e.context("AI models not found - run 'swictation download-model' first")
                    );
                }

                // For other errors, just pass through
                return Err(e.context("Failed to initialize daemon"));
            }
        };

    info!("✓ Pipeline initialized successfully");
    info!("  - Audio: 16000 Hz, 1 channel");
    info!("  - VAD: Silero VAD v6 (ort/ONNX)");
    // STT info is logged by pipeline.rs during initialization
    info!("📊 Memory usage: {} MB", get_memory_usage_mb());
    info!(
        "📡 Metrics broadcaster ready on {}",
        socket_utils::get_metrics_socket_path()
            .unwrap_or_else(|_| PathBuf::from("unknown"))
            .display()
    );

    // Initialize context-aware learning model
    let context_model = load_context_model(&config).await;
    if let Some(ref model) = context_model {
        info!(
            "🧠 Context model loaded: {} topics, {} homonym rules",
            model.topics.len(),
            model.homonym_rules.len()
        );
    } else {
        info!("⚠️  Context model not available (insufficient training data)");
    }

    // Hotkey manager was created on the main thread (required for macOS Carbon API)
    // and passed in as a parameter. It's already registered and ready.
    if hotkey_manager.is_some() {
        info!("Hotkeys active");
    } else {
        info!("Hotkeys not available - using IPC/CLI control only");
    }

    // Start IPC server for CLI/scripts (optional) with secure socket path
    let socket_path =
        socket_utils::get_ipc_socket_path().context("Failed to get IPC socket path")?;
    let socket_path_str = socket_path.to_str().context("Invalid socket path")?;
    info!("🔌 Starting IPC server on {}", socket_path_str);

    #[allow(clippy::arc_with_non_send_sync)]
    let daemon_clone = Arc::new(daemon);
    let mut ipc_server = IpcServer::new(socket_path_str, daemon_clone.clone())
        .context("Failed to start IPC server")?;

    // Spawn background metrics updater (CPU/GPU monitoring every 1 second)
    //
    // CRITICAL: Lock ordering to prevent deadlock with toggle():
    // - toggle() acquires: state -> pipeline -> session_id -> metrics
    // - This task MUST acquire state BEFORE metrics, or use try_lock
    //
    // Previous bug: This task held metrics.lock() while trying to acquire state.read(),
    // while toggle() held state.write() while trying to acquire metrics.lock() -> DEADLOCK
    let _metrics_handle = {
        let metrics = daemon_clone.pipeline.read().await.get_metrics();
        let broadcaster = daemon_clone.broadcaster.clone();
        let daemon_state = daemon_clone.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;

                // FIXED: Acquire state lock FIRST (before metrics)
                // This matches the lock order in toggle() and prevents deadlock
                let current_state = {
                    let state = daemon_state.read().await;
                    match *state {
                        DaemonState::Idle => swictation_metrics::DaemonState::Idle,
                        DaemonState::Recording => swictation_metrics::DaemonState::Recording,
                    }
                };
                // State lock released here

                // NOW safe to acquire metrics lock (no other locks held)
                let realtime = {
                    let metrics_guard = metrics.lock().unwrap();
                    metrics_guard.update_system_metrics();
                    metrics_guard.update_recording_duration();
                    let mut realtime = metrics_guard.get_realtime_metrics();
                    realtime.current_state = current_state;
                    realtime
                };
                // Metrics lock released here

                // Broadcast with no locks held
                broadcaster.update_metrics(&realtime).await;
            }
        })
    };

    // Spawn memory pressure monitor (RAM + VRAM every 5 seconds)
    let _memory_handle = {
        let _broadcaster = daemon_clone.broadcaster.clone();
        tokio::spawn(async move {
            let mut memory_monitor = match MemoryMonitor::new() {
                Ok(m) => {
                    info!("✓ Memory monitoring initialized: {}", m.gpu_device_name());
                    m
                }
                Err(e) => {
                    error!("Failed to initialize memory monitor: {}", e);
                    return;
                }
            };

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Check memory pressure
                let (ram_pressure, vram_pressure) = memory_monitor.check_pressure();

                // Handle RAM pressure
                match ram_pressure {
                    MemoryPressure::Warning => {
                        let stats = memory_monitor.get_stats();
                        warn!(
                            "⚠️  RAM usage high: {:.1}% ({} MB used / {} MB total)",
                            stats.ram.percent_used, stats.ram.used_mb, stats.ram.total_mb
                        );
                    }
                    MemoryPressure::Critical => {
                        let stats = memory_monitor.get_stats();
                        error!("🚨 RAM critical: {:.1}% ({} MB used / {} MB total) - Process using {} MB",
                              stats.ram.percent_used, stats.ram.used_mb, stats.ram.total_mb, stats.ram.process_mb);
                    }
                    MemoryPressure::Normal => {}
                }

                // Handle VRAM pressure (MANDATORY GPU monitoring)
                match vram_pressure {
                    MemoryPressure::Warning => {
                        let stats = memory_monitor.get_stats();
                        if let Some(vram) = stats.vram {
                            warn!(
                                "⚠️  VRAM usage high: {:.1}% ({} MB used / {} MB total) on {}",
                                vram.percent_used, vram.used_mb, vram.total_mb, vram.device_name
                            );
                        }
                    }
                    MemoryPressure::Critical => {
                        let stats = memory_monitor.get_stats();
                        if let Some(vram) = stats.vram {
                            error!(
                                "🚨 VRAM critical: {:.1}% ({} MB used / {} MB total) on {}",
                                vram.percent_used, vram.used_mb, vram.total_mb, vram.device_name
                            );
                            // Note: Could pause recording here if needed
                        }
                    }
                    MemoryPressure::Normal => {}
                }
            }
        })
    };

    info!("🚀 Swictation daemon ready!");
    if hotkey_manager.is_some() {
        info!("   Press {} to start/stop recording", config.hotkeys.toggle);
    }
    info!("   Or use 'swictation-cli toggle' for CLI control");

    // Handle transcription results and inject text
    //
    // On macOS, CGEventSource is not Send/Sync, so we must use a dedicated OS thread
    // for text injection and communicate via a channel.
    let (inject_tx, inject_rx) = std::sync::mpsc::channel::<String>();

    // Spawn dedicated thread for text injection (required for macOS CGEventSource)
    std::thread::spawn(move || {
        use crate::text_injection::TextInjector;

        // Initialize text injector with display server detection
        let text_injector = match TextInjector::new() {
            Ok(injector) => {
                info!(
                    "Text injector initialized for: {:?}",
                    injector.display_server_info().server_type
                );
                injector
            }
            Err(e) => {
                error!("Failed to initialize text injector: {}", e);
                error!("Text injection will be disabled. Install required tools:");
                #[cfg(target_os = "linux")]
                {
                    error!("  For X11: sudo apt install xdotool");
                    error!("  For Wayland: sudo apt install wtype");
                }
                #[cfg(target_os = "macos")]
                {
                    error!("  macOS: Grant Accessibility permissions in System Settings");
                }
                return;
            }
        };

        // Receive text to inject from channel
        while let Ok(text) = inject_rx.recv() {
            info!("Injecting text: {}", text);
            if let Err(e) = text_injector.inject_text(&text) {
                error!("Failed to inject text: {}", e);
            }
        }
    });

    // Bridge async transcription results to the sync text injection thread
    tokio::spawn(async move {
        while let Some(result) = transcription_rx.recv().await {
            match result {
                Ok(text) => {
                    if inject_tx.send(text).is_err() {
                        error!("Text injection thread has exited");
                        break;
                    }
                }
                Err(e) => {
                    error!("Transcription error: {}", e);
                }
            }
        }
    });

    // Toggle debounce: reject rapid re-toggles within this window.
    // This prevents the "double-tap race" where a second keypress during a slow
    // toggle() undoes the first toggle (e.g., stop immediately followed by start).
    const TOGGLE_DEBOUNCE_MS: u64 = 500;
    let last_toggle =
        std::sync::Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(10)); // allow first toggle immediately

    // Main event loop
    loop {
        tokio::select! {
            // Hotkey events (primary UX) - only if hotkeys are available
            Some(event) = async {
                if let Some(ref mut manager) = hotkey_manager {
                    manager.next_event().await
                } else {
                    // No hotkeys - wait forever (IPC is the only control)
                    std::future::pending().await
                }
            } => {
                match event {
                    HotkeyEvent::Toggle => {
                        // Debounce: reject if too soon after last toggle.
                        // This prevents the "double-tap race" where a second keypress
                        // during a slow toggle() undoes the first (stop->start).
                        let elapsed = last_toggle.lock().unwrap().elapsed();
                        if elapsed < std::time::Duration::from_millis(TOGGLE_DEBOUNCE_MS) {
                            info!("Toggle debounced ({}ms since last)", elapsed.as_millis());
                            // Drain any additional queued toggle events
                            if let Some(ref mut manager) = hotkey_manager {
                                let drained = manager.try_drain();
                                if drained > 0 {
                                    info!("Drained {} queued hotkey events", drained);
                                }
                            }
                            continue;
                        }
                        *last_toggle.lock().unwrap() = std::time::Instant::now();

                        // Execute toggle (serialized via toggle_lock inside Daemon)
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("Toggle error: {}", e);
                        }

                        // After toggle completes, update debounce timestamp and
                        // drain any events that queued during the slow toggle
                        *last_toggle.lock().unwrap() = std::time::Instant::now();
                        if let Some(ref mut manager) = hotkey_manager {
                            let drained = manager.try_drain();
                            if drained > 0 {
                                info!("Drained {} queued hotkey events after toggle", drained);
                            }
                        }
                    }
                    HotkeyEvent::PushToTalkPressed => {
                        info!("Push-to-talk pressed");
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("PTT start error: {}", e);
                        }
                    }
                    HotkeyEvent::PushToTalkReleased => {
                        info!("Push-to-talk released");
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("PTT stop error: {}", e);
                        }
                    }
                }
            }

            // IPC server (secondary, for CLI/scripts)
            // Note: awaited inline because Daemon is not Send (cpal raw pointers).
            // The toggle_lock inside Daemon still serializes concurrent toggles.
            Ok((stream, daemon)) = ipc_server.accept() => {
                if let Err(e) = handle_ipc_connection(stream, daemon).await {
                    error!("IPC connection error: {}", e);
                }
            }

            // Shutdown signal
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    // Cleanup
    info!("🧹 Shutting down...");

    // Stop broadcaster
    if let Err(e) = daemon_clone.broadcaster.stop().await {
        warn!("Failed to stop broadcaster cleanly: {}", e);
    }

    info!("👋 Swictation daemon stopped");

    Ok(())
}

/// Get current process memory usage in MB
fn get_memory_usage_mb() -> u64 {
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);

    if let Some(process) = sys.process(pid) {
        process.memory() / 1_048_576 // bytes to MB
    } else {
        0
    }
}
