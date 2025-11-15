# Display Server Detection Mechanism

**Version:** 1.0
**Date:** 2025-11-14
**Author:** Coder Agent (Hive Mind)
**Component:** Display Server Abstraction

---

## Overview

The detection mechanism uses an **evidence-based scoring system** to reliably identify the active display server (X11, Wayland, or Unknown) with a confidence level.

---

## Detection Algorithm

### 1. Evidence Collection

The detector examines multiple environment variables and system properties:

| Signal | Source | Wayland Score | X11 Score | Notes |
|--------|--------|---------------|-----------|-------|
| `WAYLAND_DISPLAY` | Environment | +3 | 0 | Strongest Wayland signal |
| `DISPLAY` | Environment | 0 | +1 | Present in X11 and XWayland |
| `XDG_SESSION_TYPE=wayland` | Environment | +3 | 0 | Reliable session type |
| `XDG_SESSION_TYPE=x11` | Environment | 0 | +3 | Reliable session type |
| `WAYLAND_COMPOSITOR` | Environment | +2 | 0 | Compositor name |
| `GDK_BACKEND=wayland` | Environment | +1 | 0 | Toolkit hint |
| `GDK_BACKEND=x11` | Environment | 0 | +1 | Toolkit hint |

### 2. Confidence Scoring

**Confidence Levels:**
- **High:** Score ≥ 4 (multiple confirming signals)
- **Medium:** Score ≥ 2 (single reliable signal)
- **Low:** Score < 2 (guessing/fallback)

**Examples:**

**Wayland on GNOME:**
```bash
WAYLAND_DISPLAY=wayland-0       # +3
XDG_SESSION_TYPE=wayland        # +3
WAYLAND_COMPOSITOR=gnome-shell  # +2
# Total: 8 → High confidence Wayland
```

**X11 on i3:**
```bash
DISPLAY=:0              # +1
XDG_SESSION_TYPE=x11    # +3
# Total: 4 → High confidence X11
```

**XWayland (tricky case):**
```bash
WAYLAND_DISPLAY=wayland-0  # +3
DISPLAY=:0                 # +1
XDG_SESSION_TYPE=wayland   # +3
# Wayland wins: 6 vs 1 → High confidence Wayland (correct!)
```

---

## Implementation

### Core Function

```rust
pub fn detect_display_server() -> DetectionResult {
    let mut evidence = Vec::new();
    let mut wayland_score = 0;
    let mut x11_score = 0;

    // Check WAYLAND_DISPLAY (strongest Wayland signal)
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        evidence.push("WAYLAND_DISPLAY set".to_string());
        wayland_score += 3;
    }

    // Check DISPLAY (X11 signal, but also present in XWayland)
    if let Ok(display) = std::env::var("DISPLAY") {
        evidence.push(format!("DISPLAY={}", display));
        x11_score += 1;
    }

    // Check XDG_SESSION_TYPE (reliable but can be absent)
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        evidence.push(format!("XDG_SESSION_TYPE={}", session_type));
        match session_type.as_str() {
            "wayland" => wayland_score += 3,
            "x11" => x11_score += 3,
            _ => {},
        }
    }

    // Check for Wayland compositor
    if let Ok(wc) = std::env::var("WAYLAND_COMPOSITOR") {
        evidence.push(format!("WAYLAND_COMPOSITOR={}", wc));
        wayland_score += 2;
    }

    // Check GDK backend hint
    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        evidence.push(format!("GDK_BACKEND={}", backend));
        match backend.as_str() {
            "wayland" => wayland_score += 1,
            "x11" => x11_score += 1,
            _ => {},
        }
    }

    // Determine winner and confidence
    let (server_type, confidence) = if wayland_score > x11_score {
        let conf = match wayland_score {
            s if s >= 4 => DetectionConfidence::High,
            s if s >= 2 => DetectionConfidence::Medium,
            _ => DetectionConfidence::Low,
        };
        (DisplayServerType::Wayland, conf)
    } else if x11_score > wayland_score {
        let conf = match x11_score {
            s if s >= 4 => DetectionConfidence::High,
            s if s >= 2 => DetectionConfidence::Medium,
            _ => DetectionConfidence::Low,
        };
        (DisplayServerType::X11, conf)
    } else {
        (DisplayServerType::Unknown, DetectionConfidence::Low)
    };

    DetectionResult {
        server_type,
        confidence,
        evidence,
    }
}
```

---

## XWayland Detection

XWayland is X11 running on top of Wayland. The detector correctly identifies the underlying Wayland session:

```rust
pub fn is_xwayland() -> bool {
    // XWayland has both WAYLAND_DISPLAY and DISPLAY set
    std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("DISPLAY").is_ok()
        && std::env::var("XDG_SESSION_TYPE")
            .map(|t| t == "wayland")
            .unwrap_or(false)
}
```

**Why this matters:**
- XWayland apps think they're on X11 (`DISPLAY` is set)
- But the compositor is Wayland
- Detection correctly returns `Wayland` as the display server
- Use `is_xwayland()` to detect this specific case if needed

---

## Logging and Debugging

### Detection Result Logging

```rust
let detection = detect_display_server();

info!("Display server detected: {:?}", detection.server_type);
info!("Detection confidence: {:?}", detection.confidence);
info!("Evidence:");
for ev in &detection.evidence {
    info!("  - {}", ev);
}

if detection.confidence == DetectionConfidence::Low {
    warn!("Low confidence detection! Consider setting display.backend in config");
}
```

**Example Output:**
```
INFO  Display server detected: Wayland
INFO  Detection confidence: High
INFO  Evidence:
INFO    - WAYLAND_DISPLAY set
INFO    - DISPLAY=:0
INFO    - XDG_SESSION_TYPE=wayland
INFO    - WAYLAND_COMPOSITOR=sway
```

---

## Manual Override

Users can override auto-detection in config:

```toml
# ~/.config/swictation/config.toml
[display]
backend = "wayland"  # Force Wayland (skip detection)
```

```rust
impl DisplayServerManager {
    pub fn new_with_config(config: &DisplayConfig) -> Result<Self> {
        let detection = if let Some(backend_override) = &config.backend_override {
            // Use manual override
            DetectionResult {
                server_type: backend_override.clone(),
                confidence: DetectionConfidence::High,
                evidence: vec!["Manual override from config".to_string()],
            }
        } else {
            // Auto-detect
            detect_display_server()
        };

        // ... create backend
    }
}
```

---

## Edge Cases

### Case 1: Neither WAYLAND_DISPLAY nor DISPLAY Set

**Environment:**
```bash
# No display variables
```

**Result:**
```rust
DetectionResult {
    server_type: DisplayServerType::Unknown,
    confidence: DetectionConfidence::Low,
    evidence: vec![],
}
```

**Fallback Strategy:**
```rust
match detection.server_type {
    DisplayServerType::Unknown => {
        // Try Wayland first (modern default)
        if WaylandBackend::is_available() {
            warn!("Unknown display server, trying Wayland");
            Arc::new(WaylandBackend)
        } else if X11Backend::is_available() {
            warn!("Wayland unavailable, trying X11");
            Arc::new(X11Backend)
        } else {
            bail!("No display server detected");
        }
    }
}
```

### Case 2: SSH Session (No Display)

**Environment:**
```bash
SSH_CONNECTION=192.168.1.10 52134 192.168.1.20 22
# No DISPLAY or WAYLAND_DISPLAY
```

**Behavior:**
- Detection returns `Unknown`
- Tool validation will fail (wtype/xdotool can't connect)
- Daemon should exit with helpful error message

```rust
Err(anyhow::anyhow!(
    "No display server available. Swictation requires X11 or Wayland.\n\
     Cannot run in SSH session without X11 forwarding or Wayland access."
))
```

### Case 3: Headless Server

**Environment:**
```bash
# Running as systemd service on headless server
```

**Behavior:**
- Same as SSH case
- Detection returns `Unknown`
- Daemon fails gracefully with error message

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_detection() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        std::env::set_var("XDG_SESSION_TYPE", "wayland");

        let result = detect_display_server();
        assert_eq!(result.server_type, DisplayServerType::Wayland);
        assert!(result.confidence >= DetectionConfidence::High);
    }

    #[test]
    fn test_x11_detection() {
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::set_var("DISPLAY", ":0");
        std::env::set_var("XDG_SESSION_TYPE", "x11");

        let result = detect_display_server();
        assert_eq!(result.server_type, DisplayServerType::X11);
        assert!(result.confidence >= DetectionConfidence::High);
    }

    #[test]
    fn test_xwayland_detection() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        std::env::set_var("DISPLAY", ":0");
        std::env::set_var("XDG_SESSION_TYPE", "wayland");

        let result = detect_display_server();
        assert_eq!(result.server_type, DisplayServerType::Wayland);
        assert!(is_xwayland());
    }

    #[test]
    fn test_unknown_detection() {
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");

        let result = detect_display_server();
        assert_eq!(result.server_type, DisplayServerType::Unknown);
        assert_eq!(result.confidence, DetectionConfidence::Low);
    }

    #[test]
    fn test_evidence_collection() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        std::env::set_var("WAYLAND_COMPOSITOR", "sway");

        let result = detect_display_server();
        assert!(!result.evidence.is_empty());
        assert!(result.evidence.iter().any(|e| e.contains("WAYLAND_DISPLAY")));
    }
}
```

---

## Performance

### Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_detection(c: &mut Criterion) {
    std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
    std::env::set_var("XDG_SESSION_TYPE", "wayland");

    c.bench_function("detect_display_server", |b| {
        b.iter(|| {
            let _ = black_box(detect_display_server());
        });
    });
}

criterion_group!(benches, bench_detection);
criterion_main!(benches);
```

**Expected Performance:**
- **Detection time:** < 1μs (environment variable lookups are very fast)
- **Memory overhead:** Negligible (few strings in evidence vector)
- **Cache:** Detection result should be cached, run once at startup

---

## Future Enhancements

### 1. Runtime Re-detection

```rust
impl DisplayServerManager {
    pub fn refresh_detection(&mut self) -> Result<()> {
        // Re-run detection
        let new_detection = detect_display_server();

        // Switch backend if changed
        if new_detection.server_type != self.detection.server_type {
            info!("Display server changed: {:?} -> {:?}",
                  self.detection.server_type,
                  new_detection.server_type);

            self.backend = Self::create_backend(&new_detection)?;
            self.detection = new_detection;
        }

        Ok(())
    }
}
```

**Use Case:** Hot-switching between display servers (rare, but possible)

### 2. Additional Detection Methods

```rust
// Check for running Wayland compositor process
fn detect_wayland_compositor() -> Option<String> {
    // Check for sway, wlroots, mutter, kwin_wayland processes
    // Return compositor name if found
}

// Check X11 via file system
fn detect_x11_socket() -> bool {
    // Check for /tmp/.X11-unix/X0
    std::path::Path::new("/tmp/.X11-unix/X0").exists()
}
```

### 3. Confidence Boosting

```rust
// Use process inspection to boost confidence
if let Some(compositor) = detect_wayland_compositor() {
    wayland_score += 2;
    evidence.push(format!("Running compositor: {}", compositor));
}

if detect_x11_socket() {
    x11_score += 2;
    evidence.push("X11 socket found".to_string());
}
```

---

## Conclusion

The detection mechanism provides:

✅ **Reliable identification** of X11 vs Wayland
✅ **Confidence scoring** for transparency
✅ **Evidence tracking** for debugging
✅ **XWayland handling** for edge cases
✅ **Fallback strategies** for unknown cases
✅ **Manual override** for power users

The scoring system is **robust against common pitfalls** like XWayland and provides clear feedback through confidence levels and evidence collection.
