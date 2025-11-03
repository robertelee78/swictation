#!/usr/bin/env python3
"""
Test script for MetricsBroadcaster - connects to daemon and displays events.

Usage:
    Terminal 1: python3 swictationd.py
    Terminal 2: python3 test_metrics_broadcaster.py
    Terminal 3: python3 swictation_cli.py toggle (to trigger recording)
"""

import socket
import json
import sys


def test_metrics_socket():
    """Connect to metrics socket and display events."""
    socket_path = '/tmp/swictation_metrics.sock'

    print("=" * 80)
    print("Metrics Broadcaster Test Client")
    print("=" * 80)
    print(f"Connecting to: {socket_path}")
    print("Waiting for daemon events...\n")

    try:
        # Connect to Unix socket
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(socket_path)
        print("✓ Connected to metrics broadcaster\n")

        # Receive events
        buffer = ""
        while True:
            data = client.recv(4096).decode('utf-8')
            if not data:
                print("\n✗ Connection closed by daemon")
                break

            buffer += data
            lines = buffer.split('\n')
            buffer = lines[-1]  # Keep incomplete line

            for line in lines[:-1]:
                if line.strip():
                    try:
                        event = json.loads(line)
                        event_type = event.get('type', 'unknown')

                        # Pretty print events
                        if event_type == 'session_start':
                            print(f"\n📝 SESSION START")
                            print(f"   Session ID: {event.get('session_id')}")
                            print(f"   Timestamp: {event.get('timestamp')}")

                        elif event_type == 'session_end':
                            print(f"\n🛑 SESSION END")
                            print(f"   Session ID: {event.get('session_id')}")
                            print(f"   Timestamp: {event.get('timestamp')}")

                        elif event_type == 'transcription':
                            print(f"\n🎤 TRANSCRIPTION")
                            print(f"   Time: {event.get('timestamp')}")
                            print(f"   Text: {event.get('text')}")
                            print(f"   Words: {event.get('words')}")
                            print(f"   WPM: {event.get('wpm', 0):.1f}")
                            print(f"   Latency: {event.get('latency_ms', 0):.0f}ms")

                        elif event_type == 'metrics_update':
                            print(f"\n📊 METRICS UPDATE")
                            print(f"   State: {event.get('state')}")
                            print(f"   Session: #{event.get('session_id')}")
                            print(f"   Segments: {event.get('segments')}")
                            print(f"   Words: {event.get('words')}")
                            print(f"   WPM: {event.get('wpm', 0):.1f}")
                            print(f"   Duration: {event.get('duration_s', 0):.1f}s")
                            print(f"   Latency: {event.get('latency_ms', 0):.0f}ms")
                            print(f"   GPU: {event.get('gpu_memory_mb', 0):.0f}MB ({event.get('gpu_memory_percent', 0):.1f}%)")
                            print(f"   CPU: {event.get('cpu_percent', 0):.1f}%")

                        elif event_type == 'state_change':
                            print(f"\n🔄 STATE CHANGE")
                            print(f"   New State: {event.get('state')}")
                            print(f"   Timestamp: {event.get('timestamp')}")

                        else:
                            print(f"\n❓ UNKNOWN EVENT: {event_type}")
                            print(f"   Data: {json.dumps(event, indent=2)}")

                    except json.JSONDecodeError as e:
                        print(f"✗ JSON decode error: {e}")
                        print(f"   Line: {line}")

    except FileNotFoundError:
        print(f"✗ Socket not found: {socket_path}")
        print("  Is the daemon running?")
        sys.exit(1)
    except ConnectionRefusedError:
        print(f"✗ Connection refused to: {socket_path}")
        print("  Is the daemon running?")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n\n✓ Test client stopped")
    except Exception as e:
        print(f"✗ Error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        try:
            client.close()
        except:
            pass


if __name__ == '__main__':
    test_metrics_socket()
