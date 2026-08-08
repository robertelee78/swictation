# Phase 3: Adaptive Retraining Cadence

**Status**: Design Document
**Date**: November 21, 2025
**Related**: phase3-context-learning-summary.md

## Problem

**Question**: "Is weekly too infrequent or is that the right cadence?"

Weekly retraining is **too infrequent** for context-aware learning. User's dictation topics can shift multiple times per day (coding → emails → meetings), and training is fast enough (< 1 second) to support much more frequent updates.

## Recommended Solution: Adaptive Retraining

### Configuration

```rust
// In swictation-context-learning/src/lib.rs

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
            max_model_age_days: 1,          // Force retrain daily
            min_retrain_interval_hours: 6,  // But never more than 4x/day
            auto_retrain: true,
        }
    }
}
```

### Retrain Decision Logic

```rust
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
        return Ok(true); // Always retrain if no model
    }

    // Check 2: When was model last trained?
    let model_metadata = fs::metadata(model_path)?;
    let model_age = SystemTime::now()
        .duration_since(model_metadata.modified()?)?;

    let last_retrain = model_age.as_secs() / 3600; // Hours

    // Check 3: Don't retrain too frequently
    if last_retrain < config.min_retrain_interval_hours {
        return Ok(false);
    }

    // Check 4: Force retrain if model too old
    if model_age.as_secs() > (config.max_model_age_days * 86400) {
        return Ok(true);
    }

    // Check 5: Count new segments since last training
    let new_segment_count = count_segments_since(
        db_path,
        model_metadata.modified()?,
    )?;

    if new_segment_count >= config.min_new_segments {
        return Ok(true);
    }

    Ok(false)
}

fn count_segments_since(db_path: &Path, since: SystemTime) -> Result<usize> {
    let conn = Connection::open(db_path)?;
    let since_timestamp = since
        .duration_since(UNIX_EPOCH)?
        .as_secs() as i64;

    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM segments WHERE timestamp > ?",
        params![since_timestamp],
        |row| row.get(0),
    )?;

    Ok(count)
}
```

### When Retrain Triggers

| Scenario | Action | Reason |
|----------|--------|--------|
| No model exists | ✅ Retrain | Initial training |
| Model > 24 hours old | ✅ Retrain | Freshness (even if 0 new segments) |
| 25+ new segments added | ✅ Retrain | Sufficient new data |
| < 6 hours since last retrain | ❌ Skip | Prevent thrashing |
| 10 segments in 3 hours | ❌ Skip | Below threshold + too recent |

### Example Timeline

```
Monday 9:00am - 10:00am: Dictation session (30 segments)
  → Retrain: No model exists
  → Training: Topics = {Coding, API Design}

Monday 2:00pm - 3:00pm: Email session (20 segments)
  → Skip: Only 4 hours since last retrain

Tuesday 9:00am - 10:00am: Meeting notes (40 segments)
  → Retrain: 24h old + 60 new segments total
  → Training: Topics = {Coding, API Design, Business Meetings}

Tuesday 3:00pm - 4:00pm: Code review (15 segments)
  → Skip: Only 6 hours + below threshold (15 < 25)

Wednesday 10:00am: Check (0 new segments)
  → Retrain: 25h old (force daily refresh)
  → Training: No new topics, confidence scores refined
```

## Integration Points

### 1. Daemon Startup Check

```rust
// In swictation-daemon/src/main.rs

async fn main() -> Result<()> {
    // ... existing initialization ...

    // Load or train context model
    let model_path = data_dir.join("context-model.json");
    let metrics_db = data_dir.join("metrics.db");

    let retrain_config = RetrainingConfig::default();

    let context_model = if should_retrain(&model_path, &metrics_db, &retrain_config)? {
        info!("Retraining context model...");
        let mut learner = ContextLearner::new(LearningConfig::default());
        let data = learner.load_training_data(&metrics_db, 6)?;
        let model = learner.train(&data)?;

        // Save model
        let model_json = serde_json::to_string_pretty(&model)?;
        fs::write(&model_path, model_json)?;

        info!("Context model retrained successfully");
        Some(model)
    } else {
        // Load existing model
        if model_path.exists() {
            info!("Loading existing context model");
            let model_json = fs::read_to_string(&model_path)?;
            Some(serde_json::from_str(&model_json)?)
        } else {
            None
        }
    };

    // ... rest of daemon startup ...
}
```

### 2. Background Retrain Task

```rust
// Optional: Check for retrain every hour in background
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour

    loop {
        interval.tick().await;

        if let Ok(true) = should_retrain(&model_path, &metrics_db, &retrain_config) {
            info!("Background retrain triggered");
            // Send signal to main thread to retrain
            retrain_tx.send(()).await.ok();
        }
    }
});
```

### 3. Manual Retrain Trigger

```rust
// In daemon IPC handlers
async fn handle_retrain_request(&self) -> Result<()> {
    info!("Manual retrain requested via UI");

    let mut learner = ContextLearner::new(LearningConfig::default());
    let data = learner.load_training_data(&self.metrics_db, 6)?;
    let model = learner.train(&data)?;

    // Save model
    let model_json = serde_json::to_string_pretty(&model)?;
    fs::write(&self.model_path, model_json)?;

    // Update in-memory model
    self.context_model.lock().await.replace(model);

    Ok(())
}
```

## Why Not Weekly?

**Weekly is too slow for these reasons:**

1. **Context shifts happen daily** - You might code Monday-Wednesday, write emails Thursday-Friday. Weekly retrain wouldn't capture this.

2. **Recent data is 3x more predictive** - Meta-level 2 learning showed recent segments (< 60 seconds) are significantly more predictive than older data. Weekly retrain ignores this recency bias.

3. **Training is instant** - With < 1 second training time, there's no performance penalty for frequent retraining.

4. **No resource cost** - Training is local (no network), uses minimal CPU (< 1 second), and minimal memory (~23 MB).

5. **Model size is tiny** - 10-100 KB JSON file, so frequent saves don't impact disk usage.

## Why Not More Frequent Than Daily?

**Daily + adaptive threshold strikes the right balance:**

1. **Prevents thrashing** - 6-hour minimum interval prevents retraining on every small session
2. **Captures intra-day shifts** - 25-segment threshold catches significant context changes within a day
3. **Maintains freshness** - 24-hour force ensures model never gets stale even with light usage
4. **Resource efficient** - Max 4 retrains/day (every 6 hours) is negligible overhead

## Configuration Tuning

Users might want different cadences based on usage patterns:

### Power User (Frequent, Diverse Topics)
```rust
RetrainingConfig {
    min_new_segments: 15,           // More sensitive
    max_model_age_days: 1,          // Daily refresh
    min_retrain_interval_hours: 4,  // Up to 6x/day
    auto_retrain: true,
}
```

### Light User (Infrequent, Consistent Topics)
```rust
RetrainingConfig {
    min_new_segments: 50,           // Less sensitive
    max_model_age_days: 3,          // 3-day refresh
    min_retrain_interval_hours: 24, // Max 1x/day
    auto_retrain: true,
}
```

### Manual Control
```rust
RetrainingConfig {
    auto_retrain: false,  // Disable automatic retraining
    // User triggers via UI button
}
```

## Storage Impact

**Daily retraining storage:**
- Model file: 10-100 KB (overwrites previous)
- No history kept (just current model)
- Total storage: Same as weekly (~50 KB)

**Model versioning (optional future enhancement):**
```
~/.local/share/swictation/
├── context-model.json          (current model)
└── context-models/
    ├── 2025-11-21.json        (daily snapshots)
    ├── 2025-11-20.json
    └── 2025-11-19.json
```

This would allow rollback if a bad training occurs, but adds storage overhead.

## Recommendation Summary

**Proposed Default**: Adaptive with daily cap
- Retrain every 25 new segments OR 24 hours (whichever comes first)
- Never more than once per 6 hours
- Auto-retrain enabled by default
- Manual trigger always available

**Reasoning**:
- Balances freshness (daily) with efficiency (segment threshold)
- Captures intra-day context shifts (25-segment threshold)
- Prevents thrashing (6-hour minimum interval)
- Zero user configuration needed (works well out of box)
- Power users can tune if desired

**Answer to "Is weekly too infrequent?"**: **Yes, weekly is too infrequent.** Daily with adaptive thresholds is the right cadence for context-aware learning.

---

**Next Steps**:
1. Implement `should_retrain()` logic in lib.rs
2. Add retrain check to daemon startup
3. Add manual retrain IPC handler
4. Document configuration in README
5. Test with real segment data when available
