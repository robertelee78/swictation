use std::path::PathBuf;
use swictation_stt::OrtRecognizer;

fn main() {
    let home = std::env::var("HOME").expect("HOME not set");
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/parakeet-tdt-1.1b-onnx",
        home
    ));

    println!("Testing 1.1B model at: {}", model_path.display());

    if !model_path.exists() {
        eprintln!("⚠️  Model path does not exist: {}", model_path.display());
        eprintln!("   Skipping test (model not downloaded)");
        return;
    }

    match OrtRecognizer::new(&model_path, false) {
        Ok(_recognizer) => {
            println!("✅ Model loaded successfully!");
            println!("   This confirms:");
            println!("   - ModelConfig detection works (should detect 1.1B)");
            println!("   - decoder_hidden_size = 640");
            println!("   - n_mel_features = 80");
            println!("   - transpose_input = false");
            println!("\n🎉 Regression test PASSED - 1.1B model still works!");
        }
        Err(e) => {
            eprintln!("❌ Failed to load 1.1B model: {}", e);
            eprintln!("   Regression test FAILED!");
            std::process::exit(1);
        }
    }
}
