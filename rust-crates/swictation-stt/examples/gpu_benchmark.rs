/// GPU vs CPU benchmark for different Parakeet-TDT models
///
/// Usage: cargo run --release --example gpu_benchmark <audio_file.wav>

use swictation_stt::Recognizer;
use std::env;
use std::process;
use std::time::Instant;

const MODELS: &[(&str, &str)] = &[
    ("0.6B int8", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"),
    ("0.6B fp16", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-fp16"),
    ("110M fp32", "/opt/swictation/models/sherpa-onnx-nemo-parakeet_tdt_transducer_110m-en-36000"),
];

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file.wav>", args[0]);
        process::exit(1);
    }

    let audio_file = &args[1];

    println!("🎤 Benchmarking models with: {}\n", audio_file);
    println!("{:<15} {:<8} {:<12} {:<25}", "Model", "Device", "Time (ms)", "Result");
    println!("{}", "=".repeat(70));

    for (name, model_path) in MODELS {
        // Test CPU
        match test_model(model_path, audio_file, false) {
            Ok((time, text)) => {
                let truncated = if text.len() > 25 {
                    format!("{}...", &text[..22])
                } else {
                    text.clone()
                };
                println!("{:<15} {:<8} {:<12.2} {:<25}", name, "CPU", time, truncated);
            }
            Err(e) => {
                println!("{:<15} {:<8} {:<12} {:<25}", name, "CPU", "ERROR", format!("{}", e));
            }
        }

        // Test GPU
        match test_model(model_path, audio_file, true) {
            Ok((time, text)) => {
                let truncated = if text.len() > 25 {
                    format!("{}...", &text[..22])
                } else {
                    text.clone()
                };
                println!("{:<15} {:<8} {:<12.2} {:<25}", name, "GPU", time, truncated);
            }
            Err(e) => {
                println!("{:<15} {:<8} {:<12} {:<25}", name, "GPU", "ERROR", format!("{}", e));
            }
        }
        println!();
    }
}

fn test_model(model_path: &str, audio_file: &str, use_gpu: bool) -> Result<(f64, String), Box<dyn std::error::Error>> {
    // Load model (not timed)
    let mut recognizer = Recognizer::new(model_path, use_gpu)?;

    // Time inference only
    let start = Instant::now();
    let result = recognizer.recognize_file(audio_file)?;
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok((elapsed, result.text))
}
