/// Analyze audio waveform in time windows to see what's actually captured

use hound::WavReader;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = "/tmp/acoustic_test.wav";

    println!("Analyzing: {}\n", path);

    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    let samples_i16: Vec<i16> = reader.samples::<i16>()
        .map(|s| s.unwrap())
        .collect();

    let samples_f32: Vec<f32> = samples_i16.iter()
        .map(|&s| s as f32 / 32768.0)
        .collect();

    // Analyze 1-second windows
    let sample_rate = spec.sample_rate as usize;
    let num_windows = (samples_f32.len() / sample_rate).min(10);

    println!("Time Window Analysis (1-second windows):");
    println!("{:=<80}", "");
    println!("{:<10} {:>12} {:>12} {:>12} {:>12}",
             "Time (s)", "RMS", "Peak", "Mean", "Non-Zero%");
    println!("{:-<80}", "");

    for i in 0..num_windows {
        let start = i * sample_rate;
        let end = (start + sample_rate).min(samples_f32.len());
        let window = &samples_f32[start..end];

        // Calculate RMS
        let rms = (window.iter().map(|&s| s * s).sum::<f32>() / window.len() as f32).sqrt();

        // Calculate peak
        let peak = window.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);

        // Calculate mean (DC offset)
        let mean = window.iter().sum::<f32>() / window.len() as f32;

        // Count non-zero samples (> 1% amplitude)
        let non_zero = window.iter().filter(|&&s| s.abs() > 0.01).count();
        let non_zero_pct = (non_zero as f32 / window.len() as f32) * 100.0;

        println!("{:<10} {:>12.6} {:>12.6} {:>12.6} {:>11.1}%",
                 format!("{}-{}", i, i+1),
                 rms, peak, mean, non_zero_pct);
    }

    println!("{:=<80}\n", "");

    // Find where speech should be (t=1-3s based on "Hello world. Testing, one, two, three")
    println!("Expected Speech Regions:");
    println!("  t=1-2s: 'Hello world'");
    println!("  t=3-4s: 'Testing, one, two, three'\n");

    // Detailed analysis of speech windows
    analyze_window(&samples_f32, sample_rate, 0, 1, "Pre-speech (silence expected)");
    analyze_window(&samples_f32, sample_rate, 1, 2, "SPEECH: 'Hello world'");
    analyze_window(&samples_f32, sample_rate, 2, 3, "Pause between phrases");
    analyze_window(&samples_f32, sample_rate, 3, 4, "SPEECH: 'Testing, one, two, three'");
    analyze_window(&samples_f32, sample_rate, 4, 5, "Post-speech (silence expected)");

    Ok(())
}

fn analyze_window(samples: &[f32], sample_rate: usize, start_sec: usize, end_sec: usize, label: &str) {
    let start_idx = start_sec * sample_rate;
    let end_idx = (end_sec * sample_rate).min(samples.len());

    if start_idx >= samples.len() {
        return;
    }

    let window = &samples[start_idx..end_idx];

    let rms = (window.iter().map(|&s| s * s).sum::<f32>() / window.len() as f32).sqrt();
    let peak = window.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
    let mean = window.iter().sum::<f32>() / window.len() as f32;

    // Find peak location within window
    let peak_idx = window.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    let peak_time = start_sec as f32 + (peak_idx as f32 / sample_rate as f32);

    println!("{}:", label);
    println!("  Time range: {:.1}s - {:.1}s", start_sec, end_sec);
    println!("  RMS: {:.6} ({:.2} dB)", rms, 20.0 * rms.log10());
    println!("  Peak amplitude: {:.6} at t={:.2}s", peak, peak_time);
    println!("  DC offset: {:.6}", mean);

    // Sample distribution
    let loud_samples = window.iter().filter(|&&s| s.abs() > 0.1).count();
    let medium_samples = window.iter().filter(|&&s| s.abs() > 0.01 && s.abs() <= 0.1).count();
    let quiet_samples = window.len() - loud_samples - medium_samples;

    println!("  Sample distribution:");
    println!("    Loud (>0.1):     {:6} ({:5.1}%)", loud_samples, loud_samples as f32 / window.len() as f32 * 100.0);
    println!("    Medium (0.01-0.1): {:6} ({:5.1}%)", medium_samples, medium_samples as f32 / window.len() as f32 * 100.0);
    println!("    Quiet (<0.01):   {:6} ({:5.1}%)", quiet_samples, quiet_samples as f32 / window.len() as f32 * 100.0);

    // Show a few sample values
    println!("  Sample values (first 10): {:?}", &window[..10.min(window.len())]);
    println!();
}
