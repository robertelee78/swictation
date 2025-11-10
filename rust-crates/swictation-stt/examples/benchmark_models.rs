//! Benchmark Parakeet-TDT 0.6B vs 1.1B models
//!
//! Compares inference speed and GPU memory usage between the two model sizes.
//!
//! Usage:
//!   cargo run --release --example benchmark_models -- <audio_file>

use swictation_stt::{Recognizer, Result};
use std::env;
use std::time::Instant;

fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{:.2}ms", ms)
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}

fn run_benchmark(model_path: &str, model_name: &str, audio_file: &str, use_gpu: bool) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Testing: {} (GPU: {})", model_name, use_gpu);
    println!("{}", "=".repeat(80));

    // Load model
    println!("Loading model from: {}", model_path);
    let load_start = Instant::now();
    let mut recognizer = Recognizer::new(model_path, use_gpu)?;
    let load_time = load_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Model loaded in {}", format_duration(load_time));

    // Warm-up run (first run is slower due to initialization)
    println!("\nWarm-up run...");
    let _ = recognizer.recognize_file(audio_file)?;

    // Benchmark run
    println!("\nBenchmark run (3 iterations)...");
    let mut total_time = 0.0;
    let mut results = Vec::new();

    for i in 1..=3 {
        let result = recognizer.recognize_file(audio_file)?;
        total_time += result.processing_time_ms;
        println!("  Run {}: {} - '{}'",
            i,
            format_duration(result.processing_time_ms),
            result.text.chars().take(60).collect::<String>()
        );
        results.push(result);
    }

    let avg_time = total_time / 3.0;
    println!("\n📊 Average processing time: {}", format_duration(avg_time));
    println!("📝 Final transcription:\n  {}", results[0].text);

    Ok(())
}

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

    println!("🎵 Audio file: {}", audio_file);

    // Model paths
    let model_0_6b = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
    let model_1_1b = "/opt/swictation/models/parakeet-tdt-1.1b";

    // Check which models exist
    let has_0_6b = std::path::Path::new(model_0_6b).exists();
    let has_1_1b = std::path::Path::new(model_1_1b).exists();

    println!("\n📦 Available models:");
    println!("  0.6B: {}", if has_0_6b { "✅" } else { "❌" });
    println!("  1.1B: {}", if has_1_1b { "✅" } else { "❌" });

    // Test 0.6B model (baseline)
    if has_0_6b {
        run_benchmark(model_0_6b, "Parakeet-TDT 0.6B (INT8)", audio_file, true)?;
    }

    // Test 1.1B model
    if has_1_1b {
        run_benchmark(model_1_1b, "Parakeet-TDT 1.1B (INT8)", audio_file, true)?;
    }

    // Summary
    println!("\n{}", "=".repeat(80));
    println!("Benchmark Complete");
    println!("{}", "=".repeat(80));

    if !has_0_6b && !has_1_1b {
        eprintln!("\n❌ No models found! Please ensure models are exported to:");
        eprintln!("  - {}", model_0_6b);
        eprintln!("  - {}", model_1_1b);
    }

    Ok(())
}
