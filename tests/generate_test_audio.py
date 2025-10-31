#!/usr/bin/env python3
"""
Generate synthetic test audio for E2E testing.

Creates test audio files with known transcriptions for validation.
Uses text-to-speech for reproducible test cases.
"""

import subprocess
import sys
from pathlib import Path


TEST_DATA_DIR = Path(__file__).parent / 'data'
TEST_DATA_DIR.mkdir(exist_ok=True)


def check_dependencies():
    """Check if required tools are installed"""
    required = ['espeak', 'ffmpeg']
    missing = []

    for tool in required:
        try:
            subprocess.run(
                ['which', tool],
                capture_output=True,
                check=True,
                timeout=1
            )
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            missing.append(tool)

    if missing:
        print(f"❌ Missing required tools: {', '.join(missing)}")
        print(f"\nInstall with:")
        if 'espeak' in missing:
            print(f"  sudo apt install espeak")
        if 'ffmpeg' in missing:
            print(f"  sudo apt install ffmpeg")
        return False

    return True


def generate_audio(text: str, output_file: Path, duration: float = None):
    """
    Generate audio from text using espeak.

    Args:
        text: Text to synthesize
        output_file: Output MP3 file path
        duration: Optional target duration (for padding)
    """
    wav_file = output_file.with_suffix('.wav')

    print(f"Generating: {output_file.name}")
    print(f"  Text: '{text}'")

    # Generate with espeak
    try:
        subprocess.run(
            ['espeak', text, '-w', str(wav_file)],
            check=True,
            capture_output=True,
            timeout=10
        )
    except subprocess.CalledProcessError as e:
        print(f"❌ espeak failed: {e.stderr.decode()}")
        return False

    # Convert to 16kHz mono MP3
    try:
        cmd = [
            'ffmpeg', '-i', str(wav_file),
            '-ar', '16000',  # 16kHz sample rate
            '-ac', '1',       # Mono
            '-acodec', 'libmp3lame',
            '-y',             # Overwrite
            str(output_file)
        ]

        subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            timeout=10
        )
    except subprocess.CalledProcessError as e:
        print(f"❌ ffmpeg failed: {e.stderr.decode()}")
        return False
    finally:
        # Cleanup WAV
        if wav_file.exists():
            wav_file.unlink()

    print(f"✅ Generated: {output_file}")
    return True


def generate_silent_audio(duration: float, output_file: Path):
    """Generate silent audio for hallucination testing"""
    print(f"Generating: {output_file.name} ({duration}s silent)")

    try:
        cmd = [
            'ffmpeg',
            '-f', 'lavfi',
            '-i', f'anullsrc=r=16000:cl=mono',
            '-t', str(duration),
            '-acodec', 'libmp3lame',
            '-y',
            str(output_file)
        ]

        subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            timeout=15
        )

        print(f"✅ Generated: {output_file}")
        return True

    except subprocess.CalledProcessError as e:
        print(f"❌ Failed: {e.stderr.decode()}")
        return False


def main():
    """Generate all test audio files"""
    print("=" * 60)
    print("TEST AUDIO GENERATOR")
    print("=" * 60)

    if not check_dependencies():
        return 1

    print(f"\nOutput directory: {TEST_DATA_DIR}")
    print()

    success_count = 0
    total_count = 0

    # Test cases
    test_cases = [
        {
            'text': "Hello world. Testing, one, two, three.",
            'file': 'en-short-synthetic.mp3',
            'description': 'Short utterance for accuracy testing'
        },
        {
            'text': "The cat sat on the mat. The cat was orange.",
            'file': 'context-test.mp3',
            'description': 'Context preservation test'
        },
        {
            'text': "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.",
            'file': 'pangram-test.mp3',
            'description': 'Pangram for comprehensive character coverage'
        },
        {
            'text': "Testing punctuation: comma, period. Question mark? Exclamation point! And numbers: one, two, three, four, five.",
            'file': 'punctuation-test.mp3',
            'description': 'Punctuation and number test'
        },
    ]

    # Generate speech audio
    for test in test_cases:
        total_count += 1
        output_path = TEST_DATA_DIR / test['file']

        print(f"[{total_count}] {test['description']}")

        if generate_audio(test['text'], output_path):
            success_count += 1
        print()

    # Generate silent audio
    silent_cases = [
        (10, 'silent-10s.mp3'),
        (5, 'silent-5s.mp3'),
    ]

    for duration, filename in silent_cases:
        total_count += 1
        output_path = TEST_DATA_DIR / filename

        if generate_silent_audio(duration, output_path):
            success_count += 1
        print()

    # Summary
    print("=" * 60)
    print(f"Generated {success_count}/{total_count} files successfully")
    print("=" * 60)

    if success_count == total_count:
        print("\n✅ All test audio files generated")
        print(f"\nFiles in {TEST_DATA_DIR}:")
        for file in sorted(TEST_DATA_DIR.glob('*.mp3')):
            size_kb = file.stat().st_size / 1024
            print(f"  {file.name} ({size_kb:.1f} KB)")
        return 0
    else:
        print(f"\n⚠️  {total_count - success_count} files failed to generate")
        return 1


if __name__ == '__main__':
    sys.exit(main())
