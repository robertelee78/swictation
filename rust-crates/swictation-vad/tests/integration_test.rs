//! Integration tests for VAD with real audio

use swictation_vad::{VadConfig, VadDetector, VadResult};

/// Locate the installed Silero VAD model, or `None` if it is not on this machine.
///
/// Model-backed tests are `#[ignore]`d by default; run them with
/// `ORT_DYLIB_PATH=<libonnxruntime> cargo test -p swictation-vad -- --ignored`.
fn installed_model_path() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(format!(
            "{home}/Library/Application Support/swictation/models/silero-vad/silero_vad.onnx"
        ));
        candidates.push(format!(
            "{home}/.local/share/swictation/models/silero-vad/silero_vad.onnx"
        ));
    }
    candidates.push("/opt/swictation/models/silero-vad/silero_vad.onnx".to_string());

    candidates
        .into_iter()
        .find(|path| std::path::Path::new(path).exists())
}

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

/// One `process_audio` call can complete several segments, and only the oldest
/// is returned — the rest are queued.
///
/// This is what makes the queue load-bearing rather than theoretical: a caller
/// that forwards only the returned value drops real dictation. `reset()` must
/// empty that queue, so a segment finished in one session can never be handed
/// to the next one.
///
/// `threshold(0.0)` makes every window count as speech and a one-window
/// `max_speech` caps every segment at one window, so the split points are
/// deterministic and the audio content is irrelevant.
#[test]
#[ignore = "Requires ONNX Runtime (ORT_DYLIB_PATH) and the installed Silero VAD model"]
fn one_call_queues_the_segments_it_cannot_return_and_reset_clears_them() {
    let model = match installed_model_path() {
        Some(path) => path,
        None => panic!("Silero VAD model not installed - cannot run model-backed test"),
    };

    const WINDOW: usize = 512;
    const FED_WINDOWS: usize = 8;

    let config = VadConfig::with_model(model)
        .threshold(0.0)
        .max_speech(WINDOW as f32 / 16000.0)
        .min_speech(0.001)
        .min_silence(0.5);

    let mut vad = VadDetector::new(config).expect("Failed to create VAD");

    let samples: Vec<f32> = (0..WINDOW * FED_WINDOWS)
        .map(|i| (i as f32 * 0.08).sin() * 0.3)
        .collect();

    // A single call, covering every window.
    let returned = vad.process_audio(&samples).expect("VAD processing failed");
    let VadResult::Speech {
        samples: first_segment,
        ..
    } = returned
    else {
        panic!("the call must return the oldest completed segment, got silence");
    };

    assert_eq!(
        vad.pending_count(),
        FED_WINDOWS - 1,
        "every segment past the first must be queued, not discarded"
    );

    let queued = vad.drain_pending();
    let mut every_sample: Vec<f32> = first_segment;
    for segment in queued {
        match segment {
            VadResult::Speech {
                samples: seg_samples,
                ..
            } => every_sample.extend(seg_samples),
            VadResult::Silence => panic!("only speech segments are ever queued"),
        }
    }
    assert_eq!(
        every_sample, samples,
        "returned segment followed by the queued ones must reproduce the input in order"
    );

    // Queue a second batch, then prove a reset drops it.
    vad.process_audio(&samples).expect("VAD processing failed");
    assert!(
        vad.pending_count() > 0,
        "test setup: the second call must leave segments queued"
    );
    vad.reset();
    assert_eq!(
        vad.pending_count(),
        0,
        "reset must clear the queue so no segment leaks into the next session"
    );
    assert!(
        vad.drain_pending().is_empty(),
        "reset must clear the queue so no segment leaks into the next session"
    );
}

/// A speaker who never pauses must still produce bounded segments.
///
/// `max_speech_duration` is documented as "segments longer than this are split",
/// so continuous speech has to be force-emitted at the cap rather than growing a
/// single unbounded buffer that only reaches STT when the stream ends.
///
/// `threshold(0.0)` makes every window count as speech, so the audio content is
/// irrelevant and the segment boundaries are deterministic.
#[test]
#[ignore = "Requires ONNX Runtime (ORT_DYLIB_PATH) and the installed Silero VAD model"]
fn test_continuous_speech_is_split_at_max_speech() {
    let model = match installed_model_path() {
        Some(path) => path,
        None => panic!("Silero VAD model not installed - cannot run model-backed test"),
    };

    const MAX_SPEECH_SECS: f32 = 1.0;
    const WINDOW: usize = 512;
    // A whole number of windows (5.12s) so no tail sample is left in the
    // partial-window carry buffer and the totals can be compared exactly.
    const TOTAL_SAMPLES: usize = WINDOW * 160;

    let config = VadConfig::with_model(model)
        .threshold(0.0) // every window is speech: no silence gap can end a segment
        .max_speech(MAX_SPEECH_SECS)
        .min_speech(0.1)
        .min_silence(0.5);

    let mut vad = VadDetector::new(config).expect("Failed to create VAD");

    let samples: Vec<f32> = (0..TOTAL_SAMPLES)
        .map(|i| (i as f32 * 0.08).sin() * 0.3)
        .collect();

    let mut segments: Vec<Vec<f32>> = Vec::new();
    for chunk in samples.chunks(WINDOW) {
        if let VadResult::Speech { samples: seg, .. } =
            vad.process_audio(chunk).expect("VAD processing failed")
        {
            segments.push(seg);
        }
    }
    if let Some(VadResult::Speech { samples: seg, .. }) = vad.flush() {
        segments.push(seg);
    }

    let lengths: Vec<usize> = segments.iter().map(Vec::len).collect();
    let cap = (MAX_SPEECH_SECS * 16000.0) as usize;
    println!("emitted {} segments: {lengths:?}", lengths.len());

    assert!(
        segments.len() >= 2,
        "continuous speech longer than the {MAX_SPEECH_SECS}s cap must yield >=2 segments, got {lengths:?}"
    );
    for len in &lengths {
        assert!(
            *len <= cap + 2 * WINDOW,
            "segment of {len} samples exceeds the {cap}-sample cap (+pre-roll): {lengths:?}"
        );
    }
    assert_eq!(
        lengths.iter().sum::<usize>(),
        TOTAL_SAMPLES,
        "splitting must preserve every sample: {lengths:?}"
    );
}
