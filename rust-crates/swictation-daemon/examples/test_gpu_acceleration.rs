//! Quick GPU acceleration test for Parakeet-TDT with sherpa-rs

use anyhow::Result;
use swictation_stt::Recognizer;
use std::time::Instant;

const MODEL_PATH: &str = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
const TEST_AUDIO: &str = "/tmp/en-short.wav";

fn main() -> Result<()> {
    println!("Sherpa-RS GPU Acceleration Test\n");

    // Test GPU
    println!("[ GPU Mode ]");
    let start = Instant::now();
    let mut gpu = Recognizer::new(MODEL_PATH, true)?;
    println!("  Load: {:.2}s", start.elapsed().as_secs_f64());

    let start = Instant::now();
    let result = gpu.recognize_file(TEST_AUDIO)?;
    println!("  Inference: {:.2}ms", result.processing_time_ms);
    println!("  Result: {}\n", result.text);

    // Test CPU
    println!("[ CPU Mode ]");
    let start = Instant::now();
    let mut cpu = Recognizer::new(MODEL_PATH, false)?;
    println!("  Load: {:.2}s", start.elapsed().as_secs_f64());

    let start = Instant::now();
    let result_cpu = cpu.recognize_file(TEST_AUDIO)?;
    println!("  Inference: {:.2}ms", result_cpu.processing_time_ms);
    println!("  Result: {}\n", result.text);

    let speedup = result_cpu.processing_time_ms / result.processing_time_ms;
    println!("GPU Speedup: {:.2}x faster", speedup);

    Ok(())
}
