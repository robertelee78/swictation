//! Export raw audio samples to CSV for comparison with Python
//!
//! This tool loads an audio file and exports the raw samples (after resampling)
//! to CSV format for direct comparison with Python implementations.

use std::fs::File;
use std::io::Write;
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
        "/tmp/rust_raw_audio.csv".to_string()
    };

    info!("=== Raw Audio Export ===");
    info!("Audio file: {}", audio_file);
    info!("Output CSV: {}", output_csv);

    let mut processor = AudioProcessor::new()?;

    // Load audio file
    info!("Loading audio...");
    let samples = processor.load_audio(&audio_file)?;
    info!(
        "Loaded {} samples ({:.2}s at 16kHz)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Calculate statistics
    let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
    let min = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    info!("Raw audio statistics:");
    info!("  Mean: {:.6}", mean);
    info!("  RMS:  {:.6}", rms);
    info!("  Min:  {:.6}", min);
    info!("  Max:  {:.6}", max);

    // Export to CSV
    info!("Exporting to CSV...");
    let mut file = File::create(&output_csv)
        .map_err(|e| SttError::AudioLoadError(format!("Failed to create CSV: {}", e)))?;

    writeln!(file, "index,value")
        .map_err(|e| SttError::AudioLoadError(format!("Failed to write header: {}", e)))?;

    for (idx, &value) in samples.iter().enumerate() {
        writeln!(file, "{},{}", idx, value)
            .map_err(|e| SttError::AudioLoadError(format!("Failed to write data: {}", e)))?;
    }

    info!("=== Export Complete ===");
    info!("CSV file: {}", output_csv);
    info!("Total samples: {}", samples.len());

    Ok(())
}
