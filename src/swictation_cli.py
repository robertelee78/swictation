#!/usr/bin/env python3
"""
Swictation CLI - Send commands to the daemon via Unix socket.
"""

import sys
import socket
import json
import argparse


SOCKET_PATH = '/tmp/swictation.sock'


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

    # Parse args
    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 0

    # Execute command
    return args.func(args)


if __name__ == '__main__':
    sys.exit(main())
