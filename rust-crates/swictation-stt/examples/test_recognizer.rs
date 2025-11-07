//! Test recognizer with placeholder inference

use swictation_stt::{Recognizer, DEFAULT_MODEL_PATH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== STT Recognizer Test ===\n");

    let recognizer = Recognizer::new(DEFAULT_MODEL_PATH)?;

    println!("\n{}", recognizer.model_info());

    // Test with dummy audio (1 second of silence at 16kHz)
    let audio = vec![0.0f32; 16000];

    println!("\n=== Testing Recognition ===");
    println!("Audio samples: {}", audio.len());

    let result = recognizer.recognize(&audio)?;

    println!("\nResult:");
    println!("  Text: '{}'", result.text);
    println!("  Confidence: {:.2}", result.confidence);
    println!("  Processing time: {:.2}ms", result.processing_time_ms);

    println!("\n✓ Recognizer test complete!");
    println!("  (Note: Actual inference not yet implemented)");

    Ok(())
}
