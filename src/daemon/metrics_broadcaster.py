"""
Real-time metrics broadcaster for Swictation UI.

Broadcasts live metrics and ephemeral transcriptions via Unix socket:
- Multiple concurrent client connections
- Session-based transcription buffer (RAM only, cleared on new session)
- JSON event protocol with newline delimiters
- Thread-safe client management
"""

import os
import socket
import threading
import json
import time
from datetime import datetime
from typing import List, Optional
from pathlib import Path


class MetricsBroadcaster:
    """
    Broadcast real-time metrics to connected UI clients.

    Event Protocol (newline-delimited JSON):
    - session_start: Recording started (clears transcription buffer)
    - session_end: Recording stopped (keeps transcriptions visible)
    - transcription: New transcription segment (ephemeral, RAM only)
    - metrics_update: Real-time metrics from RealtimeMetrics
    - state_change: Daemon state changed
    """

    def __init__(self, socket_path='/tmp/swictation_metrics.sock'):
        """
        Initialize metrics broadcaster.

        Args:
            socket_path: Unix socket path for client connections
        """
        self.socket_path = socket_path
        self.clients: List[socket.socket] = []
        self.clients_lock = threading.Lock()
        self.transcription_buffer: List[dict] = []  # Session-based, RAM only
        self.server_socket: Optional[socket.socket] = None
        self.running = False
        self.accept_thread: Optional[threading.Thread] = None

    def start(self):
        """Start broadcaster server thread."""
        # Remove existing socket
        if os.path.exists(self.socket_path):
            os.remove(self.socket_path)

        # Create Unix socket server
        self.server_socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server_socket.bind(self.socket_path)
        self.server_socket.listen(5)
        os.chmod(self.socket_path, 0o666)

        self.running = True
        self.accept_thread = threading.Thread(target=self._accept_clients, daemon=True)
        self.accept_thread.start()

        print(f"  ✓ Metrics broadcast active at {self.socket_path}", flush=True)

    def start_session(self, session_id: int):
        """
        Start new session - clear transcription buffer.

        Args:
            session_id: New session ID
        """
        # Clear transcription buffer for new session
        self.transcription_buffer = []

        # Broadcast session start event
        self.broadcast('session_start', {
            'session_id': session_id,
            'timestamp': time.time()
        })

    def end_session(self, session_id: int):
        """
        End session - keep buffer visible.

        Args:
            session_id: Ending session ID
        """
        # Don't clear buffer - UI keeps it visible
        self.broadcast('session_end', {
            'session_id': session_id,
            'timestamp': time.time()
        })

    def add_transcription(self, text: str, wpm: float, latency_ms: float, words: int):
        """
        Add transcription segment to RAM buffer and broadcast.

        Args:
            text: Transcribed text
            wpm: Words per minute for this segment
            latency_ms: Total latency in milliseconds
            words: Word count
        """
        timestamp = datetime.now().strftime('%H:%M:%S')

        segment = {
            'text': text,
            'timestamp': timestamp,
            'wpm': wpm,
            'latency_ms': latency_ms,
            'words': words
        }

        # Store in RAM buffer
        self.transcription_buffer.append(segment)

        # Broadcast to all clients
        self.broadcast('transcription', segment)

    def update_metrics(self, realtime):
        """
        Broadcast real-time metrics from MetricsCollector.

        Args:
            realtime: RealtimeMetrics object
        """
        self.broadcast('metrics_update', {
            'state': realtime.current_state.value if hasattr(realtime.current_state, 'value') else str(realtime.current_state),
            'session_id': realtime.current_session_id,
            'segments': realtime.segments_this_session,
            'words': realtime.words_this_session,
            'wpm': realtime.wpm_this_session,
            'duration_s': realtime.recording_duration_s,
            'latency_ms': realtime.last_segment_latency_ms,
            'gpu_memory_mb': realtime.gpu_memory_current_mb,
            'gpu_memory_percent': realtime.gpu_memory_percent,
            'cpu_percent': realtime.cpu_percent_current
        })

    def broadcast_state_change(self, state: str):
        """
        Broadcast daemon state change.

        Args:
            state: New daemon state (idle/recording/processing/error)
        """
        self.broadcast('state_change', {
            'state': state,
            'timestamp': time.time()
        })

    def broadcast(self, event_type: str, data: dict):
        """
        Broadcast event to all connected clients.

        Args:
            event_type: Event type (session_start, transcription, etc.)
            data: Event data dictionary
        """
        event = {
            'type': event_type,
            **data
        }
        message = json.dumps(event) + '\n'

        with self.clients_lock:
            dead_clients = []
            for client in self.clients:
                try:
                    client.sendall(message.encode('utf-8'))
                except (BrokenPipeError, OSError, ConnectionResetError):
                    dead_clients.append(client)

            # Remove disconnected clients
            for client in dead_clients:
                self.clients.remove(client)
                try:
                    client.close()
                except:
                    pass

    def _accept_clients(self):
        """Accept client connections in background thread."""
        while self.running:
            try:
                self.server_socket.settimeout(1.0)
                client, _ = self.server_socket.accept()
                with self.clients_lock:
                    self.clients.append(client)
                print(f"  ✓ Metrics client connected (total: {len(self.clients)})", flush=True)

                # Send current transcription buffer to new client
                if self.transcription_buffer:
                    for segment in self.transcription_buffer:
                        try:
                            event = {'type': 'transcription', **segment}
                            message = json.dumps(event) + '\n'
                            client.sendall(message.encode('utf-8'))
                        except:
                            pass

            except socket.timeout:
                continue
            except Exception as e:
                if self.running:
                    print(f"  ⚠️  Client accept error: {e}", flush=True)

    def stop(self):
        """Stop broadcaster and cleanup."""
        self.running = False

        # Close all client connections
        with self.clients_lock:
            for client in self.clients:
                try:
                    client.close()
                except:
                    pass
            self.clients = []

        # Close server socket
        if self.server_socket:
            try:
                self.server_socket.close()
            except:
                pass

        # Remove socket file
        if os.path.exists(self.socket_path):
            try:
                os.remove(self.socket_path)
            except:
                pass

        # Wait for accept thread
        if self.accept_thread and self.accept_thread.is_alive():
            self.accept_thread.join(timeout=2.0)
