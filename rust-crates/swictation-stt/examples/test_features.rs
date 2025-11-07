//! Test feature extraction with real audio

use swictation_stt::{features::{FeatureExtractor, FeatureConfig}, Recognizer, DEFAULT_MODEL_PATH};

fn load_wav(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    println!("WAV file info:");
    println!("  Channels: {}", spec.channels);
    println!("  Sample rate: {} Hz", spec.sample_rate);
    println!("  Bits per sample: {}", spec.bits_per_sample);
    println!("  Sample format: {:?}", spec.sample_format);

    // Convert to f32 samples
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            reader
                .samples::<i16>()
                .map(|s| s.map(|v| v as f32 / 32768.0))
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Float => {
            reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?
        }
    };

    // Convert stereo to mono if needed
    let mono_samples = if spec.channels == 2 {
        samples
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        samples
    };

    println!("  Total samples: {}", mono_samples.len());
    println!("  Duration: {:.2}s\n", mono_samples.len() as f32 / spec.sample_rate as f32);

    Ok(mono_samples)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== STT Feature Extraction Test ===\n");

    // Test feature extraction with English audio (16kHz version)
    let test_audio = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/en_16k.wav";

    println!("Loading audio: {}\n", test_audio);
    let audio = load_wav(test_audio)?;

    println!("=== Extracting Features ===");
    let config = FeatureConfig::default();
    let extractor = FeatureExtractor::new(config);

    let features = extractor.extract(&audio)?;

    println!("Features extracted:");
    println!("  Number of frames: {}", features.len());
    println!("  Feature dimension: {}", features[0].len());
    println!("  Expected encoder time steps: ~{}\n", features.len() / 8);  // Typical 8x reduction

    println!("First frame statistics:");
    let first_frame = &features[0];
    let min = first_frame.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = first_frame.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean = first_frame.iter().sum::<f32>() / first_frame.len() as f32;
    println!("  Min: {:.2}", min);
    println!("  Max: {:.2}", max);
    println!("  Mean: {:.2}", mean);

    println!("\n=== Testing Recognizer ===");
    let mut recognizer = Recognizer::new(DEFAULT_MODEL_PATH)?;

    println!("Running recognition...");
    let result = recognizer.recognize(&audio)?;

    println!("\nResult:");
    println!("  Text: '{}'", result.text);
    println!("  Confidence: {:.2}", result.confidence);
    println!("  Processing time: {:.2}ms", result.processing_time_ms);

    println!("\n✓ ONNX inference test complete!");

    Ok(())
}
