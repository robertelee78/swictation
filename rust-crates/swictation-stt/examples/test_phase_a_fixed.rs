//! Test Phase A with fixes based on sherpa-rs learnings
//!
//! Key fixes:
//! 1. Use kaldifeat library for proper feature extraction (matches sherpa-onnx)
//! 2. Fix decoder start token (should be blank_id, not 0)
//! 3. Fix RNN-T loop logic for proper blank handling
//! 4. Add proper context caching between frames

use hound;
use swictation_stt::Recognizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Phase A (Custom Implementation) with Fixes ===\n");

    let tests = [
        (
            "Short",
            "/tmp/en-short-16k.wav",
            "Hello world. Testing, one, two, three",
        ),
        (
            "Long",
            "/tmp/en-long-16k.wav",
            "The open-source AI community has scored a significant win...",
        ),
    ];

    let model_path = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
    let mut recognizer = Recognizer::new(model_path)?;

    println!("✓ Model loaded\n");
    println!("Model info:\n{}\n", recognizer.model_info());

    for (name, path, reference) in &tests {
        println!("=== {} Sample ===", name);
        println!("Reference: {}", reference);

        // Load audio file
        let mut reader = hound::WavReader::open(path)?;
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();

        let start = std::time::Instant::now();
        let result = recognizer.recognize(&samples)?;
        let elapsed = start.elapsed().as_millis();

        let text = result.text.trim().to_string();
        println!("Result:    {}", text);
        println!("Time:      {}ms", elapsed);

        // Normalize for comparison (remove hyphens, punctuation)
        let normalized_text = text.replace("-", " ").replace(".", "").replace(",", "");
        let normalized_ref = reference.replace("-", " ").replace(".", "").replace(",", "");

        let matches = if normalized_text.to_lowercase().contains(&normalized_ref[..20].to_lowercase()) {
            "✓ MATCH!"
        } else if text.len() > 10 {
            "⚠ NO MATCH (but has output)"
        } else {
            "✗ FAILED"
        };
        println!("Status:    {}\n", matches);
    }

    Ok(())
}
