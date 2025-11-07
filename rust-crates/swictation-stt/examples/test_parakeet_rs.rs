//! Test parakeet-rs implementation for comparison

use parakeet_rs::ParakeetTDT;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing parakeet-rs Library (TDT Model) ===\n");

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

    // Load TDT model with parakeet-rs
    println!("Loading TDT model from: /opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8");
    let mut parakeet = ParakeetTDT::from_pretrained(
        "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
        None
    )?;
    println!("✓ Model loaded\n");

    for (name, path, reference) in &tests {
        println!("=== {} Sample ===", name);
        println!("Reference: {}", reference);

        let start = std::time::Instant::now();
        let result = parakeet.transcribe_file(path)?;
        let elapsed = start.elapsed().as_millis();

        println!("Result:    {}", result.text);
        println!("Time:      {}ms", elapsed);
        println!("Tokens:    {}", result.tokens.len());

        let matches = if result.text.to_lowercase().contains(&reference[..20].to_lowercase()) {
            "✓ PARTIAL MATCH"
        } else if result.text.len() > 10 {
            "⚠ NO MATCH (but has output)"
        } else {
            "✗ FAILED"
        };
        println!("Status:    {}\n", matches);
    }

    Ok(())
}
