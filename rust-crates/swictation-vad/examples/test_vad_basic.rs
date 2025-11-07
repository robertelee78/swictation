//! Basic VAD test with synthetic audio

use swictation_vad::{VadConfig, VadDetector, VadResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Swictation VAD Basic Test ===\n");

    // Configure VAD
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .min_silence(0.3)
        .min_speech(0.25)
        .threshold(0.5)
        .debug();

    println!("Creating VAD detector...");
    let mut vad = VadDetector::new(config)?;
    println!("✓ VAD initialized\n");

    println!("Configuration:");
    println!("  Model: {}", vad.config().model_path);
    println!("  Sample rate: {} Hz", vad.config().sample_rate);
    println!("  Window size: {} samples", vad.config().window_size);
    println!("  Min silence: {}s", vad.config().min_silence_duration);
    println!("  Min speech: {}s", vad.config().min_speech_duration);
    println!("  Threshold: {}", vad.config().threshold);
    println!();

    // Test 1: Silence (zeros)
    println!("Test 1: Silence (1 second of zeros)");
    let silence: Vec<f32> = vec![0.0; 16000]; // 1 second
    let result = vad.process_audio(&silence)?;
    match result {
        VadResult::Silence => println!("✓ Correctly detected silence\n"),
        VadResult::Speech { .. } => println!("✗ False positive: detected speech in silence\n"),
    }

    // Test 2: Simulated speech (sine wave with varying amplitude)
    println!("Test 2: Simulated speech (sine wave, 0.5 seconds)");
    let mut speech: Vec<f32> = Vec::new();
    for i in 0..8000 {
        // 0.5 seconds
        let t = i as f32 / 16000.0;
        let freq = 440.0; // A4 note
        let amplitude = 0.3 * (1.0 + (t * 3.0).sin()) / 2.0; // Varying amplitude
        speech.push(amplitude * (2.0 * std::f32::consts::PI * freq * t).sin());
    }

    let result = vad.process_audio(&speech)?;
    match result {
        VadResult::Speech {
            start_sample,
            samples,
        } => {
            println!("✓ Speech detected!");
            println!("  Start sample: {}", start_sample);
            println!("  Segment length: {} samples ({:.2}s)", samples.len(), samples.len() as f32 / 16000.0);
            println!();
        }
        VadResult::Silence => println!("✗ Failed to detect simulated speech\n"),
    }

    // Test 3: Mixed audio (silence -> speech -> silence)
    println!("Test 3: Mixed audio (silence -> speech -> silence)");
    let mut mixed = vec![0.0; 8000]; // 0.5s silence
    mixed.extend(speech.clone()); // 0.5s speech
    mixed.extend(vec![0.0; 8000]); // 0.5s silence

    println!("Processing 1.5 seconds of mixed audio...");
    let result = vad.process_audio(&mixed)?;
    match result {
        VadResult::Speech {
            start_sample,
            samples,
        } => {
            println!("✓ Speech segment extracted from mixed audio");
            println!("  Start sample: {}", start_sample);
            println!("  Segment length: {} samples ({:.2}s)", samples.len(), samples.len() as f32 / 16000.0);
            println!();
        }
        VadResult::Silence => println!("⚠ No speech detected in mixed audio\n"),
    }

    // Flush remaining audio
    vad.flush();

    println!("Statistics:");
    println!("  Total samples processed: {}", vad.samples_processed());
    println!("  Processing time: {:.2}s", vad.processing_time_seconds());
    println!("\n✓ All tests completed successfully!");

    Ok(())
}
