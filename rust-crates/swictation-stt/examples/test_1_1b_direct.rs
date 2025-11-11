//! Test Parakeet-TDT 1.1B using direct ONNX Runtime (OrtRecognizer)
//!
//! Usage:
//!   cargo run --release --example test_1_1b_direct <audio_file>

use swictation_stt::{OrtRecognizer, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Get audio file from args
    let args: Vec<String> = env::args().collect();
    let audio_file = if args.len() > 1 {
        &args[1]
    } else {
        "/opt/swictation/examples/en-short.mp3"
    };

    println!("🎵 Testing Parakeet-TDT 1.1B with ONNX Runtime");
    println!("📁 Audio: {}", audio_file);
    println!("{}", "=".repeat(80));

    // Load model
    let model_path = "/opt/swictation/models/parakeet-tdt-1.1b-onnx";
    println!("\n📦 Loading model from: {}", model_path);

    let load_start = Instant::now();
    let mut recognizer = OrtRecognizer::new(model_path, true)?; // use GPU
    let load_time = load_start.elapsed();

    println!("✅ Model loaded in {:.2}s", load_time.as_secs_f64());

    // Warm-up run
    println!("\n🔥 Warm-up run...");
    let _ = recognizer.recognize_file(audio_file)?;

    // Benchmark runs
    println!("\n📊 Benchmark (3 runs):");
    let mut times = Vec::new();
    let mut transcriptions = Vec::new();

    for i in 1..=3 {
        let start = Instant::now();
        let text = recognizer.recognize_file(audio_file)?;
        let duration = start.elapsed();

        times.push(duration.as_secs_f64() * 1000.0);
        transcriptions.push(text.clone());

        println!("  Run {}: {:.2}ms - '{}'",
            i,
            duration.as_secs_f64() * 1000.0,
            text.chars().take(60).collect::<String>()
        );
    }

    // Summary
    let avg_time = times.iter().sum::<f64>() / times.len() as f64;
    let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_time = times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    println!("\n{}", "=".repeat(80));
    println!("📈 Results:");
    println!("  Average: {:.2}ms", avg_time);
    println!("  Min:     {:.2}ms", min_time);
    println!("  Max:     {:.2}ms", max_time);
    println!("\n📝 Final Transcription:");
    println!("  {}", transcriptions[0]);
    println!("{}", "=".repeat(80));

    Ok(())
}
