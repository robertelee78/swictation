//! # Swictation Context Learning (Phase 3 Research)
//!
//! Context-aware meta-learning from segment patterns without requiring
//! inline correction monitoring. Learns from:
//! - Segment vocabulary patterns
//! - Co-occurrence statistics
//! - Temporal patterns (time of day, session clustering)
//! - Transformation success signals
//!
//! ## Research Objectives
//!
//! 1. Quantify improvement of context-aware learning vs static rules
//! 2. Measure minimum data requirements for useful learning
//! 3. Detect and prevent harmful pattern learning
//! 4. Validate safety constraints

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use midstreamer_strange_loop::{MetaLevel, StrangeLoop, StrangeLoopConfig};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

mod clustering;
mod homonym;
mod patterns;
mod validation;

pub use clustering::TopicCluster;
pub use homonym::HomonymResolver;
pub use patterns::ContextPattern;
pub use validation::ValidationReport;

/// A single segment from the metrics database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub segment_id: i64,
    pub session_id: i64,
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub words: i32,
    pub transformations_count: i32,
}

/// Training data extracted from segment history
#[derive(Debug, Clone)]
pub struct TrainingData {
    pub segments: Vec<Segment>,
    pub total_words: usize,
    pub date_range_days: i64,
}

/// Context model learned from segment patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextModel {
    /// Topic clusters discovered
    pub topics: Vec<TopicCluster>,

    /// Homonym resolution rules
    pub homonym_rules: HashMap<String, HomonymResolver>,

    /// Context patterns (co-occurrence, temporal)
    pub patterns: Vec<ContextPattern>,

    /// Meta-knowledge from strange-loop
    pub meta_level_0: Vec<String>,
    pub meta_level_1: Vec<String>,
    pub meta_level_2: Vec<String>,
}

/// Configuration for context learning
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Minimum segments required for training
    pub min_segments: usize,

    /// Number of topic clusters (k-means)
    pub num_topics: usize,

    /// Context window size (number of previous segments)
    pub context_window: usize,

    /// Minimum confidence threshold for predictions
    pub min_confidence: f64,

    /// Enable strange-loop meta-learning
    pub enable_meta_learning: bool,

    /// Max meta-learning depth
    pub max_meta_depth: usize,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_segments: 50,
            num_topics: 5,
            context_window: 10,
            min_confidence: 0.70,
            enable_meta_learning: true,
            max_meta_depth: 3,
        }
    }
}

/// Configuration for adaptive retraining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrainingConfig {
    /// Minimum new segments before considering retrain
    pub min_new_segments: usize,

    /// Maximum model age before forcing retrain (days)
    pub max_model_age_days: u64,

    /// Minimum hours between retrains (prevents thrashing)
    pub min_retrain_interval_hours: u64,

    /// Enable automatic retraining
    pub auto_retrain: bool,
}

impl Default for RetrainingConfig {
    fn default() -> Self {
        Self {
            min_new_segments: 25,          // Retrain every ~25 new segments
            max_model_age_days: 1,         // Force retrain daily
            min_retrain_interval_hours: 6, // But never more than 4x/day
            auto_retrain: true,
        }
    }
}

/// The main context learning engine
pub struct ContextLearner {
    config: LearningConfig,
    strange_loop: Option<StrangeLoop>,
}

impl ContextLearner {
    /// Create a new context learner
    pub fn new(config: LearningConfig) -> Self {
        let strange_loop = if config.enable_meta_learning {
            Some(StrangeLoop::new(StrangeLoopConfig {
                max_meta_depth: config.max_meta_depth,
                enable_self_modification: false, // SAFETY: read-only during research
                max_modifications_per_cycle: 0,
                safety_check_enabled: true,
            }))
        } else {
            None
        };

        Self {
            config,
            strange_loop,
        }
    }

    /// Load training data from metrics database
    pub fn load_training_data<P: AsRef<Path>>(
        &self,
        db_path: P,
        months_back: i64,
    ) -> Result<TrainingData> {
        let conn = Connection::open(db_path.as_ref()).context("Failed to open metrics database")?;

        // Calculate date threshold
        let threshold_timestamp =
            Utc::now().timestamp() as f64 - (months_back * 30 * 24 * 60 * 60) as f64;

        info!(
            "Loading segments from last {} months (threshold: {})",
            months_back, threshold_timestamp
        );

        let mut stmt = conn.prepare(
            "SELECT
                id, session_id, timestamp, text, words, transformations_count
             FROM segments
             WHERE timestamp >= ?1
               AND text IS NOT NULL
               AND text != ''
             ORDER BY timestamp ASC",
        )?;

        let segments: Vec<Segment> = stmt
            .query_map(params![threshold_timestamp], |row| {
                let timestamp_f64: f64 = row.get(2)?;
                let naive = DateTime::from_timestamp(timestamp_f64 as i64, 0)
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_default();
                let timestamp = DateTime::from_naive_utc_and_offset(naive, Utc);

                Ok(Segment {
                    segment_id: row.get(0)?,
                    session_id: row.get(1)?,
                    timestamp,
                    text: row.get(3)?,
                    words: row.get(4)?,
                    transformations_count: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        if segments.is_empty() {
            warn!("No segments found in database");
        } else {
            info!("Loaded {} segments", segments.len());
        }

        let total_words: usize = segments.iter().map(|s| s.words as usize).sum();

        let date_range_days = if !segments.is_empty() {
            let first = segments.first().unwrap().timestamp;
            let last = segments.last().unwrap().timestamp;
            (last - first).num_days()
        } else {
            0
        };

        Ok(TrainingData {
            segments,
            total_words,
            date_range_days,
        })
    }

    /// Train context model from segment data
    pub fn train(&mut self, data: &TrainingData) -> Result<ContextModel> {
        if data.segments.len() < self.config.min_segments {
            anyhow::bail!(
                "Insufficient training data: {} segments (need {})",
                data.segments.len(),
                self.config.min_segments
            );
        }

        info!("Training context model on {} segments", data.segments.len());

        // 1. Discover topic clusters
        info!("Discovering topic clusters...");
        let topics = clustering::discover_topics(&data.segments, self.config.num_topics)?;
        info!("Discovered {} topic clusters", topics.len());

        // 2. Learn homonym resolution rules
        info!("Learning homonym resolution...");
        let homonym_rules = homonym::learn_homonym_rules(&data.segments, &topics)?;
        info!("Learned {} homonym rules", homonym_rules.len());

        // 3. Extract context patterns
        info!("Extracting context patterns...");
        let patterns = patterns::extract_patterns(&data.segments, self.config.context_window)?;
        info!("Extracted {} context patterns", patterns.len());

        // 4. Meta-learning with strange-loop
        let (meta_level_0, meta_level_1, meta_level_2) = if let Some(ref mut sl) = self.strange_loop
        {
            info!("Running meta-learning (strange-loop)...");

            // Level 0: Raw segment patterns
            let level0_data: Vec<String> = patterns.iter().map(|p| p.to_pattern_string()).collect();

            let level0_knowledge = sl
                .learn_at_level(MetaLevel::base(), &level0_data)
                .context("Failed to learn at meta-level 0")?;

            let meta_0: Vec<String> = level0_knowledge.iter().map(|k| k.pattern.clone()).collect();

            // Level 1 & 2 are automatically learned by strange-loop
            let level1_knowledge = sl.get_knowledge_at_level(MetaLevel::base().next());
            let level2_knowledge = sl.get_knowledge_at_level(MetaLevel::base().next().next());

            let meta_1: Vec<String> = level1_knowledge.iter().map(|k| k.pattern.clone()).collect();

            let meta_2: Vec<String> = level2_knowledge.iter().map(|k| k.pattern.clone()).collect();

            info!(
                "Meta-learning complete: L0={} L1={} L2={}",
                meta_0.len(),
                meta_1.len(),
                meta_2.len()
            );

            (meta_0, meta_1, meta_2)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        Ok(ContextModel {
            topics,
            homonym_rules,
            patterns,
            meta_level_0,
            meta_level_1,
            meta_level_2,
        })
    }

    /// Evaluate model on test data
    pub fn evaluate(
        &self,
        model: &ContextModel,
        test_data: &[Segment],
    ) -> Result<ValidationReport> {
        validation::evaluate_model(model, test_data, self.config.min_confidence)
    }

    /// Get meta-learning summary
    pub fn get_meta_summary(&self) -> Option<String> {
        self.strange_loop.as_ref().map(|sl| {
            let summary = sl.get_summary();
            format!(
                "Meta-learning summary:\n\
                 - Total levels: {}\n\
                 - Total knowledge: {}\n\
                 - Learning iterations: {}\n\
                 - Modifications: {}\n\
                 - Safety violations: {}",
                summary.total_levels,
                summary.total_knowledge,
                summary.learning_iterations,
                summary.total_modifications,
                summary.safety_violations
            )
        })
    }
}

/// Split data into train/test sets
pub fn train_test_split(data: &TrainingData, train_ratio: f64) -> (Vec<Segment>, Vec<Segment>) {
    let split_idx = (data.segments.len() as f64 * train_ratio) as usize;
    let train = data.segments[..split_idx].to_vec();
    let test = data.segments[split_idx..].to_vec();
    (train, test)
}

/// Determine if model should be retrained
pub fn should_retrain(
    model_path: &Path,
    db_path: &Path,
    config: &RetrainingConfig,
) -> Result<bool> {
    if !config.auto_retrain {
        return Ok(false);
    }

    // Check 1: Does model exist?
    if !model_path.exists() {
        info!("No model exists - initial training required");
        return Ok(true);
    }

    // Check 2: When was model last trained?
    let model_metadata = fs::metadata(model_path).context("Failed to read model metadata")?;

    let model_modified = model_metadata
        .modified()
        .context("Failed to get model modification time")?;

    let model_age = SystemTime::now()
        .duration_since(model_modified)
        .context("Failed to calculate model age")?;

    let model_age_hours = model_age.as_secs() / 3600;

    // Check 3: Don't retrain too frequently
    if model_age_hours < config.min_retrain_interval_hours {
        info!(
            "Model too recent ({} hours old, minimum {})",
            model_age_hours, config.min_retrain_interval_hours
        );
        return Ok(false);
    }

    // Check 4: Force retrain if model too old
    let max_age_seconds = config.max_model_age_days * 86400;
    if model_age.as_secs() > max_age_seconds {
        info!(
            "Model too old ({} hours old, max {} days)",
            model_age_hours, config.max_model_age_days
        );
        return Ok(true);
    }

    // Check 5: Count new segments since last training
    let new_segment_count = count_segments_since(db_path, model_modified)?;

    if new_segment_count >= config.min_new_segments {
        info!(
            "Sufficient new data ({} segments >= {} threshold)",
            new_segment_count, config.min_new_segments
        );
        return Ok(true);
    }

    info!(
        "No retrain needed (model age: {}h, new segments: {})",
        model_age_hours, new_segment_count
    );
    Ok(false)
}

/// Count segments added since a specific time
fn count_segments_since(db_path: &Path, since: SystemTime) -> Result<usize> {
    let conn = Connection::open(db_path).context("Failed to open metrics database")?;

    let since_timestamp = since
        .duration_since(UNIX_EPOCH)
        .context("Failed to convert time to timestamp")?
        .as_secs() as f64;

    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM segments WHERE timestamp > ?1 AND text IS NOT NULL AND text != ''",
        params![since_timestamp],
        |row| row.get(0),
    ).context("Failed to count new segments")?;

    Ok(count)
}

/// Load or create context model with adaptive retraining
pub fn load_or_train_model(
    model_path: &Path,
    db_path: &Path,
    learning_config: &LearningConfig,
    retrain_config: &RetrainingConfig,
) -> Result<Option<ContextModel>> {
    if should_retrain(model_path, db_path, retrain_config)? {
        info!("Retraining context model...");

        let mut learner = ContextLearner::new(learning_config.clone());
        let data = learner.load_training_data(db_path, 6)?; // Last 6 months

        if data.segments.len() < learning_config.min_segments {
            warn!(
                "Insufficient data for training: {} segments (need {})",
                data.segments.len(),
                learning_config.min_segments
            );
            return Ok(None);
        }

        let model = learner.train(&data)?;

        // Save model
        let model_json =
            serde_json::to_string_pretty(&model).context("Failed to serialize model")?;
        fs::write(model_path, model_json).context("Failed to write model file")?;

        info!("Context model trained and saved successfully");
        Ok(Some(model))
    } else if model_path.exists() {
        // Load existing model
        info!("Loading existing context model");
        let model_json = fs::read_to_string(model_path).context("Failed to read model file")?;
        let model = serde_json::from_str(&model_json).context("Failed to deserialize model")?;
        Ok(Some(model))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learner_creation() {
        let config = LearningConfig::default();
        let learner = ContextLearner::new(config);
        assert!(learner.strange_loop.is_some());
    }

    #[test]
    fn test_train_test_split() {
        let segments = vec![
            create_test_segment(1, "test one"),
            create_test_segment(2, "test two"),
            create_test_segment(3, "test three"),
            create_test_segment(4, "test four"),
        ];

        let data = TrainingData {
            segments,
            total_words: 8,
            date_range_days: 1,
        };

        let (train, test) = train_test_split(&data, 0.75);
        assert_eq!(train.len(), 3);
        assert_eq!(test.len(), 1);
    }

    fn create_test_segment(id: i64, text: &str) -> Segment {
        Segment {
            segment_id: id,
            session_id: 1,
            timestamp: Utc::now(),
            text: text.to_string(),
            words: text.split_whitespace().count() as i32,
            transformations_count: 0,
        }
    }
}
