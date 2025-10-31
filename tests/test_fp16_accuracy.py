#!/usr/bin/env python3
"""
FP16 Transcription Accuracy Test
Tests the FP16 model with MP3 files using the live daemon
"""

import sys
import time
import subprocess
import json
from pathlib import Path
from typing import Dict, List, Tuple

# Test cases with expected transcriptions
TEST_CASES = [
    {
        'file': 'tests/data/en-short.mp3',
        'expected_keywords': ['hello', 'world', 'testing', 'one', 'two', 'three'],
        'expected_full': 'Hello world. Testing, one, two, three.',
        'description': 'Short audio (6s) - baseline accuracy test',
        'min_duration': 5.0,
        'target_accuracy': 0.95,  # 95% word accuracy
    },
    {
        'file': 'tests/data/silent-10s.mp3',
        'expected_keywords': [],  # Should be empty or minimal
        'max_words': 2,  # Allow up to 2 noise artifacts
        'description': 'Silent audio (10s) - hallucination test',
        'min_duration': 8.0,
        'target_accuracy': 1.0,  # No hallucinations
    },
]


def send_daemon_command(action: str) -> Dict:
    """Send command to daemon via CLI"""
    try:
        result = subprocess.run(
            ['/opt/swictation/src/swictation_cli.py', action],
            capture_output=True,
            text=True,
            timeout=5
        )

        # Parse output
        if result.returncode == 0:
            return {'success': True, 'output': result.stdout}
        else:
            return {'success': False, 'error': result.stderr}

    except subprocess.TimeoutExpired:
        return {'success': False, 'error': 'Timeout'}
    except Exception as e:
        return {'success': False, 'error': str(e)}


def check_daemon_status() -> Tuple[bool, str]:
    """Check if daemon is running and get state"""
    result = subprocess.run(
        ['systemctl', '--user', 'is-active', 'swictation.service'],
        capture_output=True,
        text=True
    )

    is_running = result.stdout.strip() == 'active'

    if is_running:
        # Get state via CLI
        response = send_daemon_command('status')
        if response['success']:
            # Parse state from output
            for line in response['output'].split('\n'):
                if 'state:' in line.lower():
                    state = line.split(':')[-1].strip()
                    return True, state
            return True, 'unknown'
        return True, 'unknown'

    return False, 'not_running'


def verify_fp16_active() -> bool:
    """Verify FP16 is active from logs"""
    result = subprocess.run(
        ['journalctl', '--user', '-u', 'swictation.service', '-n', '200'],
        capture_output=True,
        text=True
    )

    return 'float16' in result.stdout or 'FP16' in result.stdout


def get_audio_duration(file_path: str) -> float:
    """Get audio duration using ffprobe"""
    try:
        result = subprocess.run(
            ['ffprobe', '-v', 'quiet', '-show_entries', 'format=duration',
             '-of', 'default=noprint_wrappers=1:nokey=1', file_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        return float(result.stdout.strip())
    except:
        return 0.0


def play_audio_to_daemon(file_path: str) -> bool:
    """
    Play audio file through PulseAudio so daemon can capture it.
    Uses a loopback module to route audio back as input.
    """
    try:
        # Load loopback module (routes output → input)
        subprocess.run(
            ['pactl', 'load-module', 'module-loopback',
             'source=swictation.monitor', 'sink=@DEFAULT_SINK@'],
            capture_output=True,
            timeout=5
        )

        # Play the file (daemon will capture via loopback)
        subprocess.run(
            ['paplay', file_path],
            capture_output=True,
            timeout=30
        )

        # Unload loopback
        subprocess.run(
            ['pactl', 'unload-module', 'module-loopback'],
            capture_output=True,
            timeout=5
        )

        return True

    except Exception as e:
        print(f"  ⚠️  Audio playback error: {e}")
        # Try to cleanup loopback
        subprocess.run(['pactl', 'unload-module', 'module-loopback'],
                      capture_output=True)
        return False


def get_recent_transcription(since_seconds: int = 10) -> List[str]:
    """Get recent transcriptions from daemon logs"""
    result = subprocess.run(
        ['journalctl', '--user', '-u', 'swictation.service',
         '--since', f'{since_seconds} seconds ago', '--no-pager'],
        capture_output=True,
        text=True
    )

    transcriptions = []
    for line in result.stdout.split('\n'):
        if '  Text:' in line:
            # Extract text after "Text:"
            text = line.split('  Text:')[-1].strip()
            if text:
                transcriptions.append(text)

    return transcriptions


def test_case(test: Dict) -> Dict:
    """Run a single test case"""
    print(f"\n{'='*70}")
    print(f"Testing: {test['description']}")
    print(f"File: {test['file']}")
    print(f"{'='*70}")

    file_path = Path(test['file'])
    if not file_path.exists():
        return {
            'success': False,
            'error': f"File not found: {test['file']}"
        }

    # Get audio duration
    duration = get_audio_duration(str(file_path))
    print(f"  Duration: {duration:.2f}s")

    if duration < test.get('min_duration', 1.0):
        return {
            'success': False,
            'error': f"Audio too short: {duration:.2f}s"
        }

    # Start recording
    print("  Starting recording...")
    response = send_daemon_command('toggle')
    if not response['success']:
        return {
            'success': False,
            'error': f"Failed to start recording: {response.get('error')}"
        }

    time.sleep(0.5)  # Let recording stabilize

    # Play audio
    print("  Playing audio to daemon...")
    if not play_audio_to_daemon(str(file_path)):
        send_daemon_command('toggle')  # Stop recording
        return {
            'success': False,
            'error': "Failed to play audio"
        }

    # Wait for processing (duration + buffer)
    wait_time = duration + 2.0
    print(f"  Waiting {wait_time:.1f}s for transcription...")
    time.sleep(wait_time)

    # Stop recording
    print("  Stopping recording...")
    send_daemon_command('toggle')

    # Wait for processing to complete
    time.sleep(2.0)

    # Get transcriptions
    transcriptions = get_recent_transcription(since_seconds=int(wait_time + 5))

    if not transcriptions:
        return {
            'success': False,
            'error': "No transcription output found",
            'transcriptions': []
        }

    # Check for expected keywords
    full_text = ' '.join(transcriptions).lower()
    print(f"\n  Transcription: '{full_text}'")

    # Handle silent audio test (hallucination detection)
    if 'max_words' in test:
        word_count = len(full_text.split()) if full_text else 0
        max_allowed = test['max_words']
        passed = word_count <= max_allowed

        print(f"  Word count: {word_count} (max allowed: {max_allowed})")

        return {
            'success': passed,
            'transcriptions': transcriptions,
            'full_text': full_text,
            'word_count': word_count,
            'max_allowed': max_allowed,
            'accuracy': 1.0 if passed else 0.0,
            'test_type': 'hallucination'
        }

    # Handle normal transcription test (keyword matching)
    matched_keywords = []
    missing_keywords = []

    for keyword in test['expected_keywords']:
        if keyword.lower() in full_text:
            matched_keywords.append(keyword)
        else:
            missing_keywords.append(keyword)

    if len(test['expected_keywords']) > 0:
        accuracy = len(matched_keywords) / len(test['expected_keywords'])
    else:
        accuracy = 0.0

    target_accuracy = test.get('target_accuracy', 0.50)
    passed = accuracy >= target_accuracy

    return {
        'success': passed,
        'transcriptions': transcriptions,
        'full_text': full_text,
        'matched_keywords': matched_keywords,
        'missing_keywords': missing_keywords,
        'accuracy': accuracy,
        'target_accuracy': target_accuracy,
        'test_type': 'keyword_match'
    }


def main():
    """Main test execution"""
    print("="*80)
    print("FP16 TRANSCRIPTION ACCURACY TEST")
    print("="*80)

    # Check daemon is running
    print("\nChecking daemon status...")
    is_running, state = check_daemon_status()

    if not is_running:
        print("❌ Daemon is not running")
        print("   Start it: systemctl --user start swictation.service")
        return 1

    print(f"✓ Daemon is running (state: {state})")

    # Verify FP16
    print("\nVerifying FP16 precision...")
    if verify_fp16_active():
        print("✓ FP16 precision confirmed")
    else:
        print("⚠️  Cannot confirm FP16 from logs")

    # Run tests
    results = []

    for test in TEST_CASES:
        result = test_case(test)
        results.append({
            'test': test,
            'result': result
        })

        if result['success']:
            print(f"  ✓ PASS (accuracy: {result.get('accuracy', 0)*100:.0f}%)")
        else:
            print(f"  ✗ FAIL: {result.get('error', 'Unknown error')}")

    # Summary
    print("\n" + "="*80)
    print("TEST SUMMARY")
    print("="*80)

    passed = sum(1 for r in results if r['result']['success'])
    total = len(results)

    print(f"\nPassed: {passed}/{total}")

    for r in results:
        status = "✓ PASS" if r['result']['success'] else "✗ FAIL"
        print(f"\n{status} - {r['test']['description']}")

        if 'accuracy' in r['result']:
            print(f"  Accuracy: {r['result']['accuracy']*100:.0f}%")

        if r['result'].get('matched_keywords'):
            print(f"  Matched: {', '.join(r['result']['matched_keywords'])}")

        if r['result'].get('missing_keywords'):
            print(f"  Missing: {', '.join(r['result']['missing_keywords'])}")

        if not r['result']['success'] and 'error' in r['result']:
            print(f"  Error: {r['result']['error']}")

    print("\n" + "="*80)

    return 0 if passed == total else 1


if __name__ == '__main__':
    sys.exit(main())
