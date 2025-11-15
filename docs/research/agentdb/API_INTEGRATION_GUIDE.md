# AgentDB API Integration Guide

**Quick Reference for Swictation Integration**

## 1. Quick Start (5 Minutes)

```typescript
// 1. Initialize database
mcp__agentdb__agentdb_init({
  db_path: "./swictation_memory.db"
});

// 2. Store user preference
mcp__agentdb__agentdb_insert({
  text: "User prefers vertical splits for coding",
  session_id: "user-john",
  tags: ["workspace", "preference"],
  metadata: {
    window_type: "terminal",
    split_direction: "vertical",
    frequency: 0.85
  }
});

// 3. Recall relevant preferences
const prefs = await mcp__agentdb__agentdb_search({
  query: "how to split windows for coding",
  k: 5,
  filters: { session_id: "user-john" }
});
```

## 2. Swictation Use Cases

### Use Case 1: Per-User Workflow Learning

**Goal**: Learn each user's window management preferences

```typescript
// Store workflow pattern
async function recordWorkflowAction(userId: string, action: string, success: boolean) {
  await mcp__agentdb__experience_record({
    session_id: `user-${userId}`,
    tool_name: "sway-window-manager",
    action: action,
    outcome: success ? "Workflow completed efficiently" : "User reverted action",
    reward: success ? 0.9 : 0.3,
    success: success,
    state_before: { current_layout: "...", active_windows: [...] },
    state_after: { new_layout: "...", active_windows: [...] },
    latency_ms: performance.now()
  });
}

// Learn optimal actions
const session = await mcp__agentdb__learning_start_session({
  user_id: `user-${userId}`,
  session_type: "actor-critic",
  config: {
    learning_rate: 0.01,
    discount_factor: 0.95,
    exploration_rate: 0.15
  }
});

// Get recommendation
const recommendation = await mcp__agentdb__learning_predict({
  session_id: session.id,
  state: "User opened terminal and VSCode"
});

console.log("Suggested layout:", recommendation.recommended_action);
```

### Use Case 2: Tray Icon → Tauri UI Optimization

**Goal**: Learn when and how users prefer to launch Tauri UI

```typescript
// Store interaction episodes
async function recordTrayInteraction(userId: string, trigger: string, result: any) {
  await mcp__agentdb__reflexion_store({
    session_id: `tray-behavior-${userId}`,
    task: `Launch Tauri UI via ${trigger}`,
    input: JSON.stringify({
      time_of_day: new Date().getHours(),
      active_workspace: result.workspace,
      num_windows: result.windowCount
    }),
    output: result.success ? "UI launched successfully" : "Launch failed",
    reward: result.success ? 1.0 : 0.0,
    success: result.success,
    critique: result.success
      ? "Quick launch, no zombie process"
      : "Failed due to zombie process blocking - need process cleanup",
    latency_ms: result.launchTime
  });
}

// Learn from past failures
const similarFailures = await mcp__agentdb__reflexion_retrieve({
  task: "Launch Tauri UI via middle-click",
  only_failures: true,
  k: 5
});

console.log("Past failure patterns:", similarFailures);
```

### Use Case 3: Causal Workspace Optimization

**Goal**: Discover which workspace arrangements improve productivity

```typescript
// Record causal relationships
await mcp__agentdb__causal_add_edge({
  cause: "Split terminal vertically with VSCode",
  effect: "User switches workspaces 40% less",
  uplift: 0.40,
  confidence: 0.88,
  sample_size: 120
});

await mcp__agentdb__causal_add_edge({
  cause: "Auto-group related windows on same workspace",
  effect: "Task completion time reduced",
  uplift: 0.25,
  confidence: 0.92,
  sample_size: 200
});

// Query for productivity improvements
const productivityCauses = await mcp__agentdb__causal_query({
  effect: "reduced workspace switching",
  min_confidence: 0.8,
  min_uplift: 0.2,
  limit: 10
});

console.log("Actions that reduce distractions:", productivityCauses);
```

### Use Case 4: Skill Library for Window Patterns

**Goal**: Build reusable window layout templates

```typescript
// Store proven layout pattern
await mcp__agentdb__skill_create({
  name: "dev-environment-layout",
  description: "Standard development environment with terminal, editor, and browser",
  code: JSON.stringify({
    layout: "hsplit",
    windows: [
      { app: "terminal", position: "left", width: 0.3 },
      { app: "vscode", position: "center", width: 0.5 },
      { app: "firefox", position: "right", width: 0.2 }
    ]
  }),
  success_rate: 0.87
});

// Find applicable layouts
const layouts = await mcp__agentdb__skill_search({
  task: "Setup environment for web development",
  min_success_rate: 0.7,
  k: 5
});

console.log("Recommended layouts:", layouts);
```

## 3. Integration Patterns

### Pattern A: Background Learning Agent

```typescript
// Run in Tauri backend
class WorkflowLearningAgent {
  private sessionId: string;

  async init(userId: string) {
    await mcp__agentdb__agentdb_init({
      db_path: `~/.config/swictation/users/${userId}/memory.db`
    });

    const session = await mcp__agentdb__learning_start_session({
      user_id: userId,
      session_type: "ppo",
      config: { learning_rate: 0.005, discount_factor: 0.95 }
    });

    this.sessionId = session.id;
  }

  async observeWorkflow(state: string, action: string, outcome: any) {
    // Get prediction
    const prediction = await mcp__agentdb__learning_predict({
      session_id: this.sessionId,
      state
    });

    // Record feedback
    await mcp__agentdb__learning_feedback({
      session_id: this.sessionId,
      state,
      action,
      reward: outcome.userSatisfaction,
      success: outcome.completed,
      next_state: outcome.nextState
    });

    // Periodic training
    if (this.observationCount % 20 === 0) {
      await mcp__agentdb__learning_train({
        session_id: this.sessionId,
        epochs: 3,
        batch_size: 16
      });
    }
  }

  async getSuggestion(currentState: string) {
    const prediction = await mcp__agentdb__learning_predict({
      session_id: this.sessionId,
      state: currentState
    });

    return {
      action: prediction.recommended_action,
      confidence: prediction.confidence,
      alternatives: prediction.alternative_actions
    };
  }
}
```

### Pattern B: User Preference Sync

```typescript
// Sync preferences across devices
async function syncUserPreferences(userId: string, deviceId: string) {
  // Export from device A
  const preferences = await mcp__agentdb__agentdb_search({
    query: "",  // Get all
    k: 1000,
    filters: { session_id: `user-${userId}` }
  });

  // Import to device B
  await mcp__agentdb__agentdb_insert_batch({
    items: preferences.results.map(pref => ({
      text: pref.text,
      metadata: { ...pref.metadata, synced_from: deviceId },
      session_id: `user-${userId}`,
      tags: pref.tags
    })),
    batch_size: 100
  });
}
```

## 4. Performance Optimization

### Optimization 1: Lazy Loading

```typescript
// Only initialize AgentDB when needed
class LazyMemory {
  private initialized = false;

  async ensureInit() {
    if (!this.initialized) {
      await mcp__agentdb__agentdb_init({
        db_path: this.dbPath
      });
      this.initialized = true;
    }
  }

  async search(query: string) {
    await this.ensureInit();
    return mcp__agentdb__agentdb_search({ query, k: 10 });
  }
}
```

### Optimization 2: Batch Operations

```typescript
// Batch user interactions for efficiency
class InteractionBuffer {
  private buffer: any[] = [];
  private flushInterval = 60000; // 1 minute

  constructor() {
    setInterval(() => this.flush(), this.flushInterval);
  }

  record(interaction: any) {
    this.buffer.push(interaction);
    if (this.buffer.length >= 50) {
      this.flush();
    }
  }

  async flush() {
    if (this.buffer.length === 0) return;

    await mcp__agentdb__agentdb_insert_batch({
      items: this.buffer.map(i => ({
        text: i.description,
        metadata: i.metadata,
        session_id: i.userId,
        tags: i.tags
      })),
      batch_size: 50
    });

    this.buffer = [];
  }
}
```

### Optimization 3: Caching Layer

```typescript
// Cache frequent queries
class CachedMemory {
  private cache = new Map<string, any>();
  private cacheTTL = 300000; // 5 minutes

  async search(query: string) {
    const cacheKey = `search:${query}`;
    const cached = this.cache.get(cacheKey);

    if (cached && Date.now() - cached.timestamp < this.cacheTTL) {
      return cached.results;
    }

    const results = await mcp__agentdb__agentdb_search({
      query,
      k: 10
    });

    this.cache.set(cacheKey, {
      results,
      timestamp: Date.now()
    });

    return results;
  }
}
```

## 5. Error Handling

```typescript
async function safeMemoryOperation<T>(
  operation: () => Promise<T>,
  fallback: T
): Promise<T> {
  try {
    return await operation();
  } catch (error) {
    console.error("AgentDB operation failed:", error);

    // Auto-recover from database corruption
    if (error.message.includes("database disk image is malformed")) {
      console.log("Reinitializing database...");
      await mcp__agentdb__agentdb_init({ reset: true });
      return fallback;
    }

    // Return fallback for other errors
    return fallback;
  }
}

// Usage
const preferences = await safeMemoryOperation(
  () => mcp__agentdb__agentdb_search({ query: "...", k: 5 }),
  { results: [], count: 0 }
);
```

## 6. Privacy & Data Management

### Strategy 1: Per-User Isolation

```typescript
// Each user gets their own database
const userDbPath = `~/.config/swictation/users/${userId}/memory.db`;

await mcp__agentdb__agentdb_init({ db_path: userDbPath });

// All operations automatically scoped to this user
```

### Strategy 2: Data Retention Policy

```typescript
// Clean up old memories (GDPR compliance)
async function enforceRetentionPolicy(userId: string, retentionDays: number = 90) {
  const cutoffTimestamp = Date.now() - (retentionDays * 86400 * 1000);

  await mcp__agentdb__agentdb_delete({
    filters: {
      session_id: `user-${userId}`,
      before_timestamp: cutoffTimestamp
    }
  });

  console.log(`Deleted memories older than ${retentionDays} days`);
}
```

### Strategy 3: Data Export

```typescript
// Export user data for portability
async function exportUserData(userId: string) {
  const allMemories = await mcp__agentdb__agentdb_search({
    query: "",
    k: 10000,
    filters: { session_id: `user-${userId}` }
  });

  const exportData = {
    user_id: userId,
    export_date: new Date().toISOString(),
    memories: allMemories.results,
    metadata: {
      total_count: allMemories.count,
      version: "1.0"
    }
  };

  return JSON.stringify(exportData, null, 2);
}
```

## 7. Monitoring & Debugging

```typescript
// Health check
async function checkMemoryHealth() {
  const stats = await mcp__agentdb__agentdb_stats({ detailed: true });

  const health = {
    status: "healthy",
    issues: []
  };

  // Check for performance issues
  if (stats.performance.avg_search_time_ms > 100) {
    health.status = "degraded";
    health.issues.push({
      severity: "warning",
      message: `Slow search: ${stats.performance.avg_search_time_ms}ms`,
      recommendation: "Consider archiving old data or adding indexes"
    });
  }

  // Check for data growth
  if (stats.storage.database_size_mb > 500) {
    health.issues.push({
      severity: "info",
      message: `Large database: ${stats.storage.database_size_mb}MB`,
      recommendation: "Implement data retention policy"
    });
  }

  return health;
}

// Periodic health monitoring
setInterval(async () => {
  const health = await checkMemoryHealth();
  if (health.status !== "healthy") {
    console.warn("Memory health issues:", health.issues);
  }
}, 3600000); // Every hour
```

## 8. Testing Strategies

### Unit Test Example

```typescript
describe("AgentDB Integration", () => {
  beforeEach(async () => {
    await mcp__agentdb__agentdb_init({
      db_path: ":memory:", // In-memory for tests
      reset: true
    });
  });

  it("should store and retrieve user preferences", async () => {
    await mcp__agentdb__agentdb_insert({
      text: "User prefers dark theme",
      session_id: "test-user",
      tags: ["ui", "theme"]
    });

    const results = await mcp__agentdb__agentdb_search({
      query: "theme preference",
      k: 5,
      filters: { session_id: "test-user" }
    });

    expect(results.count).toBeGreaterThan(0);
    expect(results.results[0].text).toContain("dark theme");
  });

  it("should handle RL session lifecycle", async () => {
    const session = await mcp__agentdb__learning_start_session({
      user_id: "test-user",
      session_type: "q-learning",
      config: { learning_rate: 0.01, discount_factor: 0.9 }
    });

    expect(session.id).toBeDefined();

    await mcp__agentdb__learning_end_session({
      session_id: session.id
    });

    // Session should be persisted
    const metrics = await mcp__agentdb__learning_metrics({
      session_id: session.id
    });

    expect(metrics).toBeDefined();
  });
});
```

## 9. Migration Guide

### From LangChain Memory

```typescript
// Before (LangChain)
const memory = new BufferMemory();
await memory.saveContext(
  { input: "User likes vim" },
  { output: "Stored preference" }
);

// After (AgentDB)
await mcp__agentdb__agentdb_insert({
  text: "User likes vim editor",
  session_id: "user-123",
  tags: ["editor", "preference"],
  metadata: { confidence: 1.0 }
});
```

### From Redis Cache

```typescript
// Before (Redis)
await redis.setex(`user:${userId}:pref`, 3600, JSON.stringify(prefs));
const cached = await redis.get(`user:${userId}:pref`);

// After (AgentDB)
await mcp__agentdb__agentdb_insert({
  text: JSON.stringify(prefs),
  session_id: `user-${userId}`,
  tags: ["preferences"],
  metadata: { ttl: 3600 }
});

const results = await mcp__agentdb__agentdb_search({
  query: "preferences",
  filters: { session_id: `user-${userId}` },
  k: 1
});
```

## 10. Recommended Architecture for Swictation

```
┌─────────────────────────────────────────────────────────┐
│                    Swictation Tauri UI                  │
└───────────────┬─────────────────────────────────────────┘
                │
                ↓
┌───────────────────────────────────────────────────────────┐
│           Background Learning Service                     │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  WorkflowLearningAgent (per user)                   │  │
│  │  ├── AgentDB (local SQLite)                         │  │
│  │  ├── RL Session (actor-critic)                      │  │
│  │  └── Causal Memory Graph                            │  │
│  └─────────────────────────────────────────────────────┘  │
└────────────┬──────────────────────────────────────────────┘
             │
             ↓
┌────────────────────────────────────────────────────────────┐
│                  Sway IPC Interface                        │
│  ├── Window Events → AgentDB.experience_record             │
│  ├── Layout Changes → AgentDB.learning_feedback            │
│  └── User Actions → AgentDB.reflexion_store                │
└────────────────────────────────────────────────────────────┘
             │
             ↓
┌────────────────────────────────────────────────────────────┐
│                   Periodic Tasks                           │
│  ├── Every 20 actions: learning_train()                    │
│  ├── Every hour: learner_discover() (causal patterns)      │
│  ├── Daily: enforceRetentionPolicy() (cleanup)             │
│  └── Weekly: syncUserPreferences() (backup)                │
└────────────────────────────────────────────────────────────┘
```

### File Structure

```
swictation/
├── src/
│   ├── learning/
│   │   ├── workflow-agent.ts       # Main learning agent
│   │   ├── memory-manager.ts       # AgentDB wrapper
│   │   ├── causal-analyzer.ts      # Pattern discovery
│   │   └── preference-sync.ts      # Cross-device sync
│   └── ...
├── tests/
│   └── learning/
│       ├── workflow-agent.test.ts
│       └── memory-manager.test.ts
└── config/
    └── learning-config.json        # RL hyperparameters
```

## 11. Next Steps

1. **Week 1-2**: Implement basic memory storage for user preferences
2. **Week 3-4**: Add RL session for workflow optimization
3. **Week 5-6**: Integrate causal memory for pattern discovery
4. **Week 7-8**: Build UI for explaining learned patterns
5. **Week 9+**: Advanced features (transfer learning, federated learning)

---

**Reference Links**:
- Main Architecture Doc: `./TECHNICAL_ARCHITECTURE.md`
- AgentDB MCP Tools: See system context
- Swictation Issues: GitHub repo

**Contact**: Coder Agent (Hive Mind)
