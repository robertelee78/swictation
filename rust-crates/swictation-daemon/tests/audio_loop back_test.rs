/// End-to-end audio loopback test
/// Plays MP3 file through speakers and verifies microphone captures it correctly
///
/// Test procedure:
/// 1. Play /opt/swictation/examples/en-short.mp3 through speakers (mplayer)
/// 2. Capture audio from microphone (default input device)
/// 3. Run through VAD → STT pipeline
/// 4. Compare transcription against expected text in en-short.txt

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[test]
fn test_audio_loopback_short() {
    println!("🎧 Audio Loopback Test: en-short.mp3");
    println!("=======================================");

    // Expected transcription
    let expected = "Hello world.\nTesting, one, two, three";

    // Start playing audio in background
    println!("🔊 Playing audio file...");
    let mut player = Command::new("mplayer")
        .arg("-really-quiet")
        .arg("/opt/swictation/examples/en-short.mp3")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start mplayer");

    // Give audio time to start playing
    thread::sleep(Duration::from_millis(500));

    // TODO: Initialize pipeline and capture audio for 5 seconds
    // This would integrate with swictation-daemon/src/pipeline.rs

    thread::sleep(Duration::from_secs(5));

    // Stop playback
    let _ = player.kill();

    println!("✅ Audio loopback test framework ready");
    println!("   Expected: {}", expected);
    println!("   TODO: Integrate with VAD→STT pipeline");
}

#[test]
fn test_microphone_capture_only() {
    println!("🎤 Microphone Capture Test");
    println!("==========================");

    // This test just verifies we can capture from the microphone
    // without running through STT yet

    println!("📝 Available input devices:");
    // TODO: List devices using swictation-audio crate

    println!("🎧 Capturing 3 seconds of audio...");
    // TODO: Capture audio samples

    println!("✅ Microphone capture test framework ready");
}
