//! Phase 3 Research Harness
//!
//! Run complete validation experiment:
//! 1. Load segment data from metrics.db
//! 2. Split into train/test (80/20)
//! 3. Train context model
//! 4. Evaluate on test data
//! 5. Generate research report

use anyhow::Result;
use std::path::PathBuf;
use swictation_context_learning::{train_test_split, ContextLearner, LearningConfig};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Phase 3 Context Learning Research Harness ===\n");

    // Configuration
    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("swictation/metrics.db");

    println!("📊 Configuration:");
    println!("  Database: {:?}", db_path);
    println!("  Train/test split: 80/20");
    println!("  Minimum segments: 50");
    println!("  Topic clusters: 5");
    println!("  Context window: 10 segments");
    println!("  Meta-learning: Enabled (depth=3)");
    println!();

    // Create learner
    let config = LearningConfig {
        min_segments: 50,
        num_topics: 5,
        context_window: 10,
        min_confidence: 0.70,
        enable_meta_learning: true,
        max_meta_depth: 3,
    };

    let mut learner = ContextLearner::new(config.clone());

    // Load data
    println!("📂 Loading training data...");
    let data = learner.load_training_data(&db_path, 6)?; // Last 6 months

    println!("  ✓ Loaded {} segments", data.segments.len());
    println!("  ✓ Total words: {}", data.total_words);
    println!("  ✓ Date range: {} days", data.date_range_days);
    println!();

    if data.segments.len() < config.min_segments {
        eprintln!(
            "❌ Insufficient data: {} segments (need {})",
            data.segments.len(),
            config.min_segments
        );
        eprintln!("   Run swictation for a while to collect more segment data.");
        return Ok(());
    }

    // Split data
    println!("✂️  Splitting data (80% train, 20% test)...");
    let (train_segments, test_segments) = train_test_split(&data, 0.80);
    println!("  ✓ Training: {} segments", train_segments.len());
    println!("  ✓ Testing: {} segments", test_segments.len());
    println!();

    // Train model
    println!("🧠 Training context model...");
    let train_data = swictation_context_learning::TrainingData {
        segments: train_segments,
        total_words: data.total_words,
        date_range_days: data.date_range_days,
    };

    let model = learner.train(&train_data)?;
    println!("  ✓ Discovered {} topic clusters", model.topics.len());
    println!("  ✓ Learned {} homonym rules", model.homonym_rules.len());
    println!("  ✓ Extracted {} context patterns", model.patterns.len());
    println!();

    // Meta-learning summary
    if let Some(summary) = learner.get_meta_summary() {
        println!("🔮 Meta-Learning Summary:");
        for line in summary.lines() {
            println!("  {}", line);
        }
        println!();
    }

    // Evaluate
    println!("📈 Evaluating on test data...");
    let report = learner.evaluate(&model, &test_segments)?;

    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║          PHASE 3 RESEARCH RESULTS                     ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║                                                       ║");
    println!(
        "║  Topic Clustering Accuracy:    {:.1}%                   ║",
        report.topic_accuracy * 100.0
    );
    println!(
        "║  Homonym Resolution Accuracy:  {:.1}%                   ║",
        report.homonym_accuracy * 100.0
    );
    println!(
        "║  Overall Context Accuracy:     {:.1}%                   ║",
        report.context_accuracy * 100.0
    );
    println!("║                                                       ║");
    println!("║  Baseline (random guess):      67.0%                  ║");
    println!(
        "║  Improvement:                  {:+.1}%                  ║",
        report.improvement_percentage
    );
    println!("║                                                       ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║  Safety Validation                                    ║");
    println!("╠═══════════════════════════════════════════════════════╣");
    println!("║                                                       ║");
    println!(
        "║  No harmful patterns:         {}                     ║",
        if report.safety_checks.no_harmful_patterns {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!(
        "║  No profanity learning:       {}                     ║",
        if report.safety_checks.no_profanity_learning {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!(
        "║  Confidence threshold met:    {}                     ║",
        if report.safety_checks.confidence_threshold_met {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!("║                                                       ║");
    println!(
        "║  All safety checks:           {}                     ║",
        if report.safety_checks.all_checks_passed {
            "✓ PASSED"
        } else {
            "✗ FAILED"
        }
    );
    println!("║                                                       ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();

    // Discovered topics
    println!("📚 Discovered Topic Clusters:");
    for (i, topic) in model.topics.iter().enumerate() {
        println!(
            "  {}. {} (confidence: {:.0}%, {} segments)",
            i + 1,
            topic.name,
            topic.confidence * 100.0,
            topic.segment_count
        );
        println!("     Keywords: {}", topic.keywords.join(", "));
    }
    println!();

    // Homonym rules
    if !model.homonym_rules.is_empty() {
        println!("🔤 Learned Homonym Resolution Rules:");
        for (word, resolver) in model.homonym_rules.iter().take(5) {
            println!("  \"{}\":", word);
            for (i, interp) in resolver.interpretations.iter().take(3).enumerate() {
                println!(
                    "    {}. {} (confidence: {:.0}%, freq: {})",
                    i + 1,
                    interp.meaning,
                    interp.confidence * 100.0,
                    interp.frequency
                );
            }
        }
        println!();
    }

    // Context patterns
    if !model.patterns.is_empty() {
        println!("🔍 Top Context Patterns:");
        for (i, pattern) in model.patterns.iter().take(10).enumerate() {
            println!(
                "  {}. {} (support: {})",
                i + 1,
                pattern.description,
                pattern.support
            );
        }
        println!();
    }

    // Recommendation
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║  RECOMMENDATION                                       ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();

    if report.improvement_percentage >= 10.0 && report.safety_checks.all_checks_passed {
        println!("✅ DEPLOY TO PRODUCTION");
        println!();
        println!(
            "Context-aware meta-learning provides {:.1}% improvement",
            report.improvement_percentage
        );
        println!("over baseline with acceptable safety profile.");
        println!();
        println!("Suggested next steps:");
        println!("1. Integrate ContextModel into swictation-daemon");
        println!("2. Add real-time topic detection to pipeline");
        println!("3. Enable adaptive homonym resolution");
        println!("4. Run 2-week beta with user feedback");
    } else if report.improvement_percentage >= 5.0 {
        println!("🔄 ITERATE");
        println!();
        println!(
            "Shows promise ({:.1}% improvement) but needs tuning.",
            report.improvement_percentage
        );
        println!();
        println!("Suggested improvements:");
        println!(
            "1. Collect more training data (current: {} segments)",
            data.segments.len()
        );
        println!("2. Tune confidence thresholds");
        println!("3. Add more homonym examples");
        println!("4. Refine topic clustering parameters");
    } else {
        println!("❌ DON'T DEPLOY");
        println!();
        println!(
            "Insufficient improvement ({:.1}%).",
            report.improvement_percentage
        );
        println!("Meta-learning did not beat baseline significantly.");
        println!();
        println!("Consider:");
        println!("1. Alternative learning algorithms");
        println!("2. Different feature extraction");
        println!("3. More diverse training data");
    }

    println!();
    println!("Research complete! 🎉");

    Ok(())
}
