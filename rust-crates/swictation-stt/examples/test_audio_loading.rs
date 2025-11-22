use std::path::PathBuf;
/// Test audio loading to diagnose recognize_file() hang
use swictation_stt::audio::AudioProcessor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Audio Loading");

    let home = std::env::var("HOME").expect("HOME not set");
    let model_path = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));

    // Test with 24kHz WAV file
    let audio_24k = model_path.join("test_wavs/en.wav");
    println!("\n📁 Testing 24kHz WAV: {}", audio_24k.display());

    let mut processor = AudioProcessor::new()?;
    let samples_24k = processor.load_audio(&audio_24k)?;

    println!("✅ Loaded {} samples from 24kHz WAV", samples_24k.len());
    println!(
        "   Min: {:.6}, Max: {:.6}",
        samples_24k.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
        samples_24k.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
    );
    println!(
        "   Mean: {:.6}",
        samples_24k.iter().sum::<f32>() / samples_24k.len() as f32
    );

    // Test with 16kHz WAV file
    let audio_16k = PathBuf::from("/tmp/test-audio-16khz.wav");
    println!("\n📁 Testing 16kHz WAV: {}", audio_16k.display());

    let samples_16k = processor.load_audio(&audio_16k)?;

    println!("✅ Loaded {} samples from 16kHz WAV", samples_16k.len());
    println!(
        "   Min: {:.6}, Max: {:.6}",
        samples_16k.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
        samples_16k.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
    );
    println!(
        "   Mean: {:.6}",
        samples_16k.iter().sum::<f32>() / samples_16k.len() as f32
    );

    // Extract mel features from both
    println!("\n🔄 Extracting mel features from 24kHz-loaded samples...");
    let features_24k = processor.extract_mel_features(&samples_24k)?;
    println!("✅ 24kHz features: {:?}", features_24k.shape());

    println!("\n🔄 Extracting mel features from 16kHz-loaded samples...");
    let features_16k = processor.extract_mel_features(&samples_16k)?;
    println!("✅ 16kHz features: {:?}", features_16k.shape());

    // Check if features are similar
    println!("\n📊 Feature comparison:");
    println!(
        "   Frames difference: {}",
        features_24k.nrows() as i32 - features_16k.nrows() as i32
    );

    Ok(())
}
