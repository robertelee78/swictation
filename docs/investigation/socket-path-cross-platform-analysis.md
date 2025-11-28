# Socket Path Cross-Platform Analysis

**Investigation Date:** 2025-11-28
**Project:** Swictation
**Task:** Cross-platform socket path handling assessment

## Executive Summary

**CRITICAL FINDING:** The Swictation daemon uses Unix sockets for IPC, which are **NOT available on Windows**. While Linux and macOS implementations are robust and properly abstracted, Windows support requires a complete architectural shift to **Named Pipes**.

### Platform Readiness Assessment

| Platform | Status | Socket Mechanism | Implementation Quality |
|----------|--------|------------------|------------------------|
| **Linux** | ✅ **Production Ready** | Unix Domain Sockets | Excellent - XDG_RUNTIME_DIR with fallback |
| **macOS** | ✅ **Production Ready** | Unix Domain Sockets | Excellent - Application Support directory |
| **Windows** | ❌ **Not Supported** | None (requires Named Pipes) | Not implemented |

---

## 1. Socket Path Implementation Analysis

### 1.1 Daemon Socket Utilities (`rust-crates/swictation-daemon/src/socket_utils.rs`)

**Location:** `/opt/swictation/rust-crates/swictation-daemon/src/socket_utils.rs`

#### Platform-Specific Logic

```rust
// Lines 15-67: get_socket_dir()
pub fn get_socket_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // macOS: ~/Library/Application Support/swictation
        dirs::data_local_dir()
            .context("Failed to get Application Support directory")?
            .join("swictation")
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux: $XDG_RUNTIME_DIR (primary) or ~/.local/share/swictation (fallback)
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                return Ok(path);
            }
        }

        // Fallback to ~/.local/share/swictation
        dirs::data_local_dir()
            .context("Failed to get data directory")?
            .join("swictation")
    }
}
```

#### Socket Path Functions

```rust
// Line 71-73: IPC socket (toggle commands)
pub fn get_ipc_socket_path() -> Result<PathBuf> {
    Ok(get_socket_dir()?.join("swictation.sock"))
}

// Line 76-78: Metrics socket (UI broadcasts)
pub fn get_metrics_socket_path() -> Result<PathBuf> {
    Ok(get_socket_dir()?.join("swictation_metrics.sock"))
}
```

**Assessment:**
- ✅ **EXCELLENT** Linux implementation with XDG_RUNTIME_DIR priority
- ✅ **EXCELLENT** macOS implementation using Application Support
- ✅ **EXCELLENT** Security: 0700 directory permissions, 0600 socket permissions
- ❌ **NO WINDOWS SUPPORT** - `#[cfg(not(target_os = "macos"))]` assumes Unix

---

### 1.2 Tauri UI Socket Utilities (`tauri-ui/src-tauri/src/socket/socket_utils.rs`)

**Location:** `/opt/swictation/tauri-ui/src-tauri/src/socket/socket_utils.rs`

#### Implementation (Simplified Version)

```rust
// Lines 16-40: get_socket_dir()
pub fn get_socket_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return dirs::data_local_dir()
            .expect("Failed to get Application Support directory")
            .join("swictation");
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir);
            if path.exists() {
                return path;
            }
        }

        dirs::data_local_dir()
            .expect("Failed to get data directory")
            .join("swictation")
    }
}
```

**Assessment:**
- ✅ Matches daemon implementation (consistency)
- ✅ No security permission setting (assumes daemon already secured paths)
- ❌ No Windows support (same as daemon)
- ⚠️ Uses `.expect()` instead of `Result<>` (less robust than daemon)

---

### 1.3 NPM Package Socket Paths (`npm-package/src/socket-paths.js`)

**Location:** `/opt/swictation/npm-package/src/socket-paths.js`

```javascript
// Lines 19-45: getSocketDir()
function getSocketDir() {
  // macOS: ~/Library/Application Support/swictation
  if (process.platform === 'darwin') {
    const macDir = path.join(os.homedir(), 'Library', 'Application Support', 'swictation');
    if (!fs.existsSync(macDir)) {
      fs.mkdirSync(macDir, { recursive: true, mode: 0o700 });
    }
    return macDir;
  }

  // Linux: XDG_RUNTIME_DIR > ~/.local/share/swictation
  if (process.env.XDG_RUNTIME_DIR && fs.existsSync(process.env.XDG_RUNTIME_DIR)) {
    return process.env.XDG_RUNTIME_DIR;
  }

  // Linux fallback
  const fallbackDir = path.join(os.homedir(), '.local', 'share', 'swictation');
  if (!fs.existsSync(fallbackDir)) {
    fs.mkdirSync(fallbackDir, { recursive: true, mode: 0o700 });
  }
  return fallbackDir;
}
```

**Assessment:**
- ✅ Correctly matches Rust implementation
- ✅ Creates directories with proper permissions
- ❌ No Windows support (`process.platform === 'darwin'` but no `win32`)

---

### 1.4 Python Tray App (`src/ui/swictation_tray.py`)

**Location:** `/opt/swictation/src/ui/swictation_tray.py`

```python
# Lines 15-33: get_socket_path()
def get_socket_path() -> str:
    """Get socket path using XDG_RUNTIME_DIR or fallback to ~/.local/share/swictation.

    Matches the Rust daemon's socket_utils::get_ipc_socket_path() logic.
    """
    # Try XDG_RUNTIME_DIR first
    runtime_dir = os.environ.get('XDG_RUNTIME_DIR')
    if runtime_dir and os.path.exists(runtime_dir):
        return os.path.join(runtime_dir, 'swictation.sock')

    # Fallback to ~/.local/share/swictation/swictation.sock
    home = os.environ.get('HOME')
    if home:
        socket_dir = os.path.join(home, '.local', 'share', 'swictation')
        os.makedirs(socket_dir, mode=0o700, exist_ok=True)
        return os.path.join(socket_dir, 'swictation.sock')

    # Final fallback (should rarely happen)
    return '/tmp/swictation.sock'
```

**Assessment:**
- ✅ Matches Rust implementation
- ✅ Creates directories with proper permissions
- ⚠️ Final fallback to `/tmp/` (legacy compatibility but security risk)
- ❌ Python doesn't run on Windows in this codebase (Linux/macOS only)

---

## 2. Hardcoded Socket Paths (Critical Issues)

### 2.1 Python CLI Tool (`src/swictation_cli.py`)

**Location:** `/opt/swictation/src/swictation_cli.py:12`

```python
SOCKET_PATH = '/tmp/swictation.sock'  # ❌ HARDCODED!
```

**Impact:**
- ❌ **BROKEN** - Does not use XDG_RUNTIME_DIR
- ❌ Daemon now uses `/run/user/1000/swictation.sock` (XDG_RUNTIME_DIR)
- ❌ CLI will fail to connect unless user manually specifies socket path
- 🔧 **FIX REQUIRED:** Use same logic as `swictation_tray.py`

**Recommendation:**
```python
# Replace hardcoded path with dynamic resolution
from pathlib import Path
import os

def get_socket_path() -> str:
    if runtime_dir := os.environ.get('XDG_RUNTIME_DIR'):
        return os.path.join(runtime_dir, 'swictation.sock')

    home = os.environ.get('HOME')
    if home:
        socket_dir = Path.home() / '.local' / 'share' / 'swictation'
        socket_dir.mkdir(parents=True, mode=0o700, exist_ok=True)
        return str(socket_dir / 'swictation.sock')

    return '/tmp/swictation.sock'  # Emergency fallback

SOCKET_PATH = get_socket_path()
```

---

### 2.2 NPM Package Postinstall (`npm-package/postinstall.js`)

**Location:** `/opt/swictation/npm-package/postinstall.js:2022`

```javascript
socket_path = "/tmp/swictation.sock"  // ❌ HARDCODED in config template
```

**Impact:**
- ❌ Generated config files have wrong path
- ⚠️ Line 2342 has correct example with XDG variable substitution
- 🔧 **FIX REQUIRED:** Use dynamic path in config templates

---

### 2.3 Example Config (`config/config.example.toml`)

**Location:** `/opt/swictation/config/config.example.toml:6`

```toml
socket_path = "/run/user/1000/swictation.sock"  # or /tmp/swictation.sock
```

**Assessment:**
- ⚠️ **PARTIALLY CORRECT** - Shows XDG_RUNTIME_DIR path but hardcoded UID
- ℹ️ This is example config (users expected to customize)
- 💡 **RECOMMENDATION:** Add note about dynamic resolution

**Better Documentation:**
```toml
# Socket path is auto-detected by daemon:
#   - Linux: $XDG_RUNTIME_DIR/swictation.sock (e.g., /run/user/1000/swictation.sock)
#   - macOS: ~/Library/Application Support/swictation/swictation.sock
#   - Fallback: ~/.local/share/swictation/swictation.sock
# Override only if needed:
# socket_path = "/custom/path/swictation.sock"
```

---

## 3. Platform-Specific Socket Paths

### 3.1 Linux

**Primary Path (XDG_RUNTIME_DIR):**
```
/run/user/{UID}/swictation.sock
/run/user/{UID}/swictation_metrics.sock
```

**Fallback Path:**
```
~/.local/share/swictation/swictation.sock
~/.local/share/swictation/swictation_metrics.sock
```

**Security:**
- ✅ XDG_RUNTIME_DIR is automatically mode 0700 (user-only)
- ✅ Fallback directory created with mode 0700
- ✅ Sockets created with mode 0600

---

### 3.2 macOS

**Path:**
```
~/Library/Application Support/swictation/swictation.sock
~/Library/Application Support/swictation/swictation_metrics.sock
```

**Security:**
- ✅ Directory created with mode 0700
- ✅ Sockets created with mode 0600
- ✅ Follows macOS Application Support convention

---

### 3.3 Windows (NOT IMPLEMENTED)

**Current Status:** ❌ **COMPLETELY UNSUPPORTED**

**Required Implementation:**

Windows doesn't support Unix domain sockets. Need to use **Named Pipes**:

```
\\.\pipe\swictation
\\.\pipe\swictation_metrics
```

**Implementation Requirements:**

1. **Rust Dependencies:**
```toml
[target.'cfg(windows)'.dependencies]
tokio = { version = "1", features = ["net", "io-util"] }
# tokio::net::windows::named_pipe
```

2. **Conditional Compilation:**
```rust
#[cfg(unix)]
use tokio::net::UnixListener;

#[cfg(windows)]
use tokio::net::windows::named_pipe::ServerOptions;

pub fn get_ipc_socket_path() -> Result<PathBuf> {
    #[cfg(unix)]
    {
        Ok(get_socket_dir()?.join("swictation.sock"))
    }

    #[cfg(windows)]
    {
        // Named pipes don't use PathBuf - return pipe name as path
        Ok(PathBuf::from(r"\\.\pipe\swictation"))
    }
}
```

3. **Server Creation:**
```rust
#[cfg(unix)]
let listener = UnixListener::bind(&socket_path)?;

#[cfg(windows)]
let server = ServerOptions::new()
    .first_pipe_instance(true)
    .create(r"\\.\pipe\swictation")?;
```

4. **Client Connection:**
```rust
#[cfg(unix)]
let stream = UnixStream::connect(&socket_path).await?;

#[cfg(windows)]
let client = ClientOptions::new().open(r"\\.\pipe\swictation").await?;
```

---

## 4. Socket Type Summary

### 4.1 IPC Socket (`swictation.sock`)

**Purpose:** Bidirectional command/response IPC
**Used By:**
- CLI tools (toggle, status, PTT commands)
- Python tray app (toggle, status)
- Sway/i3 keybindings (via `nc -U`)

**Paths:**
- Linux: `/run/user/1000/swictation.sock`
- macOS: `~/Library/Application Support/swictation/swictation.sock`
- Windows: `\\.\pipe\swictation` (not implemented)

**Protocol:** JSON request/response
```json
Request:  {"action": "toggle"}
Response: {"success": true, "state": "recording"}
```

---

### 4.2 Metrics Socket (`swictation_metrics.sock`)

**Purpose:** One-way broadcast stream (daemon → UI clients)
**Used By:**
- Tauri UI (real-time metrics display)
- Web dashboard (future)
- Monitoring tools

**Paths:**
- Linux: `/run/user/1000/swictation_metrics.sock`
- macOS: `~/Library/Application Support/swictation/swictation_metrics.sock`
- Windows: `\\.\pipe\swictation_metrics` (not implemented)

**Protocol:** Line-delimited JSON events
```json
{"event":"recording_started","timestamp":1732805000}
{"event":"chunk_transcribed","text":"Hello world","duration":1.5}
{"event":"recording_stopped","timestamp":1732805010}
```

---

## 5. Code References by File

### Daemon (`rust-crates/swictation-daemon/`)

| File | Line(s) | Function | Notes |
|------|---------|----------|-------|
| `src/socket_utils.rs` | 15-67 | `get_socket_dir()` | ✅ Platform abstraction (Unix only) |
| `src/socket_utils.rs` | 71-73 | `get_ipc_socket_path()` | ✅ IPC socket path |
| `src/socket_utils.rs` | 76-78 | `get_metrics_socket_path()` | ✅ Metrics socket path |
| `src/socket_utils.rs` | 83-93 | `secure_socket_permissions()` | ✅ Unix permissions |
| `src/main.rs` | 430-437 | IPC server creation | ✅ Uses `get_ipc_socket_path()` |
| `src/main.rs` | 83 | Metrics socket path | ✅ Uses `get_metrics_socket_path()` |
| `src/config.rs` | 72 | `socket_path` field | ℹ️ Config override (rarely used) |
| `src/config.rs` | 118 | Default config | ✅ Uses `get_ipc_socket_path()` |

---

### Tauri UI (`tauri-ui/src-tauri/`)

| File | Line(s) | Function | Notes |
|------|---------|----------|-------|
| `src/socket/socket_utils.rs` | 16-40 | `get_socket_dir()` | ✅ Matches daemon (simplified) |
| `src/socket/socket_utils.rs` | 43-45 | `get_metrics_socket_path()` | ✅ Metrics socket path |
| `src/socket/metrics.rs` | - | `MetricsSocket::new()` | ✅ Uses `get_metrics_socket_path()` |

---

### NPM Package (`npm-package/`)

| File | Line(s) | Function | Notes |
|------|---------|----------|-------|
| `src/socket-paths.js` | 19-45 | `getSocketDir()` | ✅ Matches Rust implementation |
| `src/socket-paths.js` | 52-54 | `getIpcSocketPath()` | ✅ Returns IPC socket path |
| `src/socket-paths.js` | 60-62 | `getMetricsSocketPath()` | ✅ Returns metrics socket path |
| `postinstall.js` | 2022 | Config template | ❌ Hardcoded `/tmp/` |
| `postinstall.js` | 2342 | Sway example | ✅ Uses `$XDG_RUNTIME_DIR` |

---

### Python Tools (`src/`)

| File | Line(s) | Function | Notes |
|------|---------|----------|-------|
| `swictation_cli.py` | 12 | `SOCKET_PATH` constant | ❌ **HARDCODED `/tmp/`** |
| `ui/swictation_tray.py` | 15-33 | `get_socket_path()` | ✅ Dynamic resolution |

---

## 6. Cross-Platform Readiness Matrix

| Component | Linux | macOS | Windows | Fix Required |
|-----------|-------|-------|---------|--------------|
| Daemon socket_utils | ✅ | ✅ | ❌ | 🔧 Add Windows Named Pipes |
| Tauri socket_utils | ✅ | ✅ | ❌ | 🔧 Add Windows Named Pipes |
| NPM socket-paths.js | ✅ | ✅ | ❌ | 🔧 Add Windows detection |
| Python tray app | ✅ | ✅ | N/A | ℹ️ Python not used on Windows |
| Python CLI | ❌ | ❌ | ❌ | 🔧 **Fix hardcoded path** |
| Config examples | ⚠️ | ⚠️ | ⚠️ | 💡 Improve documentation |

---

## 7. Recommendations

### 7.1 Immediate Fixes (Pre-Windows Support)

1. **Fix `src/swictation_cli.py` (HIGH PRIORITY)**
   - Replace hardcoded `/tmp/swictation.sock` with dynamic resolution
   - Copy logic from `swictation_tray.py`
   - **Impact:** CLI currently broken on systems using XDG_RUNTIME_DIR

2. **Fix `npm-package/postinstall.js` Config Template**
   - Line 2022: Use `${XDG_RUNTIME_DIR:-$HOME/.local/share}/swictation.sock`
   - Remove hardcoded `/tmp/` reference

3. **Improve `config/config.example.toml` Documentation**
   - Add platform-specific path examples
   - Explain auto-detection logic
   - Note that manual override is rarely needed

---

### 7.2 Windows Support Implementation

**Estimated Effort:** 40-60 hours (medium-large project)

#### Phase 1: Foundation (8-12 hours)
- [ ] Add `tokio::net::windows::named_pipe` to dependencies
- [ ] Create `socket_utils_windows.rs` module
- [ ] Implement `get_pipe_name()` functions
- [ ] Add Windows CI testing

#### Phase 2: Daemon (12-16 hours)
- [ ] Abstract IPC server behind trait
- [ ] Implement Windows Named Pipe server
- [ ] Handle pipe connection lifecycle
- [ ] Test IPC protocol on Windows

#### Phase 3: Clients (12-16 hours)
- [ ] Update Tauri UI for Windows pipes
- [ ] Update NPM package socket resolution
- [ ] Create Windows CLI tool (PowerShell/Rust)
- [ ] Test all client connections

#### Phase 4: Testing & Documentation (8-16 hours)
- [ ] Integration tests on Windows
- [ ] Update all documentation
- [ ] Create Windows installation guide
- [ ] Validate security model

**Dependencies:**
```toml
[target.'cfg(windows)'.dependencies]
tokio = { version = "1", features = ["net", "io-util"] }
windows = { version = "0.52", features = ["Win32_System_Pipes"] }
```

**Key Files to Modify:**
1. `rust-crates/swictation-daemon/src/socket_utils.rs` (add Windows branch)
2. `rust-crates/swictation-daemon/src/ipc.rs` (abstract server creation)
3. `tauri-ui/src-tauri/src/socket/socket_utils.rs` (add Windows support)
4. `npm-package/src/socket-paths.js` (add Windows pipe detection)

---

## 8. Security Considerations

### 8.1 Unix Sockets (Linux/macOS)

**Current Security Model:**
- ✅ Sockets in user-only directories (mode 0700)
- ✅ Socket files created with mode 0600
- ✅ XDG_RUNTIME_DIR automatically secured by systemd
- ✅ No world-readable paths

**Attack Surface:**
- ✅ Only accessible by socket owner
- ✅ No network exposure
- ✅ Standard Unix DAC (Discretionary Access Control)

---

### 8.2 Windows Named Pipes (Future)

**Security Requirements:**
- Must use Security Descriptors to restrict access
- Should use `PIPE_REJECT_REMOTE_CLIENTS` flag
- Need proper ACLs (Access Control Lists)
- Consider using user SID for pipe naming

**Example Secure Pipe Creation:**
```rust
use windows::Win32::System::Pipes::*;
use windows::Win32::Security::*;

let pipe_name = format!(r"\\.\pipe\swictation-{}", get_user_sid()?);
let security_attrs = create_restricted_security_descriptor()?;

let pipe = CreateNamedPipeW(
    pipe_name,
    PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_UNLIMITED_INSTANCES,
    1024,  // Out buffer
    1024,  // In buffer
    0,     // Timeout
    &security_attrs,
)?;
```

---

## 9. Conclusion

### Current State
✅ **Linux and macOS implementations are production-ready** with:
- Robust platform-specific path resolution
- Proper security permissions
- Consistent abstraction across Rust, JavaScript, and Python
- XDG compliance on Linux

❌ **Windows is completely unsupported** due to Unix socket dependency

🔧 **Minor bugs exist:**
- `src/swictation_cli.py` has hardcoded `/tmp/` path (broken)
- Config examples could be clearer

### Next Steps

**Option A: Quick Fixes Only (2-4 hours)**
- Fix `swictation_cli.py` hardcoded path
- Update config examples
- Document Unix-only limitation

**Option B: Full Windows Support (40-60 hours)**
- Implement Named Pipes backend
- Abstract socket layer
- Comprehensive testing
- Update all clients

**Recommendation:** Start with **Option A** to fix immediate bugs, then evaluate Windows support based on user demand. The architecture is well-designed for adding Windows support later without major refactoring.

---

## Appendix A: File Locations Reference

**Daemon Socket Utilities:**
```
/opt/swictation/rust-crates/swictation-daemon/src/socket_utils.rs
```

**Tauri UI Socket Utilities:**
```
/opt/swictation/tauri-ui/src-tauri/src/socket/socket_utils.rs
```

**NPM Package Socket Paths:**
```
/opt/swictation/npm-package/src/socket-paths.js
```

**Python Tray App:**
```
/opt/swictation/src/ui/swictation_tray.py
```

**Python CLI (BROKEN):**
```
/opt/swictation/src/swictation_cli.py:12
```

**Config Example:**
```
/opt/swictation/config/config.example.toml
```

---

## Appendix B: Socket Path Resolution Flowchart

```
┌─────────────────────────────────┐
│   Application Starts            │
└────────────┬────────────────────┘
             │
             ▼
      ┌─────────────┐
      │ Platform?   │
      └──┬──────┬───┘
         │      │
    macOS│      │Linux/Other
         │      │
         ▼      ▼
    ┌────────┐ ┌──────────────────┐
    │ ~/Lib  │ │ XDG_RUNTIME_DIR? │
    │ /App   │ └────┬─────────────┘
    │ Support│      │
    └────┬───┘   Yes│  No
         │          ▼   ▼
         │      ┌────┐ ┌──────────┐
         │      │Use │ │~/.local/ │
         │      │XDG │ │share/    │
         │      └──┬─┘ │swictation│
         │         │   └────┬─────┘
         │         │        │
         └─────────┴────────┘
                   │
                   ▼
         ┌──────────────────┐
         │Create directory  │
         │with mode 0700    │
         └────────┬─────────┘
                  │
                  ▼
         ┌──────────────────┐
         │Join socket name  │
         │(swictation.sock) │
         └────────┬─────────┘
                  │
                  ▼
            ┌──────────┐
            │  DONE    │
            └──────────┘
```

---

**End of Report**
