//! Real-time metrics broadcaster for Swictation UI
//!
//! This crate provides a Unix socket server that broadcasts real-time metrics
//! and transcription events to connected UI clients. It manages multiple concurrent
//! clients, session-based transcription buffers, and various event types.
//!
//! # Features
//!
//! - Unix domain socket server (`/tmp/swictation_metrics.sock`)
//! - Newline-delimited JSON protocol
//! - Multiple concurrent client connections
//! - Session-based transcription buffer (RAM only)
//! - Thread-safe client management
//! - New client catch-up (current state + buffer)
//!
//! # Event Types
//!
//! - `session_start` - Session begins, clears buffer
//! - `session_end` - Session ends, buffer stays visible
//! - `transcription` - New transcription segment
//! - `metrics_update` - Real-time metrics from daemon
//! - `state_change` - Daemon state change
//!
//! # Example Usage
//!
//! ```no_run
//! use swictation_broadcaster::MetricsBroadcaster;
//! use swictation_metrics::{MetricsCollector, DaemonState};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and start broadcaster
//!     let broadcaster = MetricsBroadcaster::new("/tmp/swictation_metrics.sock").await?;
//!     broadcaster.start().await?;
//!
//!     // Start session
//!     let session_id = 123;
//!     broadcaster.start_session(session_id).await;
//!
//!     // Add transcription
//!     broadcaster.add_transcription(
//!         "Hello world".to_string(),
//!         145.2,  // wpm
//!         234.5,  // latency_ms
//!         2,      // words
//!     ).await;
//!
//!     // Update metrics
//!     // let metrics = collector.get_realtime_metrics();
//!     // broadcaster.update_metrics(&metrics).await;
//!
//!     // Broadcast state change
//!     broadcaster.broadcast_state_change(DaemonState::Recording).await;
//!
//!     // End session
//!     broadcaster.end_session(session_id).await;
//!
//!     // Stop broadcaster
//!     broadcaster.stop().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod broadcaster;
pub mod client;
pub mod error;
pub mod events;

// Re-exports
pub use broadcaster::MetricsBroadcaster;
pub use error::{BroadcasterError, Result};
pub use events::{BroadcastEvent, TranscriptionSegment};
