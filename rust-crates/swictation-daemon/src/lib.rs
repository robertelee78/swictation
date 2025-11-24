//! Swictation daemon library
//!
//! This module re-exports the daemon's modules for integration testing.

pub mod corrections;
pub mod display_server;
pub mod socket_utils;

// macOS text injection module (conditional compilation)
#[cfg(target_os = "macos")]
pub mod macos_text_inject;
