//! Test audio recording for 5 seconds

use std::thread;
use std::time::Duration;
use swictation_audio::{AudioCapture, AudioConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Audio Capture Test ===\n");

    // List available devices
    AudioCapture::print_devices()?;

    // Create audio capture with default config
    let config = AudioConfig {
        sample_rate: 16000,
        channels: 1,
        blocksize: 1024,
        buffer_duration: 10.0,
        device_index: None, // Use default device
        streaming_mode: false,
        chunk_duration: 1.0,
    };

    let mut capture = AudioCapture::new(config)?;

    println!("\n=== Starting 5-second recording ===");
    capture.start()?;

    // Record for 5 seconds
    thread::sleep(Duration::from_secs(5));

    let audio = capture.stop()?;

    // Analyze audio
    if audio.is_empty() {
        println!("\n⚠️  Warning: No audio captured!");
    } else {
        let rms: f32 = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        let db = 20.0 * rms.log10();
        let peak = audio.iter().fold(0.0f32, |a, &b| a.max(b.abs()));

        println!("\n=== Audio Analysis ===");
        println!("Samples: {}", audio.len());
        println!("Duration: {:.2}s", audio.len() as f32 / 16000.0);
        println!("RMS: {:.6}", rms);
        println!("dB: {:.2}", db);
        println!("Peak: {:.6}", peak);

        if rms < 0.001 {
            println!("\n⚠️  Warning: Audio level very low (RMS < 0.001)");
            println!("   This might indicate no audio input or very quiet source");
        } else {
            println!("\n✓ Audio captured successfully!");
        }
    }

    Ok(())
}
