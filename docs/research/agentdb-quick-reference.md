# AgentDB Quick Reference Guide

## 🎯 What is AgentDB?

Lightning-fast vector database + AI agent memory system with:
- **150x faster** vector search (HNSW)
- **32x memory** compression (quantization)
- **9 RL algorithms** for learning
- **Reflexion** self-improvement
- **Causal reasoning** for WHY things work
- **Skill library** for reusable patterns

## 🚀 When to Use AgentDB

### ✅ Perfect For:
- Long-running agents that learn from experience
- Repetitive tasks with optimization potential
- CI/CD pipelines (learn from build successes/failures)
- Multi-session workflows needing memory
- Self-improving development systems
- Performance-critical applications

### ❌ Avoid For:
- Simple one-off tasks
- Tasks with no learning opportunity
- Real-time sub-millisecond requirements
- Massive scale (billions of vectors)

## 📦 29 MCP Tools (7 Systems)

### System 1: Core Database (6 tools)
```javascript
mcp__agentdb__agentdb_init         // Initialize DB
mcp__agentdb__agentdb_insert       // Insert single vector
mcp__agentdb__agentdb_insert_batch // Batch insert (fast)
mcp__agentdb__agentdb_search       // Semantic search
mcp__agentdb__agentdb_delete       // Delete vectors
mcp__agentdb__agentdb_stats        // Database metrics
```

### System 2: Reflexion Learning (2 tools)
```javascript
mcp__agentdb__reflexion_store      // Store episode with critique
mcp__agentdb__reflexion_retrieve   // Get past episodes
```

### System 3: Skill Management (2 tools)
```javascript
mcp__agentdb__skill_create         // Create reusable skill
mcp__agentdb__skill_search         // Find applicable skills
```

### System 4: Causal Memory (4 tools)
```javascript
mcp__agentdb__causal_add_edge      // Add cause → effect
mcp__agentdb__causal_query         // Query causality
mcp__agentdb__recall_with_certificate  // Get with provenance
mcp__agentdb__learner_discover     // Auto-discover patterns
```

### System 5: Reinforcement Learning (8 tools)
**9 Algorithms**: Q-Learning, SARSA, DQN, Policy-Gradient, Actor-Critic, PPO, Decision-Transformer, MCTS, Model-Based

```javascript
mcp__agentdb__learning_start_session   // Start RL session
mcp__agentdb__learning_predict         // Get recommendations
mcp__agentdb__learning_feedback        // Submit rewards
mcp__agentdb__learning_train           // Train policy
mcp__agentdb__learning_metrics         // Get analytics
mcp__agentdb__learning_transfer        // Transfer knowledge
mcp__agentdb__learning_explain         // Explain recommendations
mcp__agentdb__learning_end_session     // Save policy
```

### System 6: Experience Replay (2 tools)
```javascript
mcp__agentdb__experience_record    // Record tool execution
mcp__agentdb__reward_signal        // Calculate rewards
```

### System 7: Pattern Recognition (4 tools)
```javascript
mcp__agentdb__agentdb_pattern_store    // Store patterns
mcp__agentdb__agentdb_pattern_search   // Search patterns
mcp__agentdb__agentdb_pattern_stats    // Pattern analytics
mcp__agentdb__agentdb_clear_cache      // Clear cache
```

## 🔄 Integration Patterns

### Build Process Integration
```bash
# Pre-build
npx claude-flow hooks pre-task --description "Build optimization"
# Initialize AgentDB, load patterns, check causal edges

# During build
# Record metrics, performance data

# Post-build
npx claude-flow hooks post-task --task-id "build-123"
# Store episodes, update causality, discover patterns
```

### SAFLA Cycle (Self-Adaptive Feedback Loop)
```
STORE → EMBED → QUERY → RANK → LEARN → Repeat
```

Each cycle:
1. Store execution data
2. Embed to 1024-dim vectors
3. Query semantically (2-3ms)
4. Rank by: semantic + confidence + recency + diversity
5. Learn: +20% success, -15% failure (Bayesian)

### Example: Learning Build Optimization
```javascript
// 1. Initialize
await mcp__agentdb__agentdb_init({
  db_path: "./.agentdb/build-memory.db"
});

// 2. Record build episode
await mcp__agentdb__reflexion_store({
  session_id: "build-optimizer",
  task: "npm run build",
  reward: buildTime < 10000 ? 0.9 : 0.5,
  success: exitCode === 0,
  critique: "Analysis of what worked/failed",
  latency_ms: buildTime
});

// 3. Track causality
await mcp__agentdb__causal_add_edge({
  cause: "enabled webpack caching",
  effect: "faster build time",
  uplift: 0.35, // 35% improvement
  confidence: 0.85
});

// 4. Auto-discover patterns
await mcp__agentdb__learner_discover({
  min_attempts: 5,
  min_confidence: 0.7
});
```

## 🎓 Use Case Patterns

### Pattern 1: Development Assistant
```
Init → Search similar tasks → Retrieve skills →
Execute with learned params → Store results →
Update causality → Discover patterns
```

### Pattern 2: Build Optimizer
```
Pre-build causal check → Execute → Record metrics →
Post-build pattern update → Train RL policy
```

### Pattern 3: Research Agent
```
Query vector store → Investigate → Store episodes →
Build causal graph → Generate skills → Transfer learning
```

## 📊 Performance Benchmarks

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Pattern search | 15ms | 100µs | **150x** |
| Batch insert (100) | 1s | 2ms | **500x** |
| Large query (1M) | 100s | 8ms | **12,500x** |
| Memory usage | 3GB | 96MB | **32x** |

## 🧠 Key Concepts

### Reflexion Learning
- Agents **verbally reflect** on task feedback
- Maintain reflective text in episodic memory
- Induce better decision-making in subsequent trials
- Based on research: arxiv.org/abs/2303.11366

### Causal Reasoning
- Track **cause → effect** relationships
- Understand **WHY** actions work
- Build causal graphs from experience
- Auto-discover patterns in episode history

### ReasoningBank
- Transforms DB into **cognitive substrate**
- Combines vector + causal + episodic memory
- Self-learning with SAFLA architecture
- Local-first, cross-session persistence

### Skill Library
- **Reusable patterns** that improve over time
- Semantic search for applicable skills
- Track success rates
- Transfer knowledge between contexts

## 🔧 Configuration Tips

### RL Session Config
```javascript
{
  learning_rate: 0.01,      // Step size (0-1)
  discount_factor: 0.99,    // Future reward weight (0-1)
  exploration_rate: 0.1,    // Epsilon-greedy (0-1)
  batch_size: 32,           // Training batch size
  target_update_frequency: 100  // DQN target updates
}
```

### Search Parameters
```javascript
{
  k: 10,                    // Number of results
  min_similarity: 0.7,      // Threshold (0-1)
  filters: {
    session_id: "...",
    tags: ["tag1", "tag2"],
    min_reward: 0.5
  }
}
```

## 💡 Best Practices

1. **Initialize once** per session (`.agentdb/` folder)
2. **Batch operations** when possible (500x faster)
3. **Use semantic search** (not exact matching)
4. **Track causality** to understand WHY
5. **Store episodes** with meaningful critiques
6. **Discover patterns** periodically (min 5 attempts)
7. **Transfer learning** between similar tasks
8. **Clear cache** when data changes significantly

## 🌐 Resources

- Official Site: https://agentdb.ruv.io/
- Integration: Claude Flow hooks system
- Paper: ReasoningBank (arxiv.org/abs/2509.25140)
- Reflexion: arxiv.org/abs/2303.11366
- Storage: SQLite-based, portable
- Runtime: Node.js, browser (WebAssembly), edge

## 📝 Quick Start

```javascript
// 1. Initialize
const db = await mcp__agentdb__agentdb_init({
  db_path: "./.agentdb/memory.db"
});

// 2. Insert knowledge
await mcp__agentdb__agentdb_insert({
  text: "Webpack caching improves build times",
  tags: ["build", "optimization"],
  metadata: { source: "experiment-2025" }
});

// 3. Search semantically
const results = await mcp__agentdb__agentdb_search({
  query: "how to speed up builds",
  k: 5,
  min_similarity: 0.7
});

// 4. Start learning
await mcp__agentdb__learning_start_session({
  user_id: "dev-123",
  session_type: "q-learning",
  config: {
    learning_rate: 0.01,
    discount_factor: 0.99
  }
});
```

---

**Remember**: AgentDB excels at making systems **smarter over time** through multi-modal learning, causal reasoning, and experience accumulation.
