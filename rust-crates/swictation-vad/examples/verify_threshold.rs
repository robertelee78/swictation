//! Verify the default threshold (0.003) is working correctly

use swictation_vad::{VadConfig, VadDetector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Verifying Default Threshold Configuration ===\n");

    // Create config with default threshold
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .min_silence(0.5)
        .min_speech(0.25)
        .debug();

    println!("Config created:");
    println!("  Threshold: {}", config.threshold);
    println!("  Expected: 0.003\n");

    if (config.threshold - 0.003).abs() < 0.0001 {
        println!("✅ Default threshold is correctly set to 0.003");
    } else {
        println!("❌ Default threshold is {}, expected 0.003", config.threshold);
        return Err("Incorrect default threshold".into());
    }

    // Initialize VAD to ensure it works
    println!("\nInitializing VAD...");
    let mut vad = VadDetector::new(config)?;
    println!("✅ VAD initialized successfully\n");

    // Test with a simple signal
    println!("Testing with 440Hz sine wave (512 samples):");
    let sample_rate = 16000.0;
    let frequency = 440.0;
    let sine: Vec<f32> = (0..512)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
        })
        .collect();

    let result = vad.process_audio(&sine)?;
    println!("  Result: {:?}\n", result);

    println!("✅ All threshold verification tests passed!");
    println!("\nThe VAD is now configured with ONNX-appropriate threshold (0.003)");
    println!("This is ~166x lower than PyTorch default (0.5)");

    Ok(())
}
