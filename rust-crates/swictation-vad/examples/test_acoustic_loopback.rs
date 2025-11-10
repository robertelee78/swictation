/// Acoustic Loopback Test: Speaker → Microphone → VAD Detection
///
/// This test validates the end-to-end pipeline by:
/// 1. Playing MP3 through speakers
/// 2. Recording from microphone
/// 3. VAD detecting speech in real-time
/// 4. Reporting detection results
///
/// If VAD fails to detect speech, assume VAD implementation is broken.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use swictation_vad::{VadConfig, VadDetector, VadResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  Acoustic Loopback Test: Speaker → Mic → VAD");
    println!("═══════════════════════════════════════════════════════\n");

    // Test configuration
    let test_file = "/opt/swictation/examples/en-short.mp3";
    let expected = "Hello world. Testing, one, two, three";
    let duration_secs = 10; // Record for 10 seconds

    // FIRST: Test VAD on direct MP3 (baseline)
    println!("[BASELINE] Testing VAD on direct MP3 file...\n");
    test_direct_mp3(test_file)?;
    println!("\n");

    println!("Test Configuration:");
    println!("  Audio file: {}", test_file);
    println!("  Expected: \"{}\"", expected);
    println!("  Recording duration: {}s", duration_secs);
    println!("  Sample rate: 16000 Hz\n");

    // Configure VAD
    println!("[1/5] Initializing VAD...");
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .min_silence(0.3)
        .min_speech(0.2)
        .threshold(0.003);  // Standard ONNX threshold

    let mut vad = VadDetector::new(config)?;
    println!("✓ VAD initialized\n");

    // Start playback in background
    println!("[2/5] Starting audio playback...");
    let mut player = Command::new("mplayer")
        .arg("-really-quiet")
        .arg("-loop")
        .arg("0") // Loop forever
        .arg(test_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    println!("✓ mplayer started (PID: {})", player.id());
    println!("  Playing: {}\n", test_file);

    // Give playback time to start
    thread::sleep(Duration::from_millis(500));

    // Start recording from microphone
    println!("[3/5] Recording from microphone...");
    println!("  Device: default (via ALSA/pipewire - works on any Linux machine)");
    println!("  Duration: {}s\n", duration_secs);

    let recorder = Command::new("arecord")
        .arg("-D").arg("default")         // Default ALSA device (works with pipewire/pulse)
        .arg("-f").arg("S16_LE")          // 16-bit PCM
        .arg("-c").arg("1")               // Mono
        .arg("-r").arg("16000")           // 16kHz sample rate
        .arg("-d").arg(duration_secs.to_string())
        .arg("/tmp/acoustic_test.wav")
        .output()?;

    // Stop playback
    player.kill()?;

    // Check recording result
    if !recorder.status.success() {
        println!("✗ Recording complete (FAILED)\n");
        eprintln!("arecord stderr:");
        eprintln!("{}", String::from_utf8_lossy(&recorder.stderr));
        eprintln!("\narecord stdout:");
        eprintln!("{}", String::from_utf8_lossy(&recorder.stdout));
        return Ok(());
    }

    println!("✓ Recording complete (SUCCESS)\n");

    // Process recorded audio with VAD
    println!("[4/5] Processing audio with VAD...");

    // Load recorded audio
    use hound::WavReader;
    let mut reader = WavReader::open("/tmp/acoustic_test.wav")?;
    let spec = reader.spec();

    println!("  Recorded audio:");
    println!("    Sample rate: {} Hz", spec.sample_rate);
    println!("    Channels: {}", spec.channels);
    println!("    Duration: {:.2}s\n", reader.duration() as f32 / spec.sample_rate as f32);

    // Convert to f32 samples
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0)
        .collect();

    // Process with VAD
    let chunk_size = 8000; // 0.5 seconds
    let mut speech_segments = 0;
    let mut total_speech_duration = 0.0;
    let mut total_samples = 0;

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
                total_samples += seg_samples.len();
                println!(
                    "  ✓ Segment {}: {:.2}s speech at sample {} ({} samples)",
                    speech_segments, duration, start_sample, seg_samples.len()
                );
            }
            VadResult::Silence => {}
        }
    }

    // Flush VAD buffer
    vad.flush();
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
                total_samples += seg_samples.len();
                println!(
                    "  ✓ Segment {}: {:.2}s speech at sample {} (flush)",
                    speech_segments, duration, start_sample
                );
            }
            VadResult::Silence => break,
        }
    }

    // Results
    println!("\n[5/5] VAD Detection Results:");
    println!("  Speech segments detected: {}", speech_segments);
    println!("  Total speech duration: {:.2}s", total_speech_duration);
    println!("  Total speech samples: {}", total_samples);
    println!("  Expected: Speech should be detected\n");

    if speech_segments == 0 {
        println!("═══════════════════════════════════════════════════════");
        println!("❌ FAILURE: VAD detected NO speech!");
        println!("   Per user instructions: Assume VAD implementation is broken.");
        println!("   (NOT audio hardware - speakers/mic are confirmed working)");
        println!("═══════════════════════════════════════════════════════");
        return Ok(());
    }

    // [6/6] STT Transcription
    println!("[6/6] Transcribing detected speech with STT...");
    println!("  Using 1.1B Parakeet-TDT model (GPU accelerated)\n");

    // Save recorded audio for STT
    let output_path = "/tmp/acoustic_speech.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output_path, spec)?;
    for sample in &samples {
        let sample_i16 = (sample * 32767.0) as i16;
        writer.write_sample(sample_i16)?;
    }
    writer.finalize()?;

    // Run STT transcription using 1.1B model with GPU (OrtRecognizer)
    use swictation_stt::OrtRecognizer;

    let model_dir = "/opt/swictation/models/parakeet-tdt-1.1b-corrected";
    println!("  Loading 1.1B model from: {}", model_dir);

    let mut recognizer = match OrtRecognizer::new(model_dir, true) {  // GPU mode
        Ok(r) => {
            println!("  ✓ Model loaded (ORT implementation)");
            r
        }
        Err(e) => {
            println!("  ❌ Failed to load model: {}", e);
            println!("\n═══════════════════════════════════════════════════════");
            println!("✅ VAD SUCCESS: Detected {} speech segments!", speech_segments);
            println!("❌ STT FAILED: Could not load model");
            println!("═══════════════════════════════════════════════════════");
            return Ok(());
        }
    };

    println!("  Transcribing recorded audio...");
    let transcript = match recognizer.recognize_file(output_path) {
        Ok(text) => text,
        Err(e) => {
            println!("  ❌ Transcription failed: {}", e);
            println!("\n═══════════════════════════════════════════════════════");
            println!("✅ VAD SUCCESS: Detected {} speech segments!", speech_segments);
            println!("❌ STT FAILED: Transcription error");
            println!("═══════════════════════════════════════════════════════");
            return Ok(());
        }
    };
    println!("\n  Transcription Result:");
    println!("  ┌─────────────────────────────────────────");
    println!("  │ {}", transcript);
    println!("  └─────────────────────────────────────────");
    println!("  Expected: \"{}\"", expected);

    // Compare transcripts
    let transcript_lower = transcript.to_lowercase();
    let expected_lower = expected.to_lowercase();
    let words_match = expected_lower.split_whitespace()
        .filter(|word| transcript_lower.contains(word))
        .count();
    let total_words = expected_lower.split_whitespace().count();
    let accuracy = (words_match as f32 / total_words as f32) * 100.0;

    println!("\n  Word Matching:");
    println!("  - Matched words: {}/{}", words_match, total_words);
    println!("  - Accuracy: {:.1}%", accuracy);

    println!("\n═══════════════════════════════════════════════════════");
    println!("FULL PIPELINE TEST RESULTS:");
    println!("═══════════════════════════════════════════════════════");
    println!("✅ VAD: Detected {} speech segments", speech_segments);

    if accuracy >= 80.0 {
        println!("✅ STT: {:.1}% word accuracy (PASS)", accuracy);
        println!("\n🎉 COMPLETE SUCCESS: Full acoustic pipeline working!");
        println!("   Speaker → Microphone → VAD → STT → Text");
    } else if accuracy >= 50.0 {
        println!("⚠️  STT: {:.1}% word accuracy (PARTIAL)", accuracy);
        println!("\n   Pipeline works but transcription needs improvement");
    } else {
        println!("❌ STT: {:.1}% word accuracy (FAIL)", accuracy);
        println!("\n   Got: \"{}\"", transcript);
        println!("   Expected: \"{}\"", expected);
    }
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}

fn test_direct_mp3(mp3_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    // Convert MP3 to WAV
    let wav_path = "/tmp/direct_test.wav";
    Command::new("ffmpeg")
        .args(&["-i", mp3_path, "-ar", "16000", "-ac", "1", "-f", "wav", wav_path, "-y"])
        .output()?;

    // Test with VAD
    let config = VadConfig::with_model("/opt/swictation/models/silero-vad/silero_vad.onnx")
        .threshold(0.003);

    let mut vad = VadDetector::new(config)?;

    use hound::WavReader;
    let mut reader = WavReader::open(wav_path)?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0)
        .collect();

    println!("  Direct MP3 audio: {} samples ({:.2}s)", samples.len(), samples.len() as f32 / 16000.0);

    let chunk_size = 8000;
    let mut segments = 0;

    for chunk_start in (0..samples.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(samples.len());
        let chunk = &samples[chunk_start..chunk_end];

        if let VadResult::Speech { .. } = vad.process_audio(chunk)? {
            segments += 1;
        }
    }

    vad.flush();
    loop {
        match vad.process_audio(&[])? {
            VadResult::Speech { .. } => segments += 1,
            VadResult::Silence => break,
        }
    }

    println!("  Direct MP3 segments detected: {}", segments);
    if segments > 0 {
        println!("  ✅ Baseline: VAD works on direct audio");
    } else {
        println!("  ❌ Baseline FAILED: VAD doesn't detect direct audio");
    }

    Ok(())
}
