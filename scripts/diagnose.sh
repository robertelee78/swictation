#!/bin/bash
# Swictation Diagnostic Tool
# Gathers all relevant debugging information for troubleshooting
# Usage: ./scripts/diagnose.sh [output-dir]

OUTPUT_DIR="${1:-./diagnostic-output}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_DIR="$OUTPUT_DIR/swictation-diagnostic-$TIMESTAMP"

mkdir -p "$REPORT_DIR"

echo "=================================================="
echo "Swictation Diagnostic Information Collector"
echo "=================================================="
echo ""
echo "Collecting diagnostic data to: $REPORT_DIR"
echo ""

# 1. Environment Variables
echo "[1/8] Collecting environment variables..."
{
    echo "=================================================="
    echo "ENVIRONMENT VARIABLES"
    echo "=================================================="
    echo ""
    echo "Display Server Variables:"
    echo "  DISPLAY: ${DISPLAY:-<not set>}"
    echo "  WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-<not set>}"
    echo "  XDG_SESSION_TYPE: ${XDG_SESSION_TYPE:-<not set>}"
    echo "  XDG_CURRENT_DESKTOP: ${XDG_CURRENT_DESKTOP:-<not set>}"
    echo "  XDG_SESSION_DESKTOP: ${XDG_SESSION_DESKTOP:-<not set>}"
    echo ""
    echo "User Variables:"
    echo "  USER: ${USER:-<not set>}"
    echo "  HOME: ${HOME:-<not set>}"
    echo "  SHELL: ${SHELL:-<not set>}"
    echo ""
    echo "All Environment Variables:"
    env | sort
} > "$REPORT_DIR/environment.txt"

# 2. System Information
echo "[2/8] Collecting system information..."
{
    echo "=================================================="
    echo "SYSTEM INFORMATION"
    echo "=================================================="
    echo ""
    echo "OS:"
    cat /etc/os-release 2>/dev/null || echo "Not available"
    echo ""
    echo "Kernel:"
    uname -a
    echo ""
    echo "Desktop Session:"
    loginctl show-session $(loginctl | grep $(whoami) | awk '{print $1}') 2>/dev/null || echo "Not available"
    echo ""
    echo "Display Manager:"
    systemctl status display-manager 2>/dev/null | head -20 || echo "Not available"
} > "$REPORT_DIR/system-info.txt"

# 3. Tool Availability
echo "[3/8] Checking tool availability..."
{
    echo "=================================================="
    echo "TEXT INJECTION TOOLS"
    echo "=================================================="
    echo ""

    echo "xdotool:"
    if command -v xdotool &> /dev/null; then
        echo "  Path: $(which xdotool)"
        echo "  Version: $(xdotool --version 2>&1 || echo 'Version not available')"
    else
        echo "  Status: NOT INSTALLED"
    fi
    echo ""

    echo "wtype:"
    if command -v wtype &> /dev/null; then
        echo "  Path: $(which wtype)"
        echo "  Version: $(wtype --version 2>&1 || echo 'Version not available')"
    else
        echo "  Status: NOT INSTALLED"
    fi
    echo ""

    echo "ydotool:"
    if command -v ydotool &> /dev/null; then
        echo "  Path: $(which ydotool)"
        echo "  Version: $(ydotool --version 2>&1 || echo 'Version not available')"
        echo "  User groups: $(groups)"
        if groups | grep -q input; then
            echo "  Input group: ✓ Member"
        else
            echo "  Input group: ✗ Not a member (required for ydotool)"
        fi
    else
        echo "  Status: NOT INSTALLED"
    fi
} > "$REPORT_DIR/tools.txt"

# 4. Swictation Installation
echo "[4/8] Checking Swictation installation..."
{
    echo "=================================================="
    echo "SWICTATION INSTALLATION"
    echo "=================================================="
    echo ""

    echo "swictation-daemon:"
    if command -v swictation-daemon &> /dev/null; then
        echo "  Path: $(which swictation-daemon)"
        echo "  Version: $(swictation-daemon --version 2>&1 || echo 'Version not available')"
    else
        echo "  Status: NOT FOUND IN PATH"
    fi
    echo ""

    echo "Systemd Service:"
    if systemctl --user list-unit-files | grep -q swictation-daemon; then
        echo "  Status: Installed"
        echo ""
        systemctl --user status swictation-daemon --no-pager || echo "  Service not running"
    else
        echo "  Status: Not installed as systemd service"
    fi
} > "$REPORT_DIR/swictation-install.txt"

# 5. Daemon Logs
echo "[5/8] Collecting daemon logs..."
{
    echo "=================================================="
    echo "SWICTATION DAEMON LOGS (Last 100 lines)"
    echo "=================================================="
    echo ""
    journalctl --user -u swictation-daemon -n 100 --no-pager 2>/dev/null || echo "No logs available (daemon may not be running)"
} > "$REPORT_DIR/daemon-logs.txt"

# 6. Process Information
echo "[6/8] Collecting process information..."
{
    echo "=================================================="
    echo "RUNNING PROCESSES"
    echo "=================================================="
    echo ""
    echo "Swictation processes:"
    ps aux | grep -E "[s]wictation|[S]wictation" || echo "No swictation processes found"
    echo ""
    echo "Display server processes:"
    ps aux | grep -E "[X]org|[W]ayland|[x]wayland|[m]utter|[k]win" || echo "No display server processes found"
} > "$REPORT_DIR/processes.txt"

# 7. Validation Script Output
echo "[7/8] Running validation script..."
if [ -f ./scripts/validate-x11.sh ]; then
    ./scripts/validate-x11.sh > "$REPORT_DIR/validation.txt" 2>&1
else
    echo "Validation script not found" > "$REPORT_DIR/validation.txt"
fi

# 8. Configuration Files
echo "[8/8] Collecting configuration files..."
{
    echo "=================================================="
    echo "CONFIGURATION FILES"
    echo "=================================================="
    echo ""

    if [ -d ~/.config/swictation ]; then
        echo "Swictation config directory:"
        ls -la ~/.config/swictation/
        echo ""
        echo "Config file contents:"
        if [ -f ~/.config/swictation/config.toml ]; then
            cat ~/.config/swictation/config.toml
        else
            echo "No config.toml found"
        fi
    else
        echo "No swictation config directory found"
    fi
} > "$REPORT_DIR/config.txt"

# Create summary report
echo ""
echo "Creating summary report..."
{
    echo "=================================================="
    echo "SWICTATION DIAGNOSTIC SUMMARY"
    echo "=================================================="
    echo ""
    echo "Timestamp: $TIMESTAMP"
    echo "User: $USER"
    echo "Hostname: $(hostname)"
    echo ""
    echo "Environment:"
    echo "  Display Server: ${XDG_SESSION_TYPE:-unknown}"
    echo "  Desktop: ${XDG_CURRENT_DESKTOP:-unknown}"
    echo "  DISPLAY: ${DISPLAY:-not set}"
    echo "  WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-not set}"
    echo ""
    echo "Tools Installed:"
    command -v xdotool &> /dev/null && echo "  ✓ xdotool" || echo "  ✗ xdotool"
    command -v wtype &> /dev/null && echo "  ✓ wtype" || echo "  ✗ wtype"
    command -v ydotool &> /dev/null && echo "  ✓ ydotool" || echo "  ✗ ydotool"
    echo ""
    echo "Swictation:"
    command -v swictation-daemon &> /dev/null && echo "  ✓ Installed" || echo "  ✗ Not installed"
    systemctl --user is-active swictation-daemon &> /dev/null && echo "  ✓ Running" || echo "  ✗ Not running"
    echo ""
    echo "Files Generated:"
    ls -lh "$REPORT_DIR"
    echo ""
    echo "=================================================="
    echo ""
    echo "To share this diagnostic information:"
    echo "  1. Review files in: $REPORT_DIR"
    echo "  2. Remove any sensitive information"
    echo "  3. Create archive: tar -czf swictation-diagnostic.tar.gz $REPORT_DIR"
    echo "  4. Attach to bug report or support request"
    echo ""
} | tee "$REPORT_DIR/SUMMARY.txt"

echo "=================================================="
echo "✓ Diagnostic collection complete!"
echo "=================================================="
echo ""
echo "Output directory: $REPORT_DIR"
echo ""
echo "Files created:"
find "$REPORT_DIR" -type f -exec basename {} \; | sort
echo ""
