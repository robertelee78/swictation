# Swictation UI - Architecture Diagrams

## System Context (C4 Level 1)

```
┌───────────────────────────────────────────────────────────────────┐
│                         System Context                             │
│                                                                    │
│  ┌──────────┐                                    ┌──────────────┐ │
│  │          │  Views metrics,                    │              │ │
│  │   User   │  transcriptions,                   │  Swictation  │ │
│  │          │  history                           │   Daemon     │ │
│  └────┬─────┘                                    │              │ │
│       │                                          └──────┬───────┘ │
│       │                                                 │         │
│       │  Uses UI                                        │ Writes  │
│       ▼                                                 │ metrics │
│  ┌─────────────────────────────────────┐               │ & data  │
│  │                                     │◄──────────────┘         │
│  │      Swictation UI (Tauri)         │                          │
│  │                                     │                          │
│  │  - Real-time metrics display        │                          │
│  │  - Historical session analysis      │                          │
│  │  - Transcription search & export    │                          │
│  │  - System tray integration          │                          │
│  │                                     │                          │
│  └──────────┬──────────────────────────┘                          │
│             │                                                     │
│             │ Reads from                                          │
│             ▼                                                     │
│  ┌──────────────────────────┐   ┌──────────────────────────┐    │
│  │   SQLite Database        │   │   Unix Socket            │    │
│  │  ~/.local/share/         │   │  /tmp/swictation_        │    │
│  │  swictation/metrics.db   │   │  metrics.sock            │    │
│  └──────────────────────────┘   └──────────────────────────┘    │
│                                                                    │
└───────────────────────────────────────────────────────────────────┘
```

## Container Diagram (C4 Level 2)

```
┌───────────────────────────────────────────────────────────────────────────┐
│                        Swictation UI Application                           │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │                     Frontend (React/TypeScript)                   │    │
│  │                                                                   │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │    │
│  │  │ Live Session │  │   History    │  │Transcriptions│          │    │
│  │  │     View     │  │     View     │  │     View     │          │    │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │    │
│  │         │                  │                  │                  │    │
│  │         └──────────────────┼──────────────────┘                  │    │
│  │                            │                                     │    │
│  │                            │ Tauri IPC (invoke)                  │    │
│  │                            ▼                                     │    │
│  └────────────────────────────┼─────────────────────────────────────┘    │
│                               │                                           │
│  ┌────────────────────────────┼─────────────────────────────────────┐    │
│  │           Backend (Rust)   │                                     │    │
│  │                            ▼                                     │    │
│  │  ┌─────────────────────────────────────────────────────────┐    │    │
│  │  │              Tauri Command Handlers                      │    │    │
│  │  │  - get_recent_sessions                                   │    │    │
│  │  │  - get_session_details                                   │    │    │
│  │  │  - search_transcriptions                                 │    │    │
│  │  │  - get_lifetime_stats                                    │    │    │
│  │  └───────────────┬───────────────────────────┬──────────────┘    │    │
│  │                  │                            │                   │    │
│  │                  │                            │                   │    │
│  │        ┌─────────▼──────────┐      ┌─────────▼────────┐          │    │
│  │        │   Database Module  │      │  Socket Client   │          │    │
│  │        │                    │      │                  │          │    │
│  │        │ - Connection pool  │      │ - Unix socket    │          │    │
│  │        │ - Query execution  │      │ - Event listener │          │    │
│  │        │ - Result mapping   │      │ - Auto-reconnect │          │    │
│  │        └─────────┬──────────┘      └─────────┬────────┘          │    │
│  │                  │                            │                   │    │
│  └──────────────────┼────────────────────────────┼───────────────────┘    │
│                     │                            │                        │
│                     ▼                            ▼                        │
│         ┌────────────────────┐      ┌────────────────────┐               │
│         │ SQLite Database    │      │  Unix Socket       │               │
│         │  metrics.db        │      │  /tmp/...sock      │               │
│         └────────────────────┘      └────────────────────┘               │
│                                                                            │
└───────────────────────────────────────────────────────────────────────────┘
```

## Component Diagram (C4 Level 3) - Backend

```
┌──────────────────────────────────────────────────────────────────────┐
│                     Rust Backend Components                          │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                     main.rs (Entry Point)                       │ │
│  │  - Initialize logging                                           │ │
│  │  - Create AppState                                              │ │
│  │  - Setup system tray                                            │ │
│  │  - Start socket listener                                        │ │
│  └───────────────────────┬────────────────────────────────────────┘ │
│                          │                                           │
│                          │ Manages                                   │
│                          ▼                                           │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │                      AppState                             │      │
│  │  - db: Mutex<Database>                                    │      │
│  │  - socket: Arc<SocketConnection>                          │      │
│  └─────┬──────────────────────────────────────────┬─────────┘      │
│        │                                           │                │
│        │                                           │                │
│        ▼                                           ▼                │
│  ┌──────────────────────┐             ┌────────────────────────┐   │
│  │  commands/           │             │  socket/               │   │
│  │  ├── mod.rs          │             │  └── mod.rs            │   │
│  │  └── (handlers)      │             │      └── SocketConn    │   │
│  │                      │             │                        │   │
│  │  Command Handlers:   │             │  - new()               │   │
│  │  - Validate input    │             │  - connect()           │   │
│  │  - Call DB/socket    │             │  - start_listener()    │   │
│  │  - Transform results │             │  - read_events()       │   │
│  │  - Return to frontend│             │  - toggle_recording()  │   │
│  └──────────┬───────────┘             └────────────┬───────────┘   │
│             │                                      │               │
│             │                                      │ Emits events  │
│             │                                      │ to frontend   │
│             ▼                                      │               │
│  ┌──────────────────────┐                         │               │
│  │  database/           │                         │               │
│  │  └── mod.rs          │                         │               │
│  │      └── Database    │                         │               │
│  │                      │                         │               │
│  │  Methods:            │                         │               │
│  │  - new()             │                         │               │
│  │  - get_recent_...()  │                         │               │
│  │  - get_session_...() │                         │               │
│  │  - search_...()      │                         │               │
│  │  - get_lifetime_...()│                         │               │
│  └──────────┬───────────┘                         │               │
│             │                                      │               │
│             │ Uses                                 │               │
│             ▼                                      │               │
│  ┌──────────────────────┐                         │               │
│  │  models/             │                         │               │
│  │  └── mod.rs          │                         │               │
│  │                      │                         │               │
│  │  Data Structures:    │◄────────────────────────┘               │
│  │  - SessionSummary    │  Shared types                           │
│  │  - TranscriptionRec  │                                         │
│  │  - LifetimeStats     │                                         │
│  │  - ConnectionStatus  │                                         │
│  │  - DaemonState       │                                         │
│  └──────────────────────┘                                         │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

## Component Diagram (C4 Level 3) - Frontend

```
┌──────────────────────────────────────────────────────────────────────┐
│                   React Frontend Components                          │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                     App.tsx (Root)                              │ │
│  │  - Navigation state                                             │ │
│  │  - Socket connection setup                                      │ │
│  │  - View routing                                                 │ │
│  └───────────────────────┬────────────────────────────────────────┘ │
│                          │                                           │
│                          │ Renders                                   │
│                          ▼                                           │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │                  View Components                          │      │
│  │                                                           │      │
│  │  ┌────────────────┐  ┌────────────┐  ┌────────────────┐ │      │
│  │  │ LiveSession    │  │  History   │  │ Transcriptions │ │      │
│  │  │     View       │  │    View    │  │      View      │ │      │
│  │  │                │  │            │  │                │ │      │
│  │  │ - Metrics      │  │ - Sessions │  │ - Search       │ │      │
│  │  │ - Session info │  │ - Details  │  │ - Export       │ │      │
│  │  │ - Live feed    │  │ - Graphs   │  │ - List         │ │      │
│  │  └───────┬────────┘  └─────┬──────┘  └───────┬────────┘ │      │
│  │          │                  │                  │          │      │
│  └──────────┼──────────────────┼──────────────────┼──────────┘      │
│             │                  │                  │                 │
│             │ Use hooks        │                  │                 │
│             ▼                  ▼                  ▼                 │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │                    Custom Hooks                           │      │
│  │                                                           │      │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │      │
│  │  │ useMetrics   │  │ useDatabase  │  │ useSocket    │   │      │
│  │  │              │  │              │  │              │   │      │
│  │  │ - Subscribe  │  │ - Query      │  │ - Connect    │   │      │
│  │  │ - Update     │  │ - Cache      │  │ - Listen     │   │      │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │      │
│  │         │                  │                  │           │      │
│  └─────────┼──────────────────┼──────────────────┼───────────┘      │
│            │                  │                  │                  │
│            │                  │                  │                  │
│            │ Call             │                  │ Listen to        │
│            ▼                  ▼                  ▼                  │
│  ┌──────────────────────────────────────────────────────────┐      │
│  │                  Tauri API Layer                          │      │
│  │                                                           │      │
│  │  invoke('get_recent_sessions', ...)                      │      │
│  │  invoke('search_transcriptions', ...)                    │      │
│  │  listen('metrics-update', handler)                       │      │
│  │  listen('socket-connected', handler)                     │      │
│  │                                                           │      │
│  └───────────────────────────┬───────────────────────────────┘      │
│                              │                                      │
│                              │ IPC                                  │
│                              ▼                                      │
│                      [ Rust Backend ]                               │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## Data Flow Diagrams

### 1. Application Startup

```
┌──────┐
│ User │
└───┬──┘
    │ Launches app
    ▼
┌────────────────────┐
│   Tauri Runtime    │
│   - Create window  │
└────────┬───────────┘
         │
         │ Initialize
         ▼
┌──────────────────────────────┐
│  main.rs                     │
│  1. Setup logging            │
│  2. Create Database conn     │
│  3. Create SocketClient      │
│  4. Start socket listener    │
│  5. Register commands        │
│  6. Show window              │
└────────┬─────────────────────┘
         │
         │ Ready
         ▼
┌──────────────────────────────┐
│  Frontend (React)            │
│  1. Render App.tsx           │
│  2. Setup socket listeners   │
│  3. Load initial data        │
│  4. Show LiveSession view    │
└──────────────────────────────┘
```

### 2. Real-time Metrics Flow

```
┌──────────────┐
│   Daemon     │  Writes metrics
└──────┬───────┘
       │
       │ JSON events
       ▼
┌────────────────────────┐
│  Unix Socket           │
│  /tmp/swictation_...   │
└────────┬───────────────┘
         │
         │ Read
         ▼
┌──────────────────────────┐
│  SocketConnection        │
│  - read_events()         │
│  - Parse JSON            │
└────────┬─────────────────┘
         │
         │ emit_all()
         ▼
┌──────────────────────────┐
│  Tauri Event System      │
└────────┬─────────────────┘
         │
         │ Event
         ▼
┌──────────────────────────┐
│  Frontend Hook           │
│  useMetrics()            │
└────────┬─────────────────┘
         │
         │ Update state
         ▼
┌──────────────────────────┐
│  Component Re-render     │
│  - Update chart          │
│  - Update stats          │
└──────────────────────────┘
```

### 3. Historical Data Query

```
┌──────┐
│ User │  Clicks "History"
└───┬──┘
    │
    ▼
┌──────────────────────┐
│  History View        │
│  - Select date range │
└───────┬──────────────┘
        │
        │ invoke('get_recent_sessions', limit)
        ▼
┌──────────────────────────┐
│  Command Handler         │
│  get_recent_sessions()   │
└───────┬──────────────────┘
        │
        │ Call
        ▼
┌──────────────────────────┐
│  Database                │
│  - Lock connection       │
│  - Execute query         │
│  - Map results           │
└───────┬──────────────────┘
        │
        │ Return Vec<SessionSummary>
        ▼
┌──────────────────────────┐
│  Command Handler         │
│  - Serialize to JSON     │
└───────┬──────────────────┘
        │
        │ IPC response
        ▼
┌──────────────────────────┐
│  Frontend Hook           │
│  - Deserialize           │
│  - Update state          │
└───────┬──────────────────┘
        │
        │ Render
        ▼
┌──────────────────────────┐
│  History View            │
│  - Display sessions      │
│  - Show charts           │
└──────────────────────────┘
```

### 4. Transcription Search

```
┌──────┐
│ User │  Types search query
└───┬──┘
    │
    ▼
┌──────────────────────────┐
│  Transcriptions View     │
│  - Input field           │
└───────┬──────────────────┘
        │
        │ Debounced invoke
        │ 'search_transcriptions'
        ▼
┌──────────────────────────┐
│  Command Handler         │
│  search_transcriptions() │
└───────┬──────────────────┘
        │
        │ Call
        ▼
┌──────────────────────────┐
│  Database                │
│  - LIKE query            │
│  - Order by timestamp    │
│  - Limit results         │
└───────┬──────────────────┘
        │
        │ Return results
        ▼
┌──────────────────────────┐
│  Frontend                │
│  - Update results state  │
│  - Highlight matches     │
└───────┬──────────────────┘
        │
        │ Display
        ▼
┌──────────────────────────┐
│  Results List            │
│  - Transcriptions        │
│  - Sessions              │
│  - Timestamps            │
└──────────────────────────┘
```

## Module Dependencies

### Backend (Rust)

```
main.rs
  ├─> commands/mod.rs
  │     ├─> models/mod.rs
  │     ├─> database/mod.rs
  │     └─> socket/mod.rs
  ├─> database/mod.rs
  │     ├─> rusqlite
  │     └─> models/mod.rs
  ├─> socket/mod.rs
  │     ├─> tauri::Manager
  │     └─> tokio
  ├─> models/mod.rs
  │     ├─> serde
  │     └─> chrono
  └─> utils/mod.rs
        └─> dirs
```

### Frontend (TypeScript)

```
App.tsx
  ├─> components/LiveSession.tsx
  │     ├─> hooks/useMetrics.ts
  │     └─> hooks/useSocket.ts
  ├─> components/History.tsx
  │     └─> hooks/useDatabase.ts
  ├─> components/Transcriptions.tsx
  │     └─> hooks/useDatabase.ts
  └─> types.ts

hooks/
  ├─> useMetrics.ts
  │     └─> @tauri-apps/api (listen)
  ├─> useSocket.ts
  │     └─> @tauri-apps/api (listen)
  └─> useDatabase.ts
        └─> @tauri-apps/api (invoke)
```

## Technology Stack Summary

| Layer              | Technology           | Purpose                        |
|--------------------|----------------------|--------------------------------|
| **Desktop Runtime**| Tauri 1.5           | Cross-platform app framework   |
| **Backend**        | Rust                | High-performance backend logic |
| **Database**       | rusqlite            | SQLite interface               |
| **IPC**            | Tauri IPC           | Frontend ↔ Backend communication|
| **Frontend**       | React 18 + TypeScript| UI framework                  |
| **Build Tool**     | Vite                | Fast dev server and bundler    |
| **Charts**         | Recharts            | Data visualization             |
| **State**          | React Hooks         | Local state management         |
| **Events**         | Tauri Events        | Real-time updates              |

## Security Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Context                          │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │               Frontend (Untrusted)                     │ │
│  │  - No direct file access                              │ │
│  │  - No direct socket access                            │ │
│  │  - No direct system access                            │ │
│  └────────────────────┬───────────────────────────────────┘ │
│                       │                                      │
│                       │ Tauri IPC (Validated)                │
│                       │                                      │
│  ┌────────────────────▼───────────────────────────────────┐ │
│  │               Backend (Trusted)                        │ │
│  │  - Validates all inputs                                │ │
│  │  - Parameterized queries                               │ │
│  │  - File access restricted by tauri.conf.json          │ │
│  │  - Socket access controlled                            │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

**Last Updated**: 2025-11-07
**Version**: 0.1.0
