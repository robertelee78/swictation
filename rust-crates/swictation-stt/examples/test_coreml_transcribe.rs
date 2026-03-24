//! Test CoreML recognizer on en-short.mp3
//! Run: cargo run --features coreml-native --example test_coreml_transcribe

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

        println!("Loading CoreML recognizer...");
        let mut recognizer = CoreMLRecognizer::new(&model_dir).unwrap();

        println!("Transcribing en-short.mp3...");
        let result = recognizer.recognize_file("/opt/swictation/examples/en-short.mp3");

        match result {
            Ok(text) => {
                println!("\n=== RESULT ===");
                println!("Transcription: '{}'", text);
                println!("Expected:      'hello world testing one two three'");
            }
            Err(e) => {
                println!("ERROR: {}", e);
            }
        }
    }

    #[cfg(not(all(target_os = "macos", feature = "coreml-native")))]
    println!("Requires --features coreml-native on macOS");
}
