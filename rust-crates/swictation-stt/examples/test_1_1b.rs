use swictation_stt::Recognizer;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted";
    let audio_file = "/tmp/en-short-16k.wav";

    println!("===========================================");
    println!("Testing Parakeet-TDT 1.1B Model");
    println!("===========================================\n");

    // Test CPU int8
    println!("[1/2] Testing CPU (int8 quantized)...");
    let mut recognizer_cpu = Recognizer::new(model_path, false)?;
    println!("✓ Model loaded on CPU");

    let start = Instant::now();
    let result_cpu = recognizer_cpu.recognize_file(audio_file)?;
    let cpu_time = start.elapsed().as_millis();

    println!("  Transcription: \"{}\"", result_cpu.text);
    println!("  Inference time: {}ms\n", cpu_time);

    // Test GPU fp32
    println!("[2/2] Testing GPU (fp32)...");
    let mut recognizer_gpu = Recognizer::new(model_path, true)?;
    println!("✓ Model loaded on GPU");

    // Warmup run
    let _ = recognizer_gpu.recognize_file(audio_file)?;

    // Actual timed run
    let start = Instant::now();
    let result_gpu = recognizer_gpu.recognize_file(audio_file)?;
    let gpu_time = start.elapsed().as_millis();

    println!("  Transcription: \"{}\"", result_gpu.text);
    println!("  Inference time: {}ms (after warmup)\n", gpu_time);

    // Summary
    println!("===========================================");
    println!("Summary:");
    println!("===========================================");
    println!("CPU (int8):  {}ms", cpu_time);
    println!("GPU (fp32):  {}ms", gpu_time);
    if gpu_time < cpu_time {
        println!("GPU Speedup: {:.1}x faster", cpu_time as f64 / gpu_time as f64);
    }
    println!("\n✓ 1.1B model working successfully!");

    Ok(())
}
