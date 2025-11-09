//! Swictation Daemon - Pure Rust Voice-to-Text Pipeline
//!
//! Runs as a background service (systemd), keeping models loaded in memory.
//! Communicates via Unix socket (/tmp/swictation.sock) for toggle commands.
//! Sway hotkey → socket toggle → start/stop recording (zero latency)

mod pipeline;
mod gpu;
mod config;
mod ipc;
mod hotkey;
mod text_injection;

use anyhow::{Context, Result};
use tracing::{info, error, warn};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

use crate::config::DaemonConfig;
use crate::pipeline::Pipeline;
use crate::gpu::detect_gpu_provider;
use crate::ipc::{IpcServer, handle_connection as handle_ipc_connection};
use crate::hotkey::{HotkeyManager, HotkeyEvent};
use swictation_broadcaster::MetricsBroadcaster;
use swictation_metrics::{MemoryMonitor, MemoryPressure};

#[derive(Debug, Clone, PartialEq)]
enum DaemonState {
    Idle,
    Recording,
}

struct Daemon {
    pipeline: Arc<RwLock<Pipeline>>,
    state: Arc<RwLock<DaemonState>>,
    broadcaster: Arc<MetricsBroadcaster>,
    session_id: Arc<RwLock<Option<i64>>>,
}

impl Daemon {
    async fn new(config: DaemonConfig, gpu_provider: Option<String>) -> Result<(Self, mpsc::UnboundedReceiver<Result<String>>)> {
        let (pipeline, transcription_rx) = Pipeline::new(config, gpu_provider).await?;

        // Initialize metrics broadcaster
        let broadcaster = Arc::new(
            MetricsBroadcaster::new("/tmp/swictation_metrics.sock")
                .await
                .context("Failed to create metrics broadcaster")?
        );

        // Set broadcaster in pipeline for real-time updates
        pipeline.set_broadcaster(broadcaster.clone());

        let daemon = Self {
            pipeline: Arc::new(RwLock::new(pipeline)),
            state: Arc::new(RwLock::new(DaemonState::Idle)),
            broadcaster: broadcaster.clone(),
            session_id: Arc::new(RwLock::new(None)),
        };

        // Start broadcaster Unix socket server
        daemon.broadcaster.start().await
            .context("Failed to start metrics broadcaster")?;

        Ok((daemon, transcription_rx))
    }

    async fn toggle(&self) -> Result<String> {
        let mut state = self.state.write().await;
        let mut pipeline = self.pipeline.write().await;
        let mut session_id = self.session_id.write().await;

        match *state {
            DaemonState::Idle => {
                info!("▶️ Starting recording");

                // Start metrics session
                let metrics = pipeline.get_metrics();
                let sid = metrics.lock().unwrap().start_session()?;
                *session_id = Some(sid);

                // Set session ID in pipeline so segments can be associated with it
                pipeline.set_session_id(sid);

                // Start recording pipeline
                pipeline.start_recording().await?;
                *state = DaemonState::Recording;

                // Broadcast session start
                self.broadcaster.start_session(sid).await;

                // Broadcast state change to Recording
                self.broadcaster.broadcast_state_change(swictation_metrics::DaemonState::Recording).await;

                Ok(format!("Recording started (Session #{})", sid))
            }
            DaemonState::Recording => {
                info!("⏸️ Stopping recording");
                pipeline.stop_recording().await?;
                *state = DaemonState::Idle;

                // Clear session ID in pipeline
                pipeline.clear_session_id();

                // End metrics session
                let metrics = pipeline.get_metrics();
                let session_metrics = metrics.lock().unwrap().end_session()?;

                // Broadcast session end
                if let Some(sid) = *session_id {
                    self.broadcaster.end_session(sid).await;
                }
                *session_id = None;

                // Broadcast state change to Idle
                self.broadcaster.broadcast_state_change(swictation_metrics::DaemonState::Idle).await;

                Ok(format!("Recording stopped ({} words, {:.1} WPM)",
                          session_metrics.words_dictated,
                          session_metrics.words_per_minute))
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("🎙️ Starting Swictation Daemon v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = DaemonConfig::load()
        .context("Failed to load configuration")?;

    info!("📋 Configuration loaded from {}", config.config_path.display());

    // Detect GPU provider
    let gpu_provider = detect_gpu_provider();
    match &gpu_provider {
        Some(provider) => info!("🎮 GPU detected: {}", provider),
        None => warn!("⚠️ No GPU detected, using CPU (slower)"),
    }

    // Initialize daemon with models loaded
    info!("🔧 Initializing pipeline (this may take a moment)...");
    let (daemon, mut transcription_rx) = Daemon::new(config.clone(), gpu_provider.clone())
        .await
        .context("Failed to initialize daemon")?;

    info!("✓ Pipeline initialized successfully");
    info!("  - Audio: 16000 Hz, 1 channel");
    info!("  - VAD: Silero VAD v6 (ort/ONNX)");
    info!("  - STT: Parakeet-TDT-1.1B (parakeet-rs)");
    if let Some(provider) = &gpu_provider {
        info!("  - GPU: {} acceleration enabled", provider);
    }
    info!("📊 Memory usage: {} MB", get_memory_usage_mb());
    info!("📡 Metrics broadcaster ready on /tmp/swictation_metrics.sock");

    // Initialize hotkey manager (optional - some compositors don't support it)
    let mut hotkey_manager = HotkeyManager::new(config.hotkeys.clone())
        .context("Failed to initialize hotkey manager")?;

    if let Some(ref manager) = hotkey_manager {
        info!("✓ Hotkeys initialized successfully");
    } else {
        info!("⚠️  Hotkeys not available - using IPC/CLI control only");
    }

    // Start IPC server for CLI/scripts (optional)
    let socket_path = config.socket_path.clone();
    info!("🔌 Starting IPC server on {}", socket_path);

    let daemon_clone = Arc::new(daemon);
    let mut ipc_server = IpcServer::new(&socket_path, daemon_clone.clone())
        .context("Failed to start IPC server")?;

    // Spawn background metrics updater (CPU/GPU monitoring every 1 second)
    let metrics_handle = {
        let metrics = daemon_clone.pipeline.read().await.get_metrics();
        let broadcaster = daemon_clone.broadcaster.clone();
        let daemon_state = daemon_clone.state.clone(); // Clone state for metrics thread
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;

                // Update internal metrics
                metrics.lock().unwrap().update_system_metrics();

                // Get realtime metrics and update daemon state
                let mut realtime = metrics.lock().unwrap().get_realtime_metrics();

                // Update the current state from daemon
                let state = daemon_state.read().await;
                realtime.current_state = match *state {
                    DaemonState::Idle => swictation_metrics::DaemonState::Idle,
                    DaemonState::Recording => swictation_metrics::DaemonState::Recording,
                };
                drop(state); // Release lock quickly

                // Broadcast to connected clients
                broadcaster.update_metrics(&realtime).await;
            }
        })
    };

    // Spawn memory pressure monitor (RAM + VRAM every 5 seconds)
    let memory_handle = {
        let broadcaster = daemon_clone.broadcaster.clone();
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
                        warn!("⚠️  RAM usage high: {:.1}% ({} MB used / {} MB total)",
                             stats.ram.percent_used, stats.ram.used_mb, stats.ram.total_mb);
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
                            warn!("⚠️  VRAM usage high: {:.1}% ({} MB used / {} MB total) on {}",
                                 vram.percent_used, vram.used_mb, vram.total_mb, vram.device_name);
                        }
                    }
                    MemoryPressure::Critical => {
                        let stats = memory_monitor.get_stats();
                        if let Some(vram) = stats.vram {
                            error!("🚨 VRAM critical: {:.1}% ({} MB used / {} MB total) on {}",
                                  vram.percent_used, vram.used_mb, vram.total_mb, vram.device_name);
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
    tokio::spawn(async move {
        use crate::text_injection::TextInjector;

        // Initialize text injector with display server detection
        let text_injector = match TextInjector::new() {
            Ok(injector) => {
                info!("Text injector initialized for: {:?}", injector.display_server());
                injector
            }
            Err(e) => {
                error!("Failed to initialize text injector: {}", e);
                error!("Text injection will be disabled. Install required tools:");
                error!("  For X11: sudo apt install xdotool");
                error!("  For Wayland: sudo apt install wtype");
                return;
            }
        };

        // Receive transcriptions directly from channel (no locks needed)
        while let Some(result) = transcription_rx.recv().await {
            match result {
                Ok(text) => {
                    info!("Injecting text: {}", text);
                    if let Err(e) = text_injector.inject_text(&text) {
                        error!("Failed to inject text: {}", e);
                    }
                }
                Err(e) => {
                    error!("Transcription error: {}", e);
                }
            }
        }
    });

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
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("Toggle error: {}", e);
                        }
                    }
                    HotkeyEvent::PushToTalkPressed => {
                        info!("⏺️ Push-to-talk pressed");
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("PTT start error: {}", e);
                        }
                    }
                    HotkeyEvent::PushToTalkReleased => {
                        info!("⏸️ Push-to-talk released");
                        if let Err(e) = daemon_clone.toggle().await {
                            error!("PTT stop error: {}", e);
                        }
                    }
                }
            }

            // IPC server (secondary, for CLI/scripts)
            Ok((stream, daemon)) = ipc_server.accept() => {
                // Handle connection inline (no spawn needed)
                if let Err(e) = handle_ipc_connection(stream, daemon).await {
                    error!("IPC connection error: {}", e);
                }
            }

            // Shutdown signal
            _ = tokio::signal::ctrl_c() => {
                info!("🛑 Received shutdown signal");
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
    use sysinfo::{System, Pid, ProcessesToUpdate};

    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);

    if let Some(process) = sys.process(pid) {
        process.memory() / 1_048_576 // bytes to MB
    } else {
        0
    }
}
