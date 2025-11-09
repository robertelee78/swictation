//! GPU vs CPU benchmark test for Sherpa-RS with CUDA 12
//!
//! Compares GPU and CPU inference performance to measure speedup.

use anyhow::Result;
use swictation_stt::Recognizer;
use std::time::Instant;

// Use int8 model with TensorRT EP for GPU acceleration
const MODEL_PATH: &str = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
const SHORT_AUDIO: &str = "/tmp/en-short.wav";
const LONG_AUDIO: &str = "/tmp/en-long.wav";

fn main() -> Result<()> {
    println!("═══════════════════════════════════════════════════");
    println!("  Sherpa-RS CUDA 12 GPU Benchmark");
    println!("═══════════════════════════════════════════════════\n");

    // Test GPU
    println!("[ GPU Mode - CUDA 12 ]");
    let start = Instant::now();
    let mut gpu = Recognizer::new(MODEL_PATH, true)?;
    let load_time_gpu = start.elapsed().as_secs_f64();
    println!("  Load: {:.2}s", load_time_gpu);

    let start = Instant::now();
    let result_gpu_short = gpu.recognize_file(SHORT_AUDIO)?;
    let gpu_short_ms = start.elapsed().as_millis();
    println!("  Short: {:.0}ms - \"{}\"", gpu_short_ms, result_gpu_short.text);

    let start = Instant::now();
    let result_gpu_long = gpu.recognize_file(LONG_AUDIO)?;
    let gpu_long_ms = start.elapsed().as_millis();
    println!("  Long:  {:.0}ms - \"{:.50}...\"", gpu_long_ms, result_gpu_long.text);
    println!();

    // Test CPU
    println!("[ CPU Mode ]");
    let start = Instant::now();
    let mut cpu = Recognizer::new(MODEL_PATH, false)?;
    let load_time_cpu = start.elapsed().as_secs_f64();
    println!("  Load: {:.2}s", load_time_cpu);

    let start = Instant::now();
    let result_cpu_short = cpu.recognize_file(SHORT_AUDIO)?;
    let cpu_short_ms = start.elapsed().as_millis();
    println!("  Short: {:.0}ms - \"{}\"", cpu_short_ms, result_cpu_short.text);

    let start = Instant::now();
    let result_cpu_long = cpu.recognize_file(LONG_AUDIO)?;
    let cpu_long_ms = start.elapsed().as_millis();
    println!("  Long:  {:.0}ms - \"{:.50}...\"", cpu_long_ms, result_cpu_long.text);
    println!();

    // Calculate speedup
    println!("═══════════════════════════════════════════════════");
    println!("[ GPU Speedup ]");
    let speedup_short = cpu_short_ms as f64 / gpu_short_ms as f64;
    let speedup_long = cpu_long_ms as f64 / gpu_long_ms as f64;
    let speedup_load = load_time_cpu / load_time_gpu;

    println!("  Load time:   {:.2}x faster", speedup_load);
    println!("  Short audio: {:.2}x faster", speedup_short);
    println!("  Long audio:  {:.2}x faster", speedup_long);

    if speedup_short >= 1.5 && speedup_long >= 1.5 {
        println!("\n✓ GPU acceleration is working! ({:.1}x average speedup)", (speedup_short + speedup_long) / 2.0);
    } else {
        println!("\n⚠ GPU speedup is lower than expected");
    }
    println!("═══════════════════════════════════════════════════");

    Ok(())
}
