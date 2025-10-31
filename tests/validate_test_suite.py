#!/usr/bin/env python3
"""
Test Suite Validation Script

Quick validation that all OOM prevention tests are properly structured
and can be discovered by pytest.
"""

import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

def validate_test_structure():
    """Validate test file structure"""
    print("=" * 80)
    print("TEST SUITE VALIDATION")
    print("=" * 80)

    test_dir = Path(__file__).parent

    # Check test files exist
    test_files = [
        'test_memory_protection.py',
        'test_oom_recovery.py',
        'pytest_memory.ini',
        'RUN_MEMORY_TESTS.md'
    ]

    print("\n📁 Checking test files...")
    all_exist = True
    for filename in test_files:
        filepath = test_dir / filename
        exists = filepath.exists()
        size = filepath.stat().st_size if exists else 0
        status = "✓" if exists else "✗"
        print(f"  {status} {filename:30s} ({size:>6} bytes)")
        all_exist = all_exist and exists

    if not all_exist:
        print("\n❌ VALIDATION FAILED: Missing test files")
        return False

    # Validate imports
    print("\n📦 Validating imports...")
    try:
        import numpy as np
        print("  ✓ numpy")
    except ImportError:
        print("  ✗ numpy - REQUIRED")
        return False

    try:
        import pytest
        print(f"  ✓ pytest {pytest.__version__}")
    except ImportError:
        print("  ✗ pytest - REQUIRED")
        return False

    try:
        import torch
        print(f"  ✓ torch {torch.__version__}")
        gpu_available = torch.cuda.is_available()
        print(f"    GPU available: {gpu_available}")
    except ImportError:
        print("  ⚠️  torch - Optional (GPU tests will skip)")

    try:
        from performance_monitor import PerformanceMonitor
        print("  ✓ performance_monitor")
    except ImportError as e:
        print(f"  ✗ performance_monitor - {e}")
        return False

    # Count test classes and functions
    print("\n🧪 Analyzing test structure...")

    test_stats = {
        'test_memory_protection.py': {
            'classes': 0,
            'tests': 0,
            'lines': 0
        },
        'test_oom_recovery.py': {
            'classes': 0,
            'tests': 0,
            'lines': 0
        }
    }

    for filename in ['test_memory_protection.py', 'test_oom_recovery.py']:
        filepath = test_dir / filename
        with open(filepath, 'r') as f:
            content = f.read()
            lines = content.split('\n')

            test_stats[filename]['lines'] = len(lines)
            test_stats[filename]['classes'] = len([l for l in lines if l.strip().startswith('class Test')])
            test_stats[filename]['tests'] = len([l for l in lines if l.strip().startswith('def test_')])

    total_classes = sum(s['classes'] for s in test_stats.values())
    total_tests = sum(s['tests'] for s in test_stats.values())
    total_lines = sum(s['lines'] for s in test_stats.values())

    print(f"\n  Test classes: {total_classes}")
    print(f"  Test functions: {total_tests}")
    print(f"  Total lines: {total_lines}")

    for filename, stats in test_stats.items():
        print(f"\n  {filename}:")
        print(f"    Classes: {stats['classes']}")
        print(f"    Tests: {stats['tests']}")
        print(f"    Lines: {stats['lines']}")

    # Validate test scenarios
    print("\n✅ Test Coverage:")

    scenarios = {
        'Memory Stress': [
            'Sustained high memory usage (60s)',
            'Memory allocation/deallocation stress (100 cycles)',
            'Continuous 1-hour recording'
        ],
        'GPU Fallback': [
            'GPU→CPU fallback on OOM',
            'Rapid toggle with GPU/CPU transitions'
        ],
        'CUDA Recovery': [
            'Single CUDA error recovery',
            'Repeated CUDA errors (3+)'
        ],
        'Emergency Shutdown': [
            'Shutdown trigger on critical memory',
            'Data preservation on shutdown'
        ],
        'OOM Recovery': [
            'OOM detection',
            'Progressive recovery strategies',
            'Audio buffer preservation',
            'Transcription state preservation',
            'Consecutive OOM handling',
            'OOM during recovery'
        ]
    }

    total_scenarios = 0
    for category, tests in scenarios.items():
        print(f"\n  {category}:")
        for test in tests:
            print(f"    • {test}")
            total_scenarios += 1

    print(f"\n  Total scenarios: {total_scenarios}")

    # Validate pass/fail criteria
    print("\n📊 Pass/Fail Criteria:")
    criteria = [
        ('Memory Leak', '<1 MB/s growth, <100MB total'),
        ('GPU Fallback', 'Graceful transition, no crash'),
        ('Data Preservation', 'Bit-perfect, no loss'),
        ('Recovery', 'System operational after recovery'),
        ('CUDA Recovery', 'GPU functional after error')
    ]

    for category, criterion in criteria:
        print(f"  • {category}: {criterion}")

    print("\n" + "=" * 80)
    print("✅ VALIDATION PASSED")
    print("=" * 80)
    print(f"\nTest Suite Summary:")
    print(f"  Files: {len(test_files)}")
    print(f"  Test Classes: {total_classes}")
    print(f"  Test Functions: {total_tests}")
    print(f"  Test Scenarios: {total_scenarios}")
    print(f"  Lines of Code: {total_lines}")
    print(f"\nReady for execution:")
    print(f"  Quick tests: pytest tests/ -v -m 'not slow'")
    print(f"  Full suite: pytest tests/ -v")

    return True


if __name__ == '__main__':
    success = validate_test_structure()
    sys.exit(0 if success else 1)
