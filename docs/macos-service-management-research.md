# macOS Service Management Research

**Research Date:** 2025-11-23
**Purpose:** Document auto-start service management on macOS for Swictation background daemon

## Executive Summary

**CRITICAL FINDING:** macOS has **three** auto-start mechanisms with different tradeoffs:

1. **launchd LaunchAgents** (Legacy, but universal) - XML plist files
2. **SMAppService** (Modern, macOS 13+) - Swift/Objective-C framework API
3. **Login Items** (Simple, but deprecated) - System Preferences UI

**Key Decision:** Use **launchd LaunchAgents** for npm package distribution with documented **SMAppService migration path** for future Tauri app bundle.

**Rationale:**
- npm packages can't easily include app bundles (required for SMAppService)
- launchd plists work universally (macOS 10.4+)
- User-level LaunchAgents don't require sudo
- Legacy status doesn't mean removal (stable for years)

---

## Table of Contents

1. [launchd Architecture](#launchd-architecture)
2. [LaunchAgents vs LaunchDaemons](#launchagents-vs-launchdaemons)
3. [SMAppService (Modern Alternative)](#smappservice-modern-alternative)
4. [Plist File Structure](#plist-file-structure)
5. [npm Postinstall Strategy](#npm-postinstall-strategy)
6. [Permission Requirements](#permission-requirements)
7. [Testing and Verification](#testing-and-verification)
8. [Implementation Plan](#implementation-plan)

---

## launchd Architecture

### What is launchd?

`launchd` is macOS's **first process** after the kernel (PID 1), responsible for:
- Launching system services and daemons
- Managing user agents at login
- Socket activation and on-demand launching
- Environment variable management
- Resource limits and sandboxing

**Think of it as:** systemd (Linux) or Windows Services equivalent for macOS.

### Service Types

```
┌─────────────────────────────────────────────┐
│           macOS Service Types               │
├─────────────────────────────────────────────┤
│                                             │
│  LaunchDaemons (System-level)               │
│  - Run as root                              │
│  - Start at boot (before login)             │
│  - Location: /Library/LaunchDaemons         │
│  - Example: Web servers, databases          │
│                                             │
│  LaunchAgents (User-level)                  │
│  - Run as current user                      │
│  - Start at login                           │
│  - Locations:                               │
│    • ~/Library/LaunchAgents (per-user)      │
│    • /Library/LaunchAgents (all users)      │
│  - Example: Menu bar apps, user daemons    │
│                                             │
└─────────────────────────────────────────────┘
```

---

## LaunchAgents vs LaunchDaemons

### Comparison Table

| Feature | LaunchAgent | LaunchDaemon |
|---------|-------------|--------------|
| **Runs as** | Current user | root or specified user |
| **Starts when** | User login | System boot |
| **GUI Access** | ✅ Yes (has WindowServer access) | ❌ No GUI |
| **User home access** | ✅ Yes | ❌ No (unless configured) |
| **Location (user)** | ~/Library/LaunchAgents | N/A |
| **Location (system)** | /Library/LaunchAgents | /Library/LaunchDaemons |
| **Permissions** | User-owned (644) | root-owned (644) |
| **sudo required** | ❌ No (user agents) | ✅ Yes |
| **Use case** | User apps, menu bar | System services |

### For Swictation

**Recommendation:** Use **LaunchAgent** (user-level)

**Why:**
- ✅ Needs GUI access (Accessibility API, menu bar UI)
- ✅ Needs user home directory access (config files)
- ✅ Runs at user login (expected behavior)
- ✅ No sudo required for installation
- ✅ Per-user configuration (multi-user Macs)

**NOT LaunchDaemon because:**
- ❌ No GUI access (can't show UI or inject text)
- ❌ Requires sudo (bad UX for npm install)
- ❌ Runs at boot (too early, no user session)

---

## SMAppService (Modern Alternative)

### What is SMAppService?

**Introduced:** macOS 13 Ventura (2022)
**Framework:** ServiceManagement (Swift/Objective-C)
**Purpose:** Modern replacement for SMLoginItemSetEnabled and SMJobBless

**Key Features:**
- ✅ Appears in System Settings → General → Login Items
- ✅ User approval UI (transparent, secure)
- ✅ Helper apps live **inside app bundle**
- ✅ Better sandboxing support
- ⚠️ Requires macOS 13+ (Ventura and later)
- ⚠️ Requires app bundle (not standalone binaries)

### How SMAppService Works

```
App Bundle Structure:
MyApp.app/
├── Contents/
│   ├── MacOS/
│   │   └── MyApp (main executable)
│   ├── Library/
│   │   └── LoginItems/
│   │       └── MyHelper.app/  ← Helper lives HERE
│   │           └── Contents/
│   │               └── MacOS/
│   │                   └── MyHelper
│   └── Info.plist
```

**Registration Code (Swift):**
```swift
import ServiceManagement

let service = SMAppService.loginItem(identifier: "com.example.MyHelper")

do {
    try service.register()
    print("Helper registered successfully")
} catch {
    print("Failed to register: \(error)")
}
```

**User Experience:**
- Appears in System Settings → General → Login Items
- User sees "MyApp Helper" with toggle switch
- Transparent approval process
- Can be disabled without uninstalling app

### SMAppService vs launchd Comparison

| Feature | SMAppService | launchd LaunchAgent |
|---------|--------------|---------------------|
| **macOS Version** | 13+ only | 10.4+ (universal) |
| **User approval** | Built-in UI | Manual TCC prompt |
| **App bundle** | Required | Not required |
| **Distribution** | Requires .app bundle | Works with npm/brew |
| **User transparency** | System Settings UI | launchctl list only |
| **Legacy status** | Modern, recommended | "Legacy" but stable |
| **npm compatibility** | ❌ Poor (needs .app) | ✅ Excellent |

---

## Plist File Structure

### Example: swictation-daemon LaunchAgent

**Location:** `~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Service Identification -->
    <key>Label</key>
    <string>com.agidreams.swictation.daemon</string>

    <!-- Program to Run -->
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/swictation-daemon</string>
        <string>--background</string>
    </array>

    <!-- Auto-Start Configuration -->
    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <!-- Restart if crashes -->
        <key>SuccessfulExit</key>
        <false/>
        <!-- Don't restart if clean exit -->
        <key>Crashed</key>
        <true/>
    </dict>

    <!-- Environment Variables -->
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>

    <!-- Logging -->
    <key>StandardOutPath</key>
    <string>/tmp/swictation-daemon.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/swictation-daemon.error.log</string>

    <!-- Process Management -->
    <key>ProcessType</key>
    <string>Interactive</string>

    <!-- Throttling (prevent rapid restarts) -->
    <key>ThrottleInterval</key>
    <integer>10</integer>

    <!-- Resource Limits -->
    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>1024</integer>
    </dict>
</dict>
</plist>
```

### Example: swictation-ui LaunchAgent

**Location:** `~/Library/LaunchAgents/com.agidreams.swictation.ui.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agidreams.swictation.ui</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/swictation-ui</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <!-- Keep UI alive (menu bar app should stay running) -->
    <key>KeepAlive</key>
    <true/>

    <!-- Delay start to ensure daemon is ready -->
    <key>StartInterval</key>
    <integer>5</integer>

    <key>ProcessType</key>
    <string>Interactive</string>

    <key>StandardOutPath</key>
    <string>/tmp/swictation-ui.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/swictation-ui.error.log</string>
</dict>
</plist>
```

### Key Plist Keys Explained

**Core Keys:**
- `Label` - Unique identifier (reverse DNS format)
- `ProgramArguments` - Executable path and arguments (array, not string!)
- `RunAtLoad` - Start at login (true) or on-demand (false)

**Process Management:**
- `KeepAlive` - Restart policy (true/false/dict with conditions)
- `ProcessType` - Interactive (GUI access) or Background
- `ThrottleInterval` - Minimum seconds between restarts

**Logging:**
- `StandardOutPath` - stdout redirect
- `StandardErrorPath` - stderr redirect
- **CRITICAL:** Log paths must be writable by user!

**Environment:**
- `EnvironmentVariables` - Environment vars (PATH, etc.)
- `WorkingDirectory` - Process working directory

**Timing:**
- `StartInterval` - Run every N seconds (alternative to RunAtLoad)
- `StartCalendarInterval` - Cron-like scheduling

**Resource Limits:**
- `SoftResourceLimits` - Limits (files, memory, CPU)
- `HardResourceLimits` - Hard limits (cannot exceed)

---

## npm Postinstall Strategy

### Challenge: npm Installation Permissions

**Problem:** npm global install doesn't have sudo by default.

**Solutions:**

#### Solution 1: Change npm Prefix (Recommended)

```bash
# User runs this once (documented in README)
npm config set prefix ~/.npm-global
export PATH=~/.npm-global/bin:$PATH

# Then install works without sudo
npm install -g swictation
```

**Pros:**
- ✅ No sudo required
- ✅ User-level installation
- ✅ Works with LaunchAgents in ~/Library

**Cons:**
- ⚠️ Requires user setup step
- ⚠️ PATH modification needed

#### Solution 2: System-level Install with sudo

```bash
# User runs with sudo
sudo npm install -g swictation
```

**Pros:**
- ✅ Standard npm location (/usr/local)
- ✅ Works with /Library/LaunchAgents

**Cons:**
- ❌ Requires sudo (worse UX)
- ❌ System-wide only (not per-user)

### Recommended Postinstall Script

**File:** `npm-package/postinstall.js`

```javascript
#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

// Only run on macOS
if (os.platform() !== 'darwin') {
    console.log('macOS service installation skipped (not macOS)');
    process.exit(0);
}

const homeDir = os.homedir();
const launchAgentsDir = path.join(homeDir, 'Library', 'LaunchAgents');

// Ensure LaunchAgents directory exists
if (!fs.existsSync(launchAgentsDir)) {
    fs.mkdirSync(launchAgentsDir, { recursive: true });
}

// Find swictation-daemon binary location
const npmPrefix = execSync('npm config get prefix', { encoding: 'utf8' }).trim();
const binDir = path.join(npmPrefix, 'bin');
const daemonPath = path.join(binDir, 'swictation-daemon');
const uiPath = path.join(binDir, 'swictation-ui');

// Check if binaries exist
if (!fs.existsSync(daemonPath)) {
    console.error(`Error: swictation-daemon not found at ${daemonPath}`);
    process.exit(1);
}

// Create daemon plist
const daemonPlist = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agidreams.swictation.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${daemonPath}</string>
        <string>--background</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/swictation-daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/swictation-daemon.error.log</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>`;

const daemonPlistPath = path.join(launchAgentsDir, 'com.agidreams.swictation.daemon.plist');

// Write daemon plist
fs.writeFileSync(daemonPlistPath, daemonPlist, { mode: 0o644 });
console.log(`✅ Created daemon LaunchAgent: ${daemonPlistPath}`);

// Create UI plist (if UI binary exists)
if (fs.existsSync(uiPath)) {
    const uiPlist = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agidreams.swictation.ui</string>
    <key>ProgramArguments</key>
    <array>
        <string>${uiPath}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/swictation-ui.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/swictation-ui.error.log</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>`;

    const uiPlistPath = path.join(launchAgentsDir, 'com.agidreams.swictation.ui.plist');
    fs.writeFileSync(uiPlistPath, uiPlist, { mode: 0o644 });
    console.log(`✅ Created UI LaunchAgent: ${uiPlistPath}`);
}

// Load the LaunchAgent (start service immediately)
try {
    execSync(`launchctl load ${daemonPlistPath}`, { stdio: 'inherit' });
    console.log('✅ Daemon service loaded successfully');

    if (fs.existsSync(path.join(launchAgentsDir, 'com.agidreams.swictation.ui.plist'))) {
        execSync(`launchctl load ${path.join(launchAgentsDir, 'com.agidreams.swictation.ui.plist')}`,
                 { stdio: 'inherit' });
        console.log('✅ UI service loaded successfully');
    }
} catch (error) {
    console.warn('⚠️  Failed to load service (may need manual load):');
    console.warn('  Run: launchctl load ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist');
}

console.log('\n🎉 Swictation services installed successfully!');
console.log('Services will start automatically at next login.');
console.log('\nTo start now:');
console.log('  launchctl start com.agidreams.swictation.daemon');
console.log('\nTo stop:');
console.log('  launchctl stop com.agidreams.swictation.daemon');
```

### Preuninstall Script

**File:** `npm-package/preuninstall.js`

```javascript
#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

if (os.platform() !== 'darwin') {
    process.exit(0);
}

const homeDir = os.homedir();
const launchAgentsDir = path.join(homeDir, 'Library', 'LaunchAgents');

const daemonPlistPath = path.join(launchAgentsDir, 'com.agidreams.swictation.daemon.plist');
const uiPlistPath = path.join(launchAgentsDir, 'com.agidreams.swictation.ui.plist');

// Unload and remove daemon
if (fs.existsSync(daemonPlistPath)) {
    try {
        execSync(`launchctl unload ${daemonPlistPath}`, { stdio: 'ignore' });
        console.log('✅ Daemon service unloaded');
    } catch (error) {
        // Already unloaded, continue
    }
    fs.unlinkSync(daemonPlistPath);
    console.log('✅ Daemon LaunchAgent removed');
}

// Unload and remove UI
if (fs.existsSync(uiPlistPath)) {
    try {
        execSync(`launchctl unload ${uiPlistPath}`, { stdio: 'ignore' });
        console.log('✅ UI service unloaded');
    } catch (error) {
        // Already unloaded, continue
    }
    fs.unlinkSync(uiPlistPath);
    console.log('✅ UI LaunchAgent removed');
}

console.log('✅ Swictation services removed successfully');
```

---

## Permission Requirements

### File Permissions

**LaunchAgent plist files MUST be:**
- **User-owned:** `chown $USER:staff ~/Library/LaunchAgents/*.plist`
- **Mode 644:** `chmod 644 ~/Library/LaunchAgents/*.plist`
- **Valid XML:** Validate with `plutil -lint file.plist`

**Incorrect permissions cause:**
- Service won't load
- Silent failures
- No error messages in Console

### Binary Permissions

**Swictation binaries MUST be:**
- **Executable:** `chmod +x /usr/local/bin/swictation-daemon`
- **Readable by user:** Binary should be in user's PATH
- **Not quarantined:** May need to clear quarantine attribute

**Check quarantine:**
```bash
xattr -l /usr/local/bin/swictation-daemon

# If shows com.apple.quarantine:
xattr -d com.apple.quarantine /usr/local/bin/swictation-daemon
```

### Accessibility Permissions

**Required for text injection:**
- macOS TCC (Transparency, Consent, and Control) framework
- System Settings → Privacy & Security → Accessibility
- Must grant permission to:
  - `swictation-daemon` binary
  - Terminal.app (if running from terminal)

**Checking permission in code:**
```rust
use objc::{class, msg_send, sel, sel_impl};

pub fn check_accessibility_permissions() -> bool {
    unsafe {
        let process: bool = msg_send![class!(NSWorkspace), accessibilityCheckPermission];
        process
    }
}
```

**Prompting for permission:**
```rust
use objc::{class, msg_send, sel, sel_impl, runtime::Object};

pub fn request_accessibility_permission() {
    unsafe {
        let options: *mut Object = msg_send![class!(NSDictionary), dictionary];
        let _: bool = msg_send![
            class!(NSWorkspace),
            acquirePrivilegesWithOptions: options
        ];
    }
}
```

---

## Testing and Verification

### Manual Testing Commands

**Load LaunchAgent:**
```bash
# Load (start and enable auto-start)
launchctl load ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist

# Verify loaded
launchctl list | grep swictation

# Check status
launchctl print gui/$(id -u)/com.agidreams.swictation.daemon
```

**Unload LaunchAgent:**
```bash
# Unload (stop and disable auto-start)
launchctl unload ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist
```

**Start/Stop Service:**
```bash
# Start (if loaded but stopped)
launchctl start com.agidreams.swictation.daemon

# Stop (but keep loaded for auto-start)
launchctl stop com.agidreams.swictation.daemon
```

**Check Logs:**
```bash
# View stdout
tail -f /tmp/swictation-daemon.log

# View stderr
tail -f /tmp/swictation-daemon.error.log

# View system logs
log stream --predicate 'subsystem contains "com.agidreams.swictation"'
```

### Validation Script

**File:** `scripts/test-launchd-macos.sh`

```bash
#!/bin/bash

set -e

echo "🧪 Testing macOS LaunchAgent installation..."

PLIST_PATH="$HOME/Library/LaunchAgents/com.agidreams.swictation.daemon.plist"

# 1. Check plist exists
if [ ! -f "$PLIST_PATH" ]; then
    echo "❌ Plist file not found: $PLIST_PATH"
    exit 1
fi
echo "✅ Plist file exists"

# 2. Validate plist syntax
if ! plutil -lint "$PLIST_PATH" > /dev/null 2>&1; then
    echo "❌ Invalid plist syntax"
    plutil -lint "$PLIST_PATH"
    exit 1
fi
echo "✅ Plist syntax valid"

# 3. Check permissions
PERMS=$(stat -f "%Op" "$PLIST_PATH" | tail -c 4)
if [ "$PERMS" != "0644" ]; then
    echo "⚠️  Warning: Plist permissions are $PERMS (expected 0644)"
    echo "   Fixing permissions..."
    chmod 644 "$PLIST_PATH"
fi
echo "✅ Plist permissions correct"

# 4. Check binary path in plist
BINARY_PATH=$(plutil -extract ProgramArguments.0 raw "$PLIST_PATH")
if [ ! -x "$BINARY_PATH" ]; then
    echo "❌ Binary not executable: $BINARY_PATH"
    exit 1
fi
echo "✅ Binary executable: $BINARY_PATH"

# 5. Try loading (unload first if already loaded)
echo "🔄 Loading LaunchAgent..."
launchctl unload "$PLIST_PATH" 2>/dev/null || true
sleep 1
launchctl load "$PLIST_PATH"

# 6. Verify loaded
sleep 2
if ! launchctl list | grep -q "com.agidreams.swictation.daemon"; then
    echo "❌ Service not loaded"
    exit 1
fi
echo "✅ Service loaded successfully"

# 7. Check process running
if ! pgrep -f swictation-daemon > /dev/null; then
    echo "⚠️  Warning: Process not running (may take a moment to start)"
    echo "   Check logs: tail -f /tmp/swictation-daemon.log"
else
    echo "✅ Process running"
fi

echo ""
echo "🎉 LaunchAgent test passed!"
echo ""
echo "To monitor logs:"
echo "  tail -f /tmp/swictation-daemon.log"
echo ""
echo "To unload:"
echo "  launchctl unload $PLIST_PATH"
```

### Common Issues

**Issue 1: "Nothing found to load"**
- **Cause:** Plist file doesn't exist or wrong path
- **Fix:** Check file path, ensure ~/Library/LaunchAgents exists

**Issue 2: Service loads but doesn't start**
- **Cause:** Binary path incorrect or binary not executable
- **Fix:** Verify ProgramArguments path, check `chmod +x`

**Issue 3: Service starts then immediately exits**
- **Cause:** Binary crashes or missing dependencies
- **Fix:** Check /tmp/swictation-daemon.error.log, test binary manually

**Issue 4: "Operation not permitted"**
- **Cause:** Accessibility permissions not granted
- **Fix:** Grant in System Settings → Privacy & Security → Accessibility

**Issue 5: Service doesn't restart after logout**
- **Cause:** RunAtLoad not set or plist not in correct location
- **Fix:** Ensure RunAtLoad is true, verify ~/Library/LaunchAgents location

---

## Implementation Plan

### Phase 1: Rust Binary Updates

**File:** `rust-crates/swictation-daemon/src/main.rs`

```rust
#[cfg(target_os = "macos")]
fn check_macos_permissions() -> Result<()> {
    use objc::{class, msg_send, sel, sel_impl};

    let has_permission: bool = unsafe {
        msg_send![class!(NSWorkspace), accessibilityCheckPermission]
    };

    if !has_permission {
        eprintln!("❌ Accessibility permission required!");
        eprintln!("   Go to: System Settings → Privacy & Security → Accessibility");
        eprintln!("   Enable: swictation-daemon");
        anyhow::bail!("Missing Accessibility permission");
    }

    Ok(())
}

fn main() -> Result<()> {
    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let background_mode = args.contains(&"--background".to_string());

    #[cfg(target_os = "macos")]
    check_macos_permissions()?;

    if background_mode {
        // Run as daemon (no output to stdout)
        run_daemon_mode()?;
    } else {
        // Interactive mode
        run_interactive_mode()?;
    }

    Ok(())
}
```

### Phase 2: npm Package Updates

**File:** `npm-package/package.json`

```json
{
  "name": "swictation",
  "version": "0.8.0",
  "os": ["linux", "darwin"],
  "cpu": ["x64", "arm64"],
  "scripts": {
    "postinstall": "node postinstall.js",
    "preuninstall": "node preuninstall.js"
  },
  "files": [
    "bin/",
    "lib/",
    "config/",
    "postinstall.js",
    "preuninstall.js",
    "launchd-templates/"
  ]
}
```

**New directory:** `npm-package/launchd-templates/`
- Store plist templates here
- postinstall.js will read and customize them

### Phase 3: Template Files

**File:** `npm-package/launchd-templates/daemon.plist.template`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agidreams.swictation.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{{DAEMON_PATH}}</string>
        <string>--background</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/swictation-daemon.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/swictation-daemon.error.log</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
```

**Postinstall replaces:** `{{DAEMON_PATH}}` with actual binary path

### Phase 4: Documentation

**File:** `docs/macOS-Setup.md`

```markdown
# macOS Setup Guide

## Installation

```bash
# Option 1: Global install with npm prefix change (recommended)
npm config set prefix ~/.npm-global
export PATH=~/.npm-global/bin:$PATH
npm install -g swictation

# Option 2: Global install with sudo (system-wide)
sudo npm install -g swictation
```

## First Run

After installation, grant Accessibility permission:

1. Open **System Settings**
2. Go to **Privacy & Security** → **Accessibility**
3. Click the **+** button
4. Navigate to your swictation-daemon binary:
   - npm prefix install: `~/.npm-global/bin/swictation-daemon`
   - sudo install: `/usr/local/bin/swictation-daemon`
5. Enable the checkbox

## Service Management

**Check status:**
```bash
launchctl list | grep swictation
```

**View logs:**
```bash
tail -f /tmp/swictation-daemon.log
```

**Manual start/stop:**
```bash
# Stop
launchctl stop com.agidreams.swictation.daemon

# Start
launchctl start com.agidreams.swictation.daemon
```

**Disable auto-start:**
```bash
launchctl unload ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist
```

**Re-enable auto-start:**
```bash
launchctl load ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist
```

## Troubleshooting

**Service not starting:**
1. Check logs: `tail -f /tmp/swictation-daemon.error.log`
2. Verify permissions: System Settings → Privacy & Security → Accessibility
3. Test binary manually: `swictation-daemon --background`

**"Nothing found to load":**
- Check plist exists: `ls ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist`
- Verify syntax: `plutil -lint ~/Library/LaunchAgents/com.agidreams.swictation.daemon.plist`

**Binary not found:**
- Check PATH: `echo $PATH`
- Find binary: `which swictation-daemon`
- Update plist if needed
```

---

## Future: SMAppService Migration

**When to migrate:** Once Tauri app bundle is stable

**Migration Path:**

1. **Create LoginItem helper** inside Tauri app bundle
2. **Replace npm postinstall** with SMAppService registration
3. **Keep launchd as fallback** for non-bundled installs

**Example SMAppService Registration (Swift):**

```swift
import ServiceManagement

@main
class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        let service = SMAppService.loginItem(
            identifier: "com.agidreams.swictation.daemon"
        )

        do {
            if service.status == .notRegistered {
                try service.register()
                print("✅ Service registered")
            }
        } catch {
            print("❌ Registration failed: \(error)")
        }
    }
}
```

**Benefits of migration:**
- ✅ Modern API (no "legacy" warnings)
- ✅ Better user transparency (System Settings UI)
- ✅ Improved sandboxing support
- ✅ Automatic cleanup on app deletion

**Drawbacks:**
- ❌ Requires app bundle (not standalone binaries)
- ❌ macOS 13+ only (lose 10.15-12.x support)
- ❌ More complex build/distribution

**Recommendation:** Keep launchd for now, document SMAppService migration for v1.0 Tauri release.

---

## Summary and Recommendations

### For Swictation v0.8 (npm package):

**✅ Use launchd LaunchAgents**
- Universal macOS support (10.4+)
- Works with npm global install
- No sudo required (user-level agents)
- Proven, stable, production-ready

**Implementation:**
1. Create plist templates in `npm-package/launchd-templates/`
2. Implement postinstall.js to create user LaunchAgents
3. Implement preuninstall.js for cleanup
4. Document in README.md and macOS-Setup.md
5. Add validation script for testing

**Testing Checklist:**
- [ ] Plist syntax validation (plutil)
- [ ] Permission correctness (644)
- [ ] Binary executable check
- [ ] Load/unload cycles
- [ ] Auto-start after logout/login
- [ ] Log file creation and permissions
- [ ] Process monitoring (pgrep)
- [ ] Accessibility permission handling

### For Future (Tauri app bundle):

**Document SMAppService migration path**
- Create migration guide
- Keep launchd as fallback
- Target macOS 13+ for SMAppService
- Provide both options (user choice)

---

## References

1. **Apple Developer Documentation:**
   - https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html

2. **launchd.plist man page:**
   ```bash
   man launchd.plist
   man launchctl
   ```

3. **SMAppService Documentation:**
   - https://developer.apple.com/documentation/servicemanagement/smappservice

4. **npm postinstall Best Practices:**
   - https://docs.npmjs.com/cli/v8/using-npm/scripts#npm-install

5. **TCC Framework (Accessibility Permissions):**
   - https://developer.apple.com/documentation/security/accessing_protected_resources

---

## Task Updates Required

Based on these findings, the following Archon tasks need updates:

### Task 01ffebb3 (This research task)
- ✅ Mark as complete
- Document: Use launchd LaunchAgents (not SMAppService yet)
- Document: npm postinstall/preuninstall pattern
- Document: User-level agents in ~/Library/LaunchAgents

### Task fc98d2d7 (Set up auto-start with launchd)
- Add implementation details from postinstall.js example
- Add plist template references
- Add permission handling (644, user-owned)

### Task 9c96f7db (Create build scripts for macOS)
- Add plist template generation step
- Add postinstall/preuninstall validation
- Add Accessibility permission check

### Task af6b5e6d (Update package.json for macOS)
- Add "darwin" to os field
- Add "arm64" to cpu field
- Add postinstall/preuninstall scripts
