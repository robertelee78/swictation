//! List all available audio input devices

use swictation_audio::AudioCapture;

fn main() {
    println!("\n=== Audio Input Devices ===\n");

    match AudioCapture::list_devices() {
        Ok(devices) => {
            for device in devices {
                println!("Device {}: {}", device.index, device.name);
                println!(
                    "  Channels: {} input, {} output",
                    device.max_input_channels, device.max_output_channels
                );
                println!("  Sample Rate: {} Hz", device.default_sample_rate);
                if device.is_default {
                    println!("  [DEFAULT INPUT DEVICE]");
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("Error listing devices: {}", e);
        }
    }
}
