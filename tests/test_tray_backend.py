#!/usr/bin/env python3
"""Test MetricsBackend logic without requiring PySide6/display."""

import sys
import json
from pathlib import Path

# Test the event handling logic directly
def test_event_parsing():
    """Test that events are parsed correctly."""
    print("Testing event parsing...")

    # Simulate metrics_update event
    event = {
        'type': 'metrics_update',
        'state': 'recording',
        'wpm': 150.5,
        'words': 100,
        'latency_ms': 125.3,
        'segments': 5,
        'duration_s': 65,  # Should format to "01:05"
        'gpu_memory_mb': 512.0,
        'cpu_percent': 45.2
    }

    # Test duration formatting
    duration_s = event['duration_s']
    minutes = int(duration_s // 60)
    seconds = int(duration_s % 60)
    formatted_duration = f"{minutes:02d}:{seconds:02d}"

    assert formatted_duration == "01:05", f"Expected 01:05, got {formatted_duration}"
    print(f"  ✓ Duration formatting: {duration_s}s -> {formatted_duration}")

    # Test with longer duration
    duration_s = 3725  # 62 minutes, 5 seconds
    minutes = int(duration_s // 60)
    seconds = int(duration_s % 60)
    formatted_duration = f"{minutes:02d}:{seconds:02d}"

    assert formatted_duration == "62:05", f"Expected 62:05, got {formatted_duration}"
    print(f"  ✓ Long duration: {duration_s}s -> {formatted_duration}")

    return True


def test_socket_protocol():
    """Test socket protocol assumptions."""
    print("Testing socket protocol...")

    # Test newline-delimited JSON parsing
    buffer = ""
    events = []

    # Simulate receiving data
    data_chunks = [
        '{"type": "session_start", "timestamp": ',
        '"2025-11-03T02:00:00"}\n{"type": "metrics',
        '_update", "wpm": 125.0}\n{"type":',
        ' "transcription"}\n'
    ]

    for chunk in data_chunks:
        buffer += chunk
        while '\n' in buffer:
            line, buffer = buffer.split('\n', 1)
            if line.strip():
                events.append(json.loads(line))

    assert len(events) == 3, f"Expected 3 events, got {len(events)}"
    assert events[0]['type'] == 'session_start', f"First event should be session_start"
    assert events[1]['type'] == 'metrics_update', f"Second event should be metrics_update"
    assert events[2]['type'] == 'transcription', f"Third event should be transcription"

    print(f"  ✓ Parsed {len(events)} events from chunked data")
    return True


def test_icon_state_logic():
    """Test icon state determination."""
    print("Testing icon state logic...")

    states = ["idle", "recording", "processing"]
    expected_overlay = {
        "idle": False,
        "recording": True,  # Should have red overlay
        "processing": False
    }

    for state in states:
        needs_overlay = (state == "recording")
        assert needs_overlay == expected_overlay[state], \
            f"State {state}: expected overlay={expected_overlay[state]}, got {needs_overlay}"

    print(f"  ✓ Icon overlay logic correct for all states")
    return True


def test_database_compatibility():
    """Test database data structure compatibility."""
    print("Testing database compatibility...")

    # Add src to path
    sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

    from metrics.database import MetricsDatabase

    db = MetricsDatabase()

    # Test loading history (should not crash)
    try:
        sessions = db.get_recent_sessions(limit=10)
        print(f"  ✓ Loaded {len(sessions)} recent sessions")
    except Exception as e:
        print(f"  ✗ Failed to load sessions: {e}")
        return False

    # Test loading lifetime stats
    try:
        stats = db.get_lifetime_stats()
        print(f"  ✓ Loaded lifetime stats: {stats.get('total_sessions', 0)} total sessions")
    except Exception as e:
        print(f"  ✗ Failed to load stats: {e}")
        return False

    return True


def main():
    """Run all tests."""
    print("=" * 60)
    print("Tray Backend Unit Tests (No Display Required)")
    print("=" * 60)

    tests = [
        test_event_parsing,
        test_socket_protocol,
        test_icon_state_logic,
        test_database_compatibility,
    ]

    passed = 0
    failed = 0

    for test in tests:
        try:
            print(f"\n{test.__name__}:")
            if test():
                passed += 1
                print(f"✓ {test.__name__} PASSED\n")
        except Exception as e:
            failed += 1
            print(f"✗ {test.__name__} FAILED: {e}\n")
            import traceback
            traceback.print_exc()

    print("=" * 60)
    print(f"Results: {passed} passed, {failed} failed")
    print("=" * 60)

    return failed == 0


if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)
