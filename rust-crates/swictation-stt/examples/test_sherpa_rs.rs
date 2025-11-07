//! Test sherpa-rs - Official Rust bindings for sherpa-onnx

use sherpa_rs::read_audio_file;
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing sherpa-rs (Official sherpa-onnx Rust Bindings) ===\n");

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

    // Configure sherpa-rs with our existing Parakeet TDT model files
    let config = TransducerConfig {
        encoder: "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx".to_string(),
        decoder: "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/decoder.int8.onnx".to_string(),
        joiner: "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/joiner.int8.onnx".to_string(),
        tokens: "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/tokens.txt".to_string(),
        num_threads: 4,
        sample_rate: 16000,
        feature_dim: 128,  // Parakeet uses 128-dim mel filterbank
        model_type: "nemo_transducer".to_string(),  // NeMo models require this specific type
        decoding_method: "greedy_search".to_string(),
        hotwords_file: String::new(),
        hotwords_score: 1.5,
        modeling_unit: String::new(),
        bpe_vocab: String::new(),
        blank_penalty: 0.0,  // Will experiment with this
        debug: false,
        provider: Some("cpu".to_string()),
    };

    println!("Loading model with sherpa-rs...");
    let mut recognizer = TransducerRecognizer::new(config)?;
    println!("✓ Model loaded\n");

    for (name, path, reference) in &tests {
        println!("=== {} Sample ===", name);
        println!("Reference: {}", reference);

        // Load audio file
        let (samples, sample_rate) = read_audio_file(path)?;

        if sample_rate != 16000 {
            println!("⚠ Warning: Sample rate is {} Hz, expected 16000 Hz", sample_rate);
        }

        let start = std::time::Instant::now();
        let result = recognizer.transcribe(sample_rate, &samples);
        let elapsed = start.elapsed().as_millis();

        let text = result.to_lowercase().trim().to_string();
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
