/// Test encoder in isolation
use swictation_stt::OrtRecognizer;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Encoder Only\n");
    
    let home = std::env::var("HOME").expect("HOME not set");
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));
    
    println!("Loading model...");
    let mut recognizer = OrtRecognizer::new(&model_path, true)?;
    println!("✅ Model loaded\n");
    
    // Test encoder with dummy input
    println!("🔄 Testing encoder inference...");
    let result = recognizer.test_encoder_inference()?;
    println!("✅ {}\n", result);
    
    Ok(())
}
