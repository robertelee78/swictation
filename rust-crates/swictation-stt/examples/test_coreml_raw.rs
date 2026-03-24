//! Test CoreML with raw f32 audio (bypass Symphonia MP3 decoder)
fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    #[cfg(all(target_os = "macos", feature = "coreml-native"))]
    {
        use swictation_stt::CoreMLRecognizer;

        let model_dir = format!(
            "{}/.local/share/swictation/models/parakeet-tdt-1.1b-coreml",
            std::env::var("HOME").unwrap()
        );

        // Load raw f32 audio (same bytes as Python's ffmpeg output)
        let raw_bytes = std::fs::read("/tmp/en-short-16k-f32.raw").unwrap();
        let audio: Vec<f32> = raw_bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        println!(
            "Raw audio: {} samples, first 5: {:?}",
            audio.len(),
            &audio[..5]
        );

        let mut recognizer = CoreMLRecognizer::new(&model_dir).unwrap();
        let result = recognizer.recognize_samples(&audio).unwrap();

        println!("\n=== RESULT ===");
        println!("Transcription: '{}'", result);
        println!("Expected:      'hello world testing one two three'");
    }
}
