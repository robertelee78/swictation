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
import gc
import librosa
import numpy as np
import soundfile as sf
from nemo.collections.asr.models import EncDecMultiTaskModel
from nemo.collections.asr.parts.utils.streaming_utils import FrameBatchMultiTaskAED
from omegaconf import DictConfig


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
        socket_path: str = '/tmp/swictation.sock',
        chunk_duration: float = 10.0,
        chunk_overlap: float = 1.0,
        vad_threshold: float = 0.5,
        streaming_mode: bool = True,
        streaming_chunk_size: float = 0.4,
        enable_performance_monitoring: bool = True
    ):
        """
        Initialize Swictation daemon.

        Args:
            model_name: STT model to use
            sample_rate: Audio sample rate (16kHz for Canary)
            socket_path: Unix socket path for IPC
            chunk_duration: Duration of each audio chunk in seconds (for memory optimization)
            chunk_overlap: Overlap between chunks in seconds (for context continuity)
            vad_threshold: Voice Activity Detection threshold (0-1)
            streaming_mode: Enable real-time streaming transcription (default: True)
            streaming_chunk_size: Chunk size for streaming in seconds (default: 0.4s = 400ms)
            enable_performance_monitoring: Enable performance monitoring (default: True)
        """
        self.model_name = model_name
        self.sample_rate = sample_rate
        self.socket_path = socket_path
        self.chunk_duration = chunk_duration
        self.chunk_overlap = chunk_overlap
        self.vad_threshold = vad_threshold
        self.streaming_mode = streaming_mode
        self.streaming_chunk_size = streaming_chunk_size
        self.enable_performance_monitoring = enable_performance_monitoring

        # State
        self.state = DaemonState.IDLE
        self.state_lock = threading.Lock()
        self.running = False

        # Streaming state
        self._streaming_buffer = []
        self._streaming_frames = 0
        self._last_transcription = ""
        self._last_injected = ""  # Track last injected text for deduplication
        self._streaming_thread = None

        # Components (initialized on start)
        self.audio_capture: Optional[AudioCapture] = None
        self.text_injector: Optional[TextInjector] = None
        self.stt_model = None
        self.vad_model = None
        self.get_speech_timestamps = None
        self.frame_asr = None  # NeMo streaming processor

        # Performance monitoring
        self.performance_monitor = None
        if enable_performance_monitoring:
            try:
                from performance_monitor import PerformanceMonitor

                # Define warning callbacks
                def performance_warning(message: str):
                    print(f"⚠️  Performance: {message}", flush=True)

                warning_callbacks = {
                    'high_gpu_memory': performance_warning,
                    'high_cpu': performance_warning,
                    'high_latency': performance_warning,
                    'memory_leak': performance_warning,
                }

                self.performance_monitor = PerformanceMonitor(
                    history_size=1000,
                    warning_callbacks=warning_callbacks
                )
            except ImportError:
                print("⚠️  Performance monitoring not available (psutil not installed)")
                self.performance_monitor = None

        # IPC socket
        self.server_socket: Optional[socket.socket] = None
        self.socket_thread: Optional[threading.Thread] = None

        # Temp directory for audio files
        self.temp_dir = Path(tempfile.gettempdir()) / 'swictation'
        self.temp_dir.mkdir(exist_ok=True)

        print("Swictation daemon initialized")

    def load_vad_model(self):
        """Load Silero VAD model (lightweight, 2.2 MB on GPU)"""
        print("Loading Silero VAD model...")
        try:
            # Download Silero VAD from torch hub
            # IMPORTANT: Must be loaded BEFORE STT to avoid torch.hub.load() hanging
            vad_model, utils = torch.hub.load(
                repo_or_dir='snakers4/silero-vad',
                model='silero_vad',
                force_reload=False,
                onnx=False
            )

            if torch.cuda.is_available():
                vad_model = vad_model.cuda()

            vad_model.eval()

            # Extract utility functions
            (get_speech_timestamps, _, _, *_) = utils

            self.vad_model = vad_model
            self.get_speech_timestamps = get_speech_timestamps

            gpu_mem = torch.cuda.memory_allocated() / 1e6 if torch.cuda.is_available() else 0
            print(f"✓ Silero VAD loaded ({gpu_mem:.1f} MB GPU memory)")

        except Exception as e:
            print(f"⚠ Failed to load Silero VAD: {e}")
            print("  Continuing without VAD (all audio will be transcribed)")
            self.vad_model = None
            self.get_speech_timestamps = None

    def load_stt_model(self):
        """Load STT model (heavy operation, done once on startup)"""
        print(f"Loading STT model: {self.model_name}")
        load_start = time.time()

        try:
            self.stt_model = EncDecMultiTaskModel.from_pretrained(self.model_name)
            self.stt_model.eval()

            if torch.cuda.is_available():
                self.stt_model = self.stt_model.cuda()
                print(f"  Using GPU: {torch.cuda.get_device_name(0)}", flush=True)
            else:
                print("  Using CPU (slower)", flush=True)

            load_time = time.time() - load_start
            gpu_mem = torch.cuda.memory_allocated() / 1e6 if torch.cuda.is_available() else 0

            print(f"✓ STT model loaded in {load_time:.2f}s", flush=True)
            print(f"  GPU Memory: {gpu_mem:.1f} MB", flush=True)

            # Initialize NeMo streaming for real-time transcription
            # This enables progressive text injection as the user speaks
            if self.streaming_mode:
                print("  Initializing NeMo Wait-k streaming...", flush=True)

                # Configure Wait-k streaming decoding policy
                # Wait-k is a conservative policy that waits for 'k' chunks before predicting tokens
                # This ensures high accuracy at the cost of slightly higher latency (~1.5s)
                streaming_cfg = DictConfig({
                    'strategy': 'beam',  # Beam search decoding
                    'beam': {
                        'beam_size': 1,  # Greedy decoding (size=1) for lowest latency
                        'return_best_hypothesis': True,
                    },
                    'streaming': {
                        'streaming_policy': 'waitk',  # Wait-k policy (vs AlignAtt)
                        'waitk_lagging': 2,           # Wait for 2 chunks before first prediction
                                                      # Higher = more conservative, better accuracy
                                                      # Lower = faster response, may lose accuracy
                    }
                })

                # Apply streaming configuration to the model
                # This reconfigures the decoder for streaming mode (vs batch mode)
                self.stt_model.change_decoding_strategy(streaming_cfg)

                # Initialize FrameBatchMultiTaskAED for chunk-based streaming
                # This is NeMo's high-level API for streaming with Canary models
                # It manages:
                #   1. Audio buffer with left-context sliding window
                #   2. Encoder state caching for efficiency
                #   3. Decoder state preservation across chunks
                self.frame_asr = FrameBatchMultiTaskAED(
                    asr_model=self.stt_model,
                    frame_len=1.0,        # 1-second chunks (balance of latency/context)
                                          # Smaller = lower latency, less context
                                          # Larger = higher latency, more context
                    total_buffer=10.0,    # 10-second left context window
                                          # This is the "memory" - how much past audio to remember
                                          # Larger = better accuracy (more context for coherence)
                                          # Smaller = less GPU memory usage
                    batch_size=1,         # Real-time single-user processing
                                          # For multi-stream server: increase to 4-8
                )

                print(f"  ✓ NeMo streaming configured (Wait-k policy, 1s chunks, 10s context)", flush=True)

        except Exception as e:
            print(f"✗ Failed to load STT model: {e}")
            raise

    def initialize_components(self):
        """Initialize audio capture and text injection"""
        print("Initializing components...", flush=True)

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
            mode_str = "streaming" if self.streaming_mode else "batch"
            print(f"\n🎤 Starting recording ({mode_str} mode)...")
            self.set_state(DaemonState.RECORDING)

            if self.streaming_mode:
                # Set up callback for streaming chunks
                # This callback is invoked every 64ms with new audio data
                # We accumulate these into 1-second chunks for streaming transcription
                self.audio_capture.on_audio_callback = self._on_audio_chunk

                # Reset streaming state for clean session
                self._streaming_buffer = []           # Accumulator for 1-second chunks
                self._streaming_frames = 0            # Frame counter
                self._last_transcription = ""         # Full cumulative transcription
                self._last_injected = ""              # Text already injected (for deduplication)

                # CRITICAL: Reset FrameBatchMultiTaskAED decoder state for new recording
                # This clears:
                #   1. Audio buffer (left context window)
                #   2. Decoder state (token history, attention context)
                #   3. Encoder cache
                # Without this reset, the new recording would continue from previous session!
                if self.frame_asr is not None:
                    self.frame_asr.reset()
                    print("  ✓ NeMo streaming state reset", flush=True)

            self.audio_capture.start()
            print("✓ Recording started (speak now)")

        except Exception as e:
            print(f"✗ Failed to start recording: {e}")
            self.set_state(DaemonState.ERROR)

    def _on_audio_chunk(self, audio: np.ndarray, frames: int):
        """Callback for real-time audio chunks during streaming mode"""
        self._streaming_buffer.extend(audio)
        self._streaming_frames += frames

        # Check if we have enough frames for a streaming chunk
        chunk_frames = int(self.streaming_chunk_size * self.sample_rate)
        if len(self._streaming_buffer) >= chunk_frames:
            # Extract chunk
            chunk = np.array(self._streaming_buffer[:chunk_frames])
            self._streaming_buffer = self._streaming_buffer[chunk_frames:]

            # Process chunk in background (non-blocking)
            if self._streaming_thread is None or not self._streaming_thread.is_alive():
                self._streaming_thread = threading.Thread(
                    target=self._process_streaming_chunk,
                    args=(chunk.copy(),),
                    daemon=True
                )
                self._streaming_thread.start()

    def _inject_streaming_delta(self, new_transcription: str):
        """
        Inject only NEW words from cumulative transcription to avoid duplicates.

        This is the core of progressive text injection. NeMo's streaming decoder
        returns the FULL cumulative transcription on each chunk:

        Example:
          Chunk 1: "Hello"
          Chunk 2: "Hello world"          ← Full text, not just "world"
          Chunk 3: "Hello world testing"  ← Full text again

        Without deduplication, we'd inject:
          "Hello" + "Hello world" + "Hello world testing" = DUPLICATES!

        With deduplication (this function):
          "Hello" + " world" + " testing" = CORRECT!

        Algorithm:
          1. Check if new text starts with last injected text (prefix match)
          2. If yes: Calculate delta = new[len(last):] and inject only delta
          3. If no: Transcription changed (revision), inject full text

        Args:
            new_transcription: Full cumulative transcription from NeMo decoder
        """
        if not new_transcription.strip():
            return  # Empty transcription, nothing to inject

        # Check if this is an extension of previous text (normal case)
        if new_transcription.startswith(self._last_injected):
            # Calculate delta (new portion only)
            # Example: "Hello world" starts with "Hello"
            #          delta = "Hello world"[len("Hello"):] = " world"
            delta = new_transcription[len(self._last_injected):]

            if delta.strip():  # Only inject if there's actual new content
                print(f"  🎤→ {delta.strip()}", flush=True)
                self.text_injector.inject(delta)  # Inject ONLY the delta
                self._last_injected = new_transcription  # Update state
        else:
            # Transcription changed (correction/revision)
            # This is RARE with Wait-k policy but can happen when:
            #   1. Decoder revises earlier tokens based on new context
            #   2. Punctuation changes ("Hello" → "Hello.")
            #   3. Capitalization corrections
            # In these cases, we inject the full new transcription
            print(f"  🔄 Revision detected, injecting full text: {new_transcription.strip()}", flush=True)
            self.text_injector.inject(new_transcription)
            self._last_injected = new_transcription

    def _process_streaming_chunk(self, audio_chunk: np.ndarray):
        """Process a single streaming chunk with NeMo FrameBatchMultiTaskAED"""
        # Start latency measurement
        measurement = None
        if self.performance_monitor:
            measurement = self.performance_monitor.start_latency_measurement('chunk_processing')

        try:
            if self.frame_asr is None:
                # Fallback to basic transcription if streaming not initialized
                print(f"  ⚠ FrameBatchMultiTaskAED not initialized, using basic transcription", flush=True)
                temp_path = self.temp_dir / f'stream_chunk_{int(time.time()*1000)}.wav'
                sf.write(temp_path, audio_chunk, self.sample_rate)

                hypothesis = self.stt_model.transcribe(
                    audio=str(temp_path),
                    source_lang='en',
                    target_lang='en',
                    pnc='yes',
                    batch_size=1
                )[0]

                text = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
                temp_path.unlink(missing_ok=True)
            else:
                # Use FrameBatchMultiTaskAED for proper streaming with context
                # Append audio chunk to streaming buffer
                self.frame_asr.append_audio(audio_chunk, stream_id=0)

                # Create metadata for transcription (required for Canary)
                meta_data = {
                    'source_lang': 'en',
                    'target_lang': 'en',
                    'pnc': 'yes',
                    'taskname': 'asr'
                }

                # Get input tokens for the model
                self.frame_asr.input_tokens = self.frame_asr.get_input_tokens(meta_data)

                # Transcribe with accumulated context
                self.frame_asr.transcribe(
                    tokens_per_chunk=None,
                    delay=None,
                    keep_logits=False,
                    timestamps=False
                )

                # Get the latest prediction
                if len(self.frame_asr.all_preds) > 0:
                    latest_pred = self.frame_asr.all_preds[-1]
                    text = latest_pred.text if hasattr(latest_pred, 'text') else str(latest_pred)
                else:
                    text = ""

            # Measure STT phase
            if self.performance_monitor and measurement:
                self.performance_monitor.measure_phase(measurement, 'stt')

            # Update last transcription and inject delta
            if text.strip() and text != self._last_transcription:
                self._last_transcription = text
                # Inject only delta using progressive injection
                self._inject_streaming_delta(text)

                # Measure injection phase
                if self.performance_monitor and measurement:
                    self.performance_monitor.measure_phase(measurement, 'injection')

            # Complete latency measurement
            if self.performance_monitor and measurement:
                self.performance_monitor.end_latency_measurement('chunk_processing')

                # Capture metrics periodically
                if hasattr(self, '_chunk_count'):
                    self._chunk_count += 1
                else:
                    self._chunk_count = 1

                # Capture metrics every 10 chunks
                if self._chunk_count % 10 == 0:
                    self.performance_monitor.capture_metrics({
                        'chunks_processed': self._chunk_count
                    })

        except Exception as e:
            print(f"  ⚠ Streaming chunk error: {e}", flush=True)
            import traceback
            traceback.print_exc()

            # End measurement on error
            if self.performance_monitor and measurement:
                self.performance_monitor.end_latency_measurement('chunk_processing')

    def _stop_recording_and_process(self):
        """Stop recording and process audio → STT → inject"""
        try:
            print("\n🛑 Stopping recording...")
            self.set_state(DaemonState.PROCESSING)

            # Clear audio callback if streaming
            if self.streaming_mode:
                self.audio_capture.on_audio_callback = None
                # Wait for any in-flight streaming transcription
                if self._streaming_thread and self._streaming_thread.is_alive():
                    self._streaming_thread.join(timeout=2)

            # Stop capture and get audio
            audio = self.audio_capture.stop()

            if len(audio) == 0:
                print("⚠ No audio captured")
                self.set_state(DaemonState.IDLE)
                return

            duration = len(audio) / self.sample_rate
            print(f"✓ Captured {duration:.2f}s audio")

            # In streaming mode, we've already processed audio, just clean up
            if self.streaming_mode:
                print("✓ Streaming transcription complete")
                # Reset streaming state for next recording
                self._last_injected = ""
                self._last_transcription = ""
                self._streaming_buffer = []
                self._streaming_frames = 0
                self.set_state(DaemonState.IDLE)
                print("✓ Ready for next recording\n", flush=True)
            else:
                # Batch mode: Transcribe in background thread
                thread = threading.Thread(
                    target=self._process_audio,
                    args=(audio,),
                    daemon=True
                )
                thread.start()

        except Exception as e:
            print(f"✗ Failed to stop recording: {e}")
            self.set_state(DaemonState.ERROR)

    def _detect_speech_vad(self, audio: np.ndarray) -> bool:
        """
        Use Silero VAD to detect if audio contains speech.
        Returns True if speech detected, False if silence.
        """
        if self.vad_model is None:
            return True  # No VAD available, assume speech

        try:
            # Convert to torch tensor
            audio_tensor = torch.from_numpy(audio).float()

            if torch.cuda.is_available():
                audio_tensor = audio_tensor.cuda()

            # Get speech timestamps
            speech_timestamps = self.get_speech_timestamps(
                audio_tensor,
                self.vad_model,
                threshold=self.vad_threshold,
                sampling_rate=self.sample_rate,
                min_speech_duration_ms=250,
                min_silence_duration_ms=100
            )

            has_speech = len(speech_timestamps) > 0
            return has_speech

        except Exception as e:
            print(f"    ⚠ VAD error: {e}, assuming speech present")
            return True

    def _chunk_audio(self, audio: np.ndarray):
        """
        Split audio into chunks with overlap for memory optimization.
        Yields (audio_chunk, start_time, end_time) tuples.
        """
        total_duration = len(audio) / self.sample_rate
        chunk_samples = int(self.chunk_duration * self.sample_rate)
        overlap_samples = int(self.chunk_overlap * self.sample_rate)
        stride = chunk_samples - overlap_samples

        start_sample = 0
        chunk_idx = 0

        while start_sample < len(audio):
            end_sample = min(start_sample + chunk_samples, len(audio))
            chunk = audio[start_sample:end_sample]

            start_time = start_sample / self.sample_rate
            end_time = end_sample / self.sample_rate

            yield chunk, start_time, end_time, chunk_idx

            # Move to next chunk
            start_sample += stride
            chunk_idx += 1

            # Break if this was the last chunk
            if end_sample >= len(audio):
                break

    def _process_audio(self, audio: np.ndarray):
        """Process audio: VAD → Chunking → STT → text injection"""
        try:
            duration = len(audio) / self.sample_rate
            print(f"🧠 Processing {duration:.2f}s audio...")

            # Quick VAD check on full audio first
            if self.vad_model is not None:
                print("  Checking for speech...")
                if not self._detect_speech_vad(audio):
                    print("⚠ No speech detected (silence), skipping transcription")
                    self.set_state(DaemonState.IDLE)
                    return

            transcribe_start = time.time()
            transcriptions = []
            chunks_processed = 0
            chunks_skipped = 0

            # Determine if chunking is needed
            if duration > self.chunk_duration:
                print(f"  Long audio ({duration:.1f}s), using chunking...")

                # Process in chunks
                for chunk, start_time, end_time, chunk_idx in self._chunk_audio(audio):
                    chunk_duration = end_time - start_time

                    # VAD check per chunk
                    if self.vad_model is not None:
                        if not self._detect_speech_vad(chunk):
                            print(f"    Chunk {chunk_idx+1}: [{start_time:.1f}s-{end_time:.1f}s] Silence - skipped")
                            chunks_skipped += 1
                            continue

                    print(f"    Chunk {chunk_idx+1}: [{start_time:.1f}s-{end_time:.1f}s] Transcribing...")

                    # Save chunk to temp file
                    temp_chunk_path = self.temp_dir / f'chunk_{int(time.time())}_{chunk_idx}.wav'
                    sf.write(temp_chunk_path, chunk, self.sample_rate)

                    # Transcribe chunk
                    hypothesis = self.stt_model.transcribe(
                        audio=str(temp_chunk_path),
                        source_lang='en',
                        target_lang='en',
                        pnc='yes',
                        batch_size=1
                    )[0]

                    chunk_text = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)

                    if chunk_text.strip():
                        transcriptions.append(chunk_text.strip())
                        print(f"      → \"{chunk_text[:50]}...\"" if len(chunk_text) > 50 else f"      → \"{chunk_text}\"")

                    chunks_processed += 1

                    # Cleanup chunk file
                    temp_chunk_path.unlink(missing_ok=True)

                    # Clear GPU cache between chunks
                    if torch.cuda.is_available():
                        torch.cuda.empty_cache()
                        gc.collect()

                print(f"  Processed {chunks_processed} chunks, skipped {chunks_skipped} silence chunks")

            else:
                # Short audio, process in one shot
                print("  Transcribing (single chunk)...")
                temp_audio_path = self.temp_dir / f'capture_{int(time.time())}.wav'
                sf.write(temp_audio_path, audio, self.sample_rate)

                hypothesis = self.stt_model.transcribe(
                    audio=str(temp_audio_path),
                    source_lang='en',
                    target_lang='en',
                    pnc='yes',
                    batch_size=1
                )[0]

                transcription = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)

                if transcription.strip():
                    transcriptions.append(transcription.strip())

                temp_audio_path.unlink(missing_ok=True)

            # Merge transcriptions
            full_transcription = ' '.join(transcriptions)
            transcribe_time = time.time() - transcribe_start

            print(f"✓ Transcribed in {transcribe_time*1000:.0f}ms")
            print(f"  Text: {full_transcription}")

            # Inject text
            if full_transcription.strip():
                print("⌨️ Injecting text...")
                success = self.text_injector.inject(full_transcription)

                if success:
                    print("✓ Text injected successfully")
                else:
                    print("✗ Text injection failed")
            else:
                print("⚠ Empty transcription, nothing to inject")

            # Return to idle
            self.set_state(DaemonState.IDLE)
            print("✓ Ready for next recording\n")

        except Exception as e:
            print(f"✗ Processing error: {e}")
            import traceback
            traceback.print_exc()
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
                # This is normal - just checking if we should still run
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
        print("=" * 80, flush=True)
        print("Swictation Daemon Starting", flush=True)
        print("=" * 80, flush=True)

        try:
            # Load VAD first (prevents torch.hub.load() from hanging)
            self.load_vad_model()

            # Then load STT model (slow)
            self.load_stt_model()

            # Initialize components
            self.initialize_components()

            # Set running flag FIRST (before IPC server starts the thread)
            self.running = True

            # Start IPC server
            self.start_ipc_server()

            # Start performance monitoring
            if self.performance_monitor:
                print("  Starting performance monitoring...", flush=True)
                self.performance_monitor.start_background_monitoring(interval=5.0)
                print("  ✓ Performance monitoring active", flush=True)

            self.set_state(DaemonState.IDLE)

            print("\n✓ Swictation daemon started", flush=True)
            print("  Ready to receive toggle commands", flush=True)
            if self.performance_monitor:
                print("  Performance monitoring: ENABLED", flush=True)
            print("  Press Ctrl+C to stop\n", flush=True)

            # Main loop (just keeps process alive)
            last_status_report = time.time()
            status_interval = 300  # Print status every 5 minutes

            while self.running:
                time.sleep(1)

                # Periodic status report
                if self.performance_monitor and (time.time() - last_status_report) >= status_interval:
                    self._print_status_report()
                    last_status_report = time.time()

        except KeyboardInterrupt:
            print("\n\nReceived Ctrl+C, shutting down...")
        except Exception as e:
            print(f"\n✗ Daemon error: {e}")
            sys.exit(1)
        finally:
            self.stop()

    def _print_status_report(self):
        """Print periodic status report with performance metrics"""
        print("\n" + "=" * 80, flush=True)
        print("📊 Daemon Status Report", flush=True)
        print("=" * 80, flush=True)

        # State
        print(f"State: {self.get_state().value}", flush=True)

        # GPU stats
        if self.performance_monitor:
            gpu_stats = self.performance_monitor.get_gpu_memory_stats()
            if gpu_stats['available']:
                print(f"\n🎮 GPU:", flush=True)
                print(f"  Memory: {gpu_stats['current_mb']:.1f} MB", flush=True)
                print(f"  Peak: {gpu_stats['peak_mb']:.1f} MB", flush=True)

            # CPU stats
            cpu_stats = self.performance_monitor.get_cpu_stats(window_seconds=60)
            print(f"\n🖥️  CPU (last 60s):", flush=True)
            print(f"  Mean: {cpu_stats['mean']:.1f}%", flush=True)
            print(f"  Max: {cpu_stats['max']:.1f}%", flush=True)

            # Latency stats
            latency_stats = self.performance_monitor.get_latency_stats('chunk_processing')
            if latency_stats:
                print(f"\n⏱️  Chunk Processing Latency:", flush=True)
                print(f"  Mean: {latency_stats['mean']:.1f}ms", flush=True)
                print(f"  P95: {latency_stats['p95']:.1f}ms", flush=True)
                print(f"  Count: {latency_stats['count']}", flush=True)

            # Memory leak check
            leak_result = self.performance_monitor.detect_memory_leak(window_seconds=300)
            if leak_result.get('growth_rate_mb_s') is not None:
                print(f"\n💾 Memory:", flush=True)
                print(f"  Growth rate: {leak_result['growth_rate_mb_s']:.4f} MB/s", flush=True)
                if leak_result['leak_detected']:
                    print(f"  ⚠️  LEAK DETECTED!", flush=True)

        print("=" * 80, flush=True)

    def stop(self):
        """Stop daemon and cleanup"""
        print("\nStopping daemon...")

        self.running = False

        # Stop performance monitoring
        if self.performance_monitor:
            print("  Stopping performance monitoring...")
            self.performance_monitor.stop_background_monitoring()

            # Print final performance summary
            print("\n" + "=" * 80)
            print("📊 Final Performance Summary")
            print("=" * 80)
            self.performance_monitor.print_summary()

        # Stop audio capture if active
        if self.audio_capture and self.audio_capture.is_active():
            self.audio_capture.stop()

        # Close socket
        if self.server_socket:
            self.server_socket.close()

        # Remove socket file
        if os.path.exists(self.socket_path):
            os.remove(self.socket_path)

        # Clean up temp files
        if self.temp_dir.exists():
            import shutil
            for temp_file in self.temp_dir.glob('*.wav'):
                try:
                    temp_file.unlink()
                except Exception as e:
                    print(f"  Warning: Could not delete temp file {temp_file}: {e}")

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
