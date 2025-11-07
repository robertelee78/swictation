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

use anyhow::{Context, Result};
use tracing::{info, error, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::DaemonConfig;
use crate::pipeline::Pipeline;
use crate::gpu::detect_gpu_provider;
use crate::ipc::{IpcServer, handle_connection as handle_ipc_connection};
use crate::hotkey::{HotkeyManager, HotkeyEvent};

#[derive(Debug, Clone, PartialEq)]
enum DaemonState {
    Idle,
    Recording,
}

struct Daemon {
    pipeline: Arc<RwLock<Pipeline>>,
    state: Arc<RwLock<DaemonState>>,
}

impl Daemon {
    async fn new(config: DaemonConfig, gpu_provider: Option<String>) -> Result<Self> {
        let pipeline = Pipeline::new(config, gpu_provider).await?;

        Ok(Self {
            pipeline: Arc::new(RwLock::new(pipeline)),
            state: Arc::new(RwLock::new(DaemonState::Idle)),
        })
    }

    async fn toggle(&self) -> Result<String> {
        let mut state = self.state.write().await;
        let mut pipeline = self.pipeline.write().await;

        match *state {
            DaemonState::Idle => {
                info!("▶️ Starting recording");
                pipeline.start_recording().await?;
                *state = DaemonState::Recording;
                Ok("Recording started".to_string())
            }
            DaemonState::Recording => {
                info!("⏸️ Stopping recording");
                pipeline.stop_recording().await?;
                *state = DaemonState::Idle;
                Ok("Recording stopped".to_string())
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
    let daemon = Daemon::new(config.clone(), gpu_provider.clone())
        .await
        .context("Failed to initialize daemon")?;

    info!("✓ Pipeline initialized successfully");
    info!("  - Audio: 16000 Hz, 1 channel");
    info!("  - VAD: Silero VAD (sherpa-rs)");
    info!("  - STT: Parakeet-TDT-0.6B-V3 (sherpa-onnx)");
    if let Some(provider) = &gpu_provider {
        info!("  - GPU: {} acceleration enabled", provider);
    }
    info!("📊 Memory usage: {} MB", get_memory_usage_mb());

    // Initialize hotkey manager
    let mut hotkey_manager = HotkeyManager::new(config.hotkeys.clone())
        .context("Failed to initialize hotkey manager")?;

    info!("🎹 Hotkey registered: {} (toggle)", config.hotkeys.toggle);
    info!("🎹 Hotkey registered: {} (push-to-talk)", config.hotkeys.push_to_talk);

    // Start IPC server for CLI/scripts (optional)
    let socket_path = config.socket_path.clone();
    info!("🔌 Starting IPC server on {}", socket_path);

    let daemon_clone = Arc::new(daemon);
    let mut ipc_server = IpcServer::new(&socket_path, daemon_clone.clone())
        .context("Failed to start IPC server")?;

    info!("🚀 Swictation daemon ready!");
    info!("   Press {} to start/stop recording", config.hotkeys.toggle);
    info!("   Or use 'swictation-cli toggle' for CLI control");

    // TODO: Handle transcription results in background
    // This requires making Pipeline Send + Sync safe
    // let daemon_for_transcription = daemon_clone.clone();
    // tokio::spawn(async move {
    //     loop {
    //         if let Some(result) = pipeline.next_transcription().await {
    //             // Inject text with enigo
    //         }
    //     }
    // });

    // Main event loop
    loop {
        tokio::select! {
            // Hotkey events (primary UX)
            Some(event) = hotkey_manager.next_event() => {
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
