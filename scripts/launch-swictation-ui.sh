#!/bin/bash
# Smart launcher for Swictation UI
# Uses QT tray icon on Sway/Wayland (because Tauri tray doesn't work there)
# But the tray launches the Tauri UI when clicked

set -e

# Detect Wayland/Sway environment
is_sway_wayland() {
    [[ "${XDG_SESSION_TYPE}" == "wayland" ]] && [[ -n "${SWAYSOCK}" ]]
}

# Detect X11 with problematic WMs
is_problematic_x11() {
    # Add other WMs with known tray issues here
    [[ "${XDG_CURRENT_DESKTOP}" =~ "sway" ]] || [[ "${DESKTOP_SESSION}" =~ "sway" ]]
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

# Start daemon if not running
if ! pgrep -f "swictation-daemon" > /dev/null; then
    echo "Starting Swictation daemon..."
    "${REPO_ROOT}/rust-crates/target/release/swictation-daemon" &
    sleep 2
fi

# Choose tray implementation based on environment
if is_sway_wayland || is_problematic_x11; then
    echo "Detected Sway/Wayland - using Qt tray icon (Tauri tray doesn't work here)"
    export SWICTATION_TRAY_MODE="qt"
    export SWICTATION_UI_BINARY="${REPO_ROOT}/tauri-ui/src-tauri/target/release/swictation-ui"

    # Launch Python/Qt tray (just for the tray icon, will launch Tauri UI when clicked)
    exec python3 "${REPO_ROOT}/src/ui/swictation_tray.py"
else
    echo "Detected compatible environment - using Tauri (cross-platform)"
    export SWICTATION_TRAY_MODE="tauri"

    # Launch Tauri UI (works on macOS, Windows, most Linux DEs)
    exec "${REPO_ROOT}/tauri-ui/src-tauri/target/release/swictation-ui"
fi
