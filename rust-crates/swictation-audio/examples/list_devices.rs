//! List available audio devices

use swictation_audio::AudioCapture;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Audio Devices on System:");
    AudioCapture::print_devices()?;
    Ok(())
}
