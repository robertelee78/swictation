//! Simple test utility for benchmarking a single audio file

use anyhow::Result;
use std::env;
use std::time::Instant;
use swictation_stt::Recognizer;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <model_path> <use_gpu> <audio_file>", args[0]);
        eprintln!("Example: {} /path/to/model true audio.wav", args[0]);
        std::process::exit(1);
    }

    let model_path = &args[1];
    let use_gpu: bool = args[2].parse().unwrap_or(false);
    let audio_file = &args[3];

    // Load model
    let start = Instant::now();
    let mut recognizer = Recognizer::new(model_path, use_gpu)?;
    let load_time = start.elapsed().as_millis();

    // Transcribe
    let start = Instant::now();
    let result = recognizer.recognize_file(audio_file)?;
    let inference_time = start.elapsed().as_millis();

    // Output (format for parsing)
    println!("Load: {}ms", load_time);
    println!("Inference: {}ms", inference_time);
    println!("Text: {}", result.text);

    Ok(())
}
