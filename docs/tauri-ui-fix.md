# Tauri UI "Connection Refused" Fix

## Problem
The Tauri UI displayed "Could not connect to localhost: Connection refused" instead of the React app.

## Root Cause
The Tauri binary was compiled **without** the `custom-protocol` feature flag, causing it to:
- Try to connect to `http://localhost:1420` (development mode)
- Ignore the bundled `dist` directory
- Show the connection error before React could load

## Solution
The `custom-protocol` feature **must** be enabled for release builds to use bundled assets instead of localhost.

### Changes Made

1. **Updated Cargo.toml** (`tauri-ui/src-tauri/Cargo.toml`):
   ```toml
   [features]
   custom-protocol = ["tauri/custom-protocol"]
   default = ["custom-protocol"]  # ← Added this line
   ```

2. **Updated package.json** (`tauri-ui/package.json`):
   ```json
   "scripts": {
     "build:release": "npm run build && cd src-tauri && cargo build --release --features custom-protocol"
   }
   ```

3. **Created build script** (`scripts/build-tauri-ui.sh`):
   ```bash
   #!/bin/bash
   cd tauri-ui
   npm run build
   cd src-tauri
   cargo build --release --features custom-protocol
   ```

## Build Commands

### Correct (Always use these):
```bash
# Using npm script:
cd /opt/swictation/tauri-ui
npm run build:release

# Or using shell script:
/opt/swictation/scripts/build-tauri-ui.sh

# Or manually:
cd /opt/swictation/tauri-ui
npm run build
cd src-tauri
cargo build --release --features custom-protocol
```

### Wrong (Don't use):
```bash
# This will create a broken binary:
cargo build --release  # Missing --features custom-protocol
```

## Verification

Check the binary includes the custom protocol:
```bash
strings target/release/swictation-ui | grep "tauri://localhost"
# Should show: tauri://localhost
```

Wrong builds will try to connect to:
```bash
strings target/release/swictation-ui | grep "http://localhost:1420"
# This means it's broken
```

## systemd Service
The service file already points to the correct binary:
```ini
ExecStart=/opt/swictation/tauri-ui/src-tauri/target/release/swictation-ui
```

After rebuilding with custom-protocol, restart:
```bash
systemctl --user restart swictation-tauri.service
```

## Why This Matters
- **Without custom-protocol**: App tries to connect to localhost dev server
- **With custom-protocol**: App uses bundled dist files via `tauri://localhost` protocol

## Tauri Documentation
See: https://tauri.app/v1/guides/building/app-publishing#building-your-application
