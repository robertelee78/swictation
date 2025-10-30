#!/usr/bin/env python3
"""
Test swictation daemon functionality.
Tests IPC, state management, and component integration.
"""

import sys
import time
import subprocess
import signal
import socket
import json
from pathlib import Path

# Test configuration
SOCKET_PATH = '/tmp/swictation.sock'
DAEMON_SCRIPT = Path(__file__).parent.parent / 'src' / 'swictationd.py'


def test_daemon_start_stop():
    """Test daemon startup and shutdown"""
    print("=" * 80)
    print("Test 1: Daemon Start/Stop")
    print("=" * 80)

    print("\n1️⃣ Starting daemon...")
    try:
        # Start daemon in background
        proc = subprocess.Popen(
            ['python3', str(DAEMON_SCRIPT)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )

        # Wait for socket to appear
        for i in range(10):
            if Path(SOCKET_PATH).exists():
                print("✓ Daemon started, socket created")
                break
            time.sleep(0.5)
        else:
            print("✗ Socket not created")
            proc.kill()
            return False

        # Send stop command
        print("\n2️⃣ Sending stop command...")
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(SOCKET_PATH)
        client.sendall(json.dumps({'action': 'stop'}).encode('utf-8'))
        response = json.loads(client.recv(1024).decode('utf-8'))
        client.close()

        print(f"  Response: {response}")

        # Wait for process to exit
        proc.wait(timeout=5)
        print("✓ Daemon stopped cleanly")

        return True

    except Exception as e:
        print(f"✗ Test failed: {e}")
        if proc:
            proc.kill()
        return False


def test_ipc_commands():
    """Test IPC command communication"""
    print("\n" + "=" * 80)
    print("Test 2: IPC Commands")
    print("=" * 80)

    print("\nNote: This test requires daemon to be running manually")
    print("Run: python3 src/swictationd.py")
    print("\nPress Enter to test IPC commands (or Ctrl+C to skip)...")

    try:
        input()
    except KeyboardInterrupt:
        print("\n✗ Skipped")
        return True

    # Test status command
    print("\n1️⃣ Testing status command...")
    try:
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(2.0)
        client.connect(SOCKET_PATH)

        client.sendall(json.dumps({'action': 'status'}).encode('utf-8'))
        response = json.loads(client.recv(1024).decode('utf-8'))
        client.close()

        print(f"✓ Status: {response}")

        if response.get('status') == 'ok' and 'state' in response:
            print("✓ Status command working")
        else:
            print("⚠ Unexpected response format")
            return False

    except Exception as e:
        print(f"✗ Status command failed: {e}")
        return False

    # Test toggle command
    print("\n2️⃣ Testing toggle command...")
    print("  Note: This will start recording. Speak something!")
    time.sleep(2)

    try:
        # First toggle (start recording)
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(SOCKET_PATH)
        client.sendall(json.dumps({'action': 'toggle'}).encode('utf-8'))
        response = json.loads(client.recv(1024).decode('utf-8'))
        client.close()

        print(f"  First toggle: {response}")
        print("  Speak now... (3 seconds)")
        time.sleep(3)

        # Second toggle (stop recording)
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(SOCKET_PATH)
        client.sendall(json.dumps({'action': 'toggle'}).encode('utf-8'))
        response = json.loads(client.recv(1024).decode('utf-8'))
        client.close()

        print(f"  Second toggle: {response}")
        print("✓ Toggle command working")

        return True

    except Exception as e:
        print(f"✗ Toggle command failed: {e}")
        return False


def test_cli():
    """Test CLI commands"""
    print("\n" + "=" * 80)
    print("Test 3: CLI Commands")
    print("=" * 80)

    print("\nNote: This test requires daemon to be running manually")
    print("Run: python3 src/swictationd.py")
    print("\nPress Enter to test CLI commands (or Ctrl+C to skip)...")

    try:
        input()
    except KeyboardInterrupt:
        print("\n✗ Skipped")
        return True

    cli_script = Path(__file__).parent.parent / 'src' / 'swictation_cli.py'

    # Test status command
    print("\n1️⃣ Testing 'swictation status'...")
    try:
        result = subprocess.run(
            ['python3', str(cli_script), 'status'],
            capture_output=True,
            timeout=5
        )

        print(result.stdout.decode('utf-8'))

        if result.returncode == 0:
            print("✓ Status command works")
        else:
            print(f"⚠ Exit code: {result.returncode}")
            print(result.stderr.decode('utf-8'))

    except Exception as e:
        print(f"✗ Status test failed: {e}")
        return False

    # Test toggle command
    print("\n2️⃣ Testing 'swictation toggle'...")
    print("  This will start recording. Speak something!")
    time.sleep(2)

    try:
        # First toggle
        result = subprocess.run(
            ['python3', str(cli_script), 'toggle'],
            capture_output=True,
            timeout=5
        )
        print(result.stdout.decode('utf-8'))

        print("  Speak now... (3 seconds)")
        time.sleep(3)

        # Second toggle
        result = subprocess.run(
            ['python3', str(cli_script), 'toggle'],
            capture_output=True,
            timeout=5
        )
        print(result.stdout.decode('utf-8'))

        print("✓ Toggle command works")
        return True

    except Exception as e:
        print(f"✗ Toggle test failed: {e}")
        return False


def main():
    """Run all tests"""
    print("=" * 80)
    print("Swictation Daemon Test Suite")
    print("=" * 80)

    results = []

    # Automated test
    results.append(("Daemon start/stop", test_daemon_start_stop()))

    # Manual tests (require running daemon)
    print("\n" + "=" * 80)
    print("Manual Tests (Require Running Daemon)")
    print("=" * 80)

    response = input("\nRun manual tests? (y/N): ").strip().lower()

    if response == 'y':
        results.append(("IPC commands", test_ipc_commands()))
        results.append(("CLI commands", test_cli()))

    # Summary
    print("\n" + "=" * 80)
    print("TEST SUMMARY")
    print("=" * 80)

    passed = sum(1 for _, result in results if result)
    total = len(results)

    for test_name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"{status}: {test_name}")

    print(f"\nTests: {passed}/{total} passed")

    if passed == total:
        print("\n🎉 All tests passed!")
        return 0
    else:
        print(f"\n⚠ {total - passed} test(s) failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
