#!/usr/bin/env python3
"""Swictation system tray application."""

import sys
import socket
import json
import threading
from pathlib import Path
from PySide6.QtWidgets import QApplication, QSystemTrayIcon, QMenu
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor
from PySide6.QtCore import QObject, Signal, QUrl, Slot, Property, QTimer
from PySide6.QtQml import QQmlApplicationEngine

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))
from metrics.database import MetricsDatabase


class MetricsBackend(QObject):
    """
    Dual data source backend:
    - Socket: Real-time metrics from /tmp/swictation_metrics.sock
    - Database: Historical data from ~/.local/share/swictation/metrics.db
    """

    # Qt signals for QML bindings
    stateChanged = Signal(str)
    wpmChanged = Signal(float)
    wordsChanged = Signal(int)
    latencyChanged = Signal(float)
    segmentsChanged = Signal(int)
    durationChanged = Signal(str)
    gpuMemoryChanged = Signal(float)
    cpuPercentChanged = Signal(float)
    connectedChanged = Signal(bool)  # Socket connection status

    # Transcription signals
    transcriptionAdded = Signal(str, str, float, float)  # text, timestamp, wpm, latency
    sessionCleared = Signal()

    # Session list signals
    recentSessionsChanged = Signal()
    lifetimeStatsChanged = Signal()

    def __init__(self):
        super().__init__()

        # Database access for historical data
        self.db = MetricsDatabase()

        # Current state (from socket)
        self._state = "idle"
        self._wpm = 0.0
        self._words = 0
        self._latency_ms = 0.0
        self._segments = 0
        self._duration = "00:00"
        self._gpu_memory_mb = 0.0
        self._cpu_percent = 0.0
        self._connected = False

        # Socket connection
        self.socket = None
        self.socket_thread = threading.Thread(target=self._socket_listener, daemon=True)
        self.socket_thread.start()

    # Qt properties for QML
    @Property(str, notify=stateChanged)
    def state(self):
        return self._state

    @state.setter
    def state(self, value):
        if self._state != value:
            self._state = value
            self.stateChanged.emit(value)

    @Property(float, notify=wpmChanged)
    def wpm(self):
        return self._wpm

    @wpm.setter
    def wpm(self, value):
        if self._wpm != value:
            self._wpm = value
            self.wpmChanged.emit(value)

    @Property(int, notify=wordsChanged)
    def words(self):
        return self._words

    @words.setter
    def words(self, value):
        if self._words != value:
            self._words = value
            self.wordsChanged.emit(value)

    @Property(float, notify=latencyChanged)
    def latency_ms(self):
        return self._latency_ms

    @latency_ms.setter
    def latency_ms(self, value):
        if self._latency_ms != value:
            self._latency_ms = value
            self.latencyChanged.emit(value)

    @Property(int, notify=segmentsChanged)
    def segments(self):
        return self._segments

    @segments.setter
    def segments(self, value):
        if self._segments != value:
            self._segments = value
            self.segmentsChanged.emit(value)

    @Property(str, notify=durationChanged)
    def duration(self):
        return self._duration

    @duration.setter
    def duration(self, value):
        if self._duration != value:
            self._duration = value
            self.durationChanged.emit(value)

    @Property(float, notify=gpuMemoryChanged)
    def gpu_memory_mb(self):
        return self._gpu_memory_mb

    @gpu_memory_mb.setter
    def gpu_memory_mb(self, value):
        if self._gpu_memory_mb != value:
            self._gpu_memory_mb = value
            self.gpuMemoryChanged.emit(value)

    @Property(float, notify=cpuPercentChanged)
    def cpu_percent(self):
        return self._cpu_percent

    @cpu_percent.setter
    def cpu_percent(self, value):
        if self._cpu_percent != value:
            self._cpu_percent = value
            self.cpuPercentChanged.emit(value)

    @Property(bool, notify=connectedChanged)
    def connected(self):
        return self._connected

    @connected.setter
    def connected(self, value):
        if self._connected != value:
            self._connected = value
            self.connectedChanged.emit(value)

    def _socket_listener(self):
        """Background thread: connect to metrics socket and process events."""
        while True:
            try:
                # Connect to metrics socket
                self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.socket.connect('/tmp/swictation_metrics.sock')
                print("✓ Connected to metrics socket")
                self.connected = True

                # Read events line by line
                buffer = ""
                while True:
                    data = self.socket.recv(4096).decode('utf-8')
                    if not data:
                        break

                    buffer += data
                    while '\n' in buffer:
                        line, buffer = buffer.split('\n', 1)
                        if line.strip():
                            self._handle_event(json.loads(line))

            except Exception as e:
                print(f"⚠️  Metrics socket error: {e}")
                self.connected = False
            finally:
                if self.socket:
                    self.socket.close()
                    self.socket = None
                self.connected = False

            # Reconnect after 5 seconds
            import time
            time.sleep(5)

    def _handle_event(self, event: dict):
        """Handle incoming socket event."""
        event_type = event.get('type')
        print(f"[MetricsBackend] Received event: {event_type}")

        if event_type == 'session_start':
            self.sessionCleared.emit()

        elif event_type == 'session_end':
            # Refresh history with newly completed session
            self.recentSessionsChanged.emit()

        elif event_type == 'transcription':
            self.transcriptionAdded.emit(
                event['text'],
                event['timestamp'],
                event['wpm'],
                event['latency_ms']
            )

        elif event_type == 'metrics_update':
            self.state = event['state']
            self.wpm = event['wpm']
            self.words = event['words']
            self.latency_ms = event['latency_ms']
            self.segments = event['segments']

            # Format duration
            duration_s = event['duration_s']
            minutes = int(duration_s // 60)
            seconds = int(duration_s % 60)
            self.duration = f"{minutes:02d}:{seconds:02d}"

            self.gpu_memory_mb = event['gpu_memory_mb']
            self.cpu_percent = event['cpu_percent']

        elif event_type == 'state_change':
            self.state = event['state']

    @Slot(result='QVariantList')
    def loadHistory(self):
        """Load recent sessions from database."""
        sessions = self.db.get_recent_sessions(limit=10)
        # Convert to QML-friendly format (list of dicts)
        return [dict(s) for s in sessions]

    @Slot(result='QVariantMap')
    def loadLifetimeStats(self):
        """Load lifetime statistics from database."""
        stats = self.db.get_lifetime_stats()
        return dict(stats)


class SwictationTrayApp(QApplication):
    """Main system tray application."""

    def __init__(self, argv):
        super().__init__(argv)

        # Setup paths
        self.icon_path = Path(__file__).parent.parent.parent / "docs" / "swictation_logo.png"

        # Click debounce timer (prevent single-click from interfering with double-click)
        self.click_timer = QTimer(self)
        self.click_timer.setSingleShot(True)
        self.click_timer.timeout.connect(self.toggle_recording)

        # Create system tray icon
        self.tray_icon = QSystemTrayIcon(self)
        self.tray_icon.setIcon(self._load_icon("idle"))
        self.tray_icon.activated.connect(self.on_tray_activated)

        # Create tray menu
        menu = QMenu()
        menu.addAction("Show Metrics", self.show_window)
        menu.addAction("Toggle Recording", self.toggle_recording)
        menu.addSeparator()
        menu.addAction("Quit", self.quit)
        self.tray_icon.setContextMenu(menu)

        # Show tray icon (always visible)
        self.tray_icon.show()

        # Create metrics backend
        self.backend = MetricsBackend()
        self.backend.stateChanged.connect(self.on_state_changed)

        # Load QML window (hidden by default)
        self.engine = QQmlApplicationEngine()
        self.engine.rootContext().setContextProperty("backend", self.backend)

        qml_file = Path(__file__).parent / "MetricsUI.qml"
        self.engine.load(QUrl.fromLocalFile(str(qml_file)))

        if not self.engine.rootObjects():
            print("✗ Failed to load QML")
            sys.exit(1)

        self.window = self.engine.rootObjects()[0]
        self.window.hide()

        print("✓ Swictation tray app started")

    def _load_icon(self, state: str) -> QIcon:
        """Load icon with optional recording overlay."""
        # Load base icon
        pixmap = QPixmap(str(self.icon_path))

        if state == "recording":
            # Create red overlay using QPainter
            painter = QPainter(pixmap)
            painter.setCompositionMode(QPainter.CompositionMode_SourceAtop)
            painter.fillRect(pixmap.rect(), QColor(255, 0, 0, 80))  # Red with 30% opacity
            painter.end()

        return QIcon(pixmap)

    @Slot(int)
    def on_tray_activated(self, reason):
        """Handle tray icon clicks - left=toggle, middle=UI, right=menu."""
        if reason == QSystemTrayIcon.Trigger:
            # Left-click: start debounce timer to toggle recording
            interval = QApplication.doubleClickInterval()
            self.click_timer.start(interval)

        elif reason == QSystemTrayIcon.MiddleClick:
            # Middle-click: show/hide metrics window
            self.toggle_window()

        # Right-click (Context) is handled automatically by Qt to show context menu

    @Slot()
    def toggle_recording(self):
        """Send toggle command to daemon."""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.settimeout(5.0)
            sock.connect('/tmp/swictation.sock')
            sock.sendall(json.dumps({'action': 'toggle'}).encode('utf-8'))
            response = sock.recv(1024)
            sock.close()
            print(f"✓ Toggle response: {response.decode('utf-8')}")
        except Exception as e:
            print(f"✗ Toggle failed: {e}")
            self.tray_icon.showMessage(
                "Swictation",
                f"Failed to toggle recording: {e}",
                QSystemTrayIcon.Warning
            )

    @Slot()
    def toggle_window(self):
        """Show/hide metrics window with proper focus handling."""
        if self.window.isVisible():
            self.window.hide()
        else:
            self.window.show()
            self.window.raise_()
            self.window.requestActivate()  # Request focus (important for Wayland)

    @Slot()
    def show_window(self):
        """Show metrics window."""
        self.window.show()
        self.window.raise_()

    @Slot(str)
    def on_state_changed(self, state):
        """Update tray icon when state changes."""
        self.tray_icon.setIcon(self._load_icon(state))

        # Show notification
        if state == "recording":
            self.tray_icon.showMessage(
                "Swictation",
                "Recording started",
                QSystemTrayIcon.Information,
                2000
            )


def main():
    app = SwictationTrayApp(sys.argv)
    sys.exit(app.exec())


if __name__ == '__main__':
    main()
