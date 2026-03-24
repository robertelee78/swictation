//! Test CoreML encoder output data layout.
//! Run: cargo run --features coreml-native --example test_coreml_layout

use coreml_native::{BorrowedTensor, ComputeUnits, Model};

fn main() {
    let base = format!(
        "{}/.local/share/swictation/models/parakeet-tdt-1.1b-coreml",
        std::env::var("HOME").unwrap()
    );

    println!("Loading encoder...");
    let encoder = Model::load(format!("{}/encoder.mlmodelc", base), ComputeUnits::All).unwrap();

    // Create 15s of silence with a few non-zero samples
    let mut audio = vec![0.0f32; 240000];
    // Put a 440Hz tone in the first 1 second
    for (i, sample) in audio.iter_mut().enumerate().take(16000) {
        *sample = 0.3 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin();
    }

    let audio_tensor = BorrowedTensor::from_f32(&audio, &[1, 240000]).unwrap();
    let length = [16000i32]; // 1 second of actual audio
    let length_tensor = BorrowedTensor::from_i32(&length, &[1]).unwrap();

    let pred = encoder
        .predict(&[
            ("audio_signal", &audio_tensor),
            ("audio_length", &length_tensor),
        ])
        .unwrap();

    let (features, shape) = pred.get_f32("obj_3").unwrap();
    let (valid_f32, _) = pred.get_f32("obj").unwrap();
    let valid = valid_f32[0] as usize;

    println!("Shape: {:?}, valid_frames: {}", shape, valid);
    println!("Total elements: {}", features.len());

    // Print first 5 elements of the flat array
    println!("\nFlat[0..5]: {:?}", &features[..5]);

    // If row-major [1, 1024, 188]: flat[0..188] = all time steps for feature 0
    // If column-major [1, 1024, 188]: flat[0..1024] = all features for time 0

    // Extract "frame 0" two different ways:
    let total_time = shape.get(2).copied().unwrap_or(188);
    let encoder_dim = shape.get(1).copied().unwrap_or(1024);

    // Way 1: contiguous (column-major assumption)
    let frame0_contiguous: Vec<f32> = features[..encoder_dim].to_vec();
    // Way 2: strided (row-major assumption)
    let frame0_strided: Vec<f32> = (0..encoder_dim).map(|f| features[f * total_time]).collect();

    println!(
        "\nFrame 0 contiguous (column-major): first 5 = {:?}",
        &frame0_contiguous[..5]
    );
    println!(
        "Frame 0 strided (row-major):       first 5 = {:?}",
        &frame0_strided[..5]
    );

    // Check: which one has more variation (non-zero = likely correct)?
    let var_c: f32 = frame0_contiguous.iter().map(|x| x * x).sum::<f32>() / encoder_dim as f32;
    let var_s: f32 = frame0_strided.iter().map(|x| x * x).sum::<f32>() / encoder_dim as f32;
    println!("\nVariance contiguous: {:.6}", var_c);
    println!("Variance strided:    {:.6}", var_s);

    // Also check: does flat[1] correspond to feature[0, 0, 1] (row-major)
    // or feature[0, 1, 0] (column-major)?
    println!("\nLayout test:");
    println!("  flat[0]   = {:.6}", features[0]);
    println!("  flat[1]   = {:.6}", features[1]);
    println!("  flat[188] = {:.6}", features[188]);
    println!("  flat[1024] = {:.6}", features[1024]);
    println!("\nIf row-major: flat[1] = features[0,0,1] (time=1, same feature)");
    println!("If col-major: flat[1] = features[0,1,0] (feature=1, same time)");
}
