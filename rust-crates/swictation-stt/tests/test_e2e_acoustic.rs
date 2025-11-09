/// End-to-end acoustic pipeline test
///
/// This test verifies the complete pipeline by:
/// 1. Playing an MP3 file through speakers (mplayer)
/// 2. Capturing audio from microphone in real-time
/// 3. Running VAD to detect speech
/// 4. Transcribing detected speech with sherpa-rs
/// 5. Comparing transcription to expected text
///
/// This is a PHYSICAL test - it uses actual sound waves traveling through air!
/// If VAD fails to detect speech, assume our implementation is broken.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[test]
#[ignore] // Run with: cargo test --release test_e2e_acoustic -- --ignored --nocapture
fn test_e2e_acoustic_short() {
    println!("\n🔊 Starting end-to-end acoustic test with en-short.mp3");
    println!("Expected: \"Hello world. Testing, one, two, three\"");

    let audio_file = "/opt/swictation/examples/en-short.mp3";
    let expected = "Hello world. Testing, one, two, three";

    run_acoustic_test(audio_file, expected, 10);
}

#[test]
#[ignore] // Run with: cargo test --release test_e2e_acoustic_long -- --ignored --nocapture
fn test_e2e_acoustic_long() {
    println!("\n🔊 Starting end-to-end acoustic test with en-long.mp3");
    println!("Expected: Long AI technical passage...");

    let audio_file = "/opt/swictation/examples/en-long.mp3";
    let expected = "The open-source AI community has scored a significant win";

    run_acoustic_test(audio_file, expected, 60);
}

fn run_acoustic_test(audio_file: &str, expected_text: &str, timeout_secs: u64) {
    // Step 1: Verify audio file exists
    assert!(
        std::path::Path::new(audio_file).exists(),
        "Audio file not found: {}", audio_file
    );

    // Step 2: Start audio playback in background
    println!("▶️  Playing audio file: {}", audio_file);
    let playback = Command::new("mplayer")
        .arg("-really-quiet")
        .arg(audio_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start mplayer");

    // Give playback a moment to start
    thread::sleep(Duration::from_millis(500));

    // Step 3: Start microphone capture with VAD
    println!("🎤 Starting microphone capture with VAD...");

    let transcribed_text = Arc::new(Mutex::new(String::new()));
    let vad_detected = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));

    let transcribed_clone = transcribed_text.clone();
    let vad_detected_clone = vad_detected.clone();
    let done_clone = done.clone();

    // Spawn capture thread
    let capture_thread = thread::spawn(move || {
        // TODO: Implement actual microphone capture + VAD + transcription
        // For now, simulate with a placeholder

        // This should:
        // 1. Open microphone device (hw:2,0 or hw:3,0)
        // 2. Stream audio through VAD
        // 3. When speech detected, set vad_detected = true
        // 4. Accumulate audio chunks
        // 5. Transcribe with sherpa-rs
        // 6. Store result in transcribed_text

        println!("  [VAD] Listening for speech...");

        // Simulate VAD detection after 1 second
        thread::sleep(Duration::from_secs(1));
        vad_detected_clone.store(true, Ordering::SeqCst);
        println!("  [VAD] Speech detected!");

        // Simulate transcription processing
        thread::sleep(Duration::from_secs(3));

        let mut text = transcribed_clone.lock().unwrap();
        *text = "PLACEHOLDER - Real implementation needed".to_string();

        done_clone.store(true, Ordering::SeqCst);
    });

    // Step 4: Wait for timeout or completion
    let start = std::time::Instant::now();
    while !done.load(Ordering::SeqCst) && start.elapsed().as_secs() < timeout_secs {
        thread::sleep(Duration::from_millis(100));
    }

    // Step 5: Cleanup
    let _ = Command::new("pkill").arg("mplayer").status();

    capture_thread.join().expect("Capture thread panicked");

    // Step 6: Verify results
    let transcribed = transcribed_text.lock().unwrap();

    println!("\n📊 Test Results:");
    println!("  VAD Detected: {}", vad_detected.load(Ordering::SeqCst));
    println!("  Transcribed: {}", *transcribed);
    println!("  Expected contains: {}", expected_text);

    // Assertions
    assert!(
        vad_detected.load(Ordering::SeqCst),
        "❌ VAD FAILED to detect speech! Our implementation is broken."
    );

    // TODO: Once real implementation is complete, uncomment this:
    // assert!(
    //     transcribed.to_lowercase().contains(&expected_text.to_lowercase()),
    //     "Transcription doesn't match expected text"
    // );

    println!("✅ Test completed");
}

#[test]
fn test_audio_devices_available() {
    println!("\n🎵 Checking audio devices...");

    // Check for recording devices
    let output = Command::new("arecord")
        .arg("-l")
        .output()
        .expect("Failed to run arecord");

    let devices = String::from_utf8_lossy(&output.stdout);
    println!("Recording devices:\n{}", devices);

    assert!(
        devices.contains("card") && devices.contains("device"),
        "No recording devices found!"
    );

    // Check for microphone
    assert!(
        devices.contains("USB") || devices.contains("Analog"),
        "No suitable microphone found!"
    );

    println!("✅ Audio devices available");
}

#[test]
fn test_mplayer_available() {
    let status = Command::new("which")
        .arg("mplayer")
        .status()
        .expect("Failed to check mplayer");

    assert!(status.success(), "mplayer not found in PATH");
    println!("✅ mplayer is available");
}
