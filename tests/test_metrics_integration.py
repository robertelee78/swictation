#!/usr/bin/env python3
"""
Integration test for metrics system.

Tests the complete flow: database creation → session tracking → metrics display
"""

import sys
import os
import tempfile
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

from metrics.database import MetricsDatabase
from metrics.collector import MetricsCollector
from metrics.cli import MetricsCLI


def test_database_creation():
    """Test database schema creation."""
    print("Test 1: Database Creation")
    print("-" * 50)

    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as f:
        db_path = f.name

    try:
        db = MetricsDatabase(db_path)

        # Verify tables exist
        conn = db._get_connection()
        cursor = conn.cursor()

        cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
        tables = [row[0] for row in cursor.fetchall()]

        expected_tables = ['sessions', 'segments', 'lifetime_stats']
        for table in expected_tables:
            if table in tables:
                print(f"  ✓ Table '{table}' created")
            else:
                print(f"  ✗ Table '{table}' missing")
                return False

        # Verify lifetime_stats initialized
        lifetime = db.get_lifetime_stats()
        if lifetime:
            print(f"  ✓ Lifetime stats initialized")
        else:
            print(f"  ✗ Lifetime stats not initialized")
            return False

        db.close()
        print("  ✓ Test passed\n")
        return True

    finally:
        if os.path.exists(db_path):
            os.unlink(db_path)


def test_session_lifecycle():
    """Test complete session lifecycle."""
    print("Test 2: Session Lifecycle")
    print("-" * 50)

    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as f:
        db_path = f.name

    try:
        collector = MetricsCollector(
            db_path=db_path,
            typing_baseline_wpm=40.0,
            store_transcription_text=True
        )

        # Start session
        session = collector.start_session()
        print(f"  ✓ Session started (ID: {session.session_id})")

        # Record some segments
        for i in range(3):
            transcription = f"This is test segment number {i + 1} with some words"
            collector.record_segment(
                audio_duration_s=2.5,
                transcription=transcription,
                stt_latency_ms=300.0 + i * 50,
                transform_latency_us=0.3,
                injection_latency_ms=20.0,
                gpu_memory_mb=2.2,
                cpu_percent=45.0
            )
            print(f"  ✓ Segment {i + 1} recorded")

        # End session
        final_session = collector.end_session()
        print(f"  ✓ Session ended")
        print(f"    - Words: {final_session.words_dictated}")
        print(f"    - Segments: {final_session.segments_processed}")
        print(f"    - WPM: {final_session.words_per_minute:.1f}")
        print(f"    - Avg latency: {final_session.average_latency_ms:.1f}ms")

        # Verify data persisted
        db = collector.db
        session_data = db.get_session(session.session_id)
        if session_data:
            print(f"  ✓ Session data persisted to database")
        else:
            print(f"  ✗ Session data not in database")
            return False

        segments = db.get_segments_for_session(session.session_id)
        if len(segments) == 3:
            print(f"  ✓ All segments persisted (count: {len(segments)})")
        else:
            print(f"  ✗ Segment count mismatch (expected 3, got {len(segments)})")
            return False

        # Verify lifetime stats updated
        lifetime = db.get_lifetime_stats()
        if lifetime['total_sessions'] == 1:
            print(f"  ✓ Lifetime stats updated")
        else:
            print(f"  ✗ Lifetime stats not updated")
            return False

        collector.close()
        print("  ✓ Test passed\n")
        return True

    finally:
        if os.path.exists(db_path):
            os.unlink(db_path)


def test_cli_display():
    """Test CLI display functions."""
    print("Test 3: CLI Display")
    print("-" * 50)

    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as f:
        db_path = f.name

    try:
        # Create test data
        collector = MetricsCollector(db_path=db_path, typing_baseline_wpm=40.0)

        # Create 3 sessions
        for session_num in range(3):
            session = collector.start_session()

            for seg_num in range(5):
                collector.record_segment(
                    audio_duration_s=2.0,
                    transcription=f"Test session {session_num + 1} segment {seg_num + 1} with content",
                    stt_latency_ms=250.0 + session_num * 20,
                    transform_latency_us=0.25,
                    injection_latency_ms=18.0
                )

            collector.end_session()

        collector.close()
        print(f"  ✓ Created 3 test sessions with data")

        # Test CLI
        cli = MetricsCLI(db_path)

        # Test stats display
        print("\n  Testing stats display:")
        print("  " + "=" * 48)
        cli.show_stats()
        print("  " + "=" * 48)

        # Test history display
        print("\n  Testing history display:")
        print("  " + "=" * 48)
        cli.show_history(limit=3)
        print("  " + "=" * 48)

        # Test summary display
        print("\n  Testing summary display:")
        print("  " + "=" * 48)
        cli.show_summary()
        print("  " + "=" * 48)

        cli.close()
        print("\n  ✓ All CLI displays rendered successfully")
        print("  ✓ Test passed\n")
        return True

    finally:
        if os.path.exists(db_path):
            os.unlink(db_path)


def main():
    """Run all tests."""
    print("\n" + "=" * 50)
    print("METRICS SYSTEM INTEGRATION TESTS")
    print("=" * 50 + "\n")

    tests = [
        test_database_creation,
        test_session_lifecycle,
        test_cli_display
    ]

    passed = 0
    failed = 0

    for test in tests:
        try:
            if test():
                passed += 1
            else:
                failed += 1
        except Exception as e:
            print(f"  ✗ Test failed with exception: {e}")
            import traceback
            traceback.print_exc()
            failed += 1

    print("=" * 50)
    print(f"RESULTS: {passed} passed, {failed} failed")
    print("=" * 50 + "\n")

    return 0 if failed == 0 else 1


if __name__ == '__main__':
    sys.exit(main())
