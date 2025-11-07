//! Test VAD with real audio files

use swictation_vad::{VadConfig, VadDetector, VadResult};
use hound;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Swictation VAD Real Audio Test ===\n");

    let test_files = [
        ("/tmp/en-short-16k.wav", "Short sample (6s)"),
        ("/tmp/en-long-16k.wav", "Long sample (84s)"),
    ];

    // Configure VAD
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .min_silence(0.5)
        .min_speech(0.25)
        .max_speech(30.0)
        .threshold(0.5);

    println!("Creating VAD detector...");
    let mut vad = VadDetector::new(config)?;
    println!("✓ VAD initialized\n");

    for (path, description) in &test_files {
        println!("=== {} ===", description);
        println!("File: {}", path);

        // Load audio file
        let mut reader = match hound::WavReader::open(path) {
            Ok(r) => r,
            Err(_) => {
                println!("⚠ File not found, skipping\n");
                continue;
            }
        };

        let spec = reader.spec();
        println!("  Sample rate: {} Hz", spec.sample_rate);
        println!("  Channels: {}", spec.channels);
        println!("  Duration: {:.2}s", reader.duration() as f32 / spec.sample_rate as f32);

        if spec.sample_rate != 16000 {
            println!("✗ Sample rate must be 16000 Hz\n");
            continue;
        }

        // Load samples
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();

        // Process audio in chunks
        println!("\nProcessing audio...");
        let chunk_size = 8000; // 0.5 seconds
        let mut speech_segments = 0;
        let mut total_speech_duration = 0.0;

        let start = std::time::Instant::now();

        for chunk_start in (0..samples.len()).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size).min(samples.len());
            let chunk = &samples[chunk_start..chunk_end];

            let result = vad.process_audio(chunk)?;
            match result {
                VadResult::Speech {
                    start_sample,
                    samples: seg_samples,
                } => {
                    speech_segments += 1;
                    let duration = seg_samples.len() as f32 / 16000.0;
                    total_speech_duration += duration;
                    println!(
                        "  Segment {}: {:.2}s speech at sample {}",
                        speech_segments, duration, start_sample
                    );
                }
                VadResult::Silence => {}
            }
        }

        // Flush remaining
        vad.flush();

        // Check for any remaining segments
        loop {
            let result = vad.process_audio(&[])?;
            match result {
                VadResult::Speech {
                    start_sample,
                    samples: seg_samples,
                } => {
                    speech_segments += 1;
                    let duration = seg_samples.len() as f32 / 16000.0;
                    total_speech_duration += duration;
                    println!(
                        "  Segment {}: {:.2}s speech at sample {} (from flush)",
                        speech_segments, duration, start_sample
                    );
                }
                VadResult::Silence => break,
            }
        }

        let elapsed = start.elapsed().as_millis();

        println!("\nResults:");
        println!("  Speech segments detected: {}", speech_segments);
        println!("  Total speech duration: {:.2}s", total_speech_duration);
        println!("  Processing time: {}ms", elapsed);
        println!(
            "  Real-time factor: {:.1}x",
            (samples.len() as f64 / 16000.0) / (elapsed as f64 / 1000.0)
        );

        // Reset for next file
        vad.clear();
        println!();
    }

    println!("✓ All tests completed!");

    Ok(())
}
