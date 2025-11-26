# Tauri Socket Architecture - Visual Diagrams

## Component Diagram: Dual-Socket Architecture

```mermaid
graph TB
    subgraph "Daemon Process"
        D[Daemon State Machine]
        IPC[IPC Server<br/>swictation.sock]
        MB[Metrics Broadcaster<br/>swictation_metrics.sock]

        D -->|broadcast events| MB
        IPC -->|toggle commands| D
    end

    subgraph "Tauri UI Process"
        TM[Tray Menu]
        MS[MetricsSocket]
        FE[React Frontend]

        TM -->|click| SC[send_toggle_command]
        SC -->|{"action":"toggle"}| IPC
        MS -->|listen loop| MB
        MS -->|emit events| FE
    end

    subgraph "Socket Layer"
        S1["/run/user/1000/<br/>swictation.sock"]
        S2["/run/user/1000/<br/>swictation_metrics.sock"]

        IPC -.->|bind| S1
        MB -.->|bind| S2
        SC -.->|connect| S1
        MS -.->|connect| S2
    end

    style D fill:#4CAF50,color:#fff
    style MB fill:#2196F3,color:#fff
    style IPC fill:#FF9800,color:#fff
    style MS fill:#2196F3,color:#fff
    style SC fill:#FF9800,color:#fff
    style FE fill:#9C27B0,color:#fff
```

---

## Sequence Diagram: Toggle Recording Flow

```mermaid
sequenceDiagram
    actor User
    participant TM as Tray Menu
    participant SC as send_toggle_command()
    participant Sock as swictation.sock
    participant IPC as IPC Server
    participant D as Daemon

    User->>TM: Left click
    TM->>TM: emit("toggle-recording-requested")

    Note over SC,Sock: Phase 1: Connect to IPC Socket
    SC->>Sock: UnixStream::connect()
    Sock-->>SC: Connected

    Note over SC,IPC: Phase 2: Send JSON Command
    SC->>IPC: write_all(b'{"action":"toggle"}\n')
    SC->>IPC: flush()

    Note over IPC,D: Phase 3: Process Command
    IPC->>IPC: Parse JSON
    IPC->>D: daemon.toggle()
    D->>D: State transition
    D-->>IPC: Ok(())

    Note over IPC,SC: Phase 4: Response
    IPC-->>SC: JSON response
    SC-->>TM: Result<()>

    Note over D: Daemon broadcasts new state
    D->>MB: emit state_change event
```

---

## Sequence Diagram: Metrics Streaming Flow

```mermaid
sequenceDiagram
    participant D as Daemon
    participant MB as MetricsBroadcaster
    participant Sock as swictation_metrics.sock
    participant MS as MetricsSocket
    participant App as Tauri AppHandle
    participant FE as React Frontend

    Note over MS,Sock: Initialization
    MS->>Sock: UnixStream::connect()
    Sock-->>MS: Connected
    MS->>App: emit("metrics-connected", true)
    App-->>FE: Connection status

    loop Every transcription event
        Note over D,MB: Daemon generates event
        D->>MB: broadcast(MetricsEvent::Transcription)
        MB->>Sock: write_all(json + "\n")

        Note over MS,App: Tauri receives and processes
        Sock-->>MS: read_line()
        MS->>MS: parse JSON
        MS->>MS: match MetricsEvent variant
        MS->>App: emit("transcription", event)
        App-->>FE: Event with typed data
        FE->>FE: Update React state
    end

    Note over Sock,MS: Disconnection (e.g., daemon restart)
    Sock--xMS: Connection closed
    MS->>App: emit("metrics-connected", false)
    MS->>MS: sleep(5s) then reconnect
```

---

## Data Flow Diagram: Event Types

```mermaid
graph LR
    subgraph "Daemon Events"
        D[Daemon State Machine]
        D -->|session_start| E1[SessionStart Event]
        D -->|new transcription| E2[Transcription Event]
        D -->|state change| E3[StateChange Event]
        D -->|periodic update| E4[MetricsUpdate Event]
        D -->|session_end| E5[SessionEnd Event]
    end

    subgraph "MetricsBroadcaster"
        MB[Serialize to JSON]
        E1 --> MB
        E2 --> MB
        E3 --> MB
        E4 --> MB
        E5 --> MB
    end

    subgraph "Unix Socket"
        S[swictation_metrics.sock]
        MB -->|newline-delimited JSON| S
    end

    subgraph "MetricsSocket Parser"
        MS[Deserialize with Serde]
        S --> MS
        MS -->|SessionStart| T1[Emit session-start]
        MS -->|Transcription| T2[Emit transcription]
        MS -->|StateChange| T3[Emit state-change]
        MS -->|MetricsUpdate| T4[Emit metrics-update]
        MS -->|SessionEnd| T5[Emit session-end]
    end

    subgraph "Frontend Listeners"
        L1[useMetrics hook]
        T1 --> L1
        T2 --> L1
        T3 --> L1
        T4 --> L1
        T5 --> L1
        L1 --> F[React Component State]
    end

    style D fill:#4CAF50,color:#fff
    style MB fill:#2196F3,color:#fff
    style MS fill:#2196F3,color:#fff
    style L1 fill:#9C27B0,color:#fff
```

---

## Class Diagram: Socket Module Structure

```mermaid
classDiagram
    class MetricsSocket {
        -socket_path: String
        -connected: bool
        +new() MetricsSocket
        +connect() Result~MetricsSocket~
        +listen(AppHandle) Result~()~
        +send_toggle_command() Result~()~
        -connect_and_process(AppHandle) Result~()~
        -handle_event(AppHandle, MetricsEvent) Result~()~
    }

    class MetricsEvent {
        <<enumeration>>
        SessionStart
        SessionEnd
        StateChange
        Transcription
        MetricsUpdate
    }

    class SocketUtils {
        <<module>>
        +get_socket_dir() PathBuf
        +get_ipc_socket_path() PathBuf
        +get_metrics_socket_path() PathBuf
    }

    class IpcCommand {
        +action: String
        +parse(str) Result~IpcCommand~
        +to_command_type() Result~CommandType~
    }

    class CommandType {
        <<enumeration>>
        Toggle
        Status
        Quit
    }

    MetricsSocket --> MetricsEvent : deserializes
    MetricsSocket --> SocketUtils : uses paths
    IpcCommand --> CommandType : converts to

    note for MetricsSocket "Production implementation\nAsync Tokio UnixStream\nAutomatic reconnection"
    note for SocketUtils "Shared between daemon and UI\nXDG-compliant paths\nSecurity-focused"
```

---

## State Machine Diagram: MetricsSocket Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Disconnected: new()

    Disconnected --> Connecting: listen() called
    Connecting --> Connected: UnixStream::connect() succeeds
    Connecting --> Disconnected: Connection failed (retry after 5s)

    Connected --> Reading: emit("metrics-connected", true)
    Reading --> Parsing: read_line() returns data
    Parsing --> Emitting: serde_json::from_str() succeeds
    Emitting --> Reading: emit event to frontend

    Reading --> Disconnected: Connection closed or error
    Parsing --> Reading: Parse error (log warning)

    Disconnected --> [*]: Process exit

    note right of Connected
        Socket connection established
        Buffered reader created
        Frontend notified
    end note

    note right of Parsing
        Deserialize with custom deserializers:
        - flexible_number (f64/u64)
        - flexible_timestamp (u64/f64/string)
    end note
```

---

## Deployment Diagram: Socket Filesystem Layout

```
Linux (XDG_RUNTIME_DIR):
/run/user/1000/
├── swictation.sock           ← IPC commands (permissions: 0600)
└── swictation_metrics.sock   ← Metrics broadcast (permissions: 0600)

Linux (fallback):
~/.local/share/swictation/
├── swictation.sock           ← IPC commands (permissions: 0600)
└── swictation_metrics.sock   ← Metrics broadcast (permissions: 0600)

macOS:
~/Library/Application Support/swictation/
├── swictation.sock           ← IPC commands (permissions: 0600)
└── swictation_metrics.sock   ← Metrics broadcast (permissions: 0600)
```

### Security Model

```mermaid
graph TB
    subgraph "Process Isolation"
        D[Daemon<br/>UID 1000]
        U[Tauri UI<br/>UID 1000]
        C[CLI Tool<br/>UID 1000]
    end

    subgraph "Socket Permissions (0600)"
        S1[swictation.sock<br/>owner: UID 1000<br/>rw-------]
        S2[swictation_metrics.sock<br/>owner: UID 1000<br/>rw-------]
    end

    D -->|bind/listen| S1
    D -->|bind/broadcast| S2
    U -->|connect/write| S1
    U -->|connect/read| S2
    C -->|connect/write| S1

    X[Other Users<br/>UID ≠ 1000] -.x|permission denied| S1
    X -.x|permission denied| S2

    style D fill:#4CAF50,color:#fff
    style U fill:#2196F3,color:#fff
    style C fill:#FF9800,color:#fff
    style S1 fill:#F44336,color:#fff
    style S2 fill:#F44336,color:#fff
    style X fill:#9E9E9E,color:#fff
```

---

## Component Interaction: Full System Flow

```mermaid
graph TB
    subgraph "User Interactions"
        U1[Hotkey: Ctrl+Shift+D]
        U2[Tray Menu: Toggle Recording]
        U3[CLI: echo '{\"action\":\"toggle\"}' | nc]
    end

    subgraph "Daemon Process"
        HS[Hotkey Service]
        IPC[IPC Server]
        D[Daemon Core]
        T[Transcription Engine]
        MB[Metrics Broadcaster]
    end

    subgraph "Sockets"
        S1[swictation.sock]
        S2[swictation_metrics.sock]
    end

    subgraph "Tauri UI"
        TM[Tray Menu Handler]
        SC[send_toggle_command]
        MS[MetricsSocket]
        APP[AppHandle]
        FE[React Components]
    end

    U1 -->|keyboard event| HS
    U2 -->|click| TM
    U3 -->|JSON command| S1

    HS -->|toggle()| D
    TM -->|invoke| SC
    SC -->|write JSON| S1
    S1 -->|read| IPC
    IPC -->|toggle()| D

    D -->|state change| T
    T -->|transcription| D
    D -->|events| MB
    MB -->|broadcast| S2

    S2 -->|read lines| MS
    MS -->|emit events| APP
    APP -->|props| FE

    style D fill:#4CAF50,color:#fff
    style MB fill:#2196F3,color:#fff
    style MS fill:#2196F3,color:#fff
    style FE fill:#9C27B0,color:#fff
```

---

## Performance Characteristics

### Metrics Socket Performance

```
Throughput:
- Event rate: ~1-10 events/second (during active transcription)
- Latency: <5ms from daemon broadcast to frontend receive
- Bandwidth: ~100-500 bytes/event (JSON serialized)

Reconnection Strategy:
- Automatic retry every 5 seconds on disconnect
- No event loss (daemon buffers recent events)
- Frontend displays connection status in real-time

Concurrency:
- Single async task per socket connection
- Non-blocking reads with Tokio BufReader
- Events emitted via Tauri's thread-safe AppHandle
```

### IPC Socket Performance

```
Request/Response Pattern:
- Synchronous write + flush
- No response reading (fire-and-forget for toggle)
- Latency: <10ms for round-trip

Security:
- Owner-only permissions (0600)
- JSON-only protocol (no binary exploits)
- Input validation in daemon IPC handler

Scalability:
- Single connection per toggle request
- Connection pooling not needed (infrequent use)
- CLI tools can send commands concurrently
```

---

## Error Handling Flow

```mermaid
graph TB
    START[MetricsSocket::listen]

    START --> CONNECT{Connect to socket}
    CONNECT -->|Success| LOOP[Reading loop]
    CONNECT -->|Error| LOG1[Log error]
    LOG1 --> WAIT1[Sleep 5 seconds]
    WAIT1 --> CONNECT

    LOOP --> READ{Read line}
    READ -->|Success| PARSE{Parse JSON}
    READ -->|EOF| LOG2[Connection closed]
    READ -->|Error| LOG3[Read error]

    PARSE -->|Success| HANDLE{Handle event}
    PARSE -->|Error| WARN[Log warning]

    HANDLE -->|Success| LOOP
    HANDLE -->|Error| ERROR[Log error]
    ERROR --> LOOP

    WARN --> LOOP
    LOG2 --> EMIT1[emit metrics-connected false]
    LOG3 --> EMIT1
    EMIT1 --> WAIT1

    style START fill:#4CAF50,color:#fff
    style CONNECT fill:#2196F3,color:#fff
    style PARSE fill:#FF9800,color:#fff
    style HANDLE fill:#9C27B0,color:#fff
```

---

## Migration Path: Legacy to Modern

```mermaid
graph LR
    subgraph "Legacy Implementation (DEPRECATED)"
        L1[SocketConnection]
        L2[std::os::unix::net::UnixStream]
        L3[Arc Mutex Option UnixStream]
        L4[Manual thread spawning]
        L5[Generic serde_json::Value]
    end

    subgraph "Modern Implementation (PRODUCTION)"
        M1[MetricsSocket]
        M2[tokio::net::UnixStream]
        M3[Direct ownership]
        M4[Tokio async/await]
        M5[Strongly-typed MetricsEvent]
    end

    L1 -.->|replaced by| M1
    L2 -.->|migrated to| M2
    L3 -.->|simplified to| M3
    L4 -.->|refactored to| M4
    L5 -.->|improved to| M5

    style L1 fill:#F44336,color:#fff
    style L2 fill:#F44336,color:#fff
    style L3 fill:#F44336,color:#fff
    style L4 fill:#F44336,color:#fff
    style L5 fill:#F44336,color:#fff

    style M1 fill:#4CAF50,color:#fff
    style M2 fill:#4CAF50,color:#fff
    style M3 fill:#4CAF50,color:#fff
    style M4 fill:#4CAF50,color:#fff
    style M5 fill:#4CAF50,color:#fff
```

---

## References

- Architecture Analysis: `tauri-socket-architecture-analysis.md`
- ADR: `adr/ADR-003-remove-legacy-socket-implementation.md`
- Implementation: `tauri-ui/src-tauri/src/socket/metrics.rs`
- Daemon IPC: `rust-crates/swictation-daemon/src/ipc.rs`
