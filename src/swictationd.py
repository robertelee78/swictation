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

# Apply NeMo patches BEFORE importing NeMo
# This fixes known bugs in the NeMo library
from nemo_patches import apply_all_patches
apply_all_patches()

# Import our modules
from audio_capture import AudioCapture
from text_injection import TextInjector, InjectionMethod
from memory_manager import MemoryManager, MemoryPressureLevel
from config_loader import ConfigLoader
from daemon.metrics_broadcaster import MetricsBroadcaster

# Text transformation (PyO3 native module)
try:
    import midstreamer_transform
    TRANSFORMER_AVAILABLE = True
except ImportError:
    TRANSFORMER_AVAILABLE = False
    print("⚠️  midstreamer_transform not installed - transformations disabled", flush=True)
    print("    Install with: pip install /opt/swictation/external/midstream/target/wheels/midstreamer_transform-*.whl", flush=True)

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
        silence_duration: float = 2.0,  # Configurable silence threshold
        streaming_mode: bool = True,  # VAD-triggered segmentation (auto-transcribe on silence)
        streaming_chunk_size: float = 0.4,
        enable_performance_monitoring: bool = True,
        metrics_config = None  # MetricsConfig object from config_loader
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
            silence_duration: Silence duration in seconds before processing text (configurable)
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
        self.silence_duration = silence_duration
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

        # Metrics collection
        self.metrics_collector = None
        if enable_performance_monitoring and metrics_config and metrics_config.enabled:
            try:
                from metrics.collector import MetricsCollector
                self.metrics_collector = MetricsCollector(
                    db_path=metrics_config.database_path,
                    typing_baseline_wpm=metrics_config.typing_baseline_wpm,
                    store_transcription_text=metrics_config.store_transcription_text,
                    warnings_enabled=metrics_config.warnings.enabled,
                    high_latency_threshold_ms=metrics_config.warnings.high_latency_threshold_ms,
                    gpu_memory_threshold_percent=metrics_config.warnings.gpu_memory_threshold_percent,
                    degradation_multiplier=metrics_config.warnings.degradation_multiplier,
                    accuracy_spike_multiplier=metrics_config.warnings.accuracy_spike_multiplier
                )
            except ImportError as e:
                print(f"⚠️  Metrics collection not available: {e}")
                self.metrics_collector = None

        # Metrics broadcaster (real-time UI updates)
        self.metrics_broadcaster = MetricsBroadcaster()

        # Memory pressure management
        self.memory_manager = None
        if torch.cuda.is_available():
            # Define memory pressure callbacks
            def memory_warning(status):
                print(f"⚠️  Memory Warning: {status.usage_percent*100:.1f}% usage", flush=True)

            def memory_critical(status):
                print(f"🔴 Memory Critical: {status.usage_percent*100:.1f}% usage - Aggressive cleanup!", flush=True)

            def memory_emergency(status):
                print(f"🚨 Memory Emergency: {status.usage_percent*100:.1f}% usage - Offloading models!", flush=True)

            def memory_normal(status):
                print(f"✓ Memory Normal: {status.usage_percent*100:.1f}% usage", flush=True)

            memory_callbacks = {
                'warning': memory_warning,
                'critical': memory_critical,
                'emergency': memory_emergency,
                'normal': memory_normal,
                'emergency_shutdown': lambda: self._emergency_memory_shutdown()
            }

            self.memory_manager = MemoryManager(
                check_interval=2.0,
                warning_threshold=0.80,
                critical_threshold=0.90,
                emergency_threshold=0.95,
                callbacks=memory_callbacks
            )

        # IPC socket
        self.server_socket: Optional[socket.socket] = None
        self.socket_thread: Optional[threading.Thread] = None

        # Temp directory for audio files
        self.temp_dir = Path(tempfile.gettempdir()) / 'swictation'
        self.temp_dir.mkdir(exist_ok=True)

        # Text transformation state
        self.transformer_available = TRANSFORMER_AVAILABLE
        self.transform_stats = {
            'total': 0,
            'changed': 0,
            'errors': 0,
            'latency_sum_us': 0.0,  # Microseconds for precision
            'max_latency_us': 0.0
        }

        # Test transformer if available
        if self.transformer_available:
            try:
                test = midstreamer_transform.transform("test comma")
                count, msg = midstreamer_transform.get_stats()
                print(f"  ✅ Text Transform: {msg}", flush=True)
            except Exception as e:
                print(f"  ⚠️  Text Transform error: {e}", flush=True)
                self.transformer_available = False

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

            # Try GPU with fallback to CPU on memory pressure
            if torch.cuda.is_available():
                try:
                    vad_model = vad_model.cuda()

                    # Register VAD model with memory manager
                    if self.memory_manager:
                        self.memory_manager.register_model('vad_model', vad_model)

                except RuntimeError as e:
                    if "out of memory" in str(e).lower():
                        print(f"  ⚠️  GPU OOM loading VAD, using CPU instead", flush=True)
                        vad_model = vad_model.cpu()
                    else:
                        raise

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
            # Use local models directory instead of HuggingFace cache
            # This avoids /tmp space issues and gives us full control
            models_dir = Path('/opt/swictation/models')
            models_dir.mkdir(exist_ok=True)

            # Set HuggingFace cache to local directory
            os.environ['HF_HOME'] = str(models_dir / 'huggingface')
            os.environ['TRANSFORMERS_CACHE'] = str(models_dir / 'huggingface')

            print(f"  Using models directory: {models_dir}", flush=True)

            self.stt_model = EncDecMultiTaskModel.from_pretrained(self.model_name)
            self.stt_model.eval()

            # Enable FP16 mixed precision for 50% VRAM reduction
            # This converts model weights from FP32 (32-bit) to FP16 (16-bit)
            # Expected: 3.6GB → 1.8GB with <0.5% accuracy loss
            print("  Converting model to FP16 mixed precision...", flush=True)
            self.stt_model = self.stt_model.half()
            print("  ✓ FP16 conversion complete (50% VRAM reduction)", flush=True)

            # CUDA error recovery: Try GPU first, fallback to CPU on error
            if torch.cuda.is_available():
                try:
                    self.stt_model = self.stt_model.cuda()
                    print(f"  Using GPU: {torch.cuda.get_device_name(0)}", flush=True)

                    # Register model with memory manager
                    if self.memory_manager:
                        self.memory_manager.register_model('stt_model', self.stt_model)

                except RuntimeError as e:
                    if "out of memory" in str(e).lower():
                        print(f"  ⚠️  GPU OOM during model load, falling back to CPU", flush=True)
                        # Clear CUDA cache and retry on CPU
                        torch.cuda.empty_cache()
                        gc.collect()
                        self.stt_model = self.stt_model.cpu()
                    else:
                        raise
            else:
                print("  Using CPU (slower)", flush=True)

            load_time = time.time() - load_start
            gpu_mem = torch.cuda.memory_allocated() / 1e6 if torch.cuda.is_available() else 0
            model_dtype = next(self.stt_model.parameters()).dtype

            print(f"✓ STT model loaded in {load_time:.2f}s", flush=True)
            print(f"  GPU Memory: {gpu_mem:.1f} MB", flush=True)
            print(f"  Model Precision: {model_dtype} (FP16 = torch.float16)", flush=True)

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
                    total_buffer=20.0,    # 20-second left context window (increased from 10s)
                                          # This is the "memory" - how much past audio to remember
                                          # Larger = better accuracy (more context for coherence)
                                          # 20s buffer uses ~400MB VRAM (safe with FP16's 2.2GB headroom)
                                          # Can increase to 30s (~600MB) for maximum accuracy if needed
                    batch_size=1,         # Real-time single-user processing
                                          # For multi-stream server: increase to 4-8
                )

                print(f"  ✓ NeMo streaming configured (Wait-k policy, 1s chunks, 20s context)", flush=True)

        except Exception as e:
            print(f"✗ Failed to load STT model: {e}")
            raise

    def initialize_components(self):
        """Initialize audio capture and text injection"""
        print("Initializing components...", flush=True)

        # Audio capture with streaming for VAD-triggered segmentation
        try:
            self.audio_capture = AudioCapture(
                sample_rate=self.sample_rate,
                buffer_duration=30.0,  # 30s max recording per segment
                streaming_mode=self.streaming_mode  # Enable callbacks for VAD
            )
            mode_str = "streaming (VAD-triggered)" if self.streaming_mode else "batch"
            print(f"✓ Audio capture initialized ({mode_str})")
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

            # Broadcast state change to UI clients
            if hasattr(self, 'metrics_broadcaster'):
                self.metrics_broadcaster.broadcast_state_change(new_state.value)

    def get_state(self) -> DaemonState:
        """Thread-safe state read"""
        with self.state_lock:
            return self.state

    def _safe_transform(self, text: str) -> str:
        """
        Safe text transformation with error handling and fallback.

        Transforms voice commands to symbols using MidStream PyO3 bindings.
        Also handles special keyboard actions (backspace, delete, enter, etc.)
        Performance: ~0.25µs average (tested with 10,000 iterations)

        Args:
            text: Input text with voice commands (e.g., "Hello comma world")

        Returns:
            Transformed text with symbols (e.g., "Hello, world")
            Falls back to original text on error.

        Performance metrics tracked in self.transform_stats.
        """
        if not self.transformer_available:
            return text  # Passthrough if unavailable

        if not text or not text.strip():
            return text  # Skip empty strings

        try:
            start = time.perf_counter()
            result = midstreamer_transform.transform(text)
            elapsed_us = (time.perf_counter() - start) * 1_000_000  # Microseconds

            # Update statistics
            self.transform_stats['total'] += 1
            self.transform_stats['latency_sum_us'] += elapsed_us
            self.transform_stats['max_latency_us'] = max(
                self.transform_stats['max_latency_us'],
                elapsed_us
            )

            if result != text:
                self.transform_stats['changed'] += 1

            # Warn if unexpectedly slow (>1000µs = 1ms)
            if elapsed_us > 1000:
                print(f"  ⚠️  Slow transform: {elapsed_us:.1f}µs", flush=True)

            return result

        except Exception as e:
            self.transform_stats['errors'] += 1
            print(f"  ⚠️  Transform error: {e}, using original text", flush=True)
            return text  # Fallback to original

    def _inject_text_with_keys(self, transformed_text: str) -> bool:
        """
        Inject text that may contain special key markers like <KEY:BackSpace>.

        Handles mixed text and keyboard actions intelligently:
        - Plain text → inject normally with wtype
        - <KEY:...> markers → execute as key presses

        Args:
            transformed_text: Text with potential <KEY:...> markers

        Returns:
            True if successful, False otherwise

        Examples:
            "Hello, world" → types "Hello, world"
            "Hello<KEY:BackSpace>" → types "Hello" then presses backspace
            "<KEY:Return>" → presses enter key
        """
        import re

        # Pattern to match <KEY:KeyName>
        key_pattern = re.compile(r'<KEY:([^>]+)>')

        # Split text into parts (text and key markers)
        parts = []
        last_end = 0

        for match in key_pattern.finditer(transformed_text):
            # Add text before the key marker
            if match.start() > last_end:
                text_part = transformed_text[last_end:match.start()]
                if text_part:
                    parts.append(('text', text_part))

            # Add the key marker
            key_name = match.group(1)
            parts.append(('key', key_name))
            last_end = match.end()

        # Add remaining text after last key marker
        if last_end < len(transformed_text):
            remaining = transformed_text[last_end:]
            if remaining:
                parts.append(('text', remaining))

        # If no special keys, just inject normally
        if not parts or all(part_type == 'text' for part_type, _ in parts):
            return self.text_injector.inject(transformed_text)

        # Execute mixed text and key actions
        try:
            for part_type, content in parts:
                if part_type == 'text':
                    if not self.text_injector.inject(content):
                        return False
                else:  # part_type == 'key'
                    if not self.text_injector.inject_with_keys([content]):
                        print(f"  ⚠️  Key injection failed: {content}", flush=True)
                        return False
            return True
        except Exception as e:
            print(f"  ⚠️  Mixed injection error: {e}", flush=True)
            return False

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

            # Start metrics session
            if self.metrics_collector:
                try:
                    self.metrics_collector.start_session()
                    session_id = self.metrics_collector.current_session.session_id

                    # Broadcast session start to UI clients
                    self.metrics_broadcaster.start_session(session_id)
                except Exception as e:
                    print(f"⚠️  Metrics start error: {e}", flush=True)

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

                # VAD-triggered segmentation state
                self._silence_duration = 0            # Track silence duration in seconds
                self._speech_detected = False         # Whether speech was detected in current segment

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
        """
        Callback for real-time audio chunks during streaming mode.
        Uses VAD to detect natural pauses (2s silence) and transcribe complete segments.
        """
        # 1. Add frames to continuous buffer
        self._streaming_buffer.extend(audio)
        self._streaming_frames += frames

        # 2. Check if we have enough frames for VAD (512ms window for reliable detection)
        vad_window_frames = int(0.512 * self.sample_rate)  # 512ms VAD window

        if len(self._streaming_buffer) >= vad_window_frames:
            # 3. Extract last 512ms for VAD check
            vad_chunk = np.array(self._streaming_buffer[-vad_window_frames:])

            # 4. Run VAD to detect speech/silence
            has_speech = self._detect_speech_vad(vad_chunk)

            # 5. Track silence duration
            if has_speech:
                self._silence_duration = 0
                self._speech_detected = True
            else:
                self._silence_duration += frames / self.sample_rate

            # 6. Trigger transcription on silence AFTER speech was detected
            min_segment_duration = 1.0  # Don't transcribe segments < 1s (hardcoded for quality)
            # silence_duration is now configurable via config.toml

            if (self._speech_detected and
                self._silence_duration >= self.silence_duration and
                len(self._streaming_buffer) >= int(min_segment_duration * self.sample_rate)):

                # 7. Transcribe accumulated segment (full context = perfect accuracy)
                segment = np.array(self._streaming_buffer)

                # Process in background thread
                if self._streaming_thread is None or not self._streaming_thread.is_alive():
                    self._streaming_thread = threading.Thread(
                        target=self._process_vad_segment,
                        args=(segment.copy(),),
                        daemon=True
                    )
                    self._streaming_thread.start()

                # 8. Clear buffer for next segment
                self._streaming_buffer = []
                self._silence_duration = 0
                self._speech_detected = False

    def _process_vad_segment(self, segment: np.ndarray):
        """
        Transcribe VAD-detected speech segment with full context.
        Each segment is independent (speaker paused), providing perfect accuracy.
        """
        # Timing for metrics
        segment_start = time.time()
        stt_latency_ms = 0.0
        transform_latency_us = 0.0
        injection_latency_ms = 0.0

        try:
            duration = len(segment) / self.sample_rate
            print(f"  🎤 VAD segment: {duration:.2f}s", flush=True)

            # Save segment to temp file
            audio_save_start = time.time()
            temp_path = Path(tempfile.mktemp(suffix='.wav'))
            sf.write(temp_path, segment, self.sample_rate)
            audio_save_ms = (time.time() - audio_save_start) * 1000

            # Transcribe with CUDA error recovery
            stt_start = time.time()
            try:
                # Transcribe with FULL segment context (not chunks!)
                hypothesis = self.stt_model.transcribe(
                    [str(temp_path)],
                    batch_size=1,
                    source_lang='en',
                    target_lang='en',
                    pnc='no'  # Disable auto-punctuation - let transformer handle it
                )[0]

                # Reset error count on success
                if self.memory_manager:
                    self.memory_manager.reset_error_count()

            except RuntimeError as e:
                # Handle CUDA errors
                if "CUDA" in str(e) or "out of memory" in str(e).lower():
                    print(f"  ⚠️  CUDA error during transcription: {e}", flush=True)

                    # Try recovery with memory manager
                    if self.memory_manager and not self.memory_manager.handle_cuda_error(e):
                        # Fallback to CPU
                        print(f"  → Falling back to CPU transcription", flush=True)
                        self.stt_model = self.stt_model.cpu()

                    # Retry transcription (will use CPU if offloaded)
                    hypothesis = self.stt_model.transcribe(
                        [str(temp_path)],
                        batch_size=1,
                        source_lang='en',
                        target_lang='en',
                        pnc='no'  # Disable auto-punctuation - let transformer handle it
                    )[0]
                else:
                    raise

            stt_latency_ms = (time.time() - stt_start) * 1000
            text = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
            temp_path.unlink(missing_ok=True)

            # Transform voice commands to symbols (already tracked in _safe_transform)
            transform_start = time.perf_counter()
            transformed = self._safe_transform(text)
            transform_latency_us = (time.perf_counter() - transform_start) * 1_000_000

            # Log if transformation changed text
            if transformed != text and self.transformer_available:
                print(f"  ⚡ {text} → {transformed}", flush=True)

            # Inject transformed text (handles both text and special keys)
            injection_start = time.time()
            if transformed.strip():
                if not self.transformer_available:
                    print(f"  📝 {transformed}", flush=True)

                # Only add trailing space if text doesn't end with a key action
                # Key actions should not have spaces after them
                import re
                if re.search(r'<KEY:[^>]+>$', transformed):
                    # Ends with a key action - don't add space
                    self._inject_text_with_keys(transformed)
                else:
                    # Regular text or mixed - add space between segments
                    self._inject_text_with_keys(transformed + ' ')
            injection_latency_ms = (time.time() - injection_start) * 1000

            # Record segment metrics
            if self.metrics_collector:
                try:
                    # Get current GPU/CPU usage
                    gpu_memory_mb = 0.0
                    cpu_percent = 0.0
                    if self.performance_monitor:
                        gpu_stats = self.performance_monitor.get_gpu_memory_stats()
                        gpu_memory_mb = gpu_stats.get('current_mb', 0.0)
                        cpu_stats = self.performance_monitor.get_cpu_stats(window_seconds=1.0)
                        cpu_percent = cpu_stats.get('mean', 0.0)

                    segment = self.metrics_collector.record_segment(
                        audio_duration_s=duration,
                        transcription=transformed,
                        stt_latency_ms=stt_latency_ms,
                        transform_latency_us=transform_latency_us,
                        injection_latency_ms=injection_latency_ms,
                        gpu_memory_mb=gpu_memory_mb,
                        cpu_percent=cpu_percent
                    )

                    # Broadcast transcription and metrics to UI clients
                    self.metrics_broadcaster.add_transcription(
                        text=transformed,
                        wpm=segment.calculate_wpm(),
                        latency_ms=segment.total_latency_ms,
                        words=segment.words
                    )

                    # Broadcast updated metrics
                    realtime = self.metrics_collector.get_realtime_metrics()
                    self.metrics_broadcaster.update_metrics(realtime)

                except Exception as e:
                    print(f"⚠️  Metrics recording error: {e}", flush=True)

        except Exception as e:
            print(f"  ⚠ VAD segment error: {e}", flush=True)
            import traceback
            traceback.print_exc()

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
                    pnc='no',  # Disable auto-punctuation - let transformer handle it
                    batch_size=1
                )[0]

                text = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
                temp_path.unlink(missing_ok=True)
            else:
                # FrameBatchMultiTaskAED streaming not fully working yet
                # Fall back to simple transcription for now
                # TODO: Implement proper FrameBatchMultiTaskAED integration
                # See test_nemo_streaming.py for reference

                # Save audio chunk to temp file
                temp_path = Path(tempfile.mktemp(suffix='.wav'))
                sf.write(temp_path, audio_chunk, 16000)

                # Use basic transcription with Canary metadata
                hypothesis = self.stt_model.transcribe(
                    [str(temp_path)],
                    batch_size=1,
                    source_lang='en',
                    target_lang='en',
                    pnc='no'  # Disable auto-punctuation - let transformer handle it
                )[0]

                text = hypothesis.text if hasattr(hypothesis, 'text') else str(hypothesis)
                temp_path.unlink(missing_ok=True)

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

            # End metrics session
            if self.metrics_collector:
                try:
                    session_id = self.metrics_collector.current_session.session_id
                    self.metrics_collector.end_session()

                    # Broadcast session end to UI clients
                    self.metrics_broadcaster.end_session(session_id)
                except Exception as e:
                    print(f"⚠️  Metrics end error: {e}", flush=True)

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

            # Use GPU if model is on GPU and CUDA available
            if torch.cuda.is_available() and next(self.vad_model.parameters()).is_cuda:
                try:
                    audio_tensor = audio_tensor.cuda()
                except RuntimeError as e:
                    # Handle CUDA errors with memory manager
                    if self.memory_manager and not self.memory_manager.handle_cuda_error(e):
                        # Fallback to CPU
                        print(f"    ⚠️  VAD: Falling back to CPU mode", flush=True)
                        self.vad_model = self.vad_model.cpu()
                        audio_tensor = audio_tensor.cpu()

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

        except RuntimeError as e:
            # Handle CUDA errors
            if "CUDA" in str(e) or "out of memory" in str(e).lower():
                if self.memory_manager:
                    self.memory_manager.handle_cuda_error(e)
                print(f"    ⚠ VAD CUDA error: {e}, assuming speech present")
            else:
                print(f"    ⚠ VAD error: {e}, assuming speech present")
            return True
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
                        pnc='no',  # Disable auto-punctuation - let transformer handle it
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
                    pnc='no',  # Disable auto-punctuation - let transformer handle it
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
                print("⌨️  Processing text...")

                # Transform voice commands to symbols
                transformed = self._safe_transform(full_transcription)

                # Log transformation
                if transformed != full_transcription and self.transformer_available:
                    print(f"  ⚡ Transformed text ({len(full_transcription)} → {len(transformed)} chars)")

                # Inject transformed text (handles both text and special keys)
                success = self._inject_text_with_keys(transformed)

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

            # Start memory pressure monitoring
            if self.memory_manager:
                print("  Starting memory pressure monitoring...", flush=True)
                self.memory_manager.start_monitoring()
                print("  ✓ Memory pressure monitoring active", flush=True)

            # Start metrics broadcaster
            print("  Starting metrics broadcaster...", flush=True)
            self.metrics_broadcaster.start()

            self.set_state(DaemonState.IDLE)

            print("\n✓ Swictation daemon started", flush=True)
            print("  Ready to receive toggle commands", flush=True)
            if self.performance_monitor:
                print("  Performance monitoring: ENABLED", flush=True)
            if self.memory_manager:
                print("  Memory protection: ENABLED", flush=True)
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

        # Stop memory monitoring first
        if self.memory_manager:
            print("  Stopping memory monitoring...")
            self.memory_manager.stop_monitoring()

        # Stop metrics broadcaster
        if self.metrics_broadcaster:
            print("  Stopping metrics broadcaster...")
            self.metrics_broadcaster.stop()

        # Stop performance monitoring
        if self.performance_monitor:
            print("  Stopping performance monitoring...")
            self.performance_monitor.stop_background_monitoring()

            # Print final performance summary
            print("\n" + "=" * 80)
            print("📊 Final Performance Summary")
            print("=" * 80)
            self.performance_monitor.print_summary()

        # Print transformation statistics
        if self.transform_stats['total'] > 0:
            total = self.transform_stats['total']
            changed = self.transform_stats['changed']
            errors = self.transform_stats['errors']
            avg_us = self.transform_stats['latency_sum_us'] / total
            max_us = self.transform_stats['max_latency_us']

            print("\n📊 Text Transformation Statistics:", flush=True)
            print(f"  Total transformations: {total}", flush=True)
            print(f"  Text changed: {changed} ({changed/total*100:.1f}%)", flush=True)
            print(f"  Errors: {errors}", flush=True)
            print(f"  Avg latency: {avg_us:.2f}µs", flush=True)
            print(f"  Max latency: {max_us:.2f}µs", flush=True)
            print(f"  Performance: {'✅ Excellent' if avg_us < 100 else '⚠️  Acceptable' if avg_us < 1000 else '❌ Slow'}", flush=True)

        # Print memory status
        if self.memory_manager:
            print("\n" + self.memory_manager.get_status_report())

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

    def _emergency_memory_shutdown(self):
        """Emergency shutdown triggered by memory manager"""
        print("\n🚨 Emergency memory shutdown triggered!")
        print("  Stopping all operations...")

        # Stop recording if active
        if self.get_state() == DaemonState.RECORDING:
            try:
                self.audio_capture.stop()
            except:
                pass

        # Set error state
        self.set_state(DaemonState.ERROR)

        # Stop daemon
        self.running = False


def main():
    """Main entry point"""
    # Load configuration
    config_loader = ConfigLoader()
    config = config_loader.load()

    print(f"⚙️  Configuration loaded:", flush=True)
    print(f"   VAD threshold: {config.vad.threshold}", flush=True)
    print(f"   Silence duration: {config.vad.silence_duration}s", flush=True)
    if config.metrics.enabled:
        print(f"   Metrics: enabled (db: {config.metrics.database_path})", flush=True)
    else:
        print(f"   Metrics: disabled", flush=True)
    print(f"   Config file: {config_loader.config_path}", flush=True)
    print(flush=True)

    # Setup signal handlers
    daemon = SwictationDaemon(
        vad_threshold=config.vad.threshold,
        silence_duration=config.vad.silence_duration,
        metrics_config=config.metrics
    )

    def signal_handler(signum, frame):
        print(f"\nReceived signal {signum}")
        daemon.running = False

    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)

    # Start daemon
    daemon.start()


if __name__ == '__main__':
    main()
