//! Test STT accuracy against known reference transcriptions

use swictation_stt::{Recognizer, DEFAULT_MODEL_PATH};

fn load_wav(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

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
    println!("=== STT Accuracy Test ===\n");

    let tests = [
        (
            "Short",
            "/tmp/en-short-16k.wav",
            "Hello world. Testing, one, two, three",
        ),
        (
            "Long",
            "/tmp/en-long-16k.wav",
            "The open-source AI community has scored a significant win...",
        ),
    ];

    let mut recognizer = Recognizer::new(DEFAULT_MODEL_PATH)?;
    println!("Model loaded\n");

    for (name, path, reference) in &tests {
        println!("=== {} Sample ===", name);
        let audio = load_wav(path)?;
        let duration = audio.len() as f32 / 16000.0;

        println!("Duration: {:.2}s", duration);
        println!("Reference: {}", reference);

        let result = recognizer.recognize(&audio)?;

        println!("Result:    {}", result.text);
        println!("Time:      {:.2}ms", result.processing_time_ms);
        println!("Match:     {}\n", if result.text.to_lowercase().contains(&reference[..20].to_lowercase()) {
            "✓ PARTIAL"
        } else {
            "✗ NO MATCH"
        });
    }

    Ok(())
}
