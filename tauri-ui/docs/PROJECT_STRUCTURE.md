# Swictation UI - Project Structure

## Complete Directory Tree

```
/opt/swictation/tauri-ui/
│
├── docs/                              # Documentation
│   ├── ARCHITECTURE.md                # Detailed architecture design
│   ├── ARCHITECTURE_DIAGRAM.md        # Visual diagrams (C4 model)
│   └── PROJECT_STRUCTURE.md           # This file
│
├── public/                            # Static assets
│   └── icons/                         # Application icons
│       ├── 32x32.png
│       ├── 128x128.png
│       ├── 128x128@2x.png
│       ├── icon.icns                  # macOS
│       ├── icon.ico                   # Windows
│       └── icon.png                   # Linux
│
├── src/                               # Frontend source (React/TypeScript)
│   │
│   ├── main.tsx                       # Application entry point
│   ├── App.tsx                        # Root component
│   ├── index.css                      # Global styles
│   ├── types.ts                       # Shared type definitions
│   │
│   ├── components/                    # React components
│   │   ├── Layout/
│   │   │   ├── AppLayout.tsx          # Main layout wrapper
│   │   │   ├── Sidebar.tsx            # Navigation sidebar
│   │   │   └── SystemTray.tsx         # System tray menu component
│   │   │
│   │   ├── LiveSession/
│   │   │   ├── LiveSessionView.tsx    # Main live session container
│   │   │   ├── MetricsChart.tsx       # Real-time metrics chart
│   │   │   ├── SessionInfo.tsx        # Current session info panel
│   │   │   └── TranscriptionFeed.tsx  # Live transcription feed
│   │   │
│   │   ├── History/
│   │   │   ├── HistoryView.tsx        # Main history container
│   │   │   ├── SessionList.tsx        # List of past sessions
│   │   │   ├── SessionDetail.tsx      # Detailed session view
│   │   │   └── MetricsGraph.tsx       # Historical metrics graph
│   │   │
│   │   └── Transcriptions/
│   │       ├── TranscriptionsView.tsx # Main transcriptions container
│   │       ├── TranscriptList.tsx     # List of transcriptions
│   │       ├── TranscriptSearch.tsx   # Search/filter interface
│   │       └── TranscriptExport.tsx   # Export functionality
│   │
│   ├── hooks/                         # Custom React hooks
│   │   ├── useSocket.ts               # Real-time socket connection
│   │   ├── useMetrics.ts              # Metrics data fetching
│   │   ├── useSessions.ts             # Session management
│   │   └── useTranscriptions.ts       # Transcription operations
│   │
│   ├── services/                      # API service layer
│   │   ├── api.ts                     # Tauri command wrappers
│   │   └── socket.ts                  # Socket event handling
│   │
│   ├── types/                         # TypeScript type definitions
│   │   ├── session.ts                 # Session-related types
│   │   ├── metrics.ts                 # Metrics-related types
│   │   └── transcription.ts           # Transcription-related types
│   │
│   ├── contexts/                      # React contexts
│   │   ├── AppContext.tsx             # Global app state
│   │   └── ThemeContext.tsx           # Theme management
│   │
│   └── styles/                        # CSS modules/styles
│       ├── index.css                  # Global styles
│       └── theme.css                  # Theme variables
│
├── src-tauri/                         # Rust backend
│   │
│   ├── Cargo.toml                     # Rust dependencies
│   ├── tauri.conf.json                # Tauri configuration
│   ├── build.rs                       # Build script
│   │
│   ├── icons/                         # Application icons (linked)
│   │   └── (same as public/icons/)
│   │
│   └── src/                           # Rust source code
│       │
│       ├── main.rs                    # Application entry point
│       │                              # - Initialize logging
│       │                              # - Setup system tray
│       │                              # - Create AppState
│       │                              # - Register commands
│       │                              # - Start socket listener
│       │
│       ├── commands/                  # Tauri command handlers
│       │   ├── mod.rs                 # Command module exports
│       │   │                          # - get_recent_sessions
│       │   │                          # - get_session_details
│       │   │                          # - search_transcriptions
│       │   │                          # - get_lifetime_stats
│       │   │                          # - toggle_recording
│       │   │                          # - get_connection_status
│       │   │
│       │   ├── session.rs             # Session management commands
│       │   ├── metrics.rs             # Metrics query commands
│       │   └── transcription.rs       # Transcription commands
│       │
│       ├── database/                  # Database access layer
│       │   ├── mod.rs                 # Database module
│       │   │                          # - Database struct
│       │   │                          # - Connection management
│       │   │                          # - Query methods
│       │   │
│       │   ├── connection.rs          # Connection pool (future)
│       │   ├── schema.rs              # Schema definitions (future)
│       │   └── queries.rs             # Query implementations (future)
│       │
│       ├── socket/                    # Unix socket handling
│       │   ├── mod.rs                 # Socket module
│       │   │                          # - SocketConnection struct
│       │   │                          # - connect()
│       │   │                          # - start_listener()
│       │   │                          # - read_events()
│       │   │
│       │   ├── client.rs              # Socket client (future)
│       │   └── listener.rs            # Event listener (future)
│       │
│       ├── models/                    # Data models
│       │   ├── mod.rs                 # Model exports
│       │   │                          # - SessionSummary
│       │   │                          # - TranscriptionRecord
│       │   │                          # - LifetimeStats
│       │   │                          # - ConnectionStatus
│       │   │                          # - DaemonState
│       │   │
│       │   ├── session.rs             # Session models (future)
│       │   ├── metrics.rs             # Metrics models (future)
│       │   └── transcription.rs       # Transcription models (future)
│       │
│       └── utils/                     # Utility functions
│           ├── mod.rs                 # Utility exports
│           │                          # - get_default_db_path()
│           │                          # - get_default_socket_path()
│           │
│           ├── error.rs               # Error types (future)
│           └── config.rs              # Configuration (future)
│
├── .claude-flow/                      # Claude Flow configuration
│   └── (orchestration files)
│
├── package.json                       # Node.js dependencies
├── package-lock.json                  # Locked dependency versions
│
├── tsconfig.json                      # TypeScript configuration
├── tsconfig.node.json                 # TypeScript config for Node.js tools
│
├── vite.config.ts                     # Vite build configuration
├── .eslintrc.cjs                      # ESLint configuration
│
├── .gitignore                         # Git ignore patterns
└── README.md                          # Project README

```

## File Descriptions

### Root Configuration Files

| File                   | Purpose                                      |
|------------------------|----------------------------------------------|
| `package.json`         | Frontend dependencies and scripts            |
| `tsconfig.json`        | TypeScript compiler configuration            |
| `vite.config.ts`       | Vite bundler configuration                   |
| `.eslintrc.cjs`        | ESLint linting rules                         |
| `.gitignore`           | Files to exclude from version control        |

### Frontend Files (src/)

| File/Directory          | Purpose                                     |
|-------------------------|---------------------------------------------|
| `main.tsx`              | React application entry point               |
| `App.tsx`               | Root component with routing                 |
| `types.ts`              | Shared TypeScript type definitions          |
| `components/`           | Reusable React components                   |
| `hooks/`                | Custom React hooks for data/state           |
| `services/`             | API abstraction layer                       |
| `contexts/`             | React Context providers                     |
| `styles/`               | CSS modules and global styles               |

### Backend Files (src-tauri/)

| File/Directory          | Purpose                                     |
|-------------------------|---------------------------------------------|
| `Cargo.toml`            | Rust dependencies and metadata              |
| `tauri.conf.json`       | Tauri framework configuration               |
| `src/main.rs`           | Rust application entry point                |
| `src/commands/`         | Tauri IPC command handlers                  |
| `src/database/`         | SQLite database access layer                |
| `src/socket/`           | Unix socket client and listener             |
| `src/models/`           | Rust data structures                        |
| `src/utils/`            | Utility functions                           |

## Key Architectural Files

### 1. main.rs (Rust Backend Entry Point)

**Responsibilities:**
- Initialize logging system
- Load configuration
- Create database connection
- Create socket client
- Setup system tray menu
- Register Tauri commands
- Start socket listener task
- Launch Tauri application

**Key Code:**
```rust
fn main() {
    env_logger::init();

    let db = Database::new(db_path)?;
    let socket = SocketConnection::new(socket_path, app_handle);

    let state = AppState { db, socket };

    tauri::Builder::default()
        .manage(state)
        .system_tray(system_tray)
        .invoke_handler(tauri::generate_handler![...])
        .setup(|app| {
            socket.start_listener();
            Ok(())
        })
        .run(tauri::generate_context!())
}
```

### 2. App.tsx (Frontend Root Component)

**Responsibilities:**
- Manage navigation state
- Setup socket event listeners
- Provide routing between views
- Handle global state

**Key Code:**
```typescript
function App() {
  const [activeView, setActiveView] = useState('live');
  const { connected } = useSocket();

  return (
    <AppLayout>
      <Sidebar activeView={activeView} onChange={setActiveView} />
      <main>
        {activeView === 'live' && <LiveSessionView />}
        {activeView === 'history' && <HistoryView />}
        {activeView === 'transcriptions' && <TranscriptionsView />}
      </main>
    </AppLayout>
  );
}
```

### 3. Database Module (src-tauri/src/database/mod.rs)

**Responsibilities:**
- Open/manage SQLite connection
- Execute queries with proper error handling
- Map database rows to Rust structs
- Provide typed query methods

**Key Methods:**
```rust
impl Database {
    pub fn new(path: &Path) -> Result<Self>;
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<SessionSummary>>;
    pub fn get_session_transcriptions(&self, session_id: i64) -> Result<Vec<TranscriptionRecord>>;
    pub fn search_transcriptions(&self, query: &str, limit: usize) -> Result<Vec<TranscriptionRecord>>;
    pub fn get_lifetime_stats(&self) -> Result<LifetimeStats>;
}
```

### 4. Socket Module (src-tauri/src/socket/mod.rs)

**Responsibilities:**
- Connect to Unix socket
- Listen for events in background task
- Parse JSON messages
- Emit events to frontend
- Auto-reconnect on disconnect

**Key Methods:**
```rust
impl SocketConnection {
    pub fn new(socket_path: String, app_handle: AppHandle) -> Self;
    pub fn is_connected(&self) -> bool;
    fn connect(&self) -> Result<UnixStream>;
    pub async fn start_listener(self: Arc<Self>);
    fn read_events(&self, stream: &UnixStream) -> Result<()>;
}
```

### 5. Custom Hooks (Frontend)

**useMetrics.ts** - Fetch and cache metrics data
```typescript
export function useMetrics(sessionId: string, timeRange: [Date, Date]) {
  const [metrics, setMetrics] = useState<Metrics[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Fetch metrics from backend
  }, [sessionId, timeRange]);

  return { metrics, loading };
}
```

**useSocket.ts** - Listen to real-time socket events
```typescript
export function useSocket() {
  const [connected, setConnected] = useState(false);
  const [metrics, setMetrics] = useState<Metrics | null>(null);

  useEffect(() => {
    const unlisten = listen('metrics-update', (event) => {
      setMetrics(event.payload);
    });

    return () => unlisten();
  }, []);

  return { connected, metrics };
}
```

## Data Flow Summary

### 1. Frontend → Backend (Commands)

```
Component → Hook → invoke() → Tauri IPC → Command Handler → Database/Socket → Result → IPC → Hook → Component
```

### 2. Backend → Frontend (Events)

```
Socket → Listener → Parse → emit_all() → Tauri Event → listen() → Hook → Component
```

## Build Process

### Development

```bash
# Terminal 1: Frontend dev server
npm run dev

# Terminal 2: Tauri app with hot reload
npm run tauri:dev
```

### Production

```bash
# Build frontend and bundle with Tauri
npm run tauri:build

# Output locations:
# - src-tauri/target/release/swictation-ui (binary)
# - src-tauri/target/release/bundle/ (installers)
```

## Dependencies Summary

### Rust (Cargo.toml)

| Crate              | Version | Purpose                     |
|--------------------|---------|----------------------------|
| `tauri`            | 1.5     | Application framework      |
| `serde`            | 1.0     | Serialization              |
| `serde_json`       | 1.0     | JSON handling              |
| `tokio`            | 1.35    | Async runtime              |
| `rusqlite`         | 0.30    | SQLite interface           |
| `chrono`           | 0.4     | Date/time handling         |
| `dirs`             | 5.0     | Directory utilities        |
| `thiserror`        | 1.0     | Error handling             |
| `anyhow`           | 1.0     | Error context              |
| `log`              | 0.4     | Logging facade             |
| `env_logger`       | 0.11    | Logger implementation      |

### Frontend (package.json)

| Package                | Version | Purpose                    |
|------------------------|---------|----------------------------|
| `react`                | 18.2    | UI framework               |
| `react-dom`            | 18.2    | React DOM renderer         |
| `@tauri-apps/api`      | 1.5     | Tauri frontend API         |
| `recharts`             | 2.10    | Charts library             |
| `date-fns`             | 3.0     | Date utilities             |
| `typescript`           | 5.3     | TypeScript compiler        |
| `vite`                 | 5.0     | Build tool                 |
| `@vitejs/plugin-react` | 4.2     | Vite React plugin          |

## External Resources

### Database

- **Path**: `~/.local/share/swictation/metrics.db`
- **Type**: SQLite 3
- **Owner**: Swictation daemon
- **Access**: Read-only from UI

### Unix Socket

- **Path**: `/tmp/swictation_metrics.sock`
- **Type**: Unix domain socket
- **Protocol**: Line-delimited JSON
- **Owner**: Swictation daemon

## Future Enhancements

### Planned Module Additions

1. **src-tauri/src/database/connection.rs**
   - Connection pooling for concurrent queries
   - Connection validation and recycling

2. **src-tauri/src/database/schema.rs**
   - Schema version management
   - Migration support

3. **src-tauri/src/utils/config.rs**
   - User preferences storage
   - Configuration file management

4. **src/components/Settings/**
   - Settings view for user preferences
   - Database path configuration
   - Theme selection

5. **src/components/Charts/**
   - Reusable chart components
   - Chart configuration system

## Code Organization Principles

1. **Separation of Concerns**
   - Each module has a single responsibility
   - Clear boundaries between layers

2. **Type Safety**
   - Full TypeScript on frontend
   - Strong Rust types on backend
   - Shared type definitions via serde

3. **Error Handling**
   - Result types everywhere
   - Proper error propagation
   - User-friendly error messages

4. **Testing Strategy**
   - Unit tests for business logic
   - Integration tests for commands
   - Frontend component tests

5. **Documentation**
   - Code comments for complex logic
   - Module-level documentation
   - Architecture documentation (this)

---

**Last Updated**: 2025-11-07
**Version**: 0.1.0
