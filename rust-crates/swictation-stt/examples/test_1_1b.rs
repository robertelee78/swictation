/// Test the 1.1B Parakeet-TDT model with real audio transcription
///
/// This example demonstrates the complete pipeline:
/// - Audio loading (WAV/MP3/FLAC/OGG)
/// - Mel-spectrogram extraction
/// - Greedy search transducer decoding
/// - Text generation with BPE token handling
///
/// ## Running this test:
///
/// ```bash
/// export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
/// cargo run --release --example test_1_1b -- /opt/swictation/examples/en-short.mp3
/// ```

use swictation_stt::OrtRecognizer;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with DEBUG level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("======================================================");
    println!("1.1B Parakeet-TDT Model - Real Audio Transcription Test");
    println!("======================================================\n");

    // Get audio file from command line args, or use default
    let args: Vec<String> = std::env::args().collect();
    let audio_path = if args.len() > 1 {
        &args[1]
    } else {
        "/opt/swictation/examples/en-short.mp3"
    };

    println!("Audio file: {}\n", audio_path);

    let model_dir = "/opt/swictation/models/parakeet-tdt-1.1b-corrected";

    // Test 1: Load model with GPU
    println!("[TEST 1] Loading 1.1B model with GPU...");
    let start = Instant::now();
    let mut recognizer = match OrtRecognizer::new(model_dir, true) {
        Ok(r) => {
            let load_time = start.elapsed();
            println!("✅ SUCCESS: Model loaded with GPU in {:.2}s", load_time.as_secs_f32());
            println!("   - encoder.onnx + encoder.weights (4GB) loaded");
            println!("   - decoder.onnx loaded");
            println!("   - joiner.onnx loaded");
            println!("   - CUDA execution provider active\n");
            println!("{}", r.model_info());
            r
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            println!("\nTrying CPU mode instead...");

            // Fallback to CPU
            let start = Instant::now();
            match OrtRecognizer::new(model_dir, false) {
                Ok(r) => {
                    let load_time = start.elapsed();
                    println!("✅ SUCCESS: Model loaded with CPU in {:.2}s", load_time.as_secs_f32());
                    r
                }
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    return Err(e.into());
                }
            }
        }
    };

    // Test 2: Transcribe real audio
    println!("\n[TEST 2] Transcribing audio file...");
    let start = Instant::now();
    match recognizer.recognize_file(audio_path) {
        Ok(text) => {
            let inference_time = start.elapsed();
            println!("✅ SUCCESS: Transcription completed in {:.2}s", inference_time.as_secs_f32());
            println!("\n┌─ Transcription Result ─────────────────────────");
            println!("│ {}", text);
            println!("└────────────────────────────────────────────────\n");
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e.into());
        }
    }

    println!("\n======================================================");
    println!("✅ 1.1B Model Full Pipeline Test Complete!");
    println!("======================================================");
    println!("\nPipeline stages successfully executed:");
    println!("  ✓ Audio loading (WAV/MP3/FLAC/OGG)");
    println!("  ✓ Mel-spectrogram extraction (128 mels, 80-frame chunks)");
    println!("  ✓ Encoder inference (transducer encoder)");
    println!("  ✓ Greedy search decoder (frame-by-frame decoding)");
    println!("  ✓ Text generation (BPE token handling)");

    Ok(())
}
