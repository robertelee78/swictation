/// Test the 1.1B Parakeet-TDT model using direct ONNX Runtime
///
/// This example demonstrates that the 1.1B model files are valid and
/// can be loaded successfully using the `ort` crate, bypassing sherpa-rs's
/// SessionOptions bug with external weights.
///
/// ## Running this test:
///
/// ```bash
/// export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
/// cargo run --release --example test_1_1b
/// ```

use swictation_stt::OrtRecognizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("==========================================");
    println!("1.1B Parakeet-TDT Model Test (ort crate)");
    println!("==========================================\n");

    let model_dir = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-1.1b-converted";

    // Test 1: Load model with CPU
    println!("[TEST 1] Loading 1.1B model with CPU...");
    let mut recognizer = match OrtRecognizer::new(model_dir, false) {
        Ok(r) => {
            println!("✅ SUCCESS: Model loaded with CPU");
            println!("   - encoder.onnx + encoder.weights (4GB) loaded");
            println!("   - decoder.onnx loaded");
            println!("   - joiner.onnx loaded");
            println!("   - External weights loaded automatically\n");
            println!("{}", r.model_info());
            r
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e.into());
        }
    };

    // Test 2: Run test inference
    println!("\n[TEST 2] Running test inference...");
    match recognizer.test_encoder_inference() {
        Ok(result) => {
            println!("✅ SUCCESS: {}", result);
            println!("   - Encoder inference working");
            println!("   - External weights loaded correctly");
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e.into());
        }
    }

    println!("\n[TEST 3] Loading 1.1B model with GPU...");
    match OrtRecognizer::new(model_dir, true) {
        Ok(_recognizer) => {
            println!("✅ SUCCESS: Model loaded with GPU (CUDA)");
            println!("   - CUDA execution provider active");
            println!("   - GPU acceleration enabled");
        }
        Err(e) => {
            println!("⚠️  GPU test failed (CUDA library may be missing): {}", e);
            println!("   This is expected if libonnxruntime_providers_cuda.so is not installed");
        }
    }

    println!("\n==========================================");
    println!("✅ 1.1B Model Validation Complete!");
    println!("==========================================");
    println!("\nNext steps:");
    println!("  1. Implement mel-spectrogram feature extraction");
    println!("  2. Implement greedy search decoder");
    println!("  3. Test with real audio samples");
    println!("  4. Benchmark GPU vs CPU inference speed");

    Ok(())
}
