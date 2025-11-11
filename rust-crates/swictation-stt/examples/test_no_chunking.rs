/// Test ORT recognizer WITHOUT chunking to match Python behavior
use anyhow::Result;
use std::env;
use swictation_stt::recognizer_ort::OrtRecognizer;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    println!("\n================================================================================");
    println!("🧪 Testing Parakeet-TDT 1.1B WITHOUT chunking (match Python)");
    println!("================================================================================\n");

    let model_dir = "/opt/swictation/models/parakeet-tdt-1.1b";
    let audio_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/en-short.mp3".to_string());

    println!("📦 Model: {}", model_dir);
    println!("🎵 Audio: {}\n", audio_path);

    // Create recognizer
    println!("⚙️  Loading ORT recognizer...");
    let mut recognizer = OrtRecognizer::new(model_dir)?;

    // Override: Process audio WITHOUT chunking
    println!("⚙️  Loading and processing audio WITHOUT chunking...");
    let samples = recognizer.audio_processor_mut().load_audio(&audio_path)?;
    println!("   Loaded {} samples", samples.len());

    let features = recognizer.audio_processor_mut().extract_mel_features(&samples)?;
    println!("   Extracted mel features: {} frames x {} features",
             features.nrows(), features.ncols());

    // Process ALL frames at once (no chunking)
    println!("\n🚀 Running encoder on ALL {} frames at once (no chunking)...", features.nrows());
    let encoder_out = recognizer.run_encoder_direct(&features)?;
    println!("   Encoder output: {:?}", encoder_out.shape());

    // Decode all frames at once
    println!("\n🔍 Decoding all encoder outputs...");
    let text = recognizer.decode_encoder_output_simple(&encoder_out)?;

    println!("\n================================================================================");
    println!("📝 RESULT WITHOUT CHUNKING:");
    println!("================================================================================");
    println!("{}", text);
    println!("================================================================================\n");

    Ok(())
}
