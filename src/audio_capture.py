#!/usr/bin/env python3
"""
PipeWire audio capture module for Swictation.
Captures real-time audio from microphone or system audio (loopback) for transcription.
"""

import sounddevice as sd
import numpy as np
from collections import deque
import threading
import time
import subprocess
import struct
from typing import Optional, Callable

class AudioCapture:
    """
    Real-time audio capture using sounddevice with PipeWire backend.
    Supports both microphone input and system audio loopback (for testing).
    """

    def __init__(
        self,
        sample_rate: int = 16000,
        channels: int = 1,
        dtype: str = 'float32',
        blocksize: int = 1024,
        device: Optional[int] = None,
        buffer_duration: float = 10.0,
        chunk_duration: float = 1.0,
        streaming_mode: bool = False
    ):
        """
        Initialize audio capture.

        Args:
            sample_rate: Target sample rate (Canary expects 16kHz)
            channels: Number of audio channels (1 = mono)
            dtype: Audio data type
            blocksize: Samples per callback (affects latency)
            device: Audio device index (None = default)
            buffer_duration: Maximum buffer duration in seconds
            chunk_duration: Duration of chunks for streaming mode (default: 1.0s)
            streaming_mode: Enable streaming mode with chunk callbacks
        """
        self.sample_rate = sample_rate
        self.channels = channels
        self.dtype = dtype
        self.blocksize = blocksize
        self.device = device

        # Circular buffer for audio samples (batch mode)
        self.max_buffer_samples = int(buffer_duration * sample_rate)
        self.buffer = deque(maxlen=self.max_buffer_samples)
        self.buffer_lock = threading.Lock()

        # Streaming mode configuration
        self.streaming_mode = streaming_mode
        self.chunk_duration = chunk_duration
        self.chunk_frames = int(chunk_duration * sample_rate)
        self._chunk_buffer: list = []
        self._chunk_buffer_lock = threading.Lock()

        # Recording state
        self.stream: Optional[sd.InputStream] = None
        self.parec_process: Optional[subprocess.Popen] = None
        self.parec_thread: Optional[threading.Thread] = None
        self.use_parec = False
        self.is_recording = False
        self.total_frames = 0

        # Callback for audio events
        self.on_audio_callback: Optional[Callable[[np.ndarray, int], None]] = None
        self.on_chunk_ready: Optional[Callable[[np.ndarray], None]] = None

    def list_devices(self):
        """List available audio devices"""
        devices = sd.query_devices()
        print("\n" + "=" * 80)
        print("Available Audio Devices:")
        print("=" * 80)

        for i, device in enumerate(devices):
            device_type = []
            if device['max_input_channels'] > 0:
                device_type.append('INPUT')
            if device['max_output_channels'] > 0:
                device_type.append('OUTPUT')

            type_str = '/'.join(device_type)
            default_marker = ''

            if i == sd.default.device[0]:
                default_marker = ' [DEFAULT INPUT]'
            elif i == sd.default.device[1]:
                default_marker = ' [DEFAULT OUTPUT]'

            print(f"\n{i:3d}: {device['name']}")
            print(f"     Type: {type_str}{default_marker}")
            print(f"     Channels: IN={device['max_input_channels']}, OUT={device['max_output_channels']}")
            print(f"     Sample Rate: {device['default_samplerate']} Hz")

        print("\n" + "=" * 80)
        return devices

    def _audio_callback(self, indata, frames, time_info, status):
        """
        Callback for audio stream. Called continuously during recording.

        Args:
            indata: Input audio data
            frames: Number of frames
            time_info: Timing information
            status: Stream status
        """
        if status:
            print(f"Audio stream status: {status}")

        # Convert to mono if necessary
        if indata.shape[1] > 1:
            audio = np.mean(indata, axis=1)
        else:
            audio = indata[:, 0]

        # Add to buffer (always for backward compatibility)
        with self.buffer_lock:
            self.buffer.extend(audio)
            self.total_frames += frames

        # Streaming mode: accumulate into chunks
        if self.streaming_mode:
            with self._chunk_buffer_lock:
                self._chunk_buffer.extend(audio)

                # Emit chunk when we have enough samples
                while len(self._chunk_buffer) >= self.chunk_frames:
                    # Extract exactly chunk_frames samples
                    chunk = np.array(self._chunk_buffer[:self.chunk_frames], dtype=self.dtype)

                    # Remove processed samples from buffer
                    self._chunk_buffer = self._chunk_buffer[self.chunk_frames:]

                    # Call chunk callback if provided
                    if self.on_chunk_ready:
                        self.on_chunk_ready(chunk)

        # Call external callback if provided (legacy support)
        if self.on_audio_callback:
            self.on_audio_callback(audio, frames)

    def _parec_reader(self):
        """Thread function to read from parec subprocess"""
        bytes_per_sample = 2  # int16 = 2 bytes
        samples_per_read = self.blocksize

        while self.is_recording and self.parec_process:
            try:
                # Read raw audio data
                data = self.parec_process.stdout.read(samples_per_read * bytes_per_sample * self.channels)
                if not data:
                    break

                # Convert bytes to numpy array
                audio = np.frombuffer(data, dtype=np.int16).astype(np.float32) / 32768.0

                # Convert to mono if needed
                if self.channels > 1:
                    audio = audio.reshape(-1, self.channels).mean(axis=1)

                # Add to buffer (always for backward compatibility)
                with self.buffer_lock:
                    self.buffer.extend(audio)
                    self.total_frames += len(audio)

                # Streaming mode: accumulate into chunks
                if self.streaming_mode:
                    with self._chunk_buffer_lock:
                        self._chunk_buffer.extend(audio)

                        # Emit chunk when we have enough samples
                        while len(self._chunk_buffer) >= self.chunk_frames:
                            # Extract exactly chunk_frames samples
                            chunk = np.array(self._chunk_buffer[:self.chunk_frames], dtype=self.dtype)

                            # Remove processed samples from buffer
                            self._chunk_buffer = self._chunk_buffer[self.chunk_frames:]

                            # Call chunk callback if provided
                            if self.on_chunk_ready:
                                self.on_chunk_ready(chunk)

                # Call external callback (legacy support)
                if self.on_audio_callback:
                    self.on_audio_callback(audio, len(audio))

            except Exception as e:
                if self.is_recording:
                    print(f"  parec read error: {e}")
                break

    def start(self):
        """Start audio capture"""
        if self.is_recording:
            print("⚠ Already recording")
            return

        print(f"\n🎤 Starting audio capture:")
        print(f"   Sample rate: {self.sample_rate} Hz")
        print(f"   Channels: {self.channels}")
        print(f"   Device: {self.device if self.device is not None else 'default'}")
        print(f"   Blocksize: {self.blocksize} samples ({self.blocksize / self.sample_rate * 1000:.1f}ms)")

        if self.streaming_mode:
            print(f"   Streaming mode: ENABLED")
            print(f"   Chunk duration: {self.chunk_duration}s ({self.chunk_frames} frames)")

        # Clear buffers
        with self.buffer_lock:
            self.buffer.clear()
            self.total_frames = 0

        with self._chunk_buffer_lock:
            self._chunk_buffer.clear()

        # Check if device is a PipeWire monitor source (string starting with 'alsa_')
        if isinstance(self.device, str) and (self.device.startswith('alsa_') or '.' in self.device):
            # Use parec for PipeWire/PulseAudio sources
            print(f"   Using parec for PipeWire source")
            try:
                self.parec_process = subprocess.Popen(
                    [
                        'parec',
                        '--device', self.device,
                        '--rate', str(self.sample_rate),
                        '--channels', str(self.channels),
                        '--format', 's16le',
                        '--latency-msec', '50'
                    ],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL
                )

                self.use_parec = True
                self.is_recording = True

                # Start reader thread
                self.parec_thread = threading.Thread(target=self._parec_reader, daemon=True)
                self.parec_thread.start()

                print("✓ Recording started (parec)")
                return

            except Exception as e:
                print(f"✗ Failed to start parec: {e}")
                self.parec_process = None
                raise

        # Fall back to sounddevice for regular devices
        try:
            self.stream = sd.InputStream(
                device=self.device,
                channels=self.channels,
                samplerate=self.sample_rate,
                dtype=self.dtype,
                blocksize=self.blocksize,
                callback=self._audio_callback
            )

            self.stream.start()
            self.use_parec = False
            self.is_recording = True
            print("✓ Recording started (sounddevice)")

        except Exception as e:
            print(f"✗ Failed to start recording: {e}")
            self.stream = None
            raise

    def stop(self) -> np.ndarray:
        """
        Stop audio capture and return buffered audio.

        Returns:
            Captured audio as numpy array
        """
        if not self.is_recording:
            print("⚠ Not recording")
            return np.array([], dtype=self.dtype)

        print("\n🛑 Stopping audio capture...")

        self.is_recording = False

        # Stop parec if used
        if self.use_parec and self.parec_process:
            self.parec_process.terminate()
            try:
                self.parec_process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.parec_process.kill()
            self.parec_process = None

            if self.parec_thread:
                self.parec_thread.join(timeout=2)
                self.parec_thread = None

        # Stop sounddevice stream
        if self.stream:
            self.stream.stop()
            self.stream.close()
            self.stream = None

        # Get buffered audio
        with self.buffer_lock:
            audio = np.array(list(self.buffer), dtype=self.dtype)
            duration = len(audio) / self.sample_rate

        print(f"✓ Captured {self.total_frames} frames ({duration:.2f}s)")

        return audio

    def get_buffer(self) -> np.ndarray:
        """Get current buffer contents without stopping recording"""
        with self.buffer_lock:
            return np.array(list(self.buffer), dtype=self.dtype)

    def get_buffer_duration(self) -> float:
        """Get current buffer duration in seconds"""
        with self.buffer_lock:
            return len(self.buffer) / self.sample_rate

    def is_active(self) -> bool:
        """Check if recording is active"""
        return self.is_recording

    def get_chunk_buffer_size(self) -> int:
        """Get current chunk buffer size (number of samples accumulated)"""
        with self._chunk_buffer_lock:
            return len(self._chunk_buffer)

    def get_chunk_buffer_progress(self) -> float:
        """Get chunk buffer progress as percentage (0.0 to 1.0)"""
        with self._chunk_buffer_lock:
            if self.chunk_frames == 0:
                return 0.0
            return min(len(self._chunk_buffer) / self.chunk_frames, 1.0)

    def __enter__(self):
        """Context manager entry"""
        self.start()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        if self.is_recording:
            self.stop()


def find_loopback_device():
    """
    Find system audio loopback device for testing.
    This allows capturing system audio output (for playing test MP3s).

    Returns:
        Device name string (for PipeWire/PulseAudio) or index, or None if not found
    """
    # First try to find PipeWire/PulseAudio monitor source
    try:
        import subprocess
        result = subprocess.run(
            ['pactl', 'list', 'short', 'sources'],
            capture_output=True,
            text=True,
            timeout=2
        )

        if result.returncode == 0:
            for line in result.stdout.split('\n'):
                if '.monitor' in line.lower():
                    # Extract device name (first field)
                    parts = line.split()
                    if parts:
                        device_name = parts[1]  # Second field is the name
                        print(f"  Found PipeWire monitor: {device_name}")
                        return device_name
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception) as e:
        print(f"  pactl not available or failed: {e}")

    # Fallback to sounddevice device enumeration
    devices = sd.query_devices()

    # Look for monitor/loopback devices
    for i, device in enumerate(devices):
        name_lower = device['name'].lower()
        if any(keyword in name_lower for keyword in ['monitor', 'loopback', 'what-u-hear', 'stereo mix']):
            if device['max_input_channels'] > 0:
                return i

    return None


def test_audio_capture(duration: float = 5.0, device: Optional[int] = None):
    """
    Test audio capture for specified duration.

    Args:
        duration: Recording duration in seconds
        device: Device index (None = default microphone)
    """
    print("=" * 80)
    print("Audio Capture Test")
    print("=" * 80)

    capture = AudioCapture(device=device)
    capture.list_devices()

    try:
        print(f"\nRecording for {duration} seconds...")
        print("Speak into your microphone!\n")

        capture.start()
        time.sleep(duration)
        audio = capture.stop()

        # Analyze captured audio
        if len(audio) > 0:
            rms = np.sqrt(np.mean(audio**2))
            db = 20 * np.log10(rms) if rms > 0 else -100
            peak = np.max(np.abs(audio))

            print(f"\n📊 Audio Analysis:")
            print(f"   Samples: {len(audio)}")
            print(f"   Duration: {len(audio) / capture.sample_rate:.2f}s")
            print(f"   RMS: {rms:.6f}")
            print(f"   RMS (dB): {db:.2f} dB")
            print(f"   Peak: {peak:.6f}")

            if db < -40:
                print(f"\n⚠ Very quiet audio (< -40dB). Check microphone!")
            elif db < -25:
                print(f"\n✓ Good audio levels")
            else:
                print(f"\n⚠ Very loud audio. May clip!")

            return audio
        else:
            print("\n✗ No audio captured!")
            return None

    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        capture.stop()
        return None

    except Exception as e:
        print(f"\n✗ Error: {e}")
        return None


if __name__ == '__main__':
    import sys

    # List devices if requested
    if len(sys.argv) > 1 and sys.argv[1] == 'list':
        capture = AudioCapture()
        capture.list_devices()
        sys.exit(0)

    # Run test
    duration = float(sys.argv[1]) if len(sys.argv) > 1 else 5.0
    device = int(sys.argv[2]) if len(sys.argv) > 2 else None

    test_audio_capture(duration, device)
