/// Simple test: Load WAV, call recognize_samples() like daemon does
use swictation_stt::OrtRecognizer;
use swictation_stt::audio::AudioProcessor;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing recognize_samples() with WAV file");
    
    let audio_file = PathBuf::from("/tmp/test-audio-proper.wav");
    println!("📁 Audio: {}", audio_file.display());
    
    // Load audio like daemon does
    let mut processor = AudioProcessor::new()?;
    let samples = processor.load_audio(&audio_file)?;
    println!("✅ Loaded {} samples\n", samples.len());
    
    // Load model
    let home = std::env::var("HOME").expect("HOME not set");
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));
    
    println!("Loading model...");
    let mut recognizer = OrtRecognizer::new(&model_path, true)?;
    println!("✅ Model loaded\n");
    
    // Call recognize_samples() like daemon does
    println!("🔄 Calling recognize_samples()...");
    let text = recognizer.recognize_samples(&samples)?;
    
    println!("\n{}", "=".repeat(60));
    println!("RESULT: '{}'", text);
    println!("{}\n", "=".repeat(60));
    
    Ok(())
}
