// Direct STT test for Maxwell GPU debugging
// Bypasses all audio capture - feeds WAV file directly to model

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Maxwell GPU STT Direct Test");
    println!("Testing model inference without microphone/speakers\n");

    // Use the test audio from the model directory
    let audio_file = "/tmp/test-audio.wav";
    let model_dir = std::env::var("MODEL_DIR")
        .unwrap_or_else(|_| "/home/jrl/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx".to_string());

    println!("📁 Model directory: {}", model_dir);
    println!("🎵 Audio file: {}", audio_file);
    println!("🎮 Testing with GPU...\n");

    // This will require adding swictation-stt as a dependency
    // For now, just verify files exist
    if !Path::new(&model_dir).exists() {
        eprintln!("❌ Model directory not found: {}", model_dir);
        return Err("Model not found".into());
    }

    if !Path::new(audio_file).exists() {
        eprintln!("❌ Audio file not found: {}", audio_file);
        return Err("Audio file not found".into());
    }

    println!("✓ Files verified");
    println!("\n⚠️  This is a stub - need to add STT library integration");

    Ok(())
}
