#!/usr/bin/env python3
"""
Swictation CLI - Send commands to the daemon via Unix socket.
"""

import sys
import socket
import json
import argparse
import os


def get_socket_path() -> str:
    """Get socket path using XDG_RUNTIME_DIR or fallback to ~/.local/share/swictation.

    Matches the Rust daemon's socket_utils::get_ipc_socket_path() logic.
    Cross-platform: Linux uses XDG dirs, macOS uses ~/Library/Application Support/.
    """
    # macOS: Use Application Support directory
    if sys.platform == 'darwin':
        home = os.environ.get('HOME')
        if home:
            socket_dir = os.path.join(home, 'Library', 'Application Support', 'swictation')
            os.makedirs(socket_dir, mode=0o700, exist_ok=True)
            return os.path.join(socket_dir, 'swictation.sock')

    # Linux: Try XDG_RUNTIME_DIR first (best practice for sockets)
    runtime_dir = os.environ.get('XDG_RUNTIME_DIR')
    if runtime_dir and os.path.exists(runtime_dir):
        return os.path.join(runtime_dir, 'swictation.sock')

    # Linux fallback to ~/.local/share/swictation/swictation.sock
    home = os.environ.get('HOME')
    if home:
        socket_dir = os.path.join(home, '.local', 'share', 'swictation')
        os.makedirs(socket_dir, mode=0o700, exist_ok=True)
        return os.path.join(socket_dir, 'swictation.sock')

    # Final fallback (should rarely happen)
    return '/tmp/swictation.sock'


# Get platform-appropriate socket path
SOCKET_PATH = get_socket_path()


def send_command(command: dict, socket_path: str = SOCKET_PATH) -> dict:
    """
    Send command to daemon via Unix socket.

    Args:
        command: Command dictionary (e.g., {'action': 'toggle'})
        socket_path: Path to Unix socket

    Returns:
        Response dictionary from daemon
    """
    try:
        # Connect to daemon socket
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(5.0)
        client.connect(socket_path)

        # Send command
        client.sendall(json.dumps(command).encode('utf-8'))

        # Receive response
        response_data = client.recv(1024)
        response = json.loads(response_data.decode('utf-8'))

        client.close()
        return response

    except FileNotFoundError:
        return {'error': 'Daemon not running (socket not found)'}
    except ConnectionRefusedError:
        return {'error': 'Daemon not running (connection refused)'}
    except socket.timeout:
        return {'error': 'Daemon not responding (timeout)'}
    except Exception as e:
        return {'error': f'Communication error: {e}'}


def cmd_toggle(args):
    """Toggle recording on/off"""
    print("Sending toggle command...")
    response = send_command({'action': 'toggle'})

    if 'error' in response:
        print(f"✗ Error: {response['error']}")
        return 1

    print(f"✓ State: {response.get('state', 'unknown')}")
    return 0


def cmd_status(args):
    """Get daemon status"""
    response = send_command({'action': 'status'})

    if 'error' in response:
        print(f"✗ Error: {response['error']}")
        return 1

    state = response.get('state', 'unknown')
    print(f"Daemon state: {state}")

    if state == 'idle':
        print("  Ready to record")
    elif state == 'recording':
        print("  Currently recording")
    elif state == 'processing':
        print("  Processing audio")
    elif state == 'error':
        print("  Error state")

    return 0


def cmd_stop(args):
    """Stop daemon"""
    print("Stopping daemon...")
    response = send_command({'action': 'stop'})

    if 'error' in response:
        print(f"✗ Error: {response['error']}")
        return 1

    print("✓ Daemon stopping")
    return 0


def cmd_stats(args):
    """Show session statistics"""
    from metrics.cli import MetricsCLI
    cli = MetricsCLI()
    try:
        cli.show_stats(args.session_id if hasattr(args, 'session_id') else None)
        return 0
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        return 1
    finally:
        cli.close()


def cmd_history(args):
    """Show session history"""
    from metrics.cli import MetricsCLI
    cli = MetricsCLI()
    try:
        limit = args.limit if hasattr(args, 'limit') else 10
        cli.show_history(limit)
        return 0
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        return 1
    finally:
        cli.close()


def cmd_summary(args):
    """Show lifetime statistics"""
    from metrics.cli import MetricsCLI
    cli = MetricsCLI()
    try:
        cli.show_summary()
        return 0
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        return 1
    finally:
        cli.close()


def main():
    """CLI entry point"""
    parser = argparse.ArgumentParser(
        description='Swictation voice dictation control'
    )

    subparsers = parser.add_subparsers(dest='command', help='Command to execute')

    # Toggle command
    toggle_parser = subparsers.add_parser(
        'toggle',
        help='Toggle recording on/off'
    )
    toggle_parser.set_defaults(func=cmd_toggle)

    # Status command
    status_parser = subparsers.add_parser(
        'status',
        help='Get daemon status'
    )
    status_parser.set_defaults(func=cmd_status)

    # Stop command
    stop_parser = subparsers.add_parser(
        'stop',
        help='Stop daemon'
    )
    stop_parser.set_defaults(func=cmd_stop)

    # Stats command
    stats_parser = subparsers.add_parser(
        'stats',
        help='Show session statistics (default: most recent)'
    )
    stats_parser.add_argument(
        'session_id',
        type=int,
        nargs='?',
        help='Session ID to display (default: most recent)'
    )
    stats_parser.set_defaults(func=cmd_stats)

    # History command
    history_parser = subparsers.add_parser(
        'history',
        help='Show session history'
    )
    history_parser.add_argument(
        '--limit',
        type=int,
        default=10,
        help='Number of recent sessions to show (default: 10)'
    )
    history_parser.set_defaults(func=cmd_history)

    # Summary command
    summary_parser = subparsers.add_parser(
        'summary',
        help='Show lifetime statistics'
    )
    summary_parser.set_defaults(func=cmd_summary)

    # Parse args
    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 0

    # Execute command
    return args.func(args)


if __name__ == '__main__':
    sys.exit(main())
