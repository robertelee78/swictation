//! Minimal live audio level test - just grab audio and print amplitude
//! Usage: cargo run --release --example test_live_audio [device_index]

use swictation_audio::{AudioCapture, AudioConfig};
use std::thread;
use std::time::Duration;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Live Audio Level Test ===\n");

    // List available devices
    println!("Available devices:");
    AudioCapture::print_devices()?;

    // Get device index from command line or use default
    let device_index = env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok());

    if let Some(idx) = device_index {
        println!("\n🎤 Testing device index: {}", idx);
    } else {
        println!("\n🎤 Testing default device (use 'test_live_audio <index>' to test specific device)");
    }

    // Create audio capture with minimal config
    let config = AudioConfig {
        sample_rate: 16000,
        channels: 1,
        blocksize: 1024,
        buffer_duration: 10.0,
        device_index,
        streaming_mode: false,
        chunk_duration: 1.0,
    };

    let mut capture = AudioCapture::new(config)?;

    println!("\n▶️  Recording for 10 seconds...");
    println!("    (Play audio through speakers NOW to test)\n");

    capture.start()?;

    // Sample audio every 0.5 seconds and print levels
    for i in 1..=20 {
        thread::sleep(Duration::from_millis(500));

        let buffer = capture.get_buffer();
        if !buffer.is_empty() {
            let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
            let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

            // Visual bar graph
            let bars = (peak * 50.0) as usize;
            let bar = "█".repeat(bars.min(50));

            println!("[{:2}s] Peak: {:.4}  RMS: {:.4}  {}",
                     i as f32 * 0.5, peak, rms, bar);
        }
    }

    let audio = capture.stop()?;

    // Final analysis
    println!("\n=== Final Analysis ===");
    if audio.is_empty() {
        println!("❌ No audio captured!");
        return Ok(());
    }

    let peak = audio.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    let db = 20.0 * rms.log10();

    println!("Samples: {}", audio.len());
    println!("Duration: {:.2}s", audio.len() as f32 / 16000.0);
    println!("Peak: {:.4}", peak);
    println!("RMS: {:.6}", rms);
    println!("dB: {:.2}", db);

    if peak < 0.01 {
        println!("\n⚠️  Audio level VERY LOW (peak < 0.01)");
        println!("   - Microphone may be muted or wrong device selected");
        println!("   - Try testing other device indices");
    } else if peak < 0.1 {
        println!("\n⚠️  Audio level LOW (peak < 0.1)");
        println!("   - Increase speaker volume");
        println!("   - Move microphone closer to speakers");
    } else {
        println!("\n✅ Audio level GOOD (peak >= 0.1)");
        println!("   - This device is working correctly!");
    }

    Ok(())
}
