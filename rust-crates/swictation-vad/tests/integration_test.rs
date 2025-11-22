//! Integration tests for VAD with real audio

use hound;
use swictation_vad::{VadConfig, VadDetector, VadResult};

#[test]
#[ignore = "Requires ONNX Runtime libraries and test audio files"]
fn test_vad_with_real_audio() {
    // Test with actual English voice sample (6.17 seconds)
    let test_file = "/tmp/en-short-16k.wav";

    // Configure VAD with more lenient settings and debug enabled
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .min_silence(0.3)
        .min_speech(0.1) // Lower minimum speech duration
        .threshold(0.3) // Lower threshold to catch more speech
        .debug();

    let mut vad = VadDetector::new(config).expect("Failed to create VAD");

    // Load audio file
    let mut reader = hound::WavReader::open(test_file).expect("Failed to open test file");
    let spec = reader.spec();

    assert_eq!(spec.sample_rate, 16000, "Test file must be 16kHz");

    // Load samples and convert to f32
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.expect("Failed to read sample") as f32 / 32768.0)
        .collect();

    println!(
        "Loaded {} samples ({:.2}s of audio)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Process audio in 0.5s chunks
    let chunk_size = 8000;
    let mut speech_detected = false;
    let mut total_speech_samples = 0;

    for chunk_start in (0..samples.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(samples.len());
        let chunk = &samples[chunk_start..chunk_end];

        match vad.process_audio(chunk).expect("VAD processing failed") {
            VadResult::Speech {
                start_sample,
                samples: seg_samples,
            } => {
                speech_detected = true;
                total_speech_samples += seg_samples.len();
                println!(
                    "Speech segment: {} samples at position {}",
                    seg_samples.len(),
                    start_sample
                );
            }
            VadResult::Silence => {}
        }
    }

    // Flush any remaining audio
    if let Some(VadResult::Speech {
        start_sample,
        samples: seg_samples,
    }) = vad.flush()
    {
        speech_detected = true;
        total_speech_samples += seg_samples.len();
        println!(
            "Flushed speech segment: {} samples at position {}",
            seg_samples.len(),
            start_sample
        );
    }

    println!(
        "Total speech detected: {:.2}s",
        total_speech_samples as f32 / 16000.0
    );

    // The test file should contain speech
    assert!(
        speech_detected,
        "VAD should detect speech in the test audio file"
    );
    assert!(
        total_speech_samples > 0,
        "Should have detected some speech samples"
    );
}

#[test]
#[ignore = "Requires ONNX Runtime libraries and VAD model files"]
fn test_vad_with_silence() {
    let config =
        VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx").threshold(0.5);

    let mut vad = VadDetector::new(config).expect("Failed to create VAD");

    // Generate 1 second of silence
    let silence: Vec<f32> = vec![0.0; 16000];

    // Process in 512-sample chunks
    for chunk in silence.chunks(512) {
        if chunk.len() == 512 {
            match vad.process_audio(chunk).expect("VAD processing failed") {
                VadResult::Speech { .. } => {
                    panic!("VAD incorrectly detected speech in silence");
                }
                VadResult::Silence => {}
            }
        }
    }

    // Flush should also return no speech
    assert!(
        vad.flush().is_none(),
        "Flush should return None for silence"
    );
}
