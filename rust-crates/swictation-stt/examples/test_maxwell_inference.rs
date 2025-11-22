use std::path::PathBuf;
/// Direct STT test for Maxwell GPU debugging
/// Bypasses all audio capture - feeds WAV file directly to model
/// Uses OrtRecognizer (same as daemon) with detailed logging
use swictation_stt::OrtRecognizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Maxwell GPU STT Direct Inference Test");
    println!("Testing model with WAV file (no microphone/speakers)\n");

    let home = std::env::var("HOME").expect("HOME not set");

    // Use the 0.6B model (GPU variant)
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));

    // Test audio file from model directory
    let audio_file = model_path.join("/tmp/test-audio-proper.wav");

    println!("📁 Model directory: {}", model_path.display());
    println!("🎵 Audio file: {}", audio_file.display());
    println!("🎮 Testing with GPU (CUDA 11.8 + cuDNN 8.9.7)...\n");

    // Verify files exist
    if !model_path.exists() {
        eprintln!("❌ Model directory not found: {}", model_path.display());
        return Err("Model not found".into());
    }

    if !audio_file.exists() {
        eprintln!("❌ Audio file not found: {}", audio_file.display());
        return Err("Audio file not found".into());
    }

    println!("✓ Files verified\n");

    // Load model with GPU
    println!("Loading OrtRecognizer with GPU...");
    let mut recognizer = match OrtRecognizer::new(&model_path, true) {
        Ok(r) => {
            println!("✅ Model loaded successfully!");
            r
        }
        Err(e) => {
            eprintln!("❌ Failed to load model: {}", e);
            return Err(e.into());
        }
    };

    // Process audio through model using recognize_file method
    println!("\n🔄 Processing audio through STT model...");
    println!("   (This is the critical test - will it produce text or blanks?)\n");

    let text = match recognizer.recognize_file(&audio_file) {
        Ok(t) => {
            println!("✅ Recognition completed");
            t
        }
        Err(e) => {
            eprintln!("❌ Failed to recognize audio: {}", e);
            return Err(e.into());
        }
    };

    println!("\n{}", "=".repeat(60));
    println!("🎯 TRANSCRIPTION RESULT:");
    println!("{}", "=".repeat(60));

    if text.trim().is_empty() {
        println!("❌ BLANK OUTPUT (This is the bug!)");
        println!("   The model processed the audio but produced no text.");
        println!("   This confirms the blank output problem exists even with direct inference.");
    } else {
        println!("✅ TEXT OUTPUT:");
        println!("   \"{}\"\n", text);
        println!("🎉 SUCCESS! Model is producing text correctly!");
    }

    println!("{}\n", "=".repeat(60));

    // Additional diagnostics
    println!("🔍 Additional Diagnostics:");
    println!("   ORT_DYLIB_PATH: {:?}", std::env::var("ORT_DYLIB_PATH"));
    println!("   LD_LIBRARY_PATH: {:?}", std::env::var("LD_LIBRARY_PATH"));
    println!(
        "   CUDA_VISIBLE_DEVICES: {:?}",
        std::env::var("CUDA_VISIBLE_DEVICES")
    );

    Ok(())
}
