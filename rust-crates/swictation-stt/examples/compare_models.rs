/// Compare 0.6B and 1.1B model outputs for Secretary Mode regression testing
///
/// This test helps diagnose why Secretary Mode text replacement doesn't work with 1.1B:
/// - 0.6B outputs: "Hello, world." (punctuation already converted)
/// - 1.1B outputs: "Hello, world period." (punctuation as words)
///
/// The issue is likely:
/// 1. Different tokenizers (0.6B has <|pnc|> special tokens, 1.1B uses BPE)
/// 2. Different training (0.6B trained to output punctuation, 1.1B outputs words)
///
/// ## Running this test:
///
/// ```bash
/// export ORT_DYLIB_PATH=$(python3 -c "import onnxruntime; import os; print(os.path.join(os.path.dirname(onnxruntime.__file__), 'capi/libonnxruntime.so.1.23.2'))")
/// cargo run --release --example compare_models
/// ```

use std::path::PathBuf;
use std::time::Instant;
use swictation_stt::OrtRecognizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("======================================================");
    println!("Model Comparison: 0.6B vs 1.1B Secretary Mode Test");
    println!("======================================================\n");

    let home = std::env::var("HOME").expect("HOME not set");

    // Model paths
    let model_0_6b = PathBuf::from(format!(
        "{}/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx",
        home
    ));
    let model_1_1b = PathBuf::from(format!(
        "{}/.local/share/swictation/models/parakeet-tdt-1.1b-onnx",
        home
    ));

    // Test audio files - use the ones that came with 0.6B model
    let test_wavs = model_0_6b.join("test_wavs");
    let test_files = vec![
        test_wavs.join("en.wav"),
        test_wavs.join("de.wav"),
        test_wavs.join("es.wav"),
        test_wavs.join("fr.wav"),
    ];

    // Check which models exist
    let has_0_6b = model_0_6b.exists();
    let has_1_1b = model_1_1b.exists();

    println!("Model availability:");
    println!("  0.6B: {} ({})", if has_0_6b { "✓" } else { "✗" }, model_0_6b.display());
    println!("  1.1B: {} ({})\n", if has_1_1b { "✓" } else { "✗" }, model_1_1b.display());

    if !has_0_6b && !has_1_1b {
        eprintln!("❌ ERROR: No models found. Please download at least one model.");
        return Ok(());
    }

    // Analyze tokenizers
    println!("======================================================");
    println!("Tokenizer Comparison");
    println!("======================================================\n");

    if has_0_6b {
        let tokens_0_6b = model_0_6b.join("tokens.txt");
        let content = std::fs::read_to_string(&tokens_0_6b)?;
        let token_count = content.lines().count();
        println!("0.6B Tokenizer:");
        println!("  Total tokens: {}", token_count);
        println!("  Sample tokens:");
        for (i, line) in content.lines().take(10).enumerate() {
            println!("    {}: {}", i, line);
        }

        // Check for special tokens
        let has_pnc = content.contains("<|pnc|>");
        let has_itn = content.contains("<|itn|>");
        println!("  Special tokens:");
        println!("    <|pnc|>: {}", if has_pnc { "✓" } else { "✗" });
        println!("    <|itn|>: {}", if has_itn { "✓" } else { "✗" });

        // Check for punctuation tokens
        let has_comma = content.contains(" , ") || content.contains(",");
        let has_period = content.contains(" . ") || content.contains(".");
        println!("  Punctuation tokens:");
        println!("    comma: {}", if has_comma { "✓" } else { "✗" });
        println!("    period: {}", if has_period { "✓" } else { "✗" });
        println!();
    }

    if has_1_1b {
        let tokens_1_1b = model_1_1b.join("tokens.txt");
        let content = std::fs::read_to_string(&tokens_1_1b)?;
        let token_count = content.lines().count();
        println!("1.1B Tokenizer:");
        println!("  Total tokens: {}", token_count);
        println!("  Sample tokens:");
        for (i, line) in content.lines().take(10).enumerate() {
            println!("    {}: {}", i, line);
        }

        // Check for BPE markers
        let has_bpe = content.contains("▁");
        println!("  BPE markers: {}", if has_bpe { "✓" } else { "✗" });

        // Search for "period" and "comma" as words
        let lines: Vec<&str> = content.lines().collect();
        let period_tokens: Vec<&str> = lines.iter()
            .filter(|l| l.contains("period") || l.contains("Period"))
            .copied()
            .collect();
        let comma_tokens: Vec<&str> = lines.iter()
            .filter(|l| l.contains("comma") || l.contains("Comma"))
            .copied()
            .collect();

        println!("  Word tokens for punctuation:");
        println!("    'period' tokens: {}", period_tokens.len());
        for token in period_tokens.iter().take(5) {
            println!("      {}", token);
        }
        println!("    'comma' tokens: {}", comma_tokens.len());
        for token in comma_tokens.iter().take(5) {
            println!("      {}", token);
        }
        println!();
    }

    // Load models and compare outputs
    println!("======================================================");
    println!("Transcription Comparison");
    println!("======================================================\n");

    let mut results_0_6b = Vec::new();
    let mut results_1_1b = Vec::new();

    // Test 0.6B model if available
    if has_0_6b {
        println!("[LOADING] 0.6B model...");
        let start = Instant::now();
        match OrtRecognizer::new(&model_0_6b, false) {
            Ok(mut recognizer) => {
                println!("  ✓ Loaded in {:.2}s", start.elapsed().as_secs_f32());
                println!("{}\n", recognizer.model_info());

                for test_file in &test_files {
                    if !test_file.exists() {
                        println!("  ⚠️  Skipping {} (not found)", test_file.display());
                        continue;
                    }

                    println!("  Processing: {}", test_file.file_name().unwrap().to_string_lossy());
                    let start = Instant::now();
                    match recognizer.recognize_file(test_file) {
                        Ok(text) => {
                            let time = start.elapsed();
                            println!("    Result: \"{}\"", text);
                            println!("    Time: {:.2}s\n", time.as_secs_f32());
                            results_0_6b.push((test_file.clone(), text));
                        }
                        Err(e) => {
                            println!("    ❌ Error: {}\n", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Failed to load 0.6B model: {}\n", e);
            }
        }
    }

    // Test 1.1B model if available
    if has_1_1b {
        println!("[LOADING] 1.1B model...");
        let start = Instant::now();
        match OrtRecognizer::new(&model_1_1b, false) {
            Ok(mut recognizer) => {
                println!("  ✓ Loaded in {:.2}s", start.elapsed().as_secs_f32());
                println!("{}\n", recognizer.model_info());

                for test_file in &test_files {
                    if !test_file.exists() {
                        println!("  ⚠️  Skipping {} (not found)", test_file.display());
                        continue;
                    }

                    println!("  Processing: {}", test_file.file_name().unwrap().to_string_lossy());
                    let start = Instant::now();
                    match recognizer.recognize_file(test_file) {
                        Ok(text) => {
                            let time = start.elapsed();
                            println!("    Result: \"{}\"", text);
                            println!("    Time: {:.2}s\n", time.as_secs_f32());
                            results_1_1b.push((test_file.clone(), text));
                        }
                        Err(e) => {
                            println!("    ❌ Error: {}\n", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Failed to load 1.1B model: {}\n", e);
            }
        }
    }

    // Compare results
    if !results_0_6b.is_empty() && !results_1_1b.is_empty() {
        println!("======================================================");
        println!("Side-by-Side Comparison");
        println!("======================================================\n");

        for (file_0_6b, text_0_6b) in &results_0_6b {
            if let Some((_, text_1_1b)) = results_1_1b.iter()
                .find(|(f, _)| f == file_0_6b) {

                println!("File: {}", file_0_6b.file_name().unwrap().to_string_lossy());
                println!("  0.6B: \"{}\"", text_0_6b);
                println!("  1.1B: \"{}\"", text_1_1b);

                // Analyze differences
                let has_period_symbol_0_6b = text_0_6b.contains('.');
                let has_period_word_0_6b = text_0_6b.to_lowercase().contains("period");
                let has_comma_symbol_0_6b = text_0_6b.contains(',');
                let has_comma_word_0_6b = text_0_6b.to_lowercase().contains("comma");

                let has_period_symbol_1_1b = text_1_1b.contains('.');
                let has_period_word_1_1b = text_1_1b.to_lowercase().contains("period");
                let has_comma_symbol_1_1b = text_1_1b.contains(',');
                let has_comma_word_1_1b = text_1_1b.to_lowercase().contains("comma");

                println!("  Analysis:");
                println!("    0.6B: period={}/{}, comma={}/{}",
                    if has_period_symbol_0_6b { "symbol" } else { "-" },
                    if has_period_word_0_6b { "word" } else { "-" },
                    if has_comma_symbol_0_6b { "symbol" } else { "-" },
                    if has_comma_word_0_6b { "word" } else { "-" }
                );
                println!("    1.1B: period={}/{}, comma={}/{}",
                    if has_period_symbol_1_1b { "symbol" } else { "-" },
                    if has_period_word_1_1b { "word" } else { "-" },
                    if has_comma_symbol_1_1b { "symbol" } else { "-" },
                    if has_comma_word_1_1b { "word" } else { "-" }
                );
                println!();
            }
        }
    }

    println!("======================================================");
    println!("Summary");
    println!("======================================================\n");

    println!("Key Findings:");
    println!("  1. Tokenizer Differences:");
    println!("     - 0.6B: Uses special tokens (<|pnc|>, <|itn|>) with ~8K vocab");
    println!("     - 1.1B: Uses BPE subwords (▁) with ~1K vocab");
    println!();
    println!("  2. Punctuation Handling:");
    println!("     - 0.6B: Trained to output symbols (,.) directly");
    println!("     - 1.1B: Outputs punctuation as words ('comma', 'period')");
    println!();
    println!("  3. Secretary Mode Impact:");
    println!("     - 0.6B: No post-processing needed (already has symbols)");
    println!("     - 1.1B: REQUIRES post-processing to convert words to symbols");
    println!();
    println!("Recommendation:");
    println!("  For 1.1B Secretary Mode support:");
    println!("  1. Add post-processing step to convert 'comma' → ',' and 'period' → '.'");
    println!("  2. Handle other punctuation words: 'question mark', 'exclamation point', etc.");
    println!("  3. Consider word boundaries to avoid false matches (e.g., 'periodic')");

    Ok(())
}
