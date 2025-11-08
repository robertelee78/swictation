#!/usr/bin/env python3
"""Test script to debug Tauri UI launch issue."""

import os
import subprocess
from pathlib import Path

# Test the exact same path construction as in tray app
icon_path = Path(__file__).parent / "src" / "ui" / "swictation_tray.py"
tauri_path = icon_path.parent.parent.parent / "tauri-ui" / "src-tauri" / "target" / "release" / "swictation-ui"

print(f"Script location: {Path(__file__).parent}")
print(f"Icon path would be: {icon_path}")
print(f"Tauri binary path: {tauri_path}")
print(f"Tauri binary exists: {tauri_path.exists()}")

# Get env var or default
tauri_ui_binary = os.environ.get('SWICTATION_UI_BINARY', str(tauri_path))
print(f"Using binary path: {tauri_ui_binary}")

# Try to launch it
try:
    print("\nAttempting to launch Tauri UI...")
    env = os.environ.copy()
    env['SWICTATION_NO_TRAY'] = '1'

    process = subprocess.Popen(
        [tauri_ui_binary],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        start_new_session=True
    )

    print(f"✓ Process started with PID: {process.pid}")

    # Wait a moment and check if it's still running
    import time
    time.sleep(2)

    retcode = process.poll()
    if retcode is None:
        print("✓ Process is still running")
        process.terminate()
    else:
        print(f"✗ Process exited with code: {retcode}")
        stdout, stderr = process.communicate()
        print(f"STDOUT: {stdout.decode('utf-8')}")
        print(f"STDERR: {stderr.decode('utf-8')}")

except Exception as e:
    print(f"✗ Failed to launch: {e}")
    import traceback
    traceback.print_exc()