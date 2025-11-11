#!/usr/bin/env rust-script
//! Debugging test for ORT recognizer to analyze blank token spam issue
//!
//! Run with: RUST_LOG=debug cargo run --release --example test_ort_debug

use swictation_stt::recognizer_ort::OrtRecognizer;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("\n{}", "=".repeat(80));
    println!("🔍 ORT Recognizer Debug Test - Analyzing Blank Token Spam");
    println!("{}", "=".repeat(80));

    // Model and audio paths
    let model_dir = "/opt/swictation/models/parakeet-tdt-1.1b";
    let audio_file = env::args().nth(1)
        .unwrap_or_else(|| "/opt/swictation/examples/en-short.mp3".to_string());

    println!("\n📦 Model: {}", model_dir);
    println!("🎵 Audio: {}", audio_file);

    // Create recognizer
    println!("\n⚙️  Loading ORT recognizer...");
    let mut recognizer = OrtRecognizer::new(model_dir, false)?;
    println!("✅ Recognizer loaded!");
    println!("{}", recognizer.model_info());

    // Recognize
    println!("\n🔄 Running recognition with FULL DEBUG logging...");
    println!("{}", "=".repeat(80));

    let text = recognizer.recognize_file(&audio_file)?;

    println!("\n{}", "=".repeat(80));
    println!("📝 FINAL RESULT:");
    println!("{}", "=".repeat(80));
    println!("{}", text);
    println!("{}", "=".repeat(80));

    if text.trim().is_empty() || text.contains("mmhmm") {
        println!("\n❌ FAILURE: Empty or blank-token-spam result");
        println!("   This indicates a bug in the decoder/joiner pipeline");
        std::process::exit(1);
    } else {
        println!("\n✅ SUCCESS: Got meaningful transcription");
        std::process::exit(0);
    }
}
