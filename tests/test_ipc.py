#!/usr/bin/env python3
"""
Test IPC communication with the daemon.
This script will help us debug the socket timeout issue.
"""

import socket
import json
import time

SOCKET_PATH = '/tmp/swictation.sock'

def test_ipc():
    """Test IPC connection and command"""
    print("Testing IPC connection...")

    try:
        # Connect
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(10.0)  # Longer timeout for debugging

        print(f"Connecting to {SOCKET_PATH}...")
        client.connect(SOCKET_PATH)
        print("✓ Connected!")

        # Send command
        command = {'action': 'status'}
        print(f"Sending command: {command}")
        client.sendall(json.dumps(command).encode('utf-8'))
        print("✓ Command sent!")

        # Try to receive with debug
        print("Waiting for response...")
        client.settimeout(5.0)

        try:
            response_data = client.recv(1024)
            print(f"✓ Received {len(response_data)} bytes")
            print(f"  Raw data: {response_data}")

            response = json.loads(response_data.decode('utf-8'))
            print(f"  Parsed response: {response}")

        except socket.timeout:
            print("✗ Timeout waiting for response!")
            print("  This means daemon received command but didn't respond")

        client.close()

    except FileNotFoundError:
        print(f"✗ Socket file not found: {SOCKET_PATH}")
    except ConnectionRefusedError:
        print("✗ Connection refused (daemon not accepting connections)")
    except Exception as e:
        print(f"✗ Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == '__main__':
    test_ipc()
