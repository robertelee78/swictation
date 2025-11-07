#!/bin/bash
# Test script for Unix socket connection handler
# Usage: ./test-socket.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
METRICS_SOCKET="/tmp/swictation_metrics.sock"
COMMAND_SOCKET="/tmp/swictation.sock"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Check if daemon is running
check_daemon() {
    print_header "Checking Daemon Status"

    if pgrep -f "swictation-daemon" > /dev/null; then
        print_success "Daemon is running"
        return 0
    else
        print_error "Daemon is not running"
        print_warning "Start daemon with: swictation-daemon"
        return 1
    fi
}

# Check if sockets exist
check_sockets() {
    print_header "Checking Sockets"

    local status=0

    if [ -S "$METRICS_SOCKET" ]; then
        print_success "Metrics socket exists: $METRICS_SOCKET"
        ls -lh "$METRICS_SOCKET"
    else
        print_error "Metrics socket not found: $METRICS_SOCKET"
        status=1
    fi

    if [ -S "$COMMAND_SOCKET" ]; then
        print_success "Command socket exists: $COMMAND_SOCKET"
        ls -lh "$COMMAND_SOCKET"
    else
        print_error "Command socket not found: $COMMAND_SOCKET"
        status=1
    fi

    return $status
}

# Test socket connection
test_connection() {
    print_header "Testing Socket Connection"

    if ! [ -S "$METRICS_SOCKET" ]; then
        print_error "Cannot test - metrics socket doesn't exist"
        return 1
    fi

    # Try to read from socket with timeout
    if timeout 2 socat -t 2 - UNIX-CONNECT:"$METRICS_SOCKET" < /dev/null > /dev/null 2>&1; then
        print_success "Successfully connected to metrics socket"
        return 0
    else
        print_error "Failed to connect to metrics socket"
        return 1
    fi
}

# Monitor socket events
monitor_events() {
    print_header "Monitoring Socket Events (Press Ctrl+C to stop)"

    if ! [ -S "$METRICS_SOCKET" ]; then
        print_error "Metrics socket doesn't exist"
        return 1
    fi

    print_warning "Listening for events..."
    echo

    # Connect to socket and print events
    socat -v UNIX-CONNECT:"$METRICS_SOCKET" - | while IFS= read -r line; do
        # Pretty print JSON if jq is available
        if command -v jq &> /dev/null; then
            echo "$line" | jq -C '.' 2>/dev/null || echo "$line"
        else
            echo "$line"
        fi
    done
}

# Send test command
send_command() {
    print_header "Sending Toggle Command"

    if ! [ -S "$COMMAND_SOCKET" ]; then
        print_error "Command socket doesn't exist"
        return 1
    fi

    if echo "toggle" | socat - UNIX-CONNECT:"$COMMAND_SOCKET" 2>/dev/null; then
        print_success "Toggle command sent successfully"
        return 0
    else
        print_error "Failed to send toggle command"
        return 1
    fi
}

# Run Rust unit tests
run_unit_tests() {
    print_header "Running Rust Unit Tests"

    cd "$PROJECT_ROOT/src-tauri"

    if cargo test --lib socket 2>&1 | tee /tmp/socket-test.log; then
        print_success "Unit tests passed"
        return 0
    else
        print_error "Unit tests failed"
        cat /tmp/socket-test.log
        return 1
    fi
}

# Check Rust code formatting
check_formatting() {
    print_header "Checking Code Formatting"

    cd "$PROJECT_ROOT/src-tauri"

    if cargo fmt --check 2>/dev/null; then
        print_success "Code is properly formatted"
        return 0
    else
        print_warning "Code needs formatting - run: cargo fmt"
        return 1
    fi
}

# Run clippy lints
run_clippy() {
    print_header "Running Clippy Lints"

    cd "$PROJECT_ROOT/src-tauri"

    if cargo clippy --all-targets -- -D warnings 2>&1 | tee /tmp/clippy.log; then
        print_success "No clippy warnings"
        return 0
    else
        print_warning "Clippy found issues"
        cat /tmp/clippy.log
        return 1
    fi
}

# Build project
build_project() {
    print_header "Building Project"

    cd "$PROJECT_ROOT/src-tauri"

    if cargo build 2>&1 | tee /tmp/build.log; then
        print_success "Build successful"
        return 0
    else
        print_error "Build failed"
        tail -20 /tmp/build.log
        return 1
    fi
}

# Simulate events for testing
simulate_events() {
    print_header "Simulating Events"

    if ! [ -S "$METRICS_SOCKET" ]; then
        print_error "Metrics socket doesn't exist"
        return 1
    fi

    local events=(
        '{"type":"session_start","session_id":"test-123","timestamp":1699363200}'
        '{"type":"metrics_update","state":"recording","wpm":120.5,"words":100,"latency_ms":150,"segments":10,"duration_s":60.5,"gpu_memory_mb":2048.0,"cpu_percent":45.2}'
        '{"type":"transcription","session_id":"test-123","text":"Hello world","timestamp":1699363200,"wpm":120.0,"latency_ms":100}'
        '{"type":"state_change","daemon_state":"recording","timestamp":1699363200}'
        '{"type":"session_end","session_id":"test-123","timestamp":1699363260}'
    )

    for event in "${events[@]}"; do
        echo "Sending: $event"
        echo "$event" | socat - UNIX-CONNECT:"$METRICS_SOCKET" 2>/dev/null || true
        sleep 1
    done

    print_success "All test events sent"
}

# Main menu
show_menu() {
    echo
    print_header "Socket Test Menu"
    echo "1. Check daemon status"
    echo "2. Check sockets"
    echo "3. Test connection"
    echo "4. Monitor events (live)"
    echo "5. Send toggle command"
    echo "6. Run unit tests"
    echo "7. Check formatting"
    echo "8. Run clippy"
    echo "9. Build project"
    echo "10. Simulate test events"
    echo "11. Run all checks"
    echo "0. Exit"
    echo
    read -p "Select option: " choice

    case $choice in
        1) check_daemon ;;
        2) check_sockets ;;
        3) test_connection ;;
        4) monitor_events ;;
        5) send_command ;;
        6) run_unit_tests ;;
        7) check_formatting ;;
        8) run_clippy ;;
        9) build_project ;;
        10) simulate_events ;;
        11)
            check_daemon
            check_sockets
            test_connection
            run_unit_tests
            check_formatting
            run_clippy
            build_project
            ;;
        0) exit 0 ;;
        *) print_error "Invalid option" ;;
    esac
}

# Parse command line arguments
if [ $# -eq 0 ]; then
    # Interactive mode
    while true; do
        show_menu
        echo
        read -p "Press Enter to continue..."
    done
else
    # Command line mode
    case "$1" in
        daemon) check_daemon ;;
        sockets) check_sockets ;;
        connect) test_connection ;;
        monitor) monitor_events ;;
        toggle) send_command ;;
        test) run_unit_tests ;;
        fmt) check_formatting ;;
        clippy) run_clippy ;;
        build) build_project ;;
        simulate) simulate_events ;;
        all)
            check_daemon && \
            check_sockets && \
            test_connection && \
            run_unit_tests && \
            check_formatting && \
            run_clippy && \
            build_project
            ;;
        *)
            echo "Usage: $0 [daemon|sockets|connect|monitor|toggle|test|fmt|clippy|build|simulate|all]"
            echo "Or run without arguments for interactive menu"
            exit 1
            ;;
    esac
fi
