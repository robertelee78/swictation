# Swictation UI - Architecture Design Document

## Overview

This document describes the architecture of the Swictation UI, a Tauri-based application that provides a modern interface for viewing dictation metrics, transcriptions, and system performance.

## Technology Stack

### Backend (Rust)
- **Tauri 1.5**: Application framework
- **tokio**: Async runtime for concurrent operations
- **rusqlite**: SQLite database interface
- **serde/serde_json**: Serialization/deserialization
- **chrono**: Date/time handling
- **dirs**: Cross-platform directory utilities

### Frontend (TypeScript/React)
- **React 18**: UI framework
- **TypeScript**: Type safety
- **Vite**: Build tool and dev server
- **Recharts**: Data visualization
- **date-fns**: Date formatting utilities

## Architecture Principles

1. **Separation of Concerns**: Clear boundaries between UI, business logic, and data access
2. **Type Safety**: Full TypeScript coverage on frontend, strong typing in Rust backend
3. **Reactive Data Flow**: Unidirectional data flow using React state management
4. **Async-First**: All I/O operations are asynchronous
5. **Error Handling**: Comprehensive error handling at all layers
6. **Performance**: Minimal memory footprint, efficient database queries

---

## Backend Architecture (Rust)

### Module Structure

```
src-tauri/src/
├── main.rs                 # Application entry point, Tauri setup
├── commands/               # Tauri command handlers (IPC layer)
│   ├── mod.rs
│   ├── session.rs          # Session-related commands
│   ├── metrics.rs          # Metrics query commands
│   └── transcription.rs    # Transcription commands
├── database/               # Database access layer
│   ├── mod.rs
│   ├── connection.rs       # Connection pool management
│   ├── schema.rs           # Database schema definitions
│   └── queries.rs          # SQL query implementations
├── socket/                 # Unix socket handling
│   ├── mod.rs
│   ├── client.rs           # Socket connection management
│   └── listener.rs         # Real-time metrics listener
├── models/                 # Data models
│   ├── mod.rs
│   ├── session.rs          # Session data structures
│   ├── metrics.rs          # Metrics data structures
│   └── transcription.rs    # Transcription data structures
└── utils/                  # Utility functions
    ├── mod.rs
    ├── error.rs            # Custom error types
    └── config.rs           # Configuration management
```

### Component Descriptions

#### 1. Commands Layer (`commands/`)
**Purpose**: Exposes Rust functions to the frontend via Tauri IPC

Key responsibilities:
- Validate incoming requests
- Call appropriate database/socket functions
- Transform data for frontend consumption
- Handle errors and return appropriate responses

**Example Commands**:
```rust
#[tauri::command]
async fn get_current_session(state: State<'_, AppState>) -> Result<Session, String>

#[tauri::command]
async fn get_metrics_history(
    start_time: i64,
    end_time: i64,
    state: State<'_, AppState>
) -> Result<Vec<Metrics>, String>

#[tauri::command]
async fn get_transcriptions(
    session_id: Option<String>,
    limit: usize,
    state: State<'_, AppState>
) -> Result<Vec<Transcription>, String>
```

#### 2. Database Layer (`database/`)
**Purpose**: Manages SQLite database connections and queries

**Key Components**:

- **connection.rs**: Connection pool management
  - Thread-safe connection pool
  - Lazy initialization
  - Connection validation

- **schema.rs**: Database schema definitions
  - Table creation and migrations
  - Index management

- **queries.rs**: SQL query implementations
  - Parameterized queries
  - Result mapping
  - Transaction management

**Database Schema**:
```sql
-- sessions table
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    total_words INTEGER DEFAULT 0,
    total_chars INTEGER DEFAULT 0,
    avg_wpm REAL DEFAULT 0.0,
    status TEXT DEFAULT 'active'
);

-- metrics table
CREATE TABLE metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    cpu_percent REAL,
    memory_mb REAL,
    gpu_percent REAL,
    vram_mb REAL,
    wpm REAL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- transcriptions table
CREATE TABLE transcriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL,
    word_count INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

#### 3. Socket Layer (`socket/`)
**Purpose**: Handles Unix socket communication for real-time metrics

**Key Components**:

- **client.rs**: Socket connection management
  - Connect to `/tmp/swictation_metrics.sock`
  - Reconnection logic
  - Message parsing

- **listener.rs**: Real-time metrics listener
  - Background task for continuous monitoring
  - Event emission to frontend
  - Buffer management

**Real-time Flow**:
```
Daemon → Unix Socket → Socket Listener → Event → Frontend
```

#### 4. Models Layer (`models/`)
**Purpose**: Define data structures used throughout the application

All models implement:
- `Serialize` + `Deserialize` for JSON conversion
- `Clone` for value passing
- Appropriate validation logic

**Example Models**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub total_words: u32,
    pub total_chars: u32,
    pub avg_wpm: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub id: i64,
    pub session_id: String,
    pub timestamp: i64,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub gpu_percent: Option<f64>,
    pub vram_mb: Option<f64>,
    pub wpm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    pub id: i64,
    pub session_id: String,
    pub timestamp: i64,
    pub text: String,
    pub confidence: Option<f64>,
    pub word_count: u32,
}
```

#### 5. Application State
**Purpose**: Manage shared state across commands

```rust
pub struct AppState {
    pub db_pool: Arc<Mutex<Connection>>,
    pub socket_client: Arc<Mutex<SocketClient>>,
    pub config: Config,
}
```

State is injected into commands via Tauri's state management system.

---

## Frontend Architecture (React/TypeScript)

### Component Structure

```
src/
├── main.tsx                # Application entry point
├── App.tsx                 # Root component with routing
├── components/             # UI components
│   ├── Layout/
│   │   ├── AppLayout.tsx       # Main layout wrapper
│   │   ├── Sidebar.tsx         # Navigation sidebar
│   │   └── SystemTray.tsx      # System tray menu
│   ├── LiveSession/
│   │   ├── LiveSessionView.tsx     # Main live session view
│   │   ├── MetricsChart.tsx        # Real-time metrics chart
│   │   ├── SessionInfo.tsx         # Current session info
│   │   └── TranscriptionFeed.tsx   # Live transcription feed
│   ├── History/
│   │   ├── HistoryView.tsx         # Session history view
│   │   ├── SessionList.tsx         # List of past sessions
│   │   ├── SessionDetail.tsx       # Detailed session view
│   │   └── MetricsGraph.tsx        # Historical metrics graph
│   └── Transcriptions/
│       ├── TranscriptionsView.tsx  # Transcriptions view
│       ├── TranscriptList.tsx      # List of transcriptions
│       ├── TranscriptSearch.tsx    # Search/filter interface
│       └── TranscriptExport.tsx    # Export functionality
├── hooks/                  # Custom React hooks
│   ├── useSocket.ts            # Real-time socket connection
│   ├── useMetrics.ts           # Metrics data fetching
│   ├── useSessions.ts          # Session management
│   └── useTranscriptions.ts    # Transcription operations
├── services/               # API service layer
│   ├── api.ts              # Tauri command wrappers
│   └── socket.ts           # Socket event handling
├── types/                  # TypeScript type definitions
│   ├── session.ts
│   ├── metrics.ts
│   └── transcription.ts
├── contexts/               # React contexts
│   ├── AppContext.tsx      # Global app state
│   └── ThemeContext.tsx    # Theme management
└── styles/                 # Global styles
    ├── index.css
    └── theme.css
```

### Component Descriptions

#### 1. Layout Components
**Purpose**: Provide consistent application structure

- **AppLayout.tsx**: Main layout wrapper with sidebar and content area
- **Sidebar.tsx**: Navigation between views (Live, History, Transcriptions)
- **SystemTray.tsx**: System tray integration and menu

#### 2. Live Session Components
**Purpose**: Display real-time dictation session data

- **LiveSessionView.tsx**: Container for all live session components
- **MetricsChart.tsx**: Real-time chart showing CPU, GPU, memory metrics
- **SessionInfo.tsx**: Current session statistics (WPM, word count, duration)
- **TranscriptionFeed.tsx**: Live scrolling transcription text

**Data Flow**:
```
useSocket() → Real-time events → State updates → Component re-renders
```

#### 3. History Components
**Purpose**: Browse and analyze past sessions

- **HistoryView.tsx**: Container with date range selector
- **SessionList.tsx**: Paginated list of sessions
- **SessionDetail.tsx**: Detailed metrics and transcriptions for a session
- **MetricsGraph.tsx**: Historical metrics visualization

**Data Flow**:
```
useSessions() → Fetch from DB → Cache → Display
```

#### 4. Transcriptions Components
**Purpose**: Search and export transcription data

- **TranscriptionsView.tsx**: Main container
- **TranscriptList.tsx**: Paginated transcription list
- **TranscriptSearch.tsx**: Search by text, date, session
- **TranscriptExport.tsx**: Export to TXT, JSON, CSV

### Custom Hooks

#### useSocket.ts
```typescript
export function useSocket() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    // Subscribe to socket events
    const unlisten = listen('metrics-update', (event) => {
      setMetrics(event.payload);
    });

    return () => unlisten();
  }, []);

  return { metrics, connected };
}
```

#### useMetrics.ts
```typescript
export function useMetrics(sessionId: string, timeRange: [Date, Date]) {
  const [metrics, setMetrics] = useState<Metrics[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      const data = await invoke('get_metrics_history', {
        startTime: timeRange[0].getTime(),
        endTime: timeRange[1].getTime()
      });
      setMetrics(data);
      setLoading(false);
    };

    fetchMetrics();
  }, [sessionId, timeRange]);

  return { metrics, loading };
}
```

### State Management

**Approach**: React Context API + Custom Hooks

- **Global State**: Managed via `AppContext` for theme, user preferences
- **Local State**: Component-level state for UI interactions
- **Server State**: Custom hooks for data fetching and caching

**Benefits**:
- No external dependencies (Redux, MobX)
- Type-safe with TypeScript
- Simple mental model
- Easy testing

---

## Data Flow

### 1. Initial Load
```
Frontend → invoke('get_current_session') → Backend Command
  ↓
Database Query → Return Session
  ↓
Frontend updates state → UI renders
```

### 2. Real-time Metrics
```
Daemon writes to socket → Socket Listener receives
  ↓
Emit event to frontend
  ↓
Frontend hook receives event → Update state → UI re-renders
```

### 3. Historical Data Query
```
User selects date range → invoke('get_metrics_history')
  ↓
Backend queries database with filters
  ↓
Return paginated results → Frontend caches → Display
```

---

## Error Handling

### Backend Error Strategy

1. **Custom Error Types**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Socket error: {0}")]
    Socket(String),

    #[error("Not found: {0}")]
    NotFound(String),
}
```

2. **Error Propagation**: Use `Result<T, AppError>` throughout
3. **Error Conversion**: Convert to `String` at command boundary

### Frontend Error Strategy

1. **Try-Catch**: Wrap all `invoke()` calls
2. **Error State**: Store errors in component state
3. **User Feedback**: Display user-friendly error messages
4. **Logging**: Log errors to console for debugging

---

## Performance Considerations

### Backend Optimizations

1. **Connection Pooling**: Reuse database connections
2. **Prepared Statements**: Cache query plans
3. **Indexing**: Index frequently queried columns (timestamp, session_id)
4. **Batch Operations**: Batch inserts for metrics
5. **Async I/O**: All I/O operations are non-blocking

### Frontend Optimizations

1. **Code Splitting**: Lazy load routes
2. **Memoization**: Use `React.memo()` for expensive components
3. **Virtual Scrolling**: For long transcription lists
4. **Debouncing**: Debounce search inputs
5. **Caching**: Cache fetched data to avoid redundant queries

---

## Security Considerations

1. **File System Access**: Restricted to `~/.local/share/swictation/` and socket path
2. **Input Validation**: Validate all user inputs on backend
3. **SQL Injection**: Use parameterized queries only
4. **Socket Authentication**: Verify socket ownership
5. **Error Messages**: Don't leak sensitive information in errors

---

## Testing Strategy

### Backend Tests
- Unit tests for database queries
- Integration tests for commands
- Socket connection tests

### Frontend Tests
- Component unit tests
- Hook tests
- Integration tests for data flow

---

## Deployment

### Build Process
```bash
npm install          # Install frontend dependencies
npm run build        # Build frontend
cd src-tauri
cargo build --release  # Build Rust backend
```

### Distribution
- Linux: AppImage, deb, rpm packages
- Tauri's built-in bundler handles packaging

---

## Future Enhancements

1. **Real-time Collaboration**: Multi-user session viewing
2. **Cloud Sync**: Optional cloud backup of transcriptions
3. **Plugin System**: Extensible transformation rules
4. **Analytics Dashboard**: Advanced metrics and insights
5. **Voice Commands**: Control UI via voice
6. **Dark Mode**: Theme switching
7. **Export Formats**: PDF, Word document export
8. **Search Indexing**: Full-text search with FTS5

---

## Appendix

### Development Setup
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Build for Production
```bash
npm run tauri:build
```

### Database Location
- Linux: `~/.local/share/swictation/metrics.db`
- Socket: `/tmp/swictation_metrics.sock`

---

**Last Updated**: 2025-11-07
**Version**: 0.1.0
**Status**: Initial Design
