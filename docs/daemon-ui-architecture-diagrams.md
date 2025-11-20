# Swictation Daemon-UI Architecture Diagrams

**Companion to:** `daemon-ui-communication-architecture.md`
**Date:** 2025-11-20

---

## 1. System Overview - Dual-Socket Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      SWICTATION SYSTEM                          │
└─────────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────────┐
│                    Operating System (Linux)                       │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                  systemd (Process Manager)                  │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                     │
│                              ▼                                     │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │          SWICTATION DAEMON (Independent Process)            │  │
│  │  ┌────────────────────────────────────────────────────────┐ │  │
│  │  │  Core Pipeline (Audio → VAD → STT → DB)               │ │  │
│  │  └────────────────────────────────────────────────────────┘ │  │
│  │  ┌────────────────────────────────────────────────────────┐ │  │
│  │  │  IPC Layer (Unix Domain Sockets)                      │ │  │
│  │  │  ┌──────────────────┐  ┌──────────────────────────┐   │ │  │
│  │  │  │ Metrics Socket   │  │  Command Socket         │   │ │  │
│  │  │  │ (Broadcast)      │  │  (Request-Response)     │   │ │  │
│  │  │  │                  │  │                          │   │ │  │
│  │  │  │ Path:            │  │  Path:                   │   │ │  │
│  │  │  │ /tmp/swictation_ │  │  /tmp/swictation_ipc.sock│   │ │  │
│  │  │  │      metrics.sock│  │                          │   │ │  │
│  │  │  │                  │  │  Permissions: 0600       │   │ │  │
│  │  │  │ Permissions:0600 │  │                          │   │ │  │
│  │  │  │                  │  │  Protocol: JSON          │   │ │  │
│  │  │  │ Protocol:        │  │  Commands:               │   │ │  │
│  │  │  │  JSON Lines      │  │   - toggle               │   │ │  │
│  │  │  │                  │  │   - status               │   │ │  │
│  │  │  │ Events:          │  │   - quit                 │   │ │  │
│  │  │  │  - session_start │  │                          │   │ │  │
│  │  │  │  - session_end   │  └──────────────────────────┘   │ │  │
│  │  │  │  - state_change  │                                 │ │  │
│  │  │  │  - transcription │                                 │ │  │
│  │  │  │  - metrics_update│                                 │ │  │
│  │  │  └──────────────────┘                                 │ │  │
│  │  └────────────────────────────────────────────────────────┘ │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                     │
│                ┌─────────────┴────────────────┐                    │
│                │                              │                    │
│                ▼                              ▼                    │
│  ┌──────────────────────────┐  ┌──────────────────────────────┐   │
│  │   QT TRAY UI (Client 1)  │  │   TAURI UI (Client 2)        │   │
│  │  ┌────────────────────┐  │  │  ┌────────────────────────┐  │   │
│  │  │  Unix Socket Client│  │  │  │  Unix Socket Client    │  │   │
│  │  │  - Auto-reconnect  │  │  │  │  - Auto-reconnect      │  │   │
│  │  │  - Event listener  │  │  │  │  - Event listener      │  │   │
│  │  │  - Catch-up on join│  │  │  │  - Catch-up on join    │  │   │
│  │  └────────────────────┘  │  │  └────────────────────────┘  │   │
│  │  ┌────────────────────┐  │  │  ┌────────────────────────┐  │   │
│  │  │  UI Components     │  │  │  │  React UI              │  │   │
│  │  │  - System tray icon│  │  │  │  - Live Session View   │  │   │
│  │  │  - Status indicator│  │  │  │  - Metrics Dashboard   │  │   │
│  │  │  - Quick controls  │  │  │  │  - Session History     │  │   │
│  │  └────────────────────┘  │  │  └────────────────────────┘  │   │
│  └──────────────────────────┘  └──────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
```

---

## 2. Metrics Broadcasting Flow (Daemon → UI)

```
TIME
  │
  ├──► Daemon starts recording
  │    ┌─────────────────────────────────────────┐
  │    │ 1. daemon.start_recording()             │
  │    │    - Update state: Recording            │
  │    │    - Start session in DB                │
  │    │    - Get session_id: 123                │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Broadcast session_start event
  │    ┌─────────────────────────────────────────┐
  │    │ 2. broadcaster.start_session(123)       │
  │    │    - Clear transcription buffer         │
  │    │    - Broadcast to all clients:          │
  │    │      {"type":"session_start",           │
  │    │       "session_id":123,                 │
  │    │       "timestamp":1699000000.0}         │
  │    └─────────────────────────────────────────┘
  │                    │
  │         ┌──────────┴──────────┐
  │         ▼                     ▼
  │    ┌─────────┐          ┌─────────┐
  │    │ Client 1│          │ Client 2│
  │    │ (QT)    │          │ (Tauri) │
  │    └─────────┘          └─────────┘
  │
  ├──► Audio processing produces transcription
  │    ┌─────────────────────────────────────────┐
  │    │ 3. pipeline.on_transcription()          │
  │    │    - Text: "Hello world"                │
  │    │    - WPM: 145.2                         │
  │    │    - Latency: 234ms                     │
  │    │    - Save to database                   │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Broadcast transcription event
  │    ┌─────────────────────────────────────────┐
  │    │ 4. broadcaster.add_transcription(...)   │
  │    │    - Add to in-memory buffer            │
  │    │    - Broadcast to all clients:          │
  │    │      {"type":"transcription",           │
  │    │       "text":"Hello world",             │
  │    │       "timestamp":"14:23:15",           │
  │    │       "wpm":145.2,                      │
  │    │       "latency_ms":234.5,               │
  │    │       "words":2}                        │
  │    └─────────────────────────────────────────┘
  │                    │
  │         ┌──────────┴──────────┐
  │         ▼                     ▼
  │    ┌─────────┐          ┌─────────┐
  │    │ Client 1│          │ Client 2│
  │    │ Updates │          │ Updates │
  │    │ UI      │          │ UI      │
  │    └─────────┘          └─────────┘
  │
  ├──► Periodic metrics update (every 1 second)
  │    ┌─────────────────────────────────────────┐
  │    │ 5. broadcaster.update_metrics(...)      │
  │    │    - Current state: Recording           │
  │    │    - Session stats (WPM, words, etc.)   │
  │    │    - System metrics (GPU, CPU)          │
  │    │    - Broadcast to all clients:          │
  │    │      {"type":"metrics_update",          │
  │    │       "state":"recording",              │
  │    │       "wpm":145.2,                      │
  │    │       "gpu_memory_mb":1823.4, ...}      │
  │    └─────────────────────────────────────────┘
  │                    │
  │         ┌──────────┴──────────┐
  │         ▼                     ▼
  │    ┌─────────┐          ┌─────────┐
  │    │ Client 1│          │ Client 2│
  │    │ Updates │          │ Updates │
  │    │ Metrics │          │ Metrics │
  │    └─────────┘          └─────────┘
  │
  ├──► User stops recording
  │    ┌─────────────────────────────────────────┐
  │    │ 6. daemon.stop_recording()              │
  │    │    - Update state: Idle                 │
  │    │    - Finalize session in DB             │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Broadcast session_end event
  │    ┌─────────────────────────────────────────┐
  │    │ 7. broadcaster.end_session(123)         │
  │    │    - Keep buffer visible                │
  │    │    - Broadcast to all clients:          │
  │    │      {"type":"session_end",             │
  │    │       "session_id":123,                 │
  │    │       "timestamp":1699000000.0}         │
  │    └─────────────────────────────────────────┘
  │                    │
  │         ┌──────────┴──────────┐
  │         ▼                     ▼
  │    ┌─────────┐          ┌─────────┐
  │    │ Client 1│          │ Client 2│
  │    │ Updates │          │ Updates │
  │    │ UI      │          │ UI      │
  │    └─────────┘          └─────────┘
  ▼
```

---

## 3. Client Catch-Up Protocol (New Client Joins)

```
┌─────────────────────────────────────────────────────────────────┐
│                    New Client Connects                          │
└─────────────────────────────────────────────────────────────────┘

TIME
  │
  ├──► Daemon is already recording
  │    ┌─────────────────────────────────────────┐
  │    │ Current State:                          │
  │    │  - State: Recording                     │
  │    │  - Session ID: 123                      │
  │    │  - Buffer: ["Hello", "world", "test"]   │
  │    └─────────────────────────────────────────┘
  │
  ├──► New client (Tauri UI) starts
  │    ┌─────────────────────────────────────────┐
  │    │ 1. Tauri UI connects to socket          │
  │    │    - UnixStream::connect()              │
  │    │    - Connection accepted                │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Daemon detects new connection
  │    ┌─────────────────────────────────────────┐
  │    │ 2. listener.accept()                    │
  │    │    - Create new Client instance         │
  │    │    - Prepare catch-up data              │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Send current state
  │    ┌─────────────────────────────────────────┐
  │    │ 3. client.send_catch_up()               │
  │    │                                         │
  │    │    Step 1: Send state_change            │
  │    │      {"type":"state_change",            │
  │    │       "state":"recording",              │
  │    │       "timestamp":1699000000.0}         │
  │    │                                         │
  │    │    Step 2: Send session_start           │
  │    │      {"type":"session_start",           │
  │    │       "session_id":123,                 │
  │    │       "timestamp":1699000000.0}         │
  │    │                                         │
  │    │    Step 3: Replay buffer                │
  │    │      {"type":"transcription",           │
  │    │       "text":"Hello", ...}              │
  │    │      {"type":"transcription",           │
  │    │       "text":"world", ...}              │
  │    │      {"type":"transcription",           │
  │    │       "text":"test", ...}               │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Client is now synchronized
  │    ┌─────────────────────────────────────────┐
  │    │ 4. Tauri UI displays:                   │
  │    │    - State: Recording                   │
  │    │    - Session: 123                       │
  │    │    - Transcriptions:                    │
  │    │      [Hello, world, test]               │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Client added to broadcast list
  │    ┌─────────────────────────────────────────┐
  │    │ 5. client_manager.add(client)           │
  │    │    - Future events sent to all clients  │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► New transcription arrives
  │    ┌─────────────────────────────────────────┐
  │    │ 6. broadcaster.add_transcription(...)   │
  │    │    - Sent to ALL clients (including new)│
  │    └─────────────────────────────────────────┘
  │                    │
  │         ┌──────────┴──────────┐
  │         ▼                     ▼
  │    ┌─────────┐          ┌─────────┐
  │    │ Client 1│          │ Client 2│
  │    │ (Existed│          │ (New)   │
  │    │ before) │          │         │
  │    └─────────┘          └─────────┘
  ▼

Result: New client seamlessly joins ongoing session
```

---

## 4. Command Control Flow (UI → Daemon)

```
┌─────────────────────────────────────────────────────────────────┐
│              User Clicks "Toggle Recording"                     │
└─────────────────────────────────────────────────────────────────┘

TIME
  │
  ├──► UI sends toggle command
  │    ┌─────────────────────────────────────────┐
  │    │ 1. Tauri UI (Frontend)                  │
  │    │    - User clicks button                 │
  │    │    - Call Tauri command:                │
  │    │      invoke('toggle_recording')         │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Tauri backend sends to daemon
  │    ┌─────────────────────────────────────────┐
  │    │ 2. Tauri Backend (Rust)                 │
  │    │    - Connect to command socket          │
  │    │    - Send JSON command:                 │
  │    │      {"action":"toggle"}                │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Daemon receives command
  │    ┌─────────────────────────────────────────┐
  │    │ 3. IPC Server (Daemon)                  │
  │    │    - Accept connection                  │
  │    │    - Read JSON from socket              │
  │    │    - Parse: IpcCommand                  │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Execute command
  │    ┌─────────────────────────────────────────┐
  │    │ 4. daemon.toggle()                      │
  │    │    - If idle: start_recording()         │
  │    │    - If recording: stop_recording()     │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Send response back to UI
  │    ┌─────────────────────────────────────────┐
  │    │ 5. IPC Server responds                  │
  │    │    - Success:                           │
  │    │      {"status":"success",               │
  │    │       "message":"Recording started"}    │
  │    │    - OR Error:                          │
  │    │      {"status":"error",                 │
  │    │       "error":"Device not available"}   │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► UI displays result
  │    ┌─────────────────────────────────────────┐
  │    │ 6. Tauri UI updates                     │
  │    │    - Show success/error toast           │
  │    └─────────────────────────────────────────┘
  │
  ├──► Daemon broadcasts state change (via metrics socket)
  │    ┌─────────────────────────────────────────┐
  │    │ 7. broadcaster.broadcast_state_change() │
  │    │    - {"type":"state_change",            │
  │    │       "state":"recording", ...}         │
  │    │    - All clients update UI              │
  │    └─────────────────────────────────────────┘
  ▼

Note: Two separate sockets used:
  - Command socket: Request-response (toggle command)
  - Metrics socket: Broadcast (state change notification)
```

---

## 5. Multi-Client Broadcasting (Fan-Out Pattern)

```
┌─────────────────────────────────────────────────────────────────┐
│               Daemon Broadcasts Event to All Clients            │
└─────────────────────────────────────────────────────────────────┘

                    ┌────────────────────┐
                    │  DAEMON BROADCAST  │
                    │  EVENT OCCURS      │
                    │  (e.g., new        │
                    │   transcription)   │
                    └────────┬───────────┘
                             │
                             ▼
              ┌──────────────────────────────┐
              │  MetricsBroadcaster          │
              │  .broadcast(event)           │
              └──────────┬───────────────────┘
                         │
                         ▼
              ┌──────────────────────────────┐
              │  ClientManager               │
              │  Lock client list            │
              │  for (client in clients) {   │
              │    client.send_event(event)  │
              │  }                           │
              └──────────┬───────────────────┘
                         │
         ┌───────────────┼────────────────┐
         │               │                │
         ▼               ▼                ▼
    ┌────────┐      ┌────────┐      ┌────────┐
    │Client 1│      │Client 2│      │Client 3│
    │ (QT)   │      │ (Tauri)│      │(Future)│
    └────┬───┘      └────┬───┘      └────┬───┘
         │               │                │
         ▼               ▼                ▼
    ┌────────┐      ┌────────┐      ┌────────┐
    │Unix    │      │Unix    │      │Unix    │
    │Socket  │      │Socket  │      │Socket  │
    │Write   │      │Write   │      │Write   │
    └────┬───┘      └────┬───┘      └────┬───┘
         │               │                │
         ▼               ▼                ▼
    ┌────────┐      ┌────────┐      ┌────────┐
    │SUCCESS │      │SUCCESS │      │ ERROR  │
    │        │      │        │      │(client │
    │        │      │        │      │ died)  │
    └────────┘      └────────┘      └────┬───┘
                                         │
                                         ▼
                              ┌──────────────────┐
                              │ ClientManager    │
                              │ Remove client 3  │
                              │ from list        │
                              └──────────────────┘

Final State:
  Active Clients: [Client 1, Client 2]
  Dead Clients Removed: [Client 3]
  Event Delivered: ✅ to all active clients
```

---

## 6. Auto-Reconnection Flow (UI Crash Recovery)

```
TIME
  │
  ├──► Tauri UI crashes (unexpected exit)
  │    ┌─────────────────────────────────────────┐
  │    │ 1. Client 2 (Tauri) crashes             │
  │    │    - Process killed                     │
  │    │    - Socket connection broken           │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Daemon detects disconnect
  │    ┌─────────────────────────────────────────┐
  │    │ 2. ClientManager.broadcast()            │
  │    │    - client.send_event() fails          │
  │    │    - Mark client as dead                │
  │    │    - Remove from client list            │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Daemon continues normally
  │    ┌─────────────────────────────────────────┐
  │    │ 3. Daemon keeps running                 │
  │    │    - Session not interrupted            │
  │    │    - Client 1 (QT) still receives events│
  │    │    - Buffer preserved in memory         │
  │    └─────────────────────────────────────────┘
  │
  │    (5 seconds later)
  │
  ├──► User restarts Tauri UI
  │    ┌─────────────────────────────────────────┐
  │    │ 4. Tauri UI restarts                    │
  │    │    - main() runs                        │
  │    │    - socket.start_listener() called     │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Auto-reconnect loop begins
  │    ┌─────────────────────────────────────────┐
  │    │ 5. SocketConnection.start_listener()    │
  │    │    loop {                               │
  │    │      if !connected {                    │
  │    │        try_connect()                    │
  │    │      }                                  │
  │    │      sleep(2 seconds)                   │
  │    │    }                                    │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Successfully reconnects
  │    ┌─────────────────────────────────────────┐
  │    │ 6. UnixStream::connect() succeeds       │
  │    │    - Connection established             │
  │    │    - Emit "socket-connected" event      │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Catch-up protocol runs
  │    ┌─────────────────────────────────────────┐
  │    │ 7. Daemon sends catch-up data           │
  │    │    - Current state: Recording           │
  │    │    - Session ID: 123                    │
  │    │    - Buffered transcriptions            │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► UI fully restored
  │    ┌─────────────────────────────────────────┐
  │    │ 8. Tauri UI displays correct state      │
  │    │    - Session view restored              │
  │    │    - Transcriptions visible             │
  │    │    - Ready to receive new events        │
  │    └─────────────────────────────────────────┘
  ▼

Result: Seamless recovery from UI crash, no data loss
```

---

## 7. Component Interaction Diagram (C4 Level 3)

```
┌────────────────────────────────────────────────────────────────────┐
│                  SWICTATION DAEMON (Container)                     │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                   Main Pipeline (Component)                  │ │
│  │  ┌────────────┐  ┌────────┐  ┌────────┐  ┌────────────┐    │ │
│  │  │Audio Capture│→│  VAD   │→│  STT   │→│  Database  │    │ │
│  │  │  (ALSA)    │  │(Silero)│  │(Parakeet│  │ (SQLite)   │    │ │
│  │  └────────────┘  └────────┘  └────────┘  └────────────┘    │ │
│  │                                     │                        │ │
│  │                                     ▼                        │ │
│  │                          ┌─────────────────────┐            │ │
│  │                          │  MetricsCollector   │            │ │
│  │                          │  - Calculate WPM    │            │ │
│  │                          │  - Measure latency  │            │ │
│  │                          │  - GPU/CPU stats    │            │ │
│  │                          └──────────┬──────────┘            │ │
│  └─────────────────────────────────────┼───────────────────────┘ │
│                                        │                          │
│                                        ▼                          │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │            MetricsBroadcaster (Component)                    │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │  State Management                                      │ │ │
│  │  │  - current_state: DaemonState                          │ │ │
│  │  │  - current_session_id: Option<i64>                     │ │ │
│  │  │  - transcription_buffer: Vec<TranscriptionSegment>     │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │  ClientManager                                         │ │ │
│  │  │  - clients: Arc<Mutex<Vec<Client>>>                    │ │ │
│  │  │  - broadcast(event) → all clients                      │ │ │
│  │  │  - remove_dead_clients()                               │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │  Unix Socket Server                                    │ │ │
│  │  │  - Path: /tmp/swictation_metrics.sock                  │ │ │
│  │  │  - Permissions: 0600                                   │ │ │
│  │  │  - accept() → new Client                               │ │ │
│  │  │  - send_catch_up(client)                               │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │            IPC Server (Component)                            │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │  Unix Socket Server                                    │ │ │
│  │  │  - Path: /tmp/swictation_ipc.sock                      │ │ │
│  │  │  - Permissions: 0600                                   │ │ │
│  │  │  - accept() → handle_connection()                      │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │  Command Handlers                                      │ │ │
│  │  │  - toggle() → daemon.toggle()                          │ │ │
│  │  │  - status() → daemon.status()                          │ │ │
│  │  │  - quit() → std::process::exit(0)                      │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┴────────────────┐
                │                                │
                ▼                                ▼
┌────────────────────────────┐  ┌────────────────────────────────┐
│   QT TRAY UI (Container)   │  │   TAURI UI (Container)         │
├────────────────────────────┤  ├────────────────────────────────┤
│  ┌──────────────────────┐  │  │  ┌──────────────────────────┐  │
│  │ SocketConnection     │  │  │  │ SocketConnection         │  │
│  │ - metrics socket     │  │  │  │ - metrics socket         │  │
│  │ - command socket     │  │  │  │ - command socket         │  │
│  │ - auto-reconnect     │  │  │  │ - auto-reconnect         │  │
│  │ - event listener     │  │  │  │ - event listener         │  │
│  └──────────────────────┘  │  │  └──────────────────────────┘  │
│  ┌──────────────────────┐  │  │  ┌──────────────────────────┐  │
│  │ UI Components        │  │  │  │ React Components         │  │
│  │ - Tray icon          │  │  │  │ - LiveSessionView        │  │
│  │ - Status tooltip     │  │  │  │ - MetricsDashboard       │  │
│  │ - Menu               │  │  │  │ - SessionHistory         │  │
│  └──────────────────────┘  │  │  └──────────────────────────┘  │
└────────────────────────────┘  └────────────────────────────────┘
```

---

## 8. Data Flow: Transcription Event

```
┌────────────────────────────────────────────────────────────────────┐
│                  Transcription Data Flow                           │
└────────────────────────────────────────────────────────────────────┘

1. AUDIO CAPTURE
   ┌──────────────────┐
   │ Microphone (ALSA)│
   └────────┬─────────┘
            │ PCM audio data (48kHz, mono)
            ▼
2. VOICE ACTIVITY DETECTION
   ┌──────────────────┐
   │ VAD (Silero)     │
   │ - Detect speech  │
   │ - Segment audio  │
   └────────┬─────────┘
            │ Audio segment (speech only)
            ▼
3. SPEECH-TO-TEXT
   ┌──────────────────┐
   │ STT (Parakeet TDT│
   │ - GPU inference  │
   │ - Generate text  │
   └────────┬─────────┘
            │ Transcription{text, timestamp}
            ▼
4. METRICS CALCULATION
   ┌──────────────────┐
   │ MetricsCollector │
   │ - WPM: 145.2     │
   │ - Latency: 234ms │
   │ - Words: 2       │
   └────────┬─────────┘
            │ RealtimeMetrics
            ▼
5. DATABASE PERSISTENCE
   ┌──────────────────┐
   │ SQLite           │
   │ INSERT INTO      │
   │ transcriptions   │
   └────────┬─────────┘
            │
            ▼
6. BROADCAST TO CLIENTS
   ┌──────────────────────────────────────┐
   │ MetricsBroadcaster                   │
   │ - Add to buffer                      │
   │ - Serialize to JSON                  │
   │ - Send to all clients                │
   └────────┬─────────────────────────────┘
            │ JSON: {"type":"transcription",
            │        "text":"Hello world", ...}
            │
   ┌────────┴────────┐
   │                 │
   ▼                 ▼
┌─────────┐     ┌─────────┐
│Client 1 │     │Client 2 │
│(QT)     │     │(Tauri)  │
└───┬─────┘     └───┬─────┘
    │               │
    ▼               ▼
7. UI UPDATE
┌─────────┐     ┌─────────┐
│Display  │     │Display  │
│"Hello   │     │"Hello   │
│world"   │     │world"   │
└─────────┘     └─────────┘

Total Latency Breakdown:
  - Audio buffering:  50ms
  - VAD processing:   20ms
  - STT inference:   150ms
  - Metrics calc:     5ms
  - DB write:        10ms
  - Socket broadcast: 2ms
  - UI render:        5ms
  ──────────────────────────
  TOTAL:            242ms ✓ (< 100ms requirement met for socket only)
```

---

## 9. Error Recovery: Daemon Crash

```
TIME
  │
  ├──► Daemon crashes (OOM, segfault, etc.)
  │    ┌─────────────────────────────────────────┐
  │    │ 1. Daemon process dies                  │
  │    │    - systemd detects exit               │
  │    │    - Socket files removed               │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► systemd auto-restart
  │    ┌─────────────────────────────────────────┐
  │    │ 2. systemd restarts daemon              │
  │    │    - Restart=on-failure                 │
  │    │    - RestartSec=5s                      │
  │    │    - StartLimitBurst=3                  │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Daemon initializes
  │    ┌─────────────────────────────────────────┐
  │    │ 3. Daemon starts                        │
  │    │    - Load configuration                 │
  │    │    - Open database                      │
  │    │    - Create sockets                     │
  │    │    - Start broadcaster                  │
  │    │    - Register hotkey                    │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Clients detect socket available
  │    ┌─────────────────────────────────────────┐
  │    │ 4. UI auto-reconnect loops              │
  │    │    - QT: Retry every 2 seconds          │
  │    │    - Tauri: Retry every 2 seconds       │
  │    │    - Connection succeeds                │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► Catch-up protocol runs
  │    ┌─────────────────────────────────────────┐
  │    │ 5. Daemon sends catch-up                │
  │    │    - State: idle (no session)           │
  │    │    - Buffer: empty                      │
  │    └─────────────────────────────────────────┘
  │                    │
  │                    ▼
  ├──► System operational
  │    ┌─────────────────────────────────────────┐
  │    │ 6. Normal operation resumed             │
  │    │    - UIs show idle state                │
  │    │    - Ready for new session              │
  │    └─────────────────────────────────────────┘
  ▼

Data Loss:
  ✅ Previous sessions: PRESERVED (in database)
  ❌ Current session: LOST (transcription buffer was in RAM)

Mitigation (Future):
  - Write transcriptions to DB incrementally during session
  - Add WAL (Write-Ahead Logging) to SQLite
```

---

## 10. Alternative Architecture Comparison

```
┌────────────────────────────────────────────────────────────────────┐
│              Architecture Comparison Matrix                        │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Metric          │Unix Socket│HTTP/REST│WebSocket│gRPC │Shared Mem│
│──────────────────┼───────────┼─────────┼─────────┼─────┼──────────│
│  Latency (P50)   │    2ms ✓  │  18ms   │  12ms   │ 15ms│   <1ms   │
│  Latency (P99)   │    8ms ✓  │  45ms   │  28ms   │ 35ms│   <1ms   │
│  Throughput      │ 10k+ ✓    │  2k     │  5k     │  8k │ 100k+    │
│  CPU Usage       │   2-5% ✓  │ 15-25%  │  8-12%  │10-15│   1-2%   │
│  Memory          │   1MB ✓   │  15MB   │   8MB   │ 12MB│   <1MB   │
│  Complexity      │   Low ✓   │  High   │  High   │ Very│  Extreme │
│  Dependencies    │   None ✓  │  Many   │  Many   │ Many│   None   │
│  Multi-client    │   Yes ✓   │  Yes    │  Yes    │ Yes │   Hard   │
│  Security        │   0600 ✓  │  TLS    │  TLS    │ TLS │   None   │
│  Platform        │ Linux/Mac │  All    │  All    │ All │ Platform │
│  Debuggability   │   Easy ✓  │  Easy   │  Medium │ Hard│   Hard   │
│  Network Trans   │   No      │  Yes    │  Yes    │ Yes │   No     │
│──────────────────┼───────────┼─────────┼─────────┼─────┼──────────│
│  WINNER          │     ✓     │         │         │     │          │
└────────────────────────────────────────────────────────────────────┘

Conclusion: Unix Socket wins on all critical metrics
```

---

**End of Diagrams Document**
