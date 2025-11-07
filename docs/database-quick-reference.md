# Swictation Metrics Database - Quick Reference

## 🚀 Quick Start

```rust
use swictation_metrics::MetricsDatabase;

// Initialize database
let db = MetricsDatabase::new("~/.local/share/swictation/metrics.db")?;

// Get recent sessions
let sessions = db.get_recent_sessions(10)?;

// Get session details
let transcriptions = db.get_session_segments(session_id)?;

// Search transcriptions
let results = db.search_transcriptions("query text", 20)?;

// Get lifetime stats
let stats = db.get_lifetime_stats()?;
```

## 📊 Core Functions

| Function | Purpose | Returns |
|----------|---------|---------|
| `get_recent_sessions(limit)` | Get N most recent sessions | `Vec<SessionMetrics>` |
| `get_session_segments(session_id)` | Get all transcriptions for session | `Vec<SegmentMetrics>` |
| `search_transcriptions(query, limit)` | Full-text search transcriptions | `Vec<SegmentMetrics>` |
| `get_lifetime_stats()` | Get aggregate statistics | `LifetimeMetrics` |
| `get_sessions_last_n_days(days)` | Get sessions from last N days | `Vec<SessionMetrics>` |
| `get_database_size_mb()` | Get database file size | `f64` |

## 🔧 Tauri Commands

```rust
#[tauri::command]
async fn get_recent_sessions(
    db: State<'_, DbState>,
    limit: usize,
) -> Result<Vec<SessionMetrics>, String> {
    db.db.get_recent_sessions(limit)
        .map_err(|e| e.to_string())
}
```

## 💾 Data Structures

### SessionMetrics (22 fields)
- **Identity**: `session_id`, `session_start`, `session_end`
- **Timing**: `total_duration_s`, `active_dictation_time_s`, `pause_time_s`
- **Content**: `words_dictated`, `characters_typed`, `segments_processed`
- **Performance**: `words_per_minute`, `average_latency_ms`, `median_latency_ms`, `p95_latency_ms`
- **Resources**: `gpu_memory_peak_mb`, `cpu_usage_mean_percent`

### SegmentMetrics (14 fields)
- **Identity**: `segment_id`, `session_id`, `timestamp`
- **Content**: `text`, `words`, `characters`, `duration_s`
- **Latency**: `vad_latency_ms`, `stt_latency_ms`, `total_latency_ms`, etc.

### LifetimeMetrics (22 fields)
- **Totals**: `total_words`, `total_sessions`, `total_dictation_time_minutes`
- **Averages**: `average_wpm`, `average_latency_ms`
- **Productivity**: `speedup_factor`, `estimated_time_saved_minutes`
- **Records**: `best_wpm_value`, `longest_session_words`, `lowest_latency_ms`

## 🎯 Frontend Integration (TypeScript)

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Get recent sessions
const sessions = await invoke<SessionMetrics[]>('get_recent_sessions', {
  limit: 10
});

// Get session transcriptions
const transcriptions = await invoke<SegmentMetrics[]>(
  'get_session_transcriptions',
  { sessionId: 123 }
);

// Search
const results = await invoke<SegmentMetrics[]>(
  'search_transcriptions',
  { query: "search term", limit: 20 }
);

// Lifetime stats
const stats = await invoke<LifetimeMetrics>('get_lifetime_stats');
```

## 📁 File Locations

- **Database Module**: `/opt/swictation/rust-crates/swictation-metrics/src/database.rs`
- **Data Models**: `/opt/swictation/rust-crates/swictation-metrics/src/models.rs`
- **Database File**: `~/.local/share/swictation/metrics.db`
- **Documentation**: `/opt/swictation/docs/rust-database-module.md`
- **Example Integration**: `/opt/swictation/docs/tauri-integration-example.rs`

## 🧪 Testing

```bash
cd /opt/swictation/rust-crates/swictation-metrics
cargo test         # Run all tests
cargo test --lib   # Run library tests only
```

**Test Coverage**: 15 tests passing
- ✅ Database creation and schema
- ✅ Session CRUD operations
- ✅ Segment insertion and retrieval
- ✅ Recent sessions query
- ✅ Session segments query
- ✅ Text search functionality
- ✅ Lifetime statistics
- ✅ Time-based queries
- ✅ Cleanup operations

## ⚡ Performance

- **Recent Sessions**: O(log n) with index
- **Session Segments**: O(log n) with index
- **Text Search**: O(n) with LIKE (upgradeable to FTS)
- **Lifetime Stats**: O(1) single-row lookup

## 🔒 Thread Safety

Uses `Arc<Mutex<Connection>>` for thread-safe database access:
- Safe for use in async Tauri handlers
- Multiple threads can access simultaneously
- Automatic mutex locking per operation

## 📦 Dependencies

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
anyhow = { workspace = true }
dirs = "5.0"
```

## 🎨 Example Usage Patterns

### Dashboard View
```rust
// Get recent 10 sessions with stats
let sessions = db.get_recent_sessions(10)?;
let stats = db.get_lifetime_stats()?;

// Display session list + aggregate metrics
```

### Session Detail View
```rust
// Get specific session + all transcriptions
let session = db.get_session(session_id)?;
let transcriptions = db.get_session_segments(session_id)?;

// Display session timeline with segments
```

### Search View
```rust
// Search across all transcriptions
let results = db.search_transcriptions(query, 50)?;

// Display matching segments with context
```

### Analytics View
```rust
// Get weekly performance
let week_sessions = db.get_sessions_last_n_days(7)?;
let stats = db.get_lifetime_stats()?;

// Calculate trends and visualize
```

## ✅ Production Ready

- ✅ Complete implementation
- ✅ Comprehensive tests
- ✅ Thread-safe design
- ✅ Error handling
- ✅ Documentation
- ✅ Type safety with serde
- ✅ Performance indexed
- ✅ Cross-platform compatible

## 🚧 Future Enhancements

1. **FTS Integration**: Upgrade text search with SQLite FTS5
2. **Connection Pooling**: Consider r2d2 for high-concurrency
3. **Streaming Results**: For large result sets
4. **Pagination**: Add offset/limit support
5. **Filtering**: Advanced filters for sessions/segments
6. **Export**: JSON/CSV export functionality

## 📚 Additional Resources

- **Full API Documentation**: `/opt/swictation/docs/rust-database-module.md`
- **Integration Example**: `/opt/swictation/docs/tauri-integration-example.rs`
- **Python Reference**: `/opt/swictation/src/metrics/database.py`
- **Database Schema**: See `init_schema()` in database.rs
