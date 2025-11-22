//! Swictation Metrics Collection System
//!
//! Pure Rust metrics tracking for sessions, segments, and lifetime statistics.
//! Matches Python implementation in src/metrics/
//!
//! ## Features
//!
//! - **Native**: Full metrics collection with GPU monitoring
//! - **WASM**: Read-only database queries for npm distribution

pub mod collector;
pub mod database;
pub mod gpu;
pub mod memory;
pub mod models;

// WASM bindings (only when compiled to WebAssembly)
#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export main types
pub use collector::MetricsCollector;
pub use database::MetricsDatabase;
pub use gpu::{GpuMetrics, GpuMonitor};
pub use memory::{
    MemoryError, MemoryMonitor, MemoryPressure, MemoryStats, MemoryThresholds, RamStats, VramStats,
};
pub use models::{DaemonState, LifetimeMetrics, RealtimeMetrics, SegmentMetrics, SessionMetrics};

#[cfg(feature = "wasm")]
pub use wasm::MetricsDatabaseWasm;
