/// Diagnose audio amplitude issues in recorded files
///
/// This tool analyzes WAV files to check for:
/// - Sample amplitude ranges
/// - RMS energy levels
/// - DC offset
/// - Clipping
/// - Whether audio is too quiet for VAD

use hound::WavReader;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let files = vec![
        ("/tmp/acoustic_test.wav", "Acoustic loopback recording"),
        ("/tmp/direct_test.wav", "Direct MP3 conversion"),
    ];

    for (path, description) in files {
        println!("\n══════════════════════════════════════════════════════════");
        println!("Analyzing: {}", description);
        println!("File: {}", path);
        println!("══════════════════════════════════════════════════════════\n");

        let result = std::panic::catch_unwind(|| {
            analyze_wav(path)
        });

        match result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => println!("  ⚠️  Error: {}\n", e),
            Err(_) => println!("  ⚠️  File not found or panic\n"),
        }
    }

    Ok(())
}

fn analyze_wav(path: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    println!("Format:");
    println!("  Sample rate: {} Hz", spec.sample_rate);
    println!("  Channels: {}", spec.channels);
    println!("  Bits per sample: {}", spec.bits_per_sample);
    println!("  Duration: {:.2}s\n", reader.duration() as f32 / spec.sample_rate as f32);

    // Read samples
    let samples_i16: Vec<i16> = reader.samples::<i16>()
        .map(|s| s.unwrap())
        .collect();

    // Convert to f32 with different normalization methods
    let samples_32768: Vec<f32> = samples_i16.iter()
        .map(|&s| s as f32 / 32768.0)
        .collect();

    let samples_32767: Vec<f32> = samples_i16.iter()
        .map(|&s| s as f32 / 32767.0)
        .collect();

    // Calculate statistics for i16 samples
    let i16_min = samples_i16.iter().copied().min().unwrap_or(0);
    let i16_max = samples_i16.iter().copied().max().unwrap_or(0);
    let i16_abs_max = samples_i16.iter().map(|&s| s.abs()).max().unwrap_or(0);

    println!("Raw i16 Sample Statistics:");
    println!("  Min value: {}", i16_min);
    println!("  Max value: {}", i16_max);
    println!("  Max absolute: {} ({:.2}% of i16::MAX)", i16_abs_max, i16_abs_max as f32 / 32767.0 * 100.0);
    println!("  Range: {} to {}\n", i16_min, i16_max);

    // Calculate statistics for f32 (normalized with /32768)
    let f32_min = samples_32768.iter().copied().fold(f32::INFINITY, f32::min);
    let f32_max = samples_32768.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let f32_abs_max = samples_32768.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);

    println!("Normalized f32 Statistics (/32768.0):");
    println!("  Min value: {:.6}", f32_min);
    println!("  Max value: {:.6}", f32_max);
    println!("  Max absolute: {:.6} ({:.2}% of 1.0)", f32_abs_max, f32_abs_max * 100.0);
    println!("  Range: {:.6} to {:.6}\n", f32_min, f32_max);

    // Calculate RMS energy
    let rms = (samples_32768.iter().map(|&s| s * s).sum::<f32>() / samples_32768.len() as f32).sqrt();
    let rms_db = 20.0 * rms.log10();

    println!("Energy Analysis:");
    println!("  RMS amplitude: {:.6} ({:.2} dB)", rms, rms_db);

    // Calculate RMS for 1-second windows to find speech regions
    let window_size = spec.sample_rate as usize;
    let mut max_rms = 0.0f32;
    let mut max_rms_time = 0.0f32;

    for window_start in (0..samples_32768.len()).step_by(window_size / 10) {
        if window_start + window_size > samples_32768.len() {
            break;
        }
        let window = &samples_32768[window_start..window_start + window_size];
        let window_rms = (window.iter().map(|&s| s * s).sum::<f32>() / window.len() as f32).sqrt();

        if window_rms > max_rms {
            max_rms = window_rms;
            max_rms_time = window_start as f32 / spec.sample_rate as f32;
        }
    }

    println!("  Peak RMS (1s window): {:.6} at t={:.2}s", max_rms, max_rms_time);
    println!("  Peak RMS (dB): {:.2} dB\n", 20.0 * max_rms.log10());

    // DC offset analysis
    let dc_offset = samples_32768.iter().sum::<f32>() / samples_32768.len() as f32;
    println!("DC Offset: {:.6}\n", dc_offset);

    // Check for clipping
    let clipped_positive = samples_i16.iter().filter(|&&s| s >= 32760).count();
    let clipped_negative = samples_i16.iter().filter(|&&s| s <= -32760).count();
    let total_clipped = clipped_positive + clipped_negative;

    if total_clipped > 0 {
        println!("⚠️  Clipping detected: {} samples ({:.2}%)",
                 total_clipped,
                 total_clipped as f32 / samples_i16.len() as f32 * 100.0);
    } else {
        println!("✓ No clipping detected");
    }

    // Check for silence
    let silent_samples = samples_i16.iter().filter(|&&s| s.abs() < 100).count();
    let silence_pct = silent_samples as f32 / samples_i16.len() as f32 * 100.0;
    println!("Silence (< 0.3% amplitude): {:.2}% of samples\n", silence_pct);

    // VAD compatibility check
    println!("VAD Compatibility Assessment:");

    if f32_abs_max < 0.01 {
        println!("  ❌ CRITICAL: Audio is TOO QUIET (max {:.4})", f32_abs_max);
        println!("     Expected: > 0.1 for normal speech");
        println!("     Solution: Increase recording volume or microphone gain");
    } else if f32_abs_max < 0.1 {
        println!("  ⚠️  WARNING: Audio is quiet (max {:.4})", f32_abs_max);
        println!("     Expected: 0.3-0.7 for typical speech");
        println!("     This may cause low VAD probabilities");
    } else {
        println!("  ✓ Audio amplitude looks good (max {:.4})", f32_abs_max);
    }

    if rms < 0.01 {
        println!("  ❌ CRITICAL: RMS energy too low ({:.6})", rms);
        println!("     This explains low VAD probabilities!");
    } else if rms < 0.05 {
        println!("  ⚠️  WARNING: RMS energy is low ({:.6})", rms);
        println!("     Normal speech: 0.05-0.15 RMS");
    } else {
        println!("  ✓ RMS energy looks good ({:.6})", rms);
    }

    // Normalization comparison
    println!("\nNormalization Method Comparison:");
    let diff_samples: Vec<f32> = samples_32768.iter()
        .zip(samples_32767.iter())
        .map(|(a, b)| (a - b).abs())
        .collect();
    let max_diff = diff_samples.iter().copied().fold(0.0f32, f32::max);
    println!("  Max difference (/32768 vs /32767): {:.9}", max_diff);
    if max_diff > 0.00001 {
        println!("  ⚠️  Normalization difference may affect results");
    } else {
        println!("  ✓ Normalization methods produce similar results");
    }

    Ok(())
}
