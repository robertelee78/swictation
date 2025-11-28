# Swictation Cross-Platform Architecture Diagram

## System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Swictation Application                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐            │
│  │  Tauri UI    │    │   npm CLI    │    │   Daemon     │            │
│  │  (Frontend)  │    │  (Commands)  │    │  (Backend)   │            │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘            │
│         │                    │                    │                     │
│         └────────────────────┴────────────────────┘                     │
│                              │                                          │
│         ┌────────────────────┴────────────────────┐                    │
│         │     Platform Abstraction Layer          │                    │
│         └─────────────────────────────────────────┘                    │
│                              │                                          │
└──────────────────────────────┼──────────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐      ┌───────────────┐     ┌───────────────┐
│     Linux     │      │     macOS     │     │    Windows    │
│   (X11/Way)   │      │  (Quartz)     │     │   (Win32)     │
└───────────────┘      └───────────────┘     └───────────────┘
```

---

## Platform Abstraction Layer Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Platform Abstraction Layer                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐      │
│  │ swictation-    │  │ swictation-    │  │ swictation-    │      │
│  │    paths       │  │     ipc        │  │     libs       │      │
│  │                │  │                │  │                │      │
│  │ • data_dir     │  │ • IpcServer    │  │ • OnnxRuntime  │      │
│  │ • config_dir   │  │ • IpcClient    │  │ • CUDA         │      │
│  │ • runtime_dir  │  │ • UnixSocket   │  │ • CoreML       │      │
│  │ • log_dir      │  │ • NamedPipe    │  │ • ROCm         │      │
│  │ • cache_dir    │  │                │  │                │      │
│  │ • model_dir    │  │                │  │                │      │
│  └────────────────┘  └────────────────┘  └────────────────┘      │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │              Existing Platform-Specific Modules               │ │
│  │                                                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │ │
│  │  │    Text      │  │   Hotkey     │  │   Display    │      │ │
│  │  │  Injection   │  │   System     │  │   Server     │      │ │
│  │  │              │  │              │  │  Detection   │      │ │
│  │  │ • xdotool    │  │ • X11 grab   │  │ • X11        │      │ │
│  │  │ • wtype      │  │ • Wayland    │  │ • Wayland    │      │ │
│  │  │ • ydotool    │  │ • macOS CGE  │  │ • macOS      │      │ │
│  │  │ • macOS CG   │  │ • Win32 API  │  │ • Windows    │      │ │
│  │  │ • Win32 SI   │  │              │  │              │      │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │ │
│  │                                                               │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Legend**:
- CG = Core Graphics (macOS)
- CGE = Core Graphics Events (macOS)
- SI = SendInput (Windows)

---

## Path Resolution Flow

```
Application Request
       │
       ├──> PlatformPaths::data_dir()
       │           │
       │           ├─[Linux]──> ~/.local/share/swictation/
       │           │
       │           ├─[macOS]──> ~/Library/Application Support/swictation/
       │           │
       │           └─[Windows]─> %LOCALAPPDATA%\Swictation\
       │
       ├──> PlatformPaths::config_dir()
       │           │
       │           ├─[Linux]──> ~/.config/swictation/
       │           │
       │           ├─[macOS]──> ~/Library/Application Support/com.swictation.daemon/
       │           │
       │           └─[Windows]─> %APPDATA%\Swictation\
       │
       └──> PlatformPaths::runtime_dir()
                   │
                   ├─[Linux]──> $XDG_RUNTIME_DIR (or ~/.local/share/swictation/)
                   │
                   ├─[macOS]──> ~/Library/Application Support/swictation/
                   │
                   └─[Windows]─> %TEMP%\swictation\
```

---

## IPC Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          IPC Abstraction                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                     ┌───────────────────┐                          │
│                     │   IpcEndpoint     │                          │
│                     │   (enum)          │                          │
│                     │                   │                          │
│                     │ • UnixSocket(Path)│                          │
│                     │ • NamedPipe(Name) │                          │
│                     └─────────┬─────────┘                          │
│                               │                                     │
│              ┌────────────────┼────────────────┐                   │
│              │                                 │                   │
│              ▼                                 ▼                   │
│   ┌─────────────────────┐         ┌─────────────────────┐         │
│   │  UnixSocketTransport│         │ NamedPipeTransport  │         │
│   │  (Linux/macOS)      │         │  (Windows)          │         │
│   │                     │         │                     │         │
│   │ • tokio::net::      │         │ • tokio::net::      │         │
│   │   UnixListener      │         │   windows::         │         │
│   │ • Permissions: 0600 │         │   named_pipe        │         │
│   │ • Auto-cleanup      │         │ • Windows ACLs      │         │
│   └─────────────────────┘         └─────────────────────┘         │
│              │                                 │                   │
│              └────────────────┬────────────────┘                   │
│                               │                                     │
│                               ▼                                     │
│                     ┌───────────────────┐                          │
│                     │   IpcStream       │                          │
│                     │   (trait)         │                          │
│                     │                   │                          │
│                     │ • AsyncRead       │                          │
│                     │ • AsyncWrite      │                          │
│                     └───────────────────┘                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Library Loading Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Library Loader System                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                    LibraryLoader::auto_detect()                     │
│                               │                                     │
│            ┌──────────────────┼──────────────────┐                 │
│            │                  │                  │                 │
│            ▼                  ▼                  ▼                 │
│   ┌────────────────┐  ┌─────────────┐  ┌─────────────┐           │
│   │  Environment   │  │   Python    │  │   System    │           │
│   │   Variables    │  │  Packages   │  │  Locations  │           │
│   │                │  │             │  │             │           │
│   │ ORT_DYLIB_PATH │  │ python3 -c  │  │ /usr/lib    │           │
│   │ CUDA_PATH      │  │ "import ..."│  │ /usr/local  │           │
│   └────────────────┘  └─────────────┘  │ /opt/...    │           │
│                                         └─────────────┘           │
│                               │                                     │
│                               ▼                                     │
│                    ┌─────────────────────┐                         │
│                    │  Platform-Specific  │                         │
│                    │  Library Names      │                         │
│                    │                     │                         │
│                    │ Linux:   .so        │                         │
│                    │ macOS:   .dylib     │                         │
│                    │ Windows: .dll       │                         │
│                    └─────────────────────┘                         │
│                               │                                     │
│                               ▼                                     │
│                    ┌─────────────────────┐                         │
│                    │  Resolved Library   │                         │
│                    │  Path               │                         │
│                    │                     │                         │
│                    │ → Set env var       │                         │
│                    │ → Initialize ORT    │                         │
│                    └─────────────────────┘                         │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Text Injection Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Text Injection System                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                    TextInjector::new()                              │
│                          │                                          │
│                          ▼                                          │
│              ┌───────────────────────┐                             │
│              │  Display Server       │                             │
│              │  Detection            │                             │
│              └───────────┬───────────┘                             │
│                          │                                          │
│        ┌─────────────────┼─────────────────┬───────────────┐       │
│        │                 │                 │               │       │
│        ▼                 ▼                 ▼               ▼       │
│   ┌────────┐       ┌─────────┐       ┌────────┐     ┌─────────┐  │
│   │ Linux  │       │  macOS  │       │Windows │     │ Wayland │  │
│   │  X11   │       │ Quartz  │       │ Win32  │     │  (var)  │  │
│   └────┬───┘       └────┬────┘       └────┬───┘     └────┬────┘  │
│        │                │                 │               │        │
│        ▼                ▼                 ▼               ▼        │
│   ┌────────┐      ┌──────────┐      ┌─────────┐    ┌─────────┐   │
│   │xdotool │      │Core      │      │SendInput│    │ydotool/ │   │
│   │        │      │Graphics  │      │API      │    │wtype    │   │
│   │type    │      │CGEvent   │      │UNICODE  │    │         │   │
│   │--clear │      │Post      │      │events   │    │type     │   │
│   │mods    │      │          │      │         │    │         │   │
│   └────────┘      └──────────┘      └─────────┘    └─────────┘   │
│                                                                     │
│   Special handling:                                                │
│   • <KEY:...> markers → key events                                 │
│   • Clipboard paste fallback (large text)                          │
│   • GNOME Wayland → requires ydotool                               │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Hotkey System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Hotkey Management                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                    HotkeyManager::new()                             │
│                          │                                          │
│                          ▼                                          │
│              ┌───────────────────────┐                             │
│              │  Platform Detection   │                             │
│              └───────────┬───────────┘                             │
│                          │                                          │
│     ┌────────────────────┼────────────────────┬──────────────┐     │
│     │                    │                    │              │     │
│     ▼                    ▼                    ▼              ▼     │
│ ┌────────┐          ┌────────┐          ┌────────┐    ┌─────────┐│
│ │ X11    │          │ macOS  │          │Windows │    │ Wayland ││
│ └────┬───┘          └────┬───┘          └────┬───┘    └────┬────┘│
│      │                   │                   │              │     │
│      ▼                   ▼                   ▼              ▼     │
│ ┌────────────┐      ┌────────────┐     ┌────────────┐  ┌───────┐│
│ │global-     │      │global-     │     │global-     │  │Manual ││
│ │hotkey      │      │hotkey      │     │hotkey      │  │Config ││
│ │            │      │            │     │            │  │       ││
│ │X11 grab    │      │CGEvent/    │     │RegisterHot │  │Sway   ││
│ │XGrabKey    │      │NSEvent     │     │Key API     │  │GNOME  ││
│ └────────────┘      └────────────┘     └────────────┘  └───┬───┘│
│                                                              │     │
│                                                              ▼     │
│                                                    ┌──────────────┐│
│                                                    │Auto-configure││
│                                                    │compositor   ││
│                                                    │shortcuts    ││
│                                                    │             ││
│                                                    │• Sway config││
│                                                    │• gsettings  ││
│                                                    └─────────────┘│
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Voice to Text

```
┌──────────────────────────────────────────────────────────────────┐
│  1. Audio Input                                                  │
│     │                                                            │
│     ├─[swictation-audio]──> Capture microphone                  │
│     │                                                            │
│     ▼                                                            │
├──────────────────────────────────────────────────────────────────┤
│  2. Voice Activity Detection (VAD)                               │
│     │                                                            │
│     ├─[swictation-vad]──> Silero VAD (ONNX)                     │
│     │                                                            │
│     ▼                                                            │
├──────────────────────────────────────────────────────────────────┤
│  3. Speech-to-Text                                               │
│     │                                                            │
│     ├─[swictation-stt]──> Parakeet-TDT (0.6B/1.1B)             │
│     │                    ┌──────────────────┐                   │
│     │                    │  ONNX Runtime    │                   │
│     │                    │  (swictation-    │                   │
│     │                    │   libs)          │                   │
│     │                    └──────────────────┘                   │
│     │                                                            │
│     ▼                                                            │
├──────────────────────────────────────────────────────────────────┤
│  4. Text Processing                                              │
│     │                                                            │
│     ├──> Context Learning (corrections, patterns)               │
│     ├──> Capitalization                                         │
│     ├──> Voice Commands → Symbols                               │
│     │                                                            │
│     ▼                                                            │
├──────────────────────────────────────────────────────────────────┤
│  5. Text Injection                                               │
│     │                                                            │
│     ├─[Platform Detection]──┬──> Linux (xdotool/wtype/ydotool)  │
│     │                        ├──> macOS (Core Graphics)          │
│     │                        └──> Windows (SendInput)            │
│     │                                                            │
│     ▼                                                            │
│  Application receives text                                       │
└──────────────────────────────────────────────────────────────────┘
```

---

## Module Dependencies

```
swictation-daemon
    ├── swictation-paths       (NEW)
    ├── swictation-ipc         (NEW)
    ├── swictation-libs        (NEW)
    ├── swictation-audio       (existing)
    ├── swictation-vad         (existing)
    ├── swictation-stt         (existing)
    │   └── swictation-libs    (NEW - for ONNX Runtime loading)
    ├── swictation-broadcaster (existing)
    ├── swictation-metrics     (existing)
    │   └── swictation-paths   (NEW - for database path)
    └── swictation-context-learning (existing)

tauri-ui
    └── swictation-paths       (NEW - for database, socket paths)

npm-package
    └── (JavaScript equivalent of swictation-paths)
```

---

## Compilation Targets

```
Platform Matrix:

┌───────────────┬──────────────────┬──────────────────────────────┐
│ Platform      │ Target Triple    │ Key Features                 │
├───────────────┼──────────────────┼──────────────────────────────┤
│ Linux x86_64  │ x86_64-unknown-  │ • X11/Wayland                │
│               │ linux-gnu        │ • xdotool/wtype/ydotool      │
│               │                  │ • Unix sockets               │
│               │                  │ • global-hotkey (X11)        │
├───────────────┼──────────────────┼──────────────────────────────┤
│ macOS x86_64  │ x86_64-apple-    │ • Core Graphics              │
│               │ darwin           │ • Unix sockets               │
│               │                  │ • global-hotkey (CGEvent)    │
├───────────────┼──────────────────┼──────────────────────────────┤
│ macOS ARM64   │ aarch64-apple-   │ • Core Graphics              │
│ (M1/M2/M3)    │ darwin           │ • Unix sockets               │
│               │                  │ • CoreML acceleration        │
├───────────────┼──────────────────┼──────────────────────────────┤
│ Windows x86_64│ x86_64-pc-       │ • Win32 SendInput            │
│               │ windows-msvc     │ • Named pipes                │
│               │                  │ • global-hotkey (Win32)      │
│               │                  │ • CUDA support               │
└───────────────┴──────────────────┴──────────────────────────────┘

Conditional Compilation:
• #[cfg(target_os = "linux")]
• #[cfg(target_os = "macos")]
• #[cfg(target_os = "windows")]
• #[cfg(unix)]
• #[cfg(target_arch = "aarch64")]
```

---

## Security Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Security Layers                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. File System Permissions                                        │
│     ├─ Runtime directory: 0700 (owner-only)                        │
│     ├─ Unix sockets: 0600 (owner read/write only)                  │
│     ├─ Config files: 0644 (owner write, all read)                  │
│     └─ Database: 0600 (owner-only)                                 │
│                                                                     │
│  2. IPC Security                                                    │
│     ├─ Unix sockets: Filesystem permissions                        │
│     ├─ Named pipes: Windows ACLs (owner-only)                      │
│     └─ No network exposure (local IPC only)                        │
│                                                                     │
│  3. Library Loading                                                 │
│     ├─ Verify library paths (no path traversal)                    │
│     ├─ Environment variable sanitization                           │
│     └─ Optional: Signature verification                            │
│                                                                     │
│  4. Platform Isolation                                              │
│     ├─ Compile-time platform selection                             │
│     ├─ No runtime platform detection for security-critical code    │
│     └─ Type-safe abstractions (no unsafe unless necessary)         │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. **Review**: Read `cross-platform-abstraction-design.md`
2. **Implement**: Start with `swictation-paths` (see `path-abstraction-implementation.md`)
3. **Test**: Multi-platform testing strategy
4. **Document**: API documentation and migration guides
5. **Deploy**: CI/CD for all platforms

---

## Key Files

- `cross-platform-abstraction-design.md` - Complete architectural design
- `path-abstraction-implementation.md` - Implementation guide for paths module
- `ARCHITECTURE_SUMMARY.md` - This summary document
- `ARCHITECTURE_DIAGRAM.md` - This diagram document
