# Unix Socket Security Implementation

## Overview

This document describes the security improvements made to Swictation's Unix socket communication (Issue #5).

## Problem Statement

**Security Vulnerability:** Unix sockets at `/tmp/swictation.sock` and `/tmp/swictation_metrics.sock` were world-accessible, allowing any local user to:
- Toggle recording to capture private dictation
- Spam toggle commands (DoS)
- Read metrics to infer user activity

## Solution

### Approach: Filesystem Permissions Only

We chose **filesystem permissions** over complex authentication for these reasons:
1. ✅ Simple and lightweight - zero latency impact on hotkey toggles
2. ✅ OS-enforced security (no application-level auth bugs)
3. ✅ Transparent to hotkey bindings in compositors
4. ✅ Works identically across X11/Sway/Wayland

### Implementation Details

#### Socket Location Changes

**Old (Insecure):**
```
/tmp/swictation.sock          (mode: 0666, any user can access)
/tmp/swictation_metrics.sock  (mode: 0666, any user can access)
```

**New (Secure):**
```
$XDG_RUNTIME_DIR/swictation.sock          (mode: 0600, owner-only)
$XDG_RUNTIME_DIR/swictation_metrics.sock  (mode: 0600, owner-only)
```

**Fallback (if XDG_RUNTIME_DIR unavailable):**
```
~/.local/share/swictation/swictation.sock          (mode: 0600)
~/.local/share/swictation/swictation_metrics.sock  (mode: 0600)
```

#### Benefits of XDG_RUNTIME_DIR

1. **User-Specific:** Each user has their own directory (`/run/user/<uid>/`)
2. **Secure by Default:** Directory is mode 0700 (owner-only access)
3. **Auto-Cleanup:** Contents removed on logout
4. **Standard:** Part of XDG Base Directory specification

## Modified Components

### Rust Components

1. **New Module:** `swictation-daemon/src/socket_utils.rs`
   - `get_socket_dir()` - Detects XDG_RUNTIME_DIR or falls back
   - `get_ipc_socket_path()` - Returns main toggle socket path
   - `get_metrics_socket_path()` - Returns metrics broadcast socket path

2. **Updated Modules:**
   - `swictation-daemon/src/ipc.rs` - Sets mode 0600 after socket creation
   - `swictation-daemon/src/main.rs` - Uses new socket path helpers
   - `swictation-daemon/src/config.rs` - Default config uses correct socket path
   - `swictation-broadcaster/src/broadcaster.rs` - Sets mode 0600
   - `swictation-daemon/src/hotkey.rs` - Updated example commands

3. **Tauri UI:** `tauri-ui/src-tauri/src/socket/`
   - New: `socket_utils.rs` (matches daemon implementation)
   - Updated: `metrics.rs` to use new socket paths
   - Updated: `mod.rs` to export utilities

### JavaScript Components

1. **New Module:** `npm-package/src/socket-paths.js`
   - `getSocketDir()` - Same logic as Rust implementation
   - `getIpcSocketPath()` - Main toggle socket path
   - `getMetricsSocketPath()` - Metrics broadcast socket path

2. **Updated:** `npm-package/bin/swictation`
   - `handleStatus()` - Shows new socket paths
   - `handleToggle()` - Uses new socket path
   - `handleSetup()` - Updated hotkey examples

## Hotkey Configuration Updates

### Sway/i3

**Old:**
```bash
bindsym $mod+Shift+d exec echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock
```

**New (Recommended):**
```bash
bindsym $mod+Shift+d exec swictation toggle
```

**New (Direct):**
```bash
bindsym $mod+Shift+d exec echo '{"action":"toggle"}' | nc -U $XDG_RUNTIME_DIR/swictation.sock
```

### GNOME Wayland

**Command for custom keyboard shortcut:**
```bash
swictation toggle
```
(The CLI tool handles socket path resolution)

## Backward Compatibility

⚠️ **Breaking Change:** Existing hotkey configurations using `/tmp/swictation.sock` will need to be updated.

**Migration Path:**
1. Update hotkey bindings to use `swictation toggle` command
2. Or update paths to `$XDG_RUNTIME_DIR/swictation.sock`
3. Restart daemon to create sockets in new location

## Security Analysis

### Before (Vulnerable)

```bash
$ ls -la /tmp/swictation*.sock
srwxrwxrwx 1 alice users 0 Nov 16 12:00 /tmp/swictation.sock
srwxrwxrwx 1 alice users 0 Nov 16 12:00 /tmp/swictation_metrics.sock

# Attacker (bob) can:
$ echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock  # Spy on alice!
$ while true; do echo '{"action":"toggle"}' | nc -U /tmp/swictation.sock; done  # DoS
$ nc -U /tmp/swictation_metrics.sock  # Read alice's metrics
```

### After (Secure)

```bash
$ ls -la $XDG_RUNTIME_DIR/swictation*.sock
srw------- 1 alice users 0 Nov 16 12:00 /run/user/1000/swictation.sock
srw------- 1 alice users 0 Nov 16 12:00 /run/user/1000/swictation_metrics.sock

# Attacker (bob) cannot access:
$ echo '{"action":"toggle"}' | nc -U /run/user/1000/swictation.sock
nc: Permission denied

# Bob's own sockets are isolated:
$ ls -la /run/user/1001/
# (empty or bob's own sockets)
```

## Testing

### Verify Socket Security

```bash
# Check socket location
swictation status

# Verify permissions
ls -la $XDG_RUNTIME_DIR/swictation*.sock
# Should show: srw------- (mode 0600)

# Test toggle command
swictation toggle
# Should work for owner, fail for other users
```

### Integration Tests

```bash
# Start daemon
swictation start

# Test from same user (should succeed)
echo '{"action":"toggle"}' | nc -U $XDG_RUNTIME_DIR/swictation.sock

# Test from different user (should fail)
sudo -u otheruser nc -U $XDG_RUNTIME_DIR/swictation.sock
# Expected: Permission denied
```

## Performance Impact

✅ **Zero performance impact:**
- Socket creation: One-time cost at daemon startup
- Socket access: Identical performance (OS permission check is ~nanoseconds)
- Hotkey latency: Unchanged (permission check happens at socket bind, not on each access)

## Future Enhancements

While filesystem permissions are sufficient for the current threat model, future enhancements could include:

1. **Peer Credential Verification** (Linux-specific)
   - Use `SO_PEERCRED` to verify connecting process UID
   - Additional layer of defense

2. **Token-Based Authentication** (Cross-platform)
   - Generate random token on daemon startup
   - Store in `~/.local/share/swictation/auth_token` (mode 0600)
   - Clients send token with each request
   - Useful for remote access scenarios

3. **Audit Logging**
   - Log all toggle commands with timestamp and UID
   - Detect suspicious patterns

## References

- XDG Base Directory Specification: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
- Unix Socket Security: `man 7 unix`
- File Permissions: `man 2 chmod`

## Related Issues

- Issue #1: Unbounded Channels (memory safety)
- Issue #2: No Audio Backpressure
- Issue #4: Sequential VAD/STT Processing

---

**Status:** ✅ Implemented and tested
**Date:** 2025-11-17
**Author:** Archon (AI Assistant)
