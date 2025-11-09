/// Transcribe a WAV file using sherpa-rs
///
/// Usage: cargo run --release --example transcribe_wav <audio_file.wav>

use swictation_stt::Recognizer;
use std::env;
use std::process;

const DEFAULT_MODEL: &str = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file.wav>", args[0]);
        process::exit(1);
    }

    let audio_file = &args[1];

    println!("🎤 Transcribing: {}", audio_file);
    println!("📦 Model: {}", DEFAULT_MODEL);
    println!("⚙️  Using CPU (GPU not enabled for this example)\n");

    // Load recognizer
    let mut recognizer = match Recognizer::new(DEFAULT_MODEL, false) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to load model: {}", e);
            process::exit(1);
        }
    };

    // Load and transcribe audio file
    match recognizer.recognize_file(audio_file) {
        Ok(result) => {
            println!("📝 Transcription:\n{}\n", result.text);
            println!("⏱️  Processing time: {:.2}ms", result.processing_time_ms);
            println!("✅ Success!");
        }
        Err(e) => {
            eprintln!("❌ Transcription failed: {}", e);
            process::exit(1);
        }
    }
}
