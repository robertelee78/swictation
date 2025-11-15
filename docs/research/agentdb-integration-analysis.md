# AgentDB Integration Analysis for Swictation

**Analysis Date:** 2025-11-14
**Agent:** Analyst (Hive Mind Swarm)
**Project:** Swictation Voice Dictation System
**Objective:** Evaluate AgentDB integration opportunities for learning, optimization, and skill development

---

## Executive Summary

AgentDB presents **significant integration opportunities** for Swictation's voice dictation workflow, particularly in:
1. **Learning from user corrections** (reflexion-based improvement)
2. **Adaptive VAD threshold tuning** (causal analysis)
3. **Voice command pattern recognition** (vector search + skill library)
4. **User-specific accent/vocabulary adaptation** (episodic memory)

**Recommendation:** **Proceed with phased integration** - Start with reflexion logging for user corrections, expand to causal VAD tuning.

**Computational Overhead:** Low (~10-50ms per operation, mostly async). AgentDB uses SQLite + vector embeddings, which is lightweight compared to STT/VAD GPU inference.

---

## 1. Swictation Architecture Overview

### Current Build Process

**Rust Workspace:**
```toml
# rust-crates/Cargo.toml
[workspace]
members = [
    "swictation-audio",      # Audio capture (cpal/PipeWire)
    "swictation-stt",        # Speech-to-Text (Parakeet-TDT)
    "swictation-vad",        # Voice Activity Detection (Silero VAD v6)
    "swictation-daemon",     # Main orchestrator (tokio async)
    "swictation-metrics",    # Performance tracking
    "swictation-broadcaster",# Real-time metrics broadcast
]
```

**NPM Package:**
```json
// npm-package/package.json
{
  "name": "swictation",
  "version": "0.3.21",
  "scripts": {
    "postinstall": "node postinstall.js",  // Downloads AI models, GPU libs
    "test": "node test.js"
  }
}
```

**Build Script:**
```bash
# npm-package/build.sh
1. cargo build --release  (Rust daemon)
2. Build Tauri UI
3. Copy binaries to npm-package/bin/
4. Strip binaries for size reduction
5. npm pack (create tarball)
```

### Key Components

1. **swictation-daemon** - Main state machine (IDLE ↔ RECORDING)
2. **swictation-vad** - Silero VAD v6 (ONNX, threshold: 0.25)
3. **swictation-stt** - Parakeet-TDT (adaptive: 0.6B CPU/GPU or 1.1B GPU based on VRAM)
4. **swictation-audio** - cpal/PipeWire capture (lock-free ring buffer)
5. **midstream** - Text transformation (voice commands → symbols, currently 0 rules)

### Current Workflow

```
User Speech → Audio Capture → VAD Detection →
→ Silence Trigger (0.8s) → STT Transcription →
→ Text Transform → wtype Injection → Display
```

---

## 2. AgentDB Capabilities Analysis

### Available MCP Tools

AgentDB provides **27 MCP tools** across 7 categories:

#### **2.1 Vector Storage & Search**
- `mcp__agentdb__agentdb_init` - Initialize SQLite database with vector tables
- `mcp__agentdb__agentdb_insert` - Single vector insert with metadata
- `mcp__agentdb__agentdb_insert_batch` - Batch insert (parallel embedding generation)
- `mcp__agentdb__agentdb_search` - Semantic k-NN search (cosine similarity)
- `mcp__agentdb__agentdb_delete` - Delete vectors by ID or filters

**Performance:**
- Embedding generation: ~100ms per text (async)
- Vector search: ~10-50ms for k=10 results
- Storage: SQLite (low overhead, local file)

**Use Case for Swictation:**
```rust
// Store user dictation patterns for semantic search
// Example: "The user often dictates medical terms"
agentdb_insert(
  text: "User corrected 'inflammation' to 'inflamed mucosa'",
  metadata: { domain: "medical", correction: true },
  tags: ["vocabulary", "medical"]
)

// Later: Search for similar correction patterns
agentdb_search(
  query: "inflammation medical term",
  k: 5,
  min_similarity: 0.7
)
```

#### **2.2 Reflexion-Based Learning (Self-Critique)**
- `mcp__agentdb__reflexion_store` - Store episode with self-critique
- `mcp__agentdb__reflexion_retrieve` - Retrieve past episodes for learning

**Use Case for Swictation:**
```rust
// After user corrects transcription
reflexion_store(
  session_id: "user_dictation_20251114",
  task: "Transcribe medical dictation",
  input: "Audio segment with 'inflammation'",
  output: "inflammation",  // STT output
  reward: 0.0,  // User corrected it (failure)
  success: false,
  critique: "STT misheard medical term. Need medical vocabulary boost or user-specific adaptation."
)

// Before next similar task
reflexion_retrieve(
  task: "Transcribe medical dictation",
  k: 5,
  only_failures: true  // Learn from past mistakes
)
```

#### **2.3 Causal Reasoning**
- `mcp__agentdb__causal_add_edge` - Track cause → effect relationships
- `mcp__agentdb__causal_query` - Query causal effects

**Use Case for Swictation:**
```rust
// Track VAD threshold impact
causal_add_edge(
  cause: "VAD threshold 0.25",
  effect: "95% accurate silence detection",
  uplift: 0.15,  // 15% improvement over baseline
  confidence: 0.90,
  sample_size: 50  // 50 dictation sessions
)

// Query: "What VAD threshold gives best silence detection?"
causal_query(
  effect: "accurate silence detection",
  min_uplift: 0.10
)
```

#### **2.4 Skill Library**
- `mcp__agentdb__skill_create` - Create reusable skill
- `mcp__agentdb__skill_search` - Search for applicable skills

**Use Case for Swictation:**
```rust
// Store successful voice command patterns
skill_create(
  name: "punctuation_dictation",
  description: "Convert spoken punctuation to symbols",
  code: "fn transform(input: &str) -> String { ... }",
  success_rate: 0.92
)

// Search for skills when user says "comma"
skill_search(
  task: "Transform spoken punctuation",
  k: 10,
  min_success_rate: 0.80
)
```

#### **2.5 Reinforcement Learning (9 Algorithms)**
- `mcp__agentdb__learning_start_session` - Start RL session (q-learning, sarsa, dqn, ppo, etc.)
- `mcp__agentdb__learning_predict` - Get AI-recommended action
- `mcp__agentdb__learning_feedback` - Submit reward signal
- `mcp__agentdb__learning_train` - Train RL policy
- `mcp__agentdb__learning_metrics` - Get performance metrics

**Use Case for Swictation:**
```rust
// Learn optimal VAD parameters for different noise environments
learning_start_session(
  user_id: "user123",
  session_type: "actor-critic",
  config: {
    learning_rate: 0.01,
    discount_factor: 0.99
  }
)

// Get recommendation: "What VAD threshold for noisy office?"
learning_predict(
  session_id: "vad_tuning_session",
  state: "office environment, 65dB ambient noise"
)
// Returns: { action: "threshold_0.30", confidence: 0.87 }

// After dictation session, provide feedback
learning_feedback(
  session_id: "vad_tuning_session",
  state: "office environment, 65dB ambient noise",
  action: "threshold_0.30",
  reward: 0.85,  // 85% accurate silence detection
  success: true
)
```

#### **2.6 Experience Replay**
- `mcp__agentdb__experience_record` - Record tool execution as experience
- `mcp__agentdb__reward_signal` - Calculate reward for outcomes

**Use Case for Swictation:**
```rust
// Record every STT transcription as experience
experience_record(
  session_id: "user123_dictation",
  tool_name: "stt_transcribe",
  action: "Parakeet-TDT-1.1B, VAD threshold 0.25",
  outcome: "Transcribed 'Hello world' with 100% accuracy",
  reward: 1.0,
  success: true,
  latency_ms: 180,
  state_before: { vad_threshold: 0.25, noise_level: "low" },
  state_after: { text_injected: true, user_satisfied: true }
)
```

#### **2.7 Pattern Recognition**
- `mcp__agentdb__agentdb_pattern_store` - Store reasoning patterns
- `mcp__agentdb__agentdb_pattern_search` - Search patterns with embeddings
- `mcp__agentdb__agentdb_pattern_stats` - Pattern statistics

**Use Case for Swictation:**
```rust
// Store successful transcription patterns
agentdb_pattern_store(
  taskType: "medical_dictation",
  approach: "Use Parakeet-TDT-1.1B + medical vocabulary boost",
  successRate: 0.94,
  tags: ["medical", "specialized"]
)

// Search for patterns when handling new domain
agentdb_pattern_search(
  task: "legal dictation",
  k: 5,
  threshold: 0.70
)
```

---

## 3. Integration Opportunities

### **3.1 User Correction Learning (Reflexion)**

**Current Gap:** When users manually correct transcriptions, Swictation doesn't learn from these corrections.

**AgentDB Solution:**
```rust
// In swictation-daemon after user correction (detected via clipboard monitor)
impl SwictationDaemon {
    async fn handle_user_correction(&self, original: &str, corrected: &str) {
        reflexion_store(
            session_id: format!("user_{}_corrections", self.user_id),
            task: "STT transcription",
            input: original_audio_segment,
            output: original,
            reward: 0.0,  // Failure (user had to correct)
            success: false,
            critique: format!("STT output '{}' corrected to '{}'. Possible accent mismatch or vocabulary gap.", original, corrected)
        ).await;

        // Store correction pattern for future reference
        agentdb_insert(
            text: format!("User corrects '{}' to '{}'", original, corrected),
            metadata: {
                "type": "correction",
                "frequency": 1,
                "domain": self.detect_domain(&corrected)
            },
            tags: vec!["user_correction", "vocabulary"]
        ).await;
    }
}
```

**Benefits:**
- Build user-specific vocabulary database
- Identify systematic STT errors (e.g., accent mismatches)
- Enable personalized model fine-tuning suggestions

**Computational Overhead:** ~100ms per correction (async, non-blocking)

### **3.2 Adaptive VAD Threshold Tuning (Causal + RL)**

**Current Gap:** VAD threshold is hardcoded (0.25 in config). Different environments need different thresholds.

**AgentDB Solution:**
```rust
// Track causal relationships
impl VadDetector {
    async fn log_performance(&self, threshold: f32, accuracy: f32, environment: &str) {
        causal_add_edge(
            cause: format!("VAD threshold {} in {}", threshold, environment),
            effect: "silence detection accuracy",
            uplift: accuracy - 0.80,  // Baseline 80% accuracy
            confidence: 0.95,
            sample_size: self.sample_count
        ).await;
    }
}

// Use RL to recommend optimal threshold
impl SwictationDaemon {
    async fn optimize_vad_threshold(&mut self) {
        let environment = self.detect_environment();  // "quiet office", "noisy cafe", etc.

        let recommendation = learning_predict(
            session_id: "vad_optimization",
            state: environment
        ).await;

        self.vad.set_threshold(recommendation.action.threshold);
    }
}
```

**Benefits:**
- Environment-specific optimization (office vs. cafe vs. home)
- Continuous improvement through user feedback
- Automatic adaptation without manual config changes

**Computational Overhead:** ~20ms per prediction (cached after training)

### **3.3 Voice Command Pattern Library (Skills + Vector Search)**

**Current Gap:** MidStream text-transform has 0 rules (awaiting STT analysis). No learning mechanism for successful patterns.

**AgentDB Solution:**
```rust
// When user successfully uses a voice command
impl TextTransformer {
    async fn record_successful_transform(&self, input: &str, output: &str, success_rate: f32) {
        skill_create(
            name: format!("transform_{}", input.replace(" ", "_")),
            description: format!("Transform '{}' to '{}'", input, output),
            code: format!("rules.add(\"{}\", \"{}\")", input, output),
            success_rate
        ).await;

        // Also store in vector DB for semantic search
        agentdb_insert(
            text: format!("User says '{}' meaning '{}'", input, output),
            metadata: { "type": "voice_command", "category": "punctuation" },
            tags: vec!["transformation", "secretary_mode"]
        ).await;
    }

    // Search for similar patterns
    async fn find_similar_commands(&self, user_input: &str) -> Vec<VoiceCommand> {
        let results = agentdb_search(
            query: user_input,
            k: 5,
            min_similarity: 0.75
        ).await;

        // Convert to voice commands
        results.iter()
            .map(|r| VoiceCommand::from_metadata(&r.metadata))
            .collect()
    }
}
```

**Benefits:**
- Semantic search for voice commands ("full stop" finds "period", "dot")
- User-specific custom commands ("curly brace" vs. "curly bracket")
- Gradual library growth without hardcoding all patterns

**Computational Overhead:** ~30ms for semantic search (k=5)

### **3.4 Session Pattern Recognition**

**Current Gap:** No analysis of user dictation patterns (WPM, pause durations, error types).

**AgentDB Solution:**
```rust
// After each dictation session
impl SwictationDaemon {
    async fn analyze_session(&self, session: &DictationSession) {
        agentdb_pattern_store(
            taskType: "dictation_session",
            approach: format!(
                "WPM: {}, Avg pause: {}s, VAD threshold: {}, Environment: {}",
                session.words_per_minute,
                session.avg_pause_duration,
                session.vad_threshold,
                session.environment
            ),
            successRate: session.accuracy,
            tags: vec!["session_analysis", session.user_id.clone()]
        ).await;
    }

    // Find optimal session configuration for current context
    async fn optimize_for_context(&self, context: &str) -> SessionConfig {
        let patterns = agentdb_pattern_search(
            task: context,
            k: 3,
            threshold: 0.80
        ).await;

        SessionConfig::from_top_pattern(&patterns[0])
    }
}
```

**Benefits:**
- Identify user-specific optimal configurations
- Detect degradation patterns (fatigue, environmental changes)
- Benchmark against historical performance

**Computational Overhead:** ~40ms per session analysis (async)

---

## 4. Comparison with Alternatives

### Alternative 1: Custom SQLite Database

**Pros:**
- Full control over schema
- No external dependencies
- Lightweight

**Cons:**
- ❌ No vector embeddings (no semantic search)
- ❌ No RL algorithms built-in
- ❌ No causal reasoning primitives
- ❌ Manual implementation of reflexion patterns

**Verdict:** AgentDB provides **significant value-add** over raw SQLite.

### Alternative 2: Cloud-Based ML Platforms (Weights & Biases, MLflow)

**Pros:**
- Advanced visualizations
- Experiment tracking
- Model versioning

**Cons:**
- ❌ Privacy concerns (Swictation is 100% local)
- ❌ Network dependency
- ❌ Not designed for real-time inference
- ❌ Overkill for lightweight local learning

**Verdict:** **Incompatible** with Swictation's privacy-first architecture.

### Alternative 3: LangChain Memory

**Pros:**
- Vector search support
- LLM integration

**Cons:**
- ❌ Python-only (Swictation is pure Rust)
- ❌ Designed for chatbots, not STT
- ❌ No RL algorithms
- ❌ Heavier dependencies

**Verdict:** **Not suitable** for Rust-native Swictation.

### Alternative 4: Custom RL Framework (e.g., burn-rs, tch-rs)

**Pros:**
- Rust-native
- Full control

**Cons:**
- ❌ Requires deep RL expertise
- ❌ No built-in vector search
- ❌ Months of development time

**Verdict:** AgentDB provides **faster time-to-value**.

---

## 5. Performance Impact Analysis

### Computational Overhead

| Operation | Latency | Impact on Swictation |
|-----------|---------|----------------------|
| Vector insert (async) | ~100ms | **None** (background) |
| Vector search (k=10) | ~30ms | **Negligible** (0.8s VAD threshold >> 30ms) |
| RL prediction | ~20ms | **Negligible** (cached) |
| RL training | ~500ms | **None** (offline) |
| Reflexion store | ~100ms | **None** (async) |
| Causal edge insert | ~10ms | **Negligible** |

**Total Impact on Real-Time Path:** **< 50ms** (well within 800ms VAD silence threshold)

**Conclusion:** AgentDB operations are **asynchronous** and **non-blocking** for the critical STT pipeline.

### Memory Overhead

| Component | Memory |
|-----------|--------|
| SQLite database | ~50MB (grows with data) |
| Vector embeddings | ~1KB per entry |
| RL model state | ~10MB |
| **Total** | **~60MB** |

**Context:** Swictation uses **2.2GB VRAM + 150MB RAM**. AgentDB adds **+40% to RAM**, which is acceptable.

### Storage Growth

- **Per dictation session:** ~10 entries × 1KB = 10KB
- **Per month (daily use):** ~30 days × 10KB = 300KB
- **Per year:** ~3.6MB

**Conclusion:** Storage growth is **negligible**.

---

## 6. Integration Architecture

### Proposed Rust Integration

```toml
# rust-crates/swictation-daemon/Cargo.toml
[dependencies]
# ... existing dependencies ...

# AgentDB integration (hypothetical Rust client)
agentdb-client = "0.1"  # Rust client for AgentDB MCP
tokio = { version = "1.0", features = ["full"] }  # Already present
```

### MCP Communication Layer

Since AgentDB is MCP-based, we need an MCP client in Rust:

```rust
// rust-crates/swictation-daemon/src/agentdb_client.rs
use std::process::Command;
use serde_json::Value;

pub struct AgentDbClient {
    mcp_process: Child,
}

impl AgentDbClient {
    pub fn new() -> Result<Self> {
        // Spawn AgentDB MCP server as child process
        let child = Command::new("npx")
            .args(&["agentdb", "mcp", "start"])
            .spawn()?;

        Ok(Self { mcp_process: child })
    }

    pub async fn insert(&self, text: &str, metadata: Value) -> Result<()> {
        // Send MCP request via stdio
        self.send_mcp_request("agentdb_insert", json!({
            "text": text,
            "metadata": metadata
        })).await
    }

    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        self.send_mcp_request("agentdb_search", json!({
            "query": query,
            "k": k
        })).await
    }
}
```

### Alternative: Direct SQLite Access

For **lower latency**, bypass MCP and use AgentDB's SQLite database directly:

```rust
// Direct SQLite access (requires agentdb schema knowledge)
use rusqlite::{Connection, Result};

pub struct DirectAgentDb {
    conn: Connection,
}

impl DirectAgentDb {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    pub fn insert_vector(&self, text: &str, embedding: &[f32]) -> Result<()> {
        // Direct insert into AgentDB's vectors table
        self.conn.execute(
            "INSERT INTO vectors (text, embedding, metadata) VALUES (?1, ?2, ?3)",
            params![text, serde_json::to_vec(embedding)?, "{}"]
        )?;
        Ok(())
    }
}
```

**Trade-off:** Direct SQLite is faster but bypasses AgentDB's embedding generation. We'd need to generate embeddings ourselves (e.g., using sentence-transformers via Python subprocess or Rust ONNX).

---

## 7. Phased Rollout Plan

### Phase 1: Reflexion Logging (Week 1-2)
**Goal:** Start learning from user corrections

**Implementation:**
1. Add clipboard monitor to detect corrections
2. Log all corrections to AgentDB reflexion store
3. Build correction frequency dashboard

**Success Metrics:**
- 100+ corrections logged per week
- Identify top 10 systematic errors

### Phase 2: Causal VAD Tuning (Week 3-4)
**Goal:** Optimize VAD threshold per environment

**Implementation:**
1. Add environment detection (ambient noise level)
2. Log VAD threshold → accuracy causal edges
3. Query optimal threshold before each session

**Success Metrics:**
- 5+ different environments tracked
- 10% improvement in silence detection accuracy

### Phase 3: Voice Command Skills (Week 5-6)
**Goal:** Build semantic voice command library

**Implementation:**
1. Implement 10-20 basic MidStream rules
2. Log successful transformations as skills
3. Enable semantic search for commands

**Success Metrics:**
- 20+ voice commands in skill library
- 90%+ transformation accuracy

### Phase 4: RL Session Optimization (Week 7-8)
**Goal:** Learn optimal session configurations

**Implementation:**
1. Start RL session for session parameters
2. Predict optimal VAD + STT settings per context
3. Train on 50+ sessions

**Success Metrics:**
- 15% latency reduction
- 5% accuracy improvement

---

## 8. Risks & Mitigations

### Risk 1: MCP Process Overhead
**Impact:** High
**Likelihood:** Medium
**Mitigation:** Use direct SQLite access for hot paths, MCP for admin tasks

### Risk 2: SQLite Lock Contention
**Impact:** Medium
**Likelihood:** Low (WAL mode prevents most locks)
**Mitigation:** Use WAL mode, async writes

### Risk 3: Vector Search Accuracy
**Impact:** Medium
**Likelihood:** Low
**Mitigation:** Tune similarity thresholds, use min_similarity filters

### Risk 4: Privacy Concerns (Local Vector Embeddings)
**Impact:** Low (all local)
**Likelihood:** Low
**Mitigation:** Document privacy model, allow user deletion

---

## 9. Recommendations

### ✅ **Proceed with AgentDB Integration**

**Rationale:**
1. **Low overhead** (~50ms async ops, 60MB RAM)
2. **High value** (reflexion learning, causal tuning, skill library)
3. **Rust-compatible** (MCP or direct SQLite)
4. **Privacy-preserving** (all local)

### 🎯 **Prioritize:**
1. **Reflexion logging** (immediate value from user corrections)
2. **Causal VAD tuning** (biggest accuracy win)
3. **Voice command skills** (enables MidStream development)

### ⏸️ **Defer:**
- RL for full session optimization (complex, needs more data)
- Multi-model ensemble learning (overkill for current scope)

### 📊 **Success Criteria:**
- 10% accuracy improvement from VAD tuning
- 20+ voice commands learned organically
- 100+ user corrections logged per month
- Zero performance regression in STT latency

---

## 10. Next Steps

1. **Create Rust AgentDB client** (MCP wrapper or direct SQLite)
2. **Implement clipboard monitor** for correction detection
3. **Add reflexion logging** to swictation-daemon
4. **Build correction analysis dashboard** (Tauri UI)
5. **Test causal VAD tuning** on 3+ environments
6. **Document integration** for contributors

---

## Appendix A: AgentDB MCP Tool Reference

### Vector Search
- `agentdb_init` - Initialize database
- `agentdb_insert` - Single insert
- `agentdb_insert_batch` - Batch insert
- `agentdb_search` - Semantic search
- `agentdb_delete` - Delete vectors

### Reflexion
- `reflexion_store` - Store episode with critique
- `reflexion_retrieve` - Retrieve episodes

### Causal
- `causal_add_edge` - Add cause → effect
- `causal_query` - Query effects

### Skills
- `skill_create` - Create reusable skill
- `skill_search` - Search skills

### Reinforcement Learning
- `learning_start_session` - Start RL session
- `learning_predict` - Get action recommendation
- `learning_feedback` - Submit reward
- `learning_train` - Train policy
- `learning_metrics` - Get metrics
- `learning_transfer` - Transfer learning
- `learning_explain` - Explain recommendations

### Experience
- `experience_record` - Record tool execution
- `reward_signal` - Calculate rewards

### Patterns
- `agentdb_pattern_store` - Store reasoning patterns
- `agentdb_pattern_search` - Search patterns
- `agentdb_pattern_stats` - Pattern statistics

### Utilities
- `agentdb_stats` - Database statistics
- `agentdb_clear_cache` - Clear query cache

---

## Appendix B: Swictation Integration Points

### 1. User Correction Detection
**File:** `rust-crates/swictation-daemon/src/correction_monitor.rs` (new)

### 2. VAD Threshold Tuning
**File:** `rust-crates/swictation-vad/src/adaptive_threshold.rs` (new)

### 3. Voice Command Learning
**File:** `external/midstream/crates/text-transform/src/agentdb_skills.rs` (new)

### 4. Session Analysis
**File:** `rust-crates/swictation-metrics/src/agentdb_logger.rs` (new)

---

**Analysis Complete.**
**Status:** Ready for Hive Mind review and Coder implementation.
