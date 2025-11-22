//! Export mel features from audio file for comparison with Python
//!
//! This tool extracts mel-spectrogram features from an audio file and exports them
//! to CSV format for direct comparison with Python implementations.
//!
//! # Usage
//!
//! ```bash
//! cargo run --release --example export_mel_features -- <audio_file> <output_csv>
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --release --example export_mel_features -- examples/en-short.mp3 rust_mel_features.csv
//! ```

use swictation_stt::audio::AudioProcessor;
use swictation_stt::error::SttError;
use tracing::{info, Level};

fn main() -> Result<(), SttError> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let audio_file = if args.len() > 1 {
        args[1].clone()
    } else {
        "/opt/swictation/examples/en-short.mp3".to_string()
    };

    let output_csv = if args.len() > 2 {
        args[2].clone()
    } else {
        "/opt/swictation/rust_mel_features.csv".to_string()
    };

    info!("=== Mel Feature Extraction ===");
    info!("Audio file: {}", audio_file);
    info!("Output CSV: {}", output_csv);

    // CRITICAL FIX: Use 80 mel features for 1.1B model (not the default 128 for 0.6B)
    let mut processor = AudioProcessor::with_mel_features(80)?;

    // Load audio file
    info!("Loading audio...");
    let samples = processor.load_audio(&audio_file)?;
    info!(
        "Loaded {} samples ({:.2}s at 16kHz)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Extract mel features
    info!("Extracting mel features...");
    let features = processor.extract_mel_features(&samples)?;
    info!(
        "Extracted features: {} frames x {} mel bins",
        features.nrows(),
        features.ncols()
    );

    // Calculate and display statistics
    let mean = features.mean().unwrap_or(0.0);
    let std = features.std(0.0);
    let min = features.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = features.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    info!("Feature statistics:");
    info!("  Mean: {:.6}", mean);
    info!("  Std:  {:.6}", std);
    info!("  Min:  {:.6}", min);
    info!("  Max:  {:.6}", max);

    // Export to CSV
    info!("Exporting to CSV...");
    processor.export_features_csv(&features, &output_csv)?;

    info!("=== Export Complete ===");
    info!("CSV file: {}", output_csv);
    info!("Total data points: {}", features.nrows() * features.ncols());

    Ok(())
}
