//! Swictation Daemon - Pure Rust Voice-to-Text Pipeline
//!
//! Runs as a background service (systemd), keeping models loaded in memory.
//! Communicates via Unix socket (/tmp/swictation.sock) for toggle commands.
//! Sway hotkey → socket toggle → start/stop recording (zero latency)

mod pipeline;
mod gpu;
mod config;
mod ipc;

use anyhow::{Context, Result};
use tracing::{info, error, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::DaemonConfig;
use crate::pipeline::Pipeline;
use crate::gpu::detect_gpu_provider;
use crate::ipc::IpcServer;

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

    // Start IPC server for toggle commands
    let socket_path = config.socket_path.clone();
    info!("🔌 Starting IPC server on {}", socket_path);

    let daemon_clone = Arc::new(daemon);
    let mut ipc_server = IpcServer::new(&socket_path, daemon_clone.clone())
        .context("Failed to start IPC server")?;

    info!("🚀 Swictation daemon ready!");
    info!("   Use 'swictation-cli toggle' or Sway hotkey to start/stop recording");

    // Handle transcription results in background
    let daemon_for_transcription = daemon_clone.clone();
    tokio::spawn(async move {
        loop {
            let pipeline = daemon_for_transcription.pipeline.read().await;
            // TODO: Implement transcription result handling
            // This will inject text when speech is transcribed
            drop(pipeline);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Run IPC server
    tokio::select! {
        result = ipc_server.run() => {
            if let Err(e) = result {
                error!("IPC server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Received shutdown signal");
        }
    }

    // Cleanup
    info!("🧹 Shutting down...");
    info!("👋 Swictation daemon stopped");

    Ok(())
}

/// Get current process memory usage in MB
fn get_memory_usage_mb() -> u64 {
    use sysinfo::{System, Pid};

    let mut sys = System::new();
    sys.refresh_process(Pid::from_u32(std::process::id()));

    if let Some(process) = sys.process(Pid::from_u32(std::process::id())) {
        process.memory() / 1_048_576 // bytes to MB
    } else {
        0
    }
}
