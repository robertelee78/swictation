//! Test loading Parakeet model

use swictation_stt::{ParakeetModel, DEFAULT_MODEL_PATH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Parakeet Model Loading Test ===\n");

    let model = ParakeetModel::from_directory(DEFAULT_MODEL_PATH)?;

    println!("\n=== Model Information ===");
    println!("{}", model.encoder_info());
    println!("{}", model.decoder_info());
    println!("{}", model.joiner_info());
    println!("Vocabulary size: {}", model.tokens.vocab_size());
    println!("Blank token ID: {}", model.tokens.blank_id());

    // Test token decoding
    println!("\n=== Token Decoding Test ===");
    if let Some(token) = model.tokens.id_to_token(0) {
        println!("Token 0: '{}'", token);
    }
    if let Some(token) = model.tokens.id_to_token(1) {
        println!("Token 1: '{}'", token);
    }
    if let Some(token) = model.tokens.id_to_token(100) {
        println!("Token 100: '{}'", token);
    }

    println!("\n✓ Model loaded successfully!");

    Ok(())
}
