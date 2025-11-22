//! Daemon Integration Example
//!
//! Shows how to integrate context-aware learning with adaptive retraining
//! into the swictation-daemon.
//!
//! This example demonstrates:
//! 1. Load-or-train pattern at daemon startup
//! 2. Use model for real-time predictions
//! 3. Manual retrain trigger
//! 4. Background retrain monitoring

use anyhow::Result;
use std::path::PathBuf;
use swictation_context_learning::{
    load_or_train_model, ContextModel, LearningConfig, RetrainingConfig,
};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Swictation Daemon Integration Example ===\n");

    // Configuration paths (production locations)
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("swictation");

    let model_path = data_dir.join("context-model.json");
    let db_path = data_dir.join("metrics.db");

    println!("📁 Paths:");
    println!("  Model: {:?}", model_path);
    println!("  Database: {:?}", db_path);
    println!();

    // Learning configuration
    let learning_config = LearningConfig {
        min_segments: 50,
        num_topics: 5,
        context_window: 10,
        min_confidence: 0.70,
        enable_meta_learning: true,
        max_meta_depth: 3,
    };

    // Adaptive retraining configuration
    let retrain_config = RetrainingConfig {
        min_new_segments: 25,          // Retrain every 25 new segments
        max_model_age_days: 1,         // Force retrain daily
        min_retrain_interval_hours: 6, // Max 4 times per day
        auto_retrain: true,
    };

    println!("⚙️  Configuration:");
    println!("  Min segments: {}", learning_config.min_segments);
    println!("  Topics: {}", learning_config.num_topics);
    println!("  Context window: {}", learning_config.context_window);
    println!(
        "  Min confidence: {:.0}%",
        learning_config.min_confidence * 100.0
    );
    println!();

    println!("🔄 Retraining Policy:");
    println!(
        "  New segments threshold: {}",
        retrain_config.min_new_segments
    );
    println!(
        "  Max model age: {} days",
        retrain_config.max_model_age_days
    );
    println!(
        "  Min interval: {} hours",
        retrain_config.min_retrain_interval_hours
    );
    println!(
        "  Auto-retrain: {}",
        if retrain_config.auto_retrain {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    // Load or train model
    println!("🧠 Loading context model...");
    match load_or_train_model(&model_path, &db_path, &learning_config, &retrain_config)? {
        Some(model) => {
            println!("  ✓ Model loaded successfully");
            print_model_summary(&model);

            // Simulate using model for predictions
            println!("\n📊 Example Predictions:");
            example_predictions(&model);
        }
        None => {
            println!("  ⚠️  No model available");
            println!("     Reason: Insufficient training data");
            println!("     Action: Use swictation normally to collect segments");
        }
    }

    println!("\n✅ Integration example complete!");
    println!("\nNext steps:");
    println!("1. Add load_or_train_model() to daemon startup");
    println!("2. Use model.predict_topic() in transcription pipeline");
    println!("3. Add manual retrain IPC handler");
    println!("4. Optional: Background retrain monitoring task");

    Ok(())
}

fn print_model_summary(model: &ContextModel) {
    println!("\n📚 Model Summary:");
    println!("  Topics: {}", model.topics.len());
    println!("  Homonym rules: {}", model.homonym_rules.len());
    println!("  Context patterns: {}", model.patterns.len());
    println!("  Meta-knowledge levels:");
    println!("    Level 0: {} patterns", model.meta_level_0.len());
    println!("    Level 1: {} meta-patterns", model.meta_level_1.len());
    println!("    Level 2: {} meta-strategies", model.meta_level_2.len());

    if !model.topics.is_empty() {
        println!("\n  Top topics:");
        for (i, topic) in model.topics.iter().take(3).enumerate() {
            println!(
                "    {}. {} (confidence: {:.0}%, {} segments)",
                i + 1,
                topic.name,
                topic.confidence * 100.0,
                topic.segment_count
            );
        }
    }
}

fn example_predictions(model: &ContextModel) {
    // Example segments to predict
    let test_segments = [
        "refactor the authentication class to use dependency injection",
        "meeting with team about quarterly budget planning",
        "create API endpoint for user data retrieval",
        "please find the attached report as discussed",
    ];

    for segment_text in &test_segments {
        let segment_words: Vec<String> = segment_text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        // Find best matching topic
        let mut best_topic = "Unknown";
        let mut best_score = 0;
        let mut best_confidence = 0.0;

        for topic in &model.topics {
            let matches = topic
                .keywords
                .iter()
                .filter(|kw| segment_words.contains(kw))
                .count();

            if matches > best_score {
                best_score = matches;
                best_topic = &topic.name;
                best_confidence = topic.confidence;
            }
        }

        println!(
            "  \"{}...\"",
            &segment_text.chars().take(50).collect::<String>()
        );
        println!(
            "    → Topic: {} (confidence: {:.0}%, {} keyword matches)",
            best_topic,
            best_confidence * 100.0,
            best_score
        );
    }
}
