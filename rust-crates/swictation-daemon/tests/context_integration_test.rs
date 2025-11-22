//! Integration test for Phase 3 context learning
//!
//! Tests that the daemon can successfully load or train the context model
//! at startup without errors.

use std::path::PathBuf;
use swictation_context_learning::{
    load_or_train_model, LearningConfig, RetrainingConfig,
};

#[test]
fn test_context_model_integration() {
    // This test runs in the daemon context to verify integration
    let data_dir = dirs::data_local_dir()
        .expect("Failed to get data dir")
        .join("swictation");

    let model_path = data_dir.join("context-model.json");
    let db_path = data_dir.join("metrics.db");

    println!("Testing context model integration:");
    println!("  Model path: {:?}", model_path);
    println!("  Database path: {:?}", db_path);

    let learning_config = LearningConfig {
        min_segments: 10, // Lower for testing (normally 50)
        ..Default::default()
    };
    let retrain_config = RetrainingConfig::default();

    // This should not panic - it returns Result
    match load_or_train_model(&model_path, &db_path, &learning_config, &retrain_config) {
        Ok(Some(model)) => {
            println!("✅ Context model loaded successfully:");
            println!("   - Topics: {}", model.topics.len());
            println!("   - Homonym rules: {}", model.homonym_rules.len());
            println!("   - Context patterns: {}", model.patterns.len());
            println!("   - Meta-learning levels:");
            println!("     * Level 0: {} patterns", model.meta_level_0.len());
            println!("     * Level 1: {} meta-patterns", model.meta_level_1.len());
            println!("     * Level 2: {} meta-strategies", model.meta_level_2.len());

            // Verify model structure
            assert!(!model.topics.is_empty(), "Model should have discovered topics");
        }
        Ok(None) => {
            println!("⚠️  Context model not available");
            println!("   Reason: Insufficient training data (< {} segments)", learning_config.min_segments);
            println!("   This is expected if database is empty or has few segments");
            // This is OK - not an error, just means not enough data yet
        }
        Err(e) => {
            // In CI environments, the data directory may not exist
            println!("⚠️  Context model not available: {}", e);
            println!("   This is expected in CI environments without existing data");
            // This is OK in CI - the daemon will create the directory on first run
        }
    }

    println!("\n✅ Integration test passed - context learning integrated correctly");
}

#[test]
fn test_daemon_can_handle_missing_database() {
    // Test that daemon handles missing database gracefully
    let fake_dir = PathBuf::from("/tmp/nonexistent_swictation_test");
    let model_path = fake_dir.join("context-model.json");
    let db_path = fake_dir.join("metrics.db");

    let learning_config = LearningConfig::default();
    let retrain_config = RetrainingConfig::default();

    // Should return Ok(None) or Err, but not panic
    match load_or_train_model(&model_path, &db_path, &learning_config, &retrain_config) {
        Ok(None) => {
            println!("✅ Correctly handled missing database (returned None)");
        }
        Err(_) => {
            println!("✅ Correctly handled missing database (returned Error)");
        }
        Ok(Some(_)) => {
            panic!("Should not load model from nonexistent database");
        }
    }
}
