#!/usr/bin/env python3
"""
Swictation daemon - Coordinates audio capture, STT, and text injection.
Runs as background service with Unix socket IPC for toggle control.
"""

import os
import sys
import time
import threading
import socket
import signal
import json
from pathlib import Path
from typing import Optional
from enum import Enum
import tempfile

# Import our modules
from audio_capture import AudioCapture
from text_injection import TextInjector, InjectionMethod

# STT imports
import torch
import numpy as np
import soundfile as sf
from nemo.collections.asr.models import EncDecMultiTaskModel


class DaemonState(Enum):
    """Daemon states"""
    IDLE = "idle"
    RECORDING = "recording"
    PROCESSING = "processing"
    ERROR = "error"


class SwictationDaemon:
    """
    Main daemon process for Swictation.
    Coordinates audio capture → STT → text injection pipeline.
    """

    def __init__(
        self,
        model_name: str = 'nvidia/canary-1b-flash',
        sample_rate: int = 16000,
        socket_path: str = '/tmp/swictation.sock'
    ):
        """
        Initialize Swictation daemon.

        Args:
            model_name: STT model to use
            sample_rate: Audio sample rate (16kHz for Canary)
            socket_path: Unix socket path for IPC
        """
        self.model_name = model_name
        self.sample_rate = sample_rate
        self.socket_path = socket_path

        # State
        self.state = DaemonState.IDLE
        self.state_lock = threading.Lock()
        self.running = False

        # Components (initialized on start)
        self.audio_capture: Optional[AudioCapture] = None
        self.text_injector: Optional[TextInjector] = None
        self.stt_model = None

        # IPC socket
        self.server_socket: Optional[socket.socket] = None
        self.socket_thread: Optional[threading.Thread] = None

        # Temp directory for audio files
        self.temp_dir = Path(tempfile.gettempdir()) / 'swictation'
        self.temp_dir.mkdir(exist_ok=True)

        print("Swictation daemon initialized")

    def load_stt_model(self):
        """Load STT model (heavy operation, done once on startup)"""
        print(f"Loading STT model: {self.model_name}")
        load_start = time.time()

        try:
            self.stt_model = EncDecMultiTaskModel.from_pretrained(self.model_name)
            self.stt_model.eval()

            if torch.cuda.is_available():
                self.stt_model = self.stt_model.cuda()
                print(f"  Using GPU: {torch.cuda.get_device_name(0)}")
            else:
                print("  Using CPU (slower)")

            load_time = time.time() - load_start
            print(f"✓ STT model loaded in {load_time:.2f}s")

        except Exception as e:
            print(f"✗ Failed to load STT model: {e}")
            raise

    def initialize_components(self):
        """Initialize audio capture and text injection"""
        print("Initializing components...")

        # Audio capture
        try:
            self.audio_capture = AudioCapture(
                sample_rate=self.sample_rate,
                buffer_duration=30.0  # 30s max recording
            )
            print("✓ Audio capture initialized")
        except Exception as e:
            print(f"✗ Audio capture init failed: {e}")
            raise

        # Text injector
        try:
            self.text_injector = TextInjector(method=InjectionMethod.WTYPE)
            print(f"✓ Text injector initialized ({self.text_injector.method.value})")
        except Exception as e:
            print(f"✗ Text injector init failed: {e}")
            raise

    def set_state(self, new_state: DaemonState):
        """Thread-safe state update"""
        with self.state_lock:
            old_state = self.state
            self.state = new_state
            print(f"State: {old_state.value} → {new_state.value}")

    def get_state(self) -> DaemonState:
        """Thread-safe state read"""
        with self.state_lock:
            return self.state

    def toggle_recording(self):
        """Toggle recording on/off (main functionality)"""
        current_state = self.get_state()

        if current_state == DaemonState.IDLE:
            # Start recording
            self._start_recording()
        elif current_state == DaemonState.RECORDING:
            # Stop recording and process
            self._stop_recording_and_process()
        elif current_state == DaemonState.PROCESSING:
            print("⚠ Already processing, please wait...")
        else:
            print(f"⚠ Cannot toggle in state: {current_state.value}")

    def _start_recording(self):
        """Start audio capture"""
        try:
            print("\n🎤 Starting recording...")
            self.set_state(DaemonState.RECORDING)

            self.audio_capture.start()
            print("✓ Recording started (speak now)")

        except Exception as e:
            print(f"✗ Failed to start recording: {e}")
            self.set_state(DaemonState.ERROR)

    def _stop_recording_and_process(self):
        """Stop recording and process audio → STT → inject"""
        try:
            print("\n🛑 Stopping recording...")
            self.set_state(DaemonState.PROCESSING)

            # Stop capture and get audio
            audio = self.audio_capture.stop()

            if len(audio) == 0:
                print("⚠ No audio captured")
                self.set_state(DaemonState.IDLE)
                return

            duration = len(audio) / self.sample_rate
            print(f"✓ Captured {duration:.2f}s audio")

            # Transcribe in background thread
            thread = threading.Thread(
                target=self._process_audio,
                args=(audio,),
                daemon=True
            )
            thread.start()

        except Exception as e:
            print(f"✗ Failed to stop recording: {e}")
            self.set_state(DaemonState.ERROR)

    def _process_audio(self, audio: np.ndarray):
        """Process audio: STT → text injection"""
        try:
            # Save to temp file (NeMo requires file path)
            temp_audio_path = self.temp_dir / f'capture_{int(time.time())}.wav'
            sf.write(temp_audio_path, audio, self.sample_rate)

            # Transcribe
            print("🧠 Transcribing...")
            transcribe_start = time.time()

            hypothesis = self.stt_model.transcribe(
                audio=str(temp_audio_path),
                source_lang='en',
                target_lang='en',
                pnc='yes',
                batch_size=1
            )[0]

            transcription = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
            transcribe_time = time.time() - transcribe_start

            print(f"✓ Transcribed in {transcribe_time*1000:.0f}ms")
            print(f"  Text: {transcription}")

            # Inject text
            if transcription.strip():
                print("⌨️ Injecting text...")
                success = self.text_injector.inject(transcription)

                if success:
                    print("✓ Text injected successfully")
                else:
                    print("✗ Text injection failed")
            else:
                print("⚠ Empty transcription, nothing to inject")

            # Cleanup temp file
            temp_audio_path.unlink(missing_ok=True)

            # Return to idle
            self.set_state(DaemonState.IDLE)
            print("✓ Ready for next recording\n")

        except Exception as e:
            print(f"✗ Processing error: {e}")
            self.set_state(DaemonState.ERROR)
            time.sleep(1)
            self.set_state(DaemonState.IDLE)

    def start_ipc_server(self):
        """Start Unix socket server for IPC commands"""
        # Remove existing socket
        if os.path.exists(self.socket_path):
            os.remove(self.socket_path)

        try:
            self.server_socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self.server_socket.bind(self.socket_path)
            self.server_socket.listen(5)

            # Make socket accessible
            os.chmod(self.socket_path, 0o666)

            print(f"✓ IPC socket listening on {self.socket_path}")

            # Start socket thread
            self.socket_thread = threading.Thread(
                target=self._ipc_loop,
                daemon=True
            )
            self.socket_thread.start()

        except Exception as e:
            print(f"✗ Failed to start IPC server: {e}")
            raise

    def _ipc_loop(self):
        """Listen for IPC commands"""
        while self.running:
            try:
                # Accept connection with timeout
                self.server_socket.settimeout(1.0)
                conn, addr = self.server_socket.accept()

                with conn:
                    # Receive command
                    data = conn.recv(1024)
                    if not data:
                        continue

                    try:
                        command = json.loads(data.decode('utf-8'))
                        response = self._handle_command(command)
                    except json.JSONDecodeError:
                        response = {'error': 'Invalid JSON'}

                    # Send response
                    conn.sendall(json.dumps(response).encode('utf-8'))

            except socket.timeout:
                continue
            except Exception as e:
                if self.running:
                    print(f"IPC error: {e}")

    def _handle_command(self, command: dict) -> dict:
        """Handle IPC command"""
        action = command.get('action', '')

        if action == 'toggle':
            self.toggle_recording()
            return {
                'status': 'ok',
                'state': self.get_state().value
            }

        elif action == 'status':
            return {
                'status': 'ok',
                'state': self.get_state().value
            }

        elif action == 'stop':
            self.running = False
            return {'status': 'ok', 'message': 'Stopping daemon'}

        else:
            return {'error': f'Unknown action: {action}'}

    def start(self):
        """Start daemon"""
        print("=" * 80)
        print("Swictation Daemon Starting")
        print("=" * 80)

        try:
            # Load model (slow)
            self.load_stt_model()

            # Initialize components
            self.initialize_components()

            # Start IPC server
            self.start_ipc_server()

            # Set running flag
            self.running = True
            self.set_state(DaemonState.IDLE)

            print("\n✓ Swictation daemon started")
            print("  Ready to receive toggle commands")
            print("  Press Ctrl+C to stop\n")

            # Main loop (just keeps process alive)
            while self.running:
                time.sleep(1)

        except KeyboardInterrupt:
            print("\n\nReceived Ctrl+C, shutting down...")
        except Exception as e:
            print(f"\n✗ Daemon error: {e}")
            sys.exit(1)
        finally:
            self.stop()

    def stop(self):
        """Stop daemon and cleanup"""
        print("\nStopping daemon...")

        self.running = False

        # Stop audio capture if active
        if self.audio_capture and self.audio_capture.is_active():
            self.audio_capture.stop()

        # Close socket
        if self.server_socket:
            self.server_socket.close()

        # Remove socket file
        if os.path.exists(self.socket_path):
            os.remove(self.socket_path)

        print("✓ Daemon stopped")


def main():
    """Main entry point"""
    # Setup signal handlers
    daemon = SwictationDaemon()

    def signal_handler(signum, frame):
        print(f"\nReceived signal {signum}")
        daemon.running = False

    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)

    # Start daemon
    daemon.start()


if __name__ == '__main__':
    main()
