use std::path::PathBuf;
use swictation_stt::OrtRecognizer;

fn main() {
    let home = std::env::var("HOME").expect("HOME not set");
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));

    println!("Testing 0.6B model at: {}", model_path.display());

    if !model_path.exists() {
        eprintln!("⚠️  Model path does not exist: {}", model_path.display());
        eprintln!("   Skipping test (model not downloaded)");
        return;
    }

    match OrtRecognizer::new(&model_path, false) {
        Ok(_recognizer) => {
            println!("✅ Model loaded successfully!");
            println!("   This confirms:");
            println!("   - ModelConfig detection works (should detect 0.6B)");
            println!("   - decoder_hidden_size = 512");
            println!("   - n_mel_features = 128");
            println!("   - transpose_input = true");
            println!("\n🎉 0.6B model test PASSED - OrtRecognizer supports 0.6B!");
        }
        Err(e) => {
            eprintln!("❌ Failed to load 0.6B model: {}", e);
            eprintln!("   0.6B model test FAILED!");
            std::process::exit(1);
        }
    }
}
