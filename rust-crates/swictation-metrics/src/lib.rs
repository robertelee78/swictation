//! Swictation Metrics Collection System
//!
//! Pure Rust metrics tracking for sessions, segments, and lifetime statistics.
//! Matches Python implementation in src/metrics/

pub mod collector;
pub mod database;
pub mod models;

// Re-export main types
pub use collector::MetricsCollector;
pub use database::MetricsDatabase;
pub use models::{
    DaemonState, LifetimeMetrics, RealtimeMetrics, SegmentMetrics, SessionMetrics,
};
