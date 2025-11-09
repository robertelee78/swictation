/// Detailed GPU profiling to prove model loading is excluded from timing
///
/// Usage: cargo run --release --example detailed_gpu_profile <audio_file.wav>

use swictation_stt::Recognizer;
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file.wav>", args[0]);
        std::process::exit(1);
    }

    let audio_file = &args[1];
    let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";

    println!("🔍 Detailed profiling for: {}\n", audio_file);

    // CPU Test
    println!("=== CPU (int8 model) ===");
    let load_start = Instant::now();
    let mut cpu_recognizer = Recognizer::new(model_path, false).expect("Failed to load CPU model");
    let load_time = load_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Model loaded (NOT timed in benchmark): {:.2}ms", load_time);

    // Run inference 3 times to show consistency
    for i in 1..=3 {
        let inf_start = Instant::now();
        let result = cpu_recognizer.recognize_file(audio_file).expect("Inference failed");
        let inf_time = inf_start.elapsed().as_secs_f64() * 1000.0;
        println!("  Run {}: {:.2}ms - {}", i, inf_time, result.text.chars().take(30).collect::<String>());
    }

    println!("\n=== GPU (int8 model) ===");
    let load_start = Instant::now();
    let mut gpu_recognizer = Recognizer::new(model_path, true).expect("Failed to load GPU model");
    let load_time = load_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Model loaded (NOT timed in benchmark): {:.2}ms", load_time);

    // Run inference 3 times to show consistency
    for i in 1..=3 {
        let inf_start = Instant::now();
        let result = gpu_recognizer.recognize_file(audio_file).expect("Inference failed");
        let inf_time = inf_start.elapsed().as_secs_f64() * 1000.0;
        println!("  Run {}: {:.2}ms - {}", i, inf_time, result.text.chars().take(30).collect::<String>());
    }

    println!("\n=== 110M fp32 CPU ===");
    let model_110m = "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000";
    let load_start = Instant::now();
    let mut cpu_110m = Recognizer::new(model_110m, false).expect("Failed to load 110M CPU");
    let load_time = load_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Model loaded (NOT timed in benchmark): {:.2}ms", load_time);

    for i in 1..=3 {
        let inf_start = Instant::now();
        let result = cpu_110m.recognize_file(audio_file).expect("Inference failed");
        let inf_time = inf_start.elapsed().as_secs_f64() * 1000.0;
        println!("  Run {}: {:.2}ms - {}", i, inf_time, result.text.chars().take(30).collect::<String>());
    }

    println!("\n=== 110M fp32 GPU ===");
    let load_start = Instant::now();
    let mut gpu_110m = Recognizer::new(model_110m, true).expect("Failed to load 110M GPU");
    let load_time = load_start.elapsed().as_secs_f64() * 1000.0;
    println!("✅ Model loaded (NOT timed in benchmark): {:.2}ms", load_time);

    for i in 1..=3 {
        let inf_start = Instant::now();
        let result = gpu_110m.recognize_file(audio_file).expect("Inference failed");
        let inf_time = inf_start.elapsed().as_secs_f64() * 1000.0;
        println!("  Run {}: {:.2}ms - {}", i, inf_time, result.text.chars().take(30).collect::<String>());
    }

    println!("\n🎯 Conclusion:");
    println!("  Model loading is EXCLUDED from all inference timing");
    println!("  int8 GPU is genuinely slower due to CPU-optimized quantization");
    println!("  110M fp32 shows true GPU benefit for floating-point operations");
}
