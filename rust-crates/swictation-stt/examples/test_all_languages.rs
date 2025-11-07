//! Test STT inference on all language test files

use swictation_stt::{Recognizer, DEFAULT_MODEL_PATH};
use std::time::Instant;

fn load_wav(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

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

    Ok(mono_samples)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== STT Multi-Language Test Suite ===\n");

    let test_files = [
        ("English", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/en_16k.wav"),
        ("German", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/de_16k.wav"),
        ("Spanish", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/es_16k.wav"),
        ("French", "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/fr_16k.wav"),
    ];

    // Load model once
    let start = Instant::now();
    let mut recognizer = Recognizer::new(DEFAULT_MODEL_PATH)?;
    println!("Model loaded in {:.2}s\n", start.elapsed().as_secs_f64());

    println!("{:<12} {:<10} {:<12} {:<50}", "Language", "Duration", "Time (ms)", "Transcription");
    println!("{}", "=".repeat(90));

    for (lang, path) in &test_files {
        let audio = load_wav(path)?;
        let duration = audio.len() as f32 / 16000.0;

        let result = recognizer.recognize(&audio)?;

        println!("{:<12} {:<10.2}s {:<12.2} {}",
                 lang,
                 duration,
                 result.processing_time_ms,
                 result.text);
    }

    println!("\n✓ All tests complete!");

    Ok(())
}
