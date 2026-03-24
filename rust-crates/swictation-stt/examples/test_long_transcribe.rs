use std::time::Instant;
use swictation_stt::recognizer_ort::OrtRecognizer;

fn main() {
    // Try 1.1B first, fall back to 0.6B
    let model_paths = [
        (
            "/Users/robert/.local/share/swictation/models/parakeet-tdt-1.1b-onnx",
            true,
        ),
        (
            "/Users/robert/.local/share/swictation/models/parakeet-tdt-0.6b-v3-onnx",
            true,
        ),
    ];

    let mut recognizer = None;
    for (path, gpu) in &model_paths {
        if std::path::Path::new(path).exists() {
            eprintln!("Loading model from {}...", path);
            match OrtRecognizer::new(path, *gpu) {
                Ok(r) => {
                    eprintln!("Model loaded: {}", r.model_info());
                    recognizer = Some(r);
                    break;
                }
                Err(e) => eprintln!("Failed: {}", e),
            }
        }
    }

    let mut recognizer = recognizer.expect("No model found");

    let wav_path = "/tmp/test_long.wav";
    eprintln!("\nTranscribing {} (29.92s audio)...", wav_path);
    let start = Instant::now();
    match recognizer.recognize_file(wav_path) {
        Ok(text) => {
            let elapsed = start.elapsed();
            println!("\n=== RESULT ({:.2}s) ===", elapsed.as_secs_f64());
            println!("{}", text);
            println!("=== END ===");
            println!("Text length: {} chars", text.len());

            // Check if we got past "fifteen"
            let has_twenty = text.to_lowercase().contains("twenty");
            let has_thirty = text.to_lowercase().contains("thirty");
            let has_forty = text.to_lowercase().contains("forty");
            let has_fifty = text.to_lowercase().contains("fifty");
            println!("\nContains 'twenty': {}", has_twenty);
            println!("Contains 'thirty': {}", has_thirty);
            println!("Contains 'forty':  {}", has_forty);
            println!("Contains 'fifty':  {}", has_fifty);

            if has_thirty {
                println!("\n>>> STT handles >15s audio correctly. Bug is in the pipeline.");
            } else {
                println!("\n>>> STT TRUNCATES at ~15s. Bug is in the recognizer.");
            }
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
        }
    }
}
