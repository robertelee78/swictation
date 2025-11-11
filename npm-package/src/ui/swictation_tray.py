#!/usr/bin/env python3
"""Swictation minimal system tray launcher."""

import sys
import os
import socket
import json
import subprocess
from pathlib import Path
from PySide6.QtWidgets import QApplication, QSystemTrayIcon, QMenu
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor, QCursor
from PySide6.QtCore import QObject, Signal, QTimer, QEvent, Qt, Slot


class TrayEventFilter(QObject):
    """Event filter to intercept tray icon right-clicks before Qt's buggy Wayland handler.

    Based on Telegram Desktop's proven solution for reliable context menus on Wayland.
    The key insight: catch the right-click MouseButtonPress BEFORE Qt's activation signal
    fires, then consume the event to prevent Qt's broken Wayland positioning code.
    """

    context_menu_requested = Signal()

    def __init__(self, parent):
        super().__init__(parent)
        self._icon_object_name = "QSystemTrayIconSys"

    def eventFilter(self, obj, event):
        """Catch right-click on tray icon and prevent Qt's default handler."""
        if event.type() == QEvent.MouseButtonPress:
            if obj.objectName() == self._icon_object_name:
                mouse_event = event
                if mouse_event.button() == Qt.RightButton:
                    # Fire signal and consume event (prevent Qt's handler)
                    self.context_menu_requested.emit()
                    return True  # Event handled, don't propagate to Qt's buggy handler
        return False  # Let other events through


class SwictationTrayApp(QApplication):
    """Minimal system tray application for launching Tauri UI."""

    def __init__(self, argv):
        super().__init__(argv)

        # CRITICAL: Prevent app from quitting when last window closes
        # We're a system tray app - we should keep running
        self.setQuitOnLastWindowClosed(False)

        # Setup paths
        self.icon_path = Path(__file__).parent.parent.parent / "docs" / "swictation_logo.png"

        # Track Tauri UI process
        self.tauri_process = None
        # Default to npm package binary location, can be overridden with SWICTATION_UI_BINARY
        self.tauri_ui_binary = os.environ.get('SWICTATION_UI_BINARY',
                                             str(Path(__file__).parent.parent / "bin" / "swictation-ui"))

        # Click debounce timer (prevent single-click from interfering with double-click)
        self.click_timer = QTimer(self)
        self.click_timer.setSingleShot(True)
        self.click_timer.timeout.connect(self.toggle_recording)

        # Track current state for icon changes
        self.current_state = "idle"

        # Create system tray icon
        self.tray_icon = QSystemTrayIcon(self)
        self.tray_icon.setIcon(self._load_icon("idle"))
        self.tray_icon.activated.connect(self.on_tray_activated)

        # Create tray menu
        menu = QMenu()
        menu.addAction("Show Metrics", self.launch_tauri_ui)
        menu.addAction("Toggle Recording", self.toggle_recording)
        menu.addSeparator()
        menu.addAction("Quit", self.quit)
        self.tray_icon.setContextMenu(menu)

        # Install event filter to catch right-clicks reliably on Wayland
        self.event_filter = TrayEventFilter(self)
        self.instance().installEventFilter(self.event_filter)
        self.event_filter.context_menu_requested.connect(self.show_context_menu)

        # Show tray icon
        self.tray_icon.show()

        # Start monitoring socket for state changes
        self.state_timer = QTimer(self)
        self.state_timer.timeout.connect(self.check_daemon_state)
        self.state_timer.start(1000)  # Check every second

        # Also check if Tauri process is still alive (for manual closes)
        self.process_check_timer = QTimer(self)
        self.process_check_timer.timeout.connect(self.check_tauri_process)
        self.process_check_timer.start(1000)  # Check every second

        print("✓ Swictation tray launcher started (minimal)")

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
            # Middle-click: toggle metrics window
            self.toggle_tauri_ui()

        elif reason == QSystemTrayIcon.Context:
            # Right-click: handled by event filter for Wayland reliability
            pass

    @Slot()
    def toggle_recording(self):
        """Send toggle command to daemon."""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.settimeout(2.0)
            sock.connect('/tmp/swictation.sock')
            sock.sendall(json.dumps({'action': 'toggle'}).encode('utf-8'))
            response = sock.recv(1024)
            sock.close()

            # Parse response to update state immediately
            try:
                resp_data = json.loads(response.decode('utf-8'))
                if resp_data.get('success'):
                    new_state = resp_data.get('state', self.current_state)
                    self.update_state(new_state)
            except:
                pass

            print(f"✓ Toggle response: {response.decode('utf-8')}")
        except Exception as e:
            print(f"✗ Toggle failed: {e}")
            self.tray_icon.showMessage(
                "Swictation",
                f"Failed to toggle recording: {e}",
                QSystemTrayIcon.Warning
            )

    @Slot()
    def check_tauri_process(self):
        """Check if Tauri process is still alive (handles manual closes)."""
        if self.tauri_process:
            retcode = self.tauri_process.poll()
            if retcode is not None:
                # Process has terminated
                print(f"Tauri UI process terminated (exit code: {retcode})")
                self.tauri_process = None

    @Slot()
    def check_daemon_state(self):
        """Periodically check daemon state for icon updates."""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.settimeout(0.5)  # Short timeout for status check
            sock.connect('/tmp/swictation.sock')
            sock.sendall(json.dumps({'action': 'status'}).encode('utf-8'))
            response = sock.recv(1024)
            sock.close()

            # Parse response and update state
            try:
                resp_data = json.loads(response.decode('utf-8'))
                new_state = resp_data.get('state', 'idle')
                self.update_state(new_state)
            except:
                pass
        except:
            # Socket connection failed, assume idle
            self.update_state('idle')

    def update_state(self, state: str):
        """Update icon based on state change."""
        if state != self.current_state:
            old_state = self.current_state
            self.current_state = state
            self.tray_icon.setIcon(self._load_icon(state))

            # Show notification for recording state change
            if state == "recording":
                self.tray_icon.showMessage(
                    "Swictation",
                    "Recording started",
                    QSystemTrayIcon.Information,
                    2000
                )
            elif old_state == "recording" and state == "idle":
                self.tray_icon.showMessage(
                    "Swictation",
                    "Recording stopped",
                    QSystemTrayIcon.Information,
                    2000
                )

    @Slot()
    def show_context_menu(self):
        """Show context menu at cursor position (called by event filter)."""
        menu = self.tray_icon.contextMenu()
        if menu:
            menu.popup(QCursor.pos())

    @Slot()
    def toggle_tauri_ui(self):
        """Toggle the Tauri UI - open if closed, close if open."""
        try:
            # Check if Tauri UI is already running
            if self.tauri_process and self.tauri_process.poll() is None:
                # Process is still running - close it
                print(f"Closing Tauri UI (PID: {self.tauri_process.pid})")
                self.tauri_process.terminate()
                self.tauri_process = None
                print("✓ Tauri UI closed")
            else:
                # Launch Tauri UI without tray icon (we're already providing the tray)
                print(f"Launching Tauri UI: {self.tauri_ui_binary}")
                env = os.environ.copy()
                env['SWICTATION_NO_TRAY'] = '1'  # Disable Tauri tray icon
                self.tauri_process = subprocess.Popen(
                    [self.tauri_ui_binary],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    env=env,
                    start_new_session=True  # Don't tie to parent process
                )
                print(f"✓ Tauri UI launched with PID: {self.tauri_process.pid}")
        except Exception as e:
            import traceback
            print(f"✗ Failed to toggle Tauri UI: {e}")
            traceback.print_exc()
            self.tray_icon.showMessage(
                "Swictation",
                f"Failed to toggle UI: {e}",
                QSystemTrayIcon.Warning
            )

    @Slot()
    def launch_tauri_ui(self):
        """Launch the Tauri UI (for menu action - always opens, never closes)."""
        try:
            # Check if Tauri UI is already running
            if self.tauri_process and self.tauri_process.poll() is None:
                # Process is still running
                print("Tauri UI is already running")
            else:
                # Launch Tauri UI without tray icon (we're already providing the tray)
                print(f"Launching Tauri UI: {self.tauri_ui_binary}")
                env = os.environ.copy()
                env['SWICTATION_NO_TRAY'] = '1'  # Disable Tauri tray icon
                self.tauri_process = subprocess.Popen(
                    [self.tauri_ui_binary],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    env=env,
                    start_new_session=True  # Don't tie to parent process
                )
                print(f"✓ Tauri UI launched with PID: {self.tauri_process.pid}")
        except Exception as e:
            import traceback
            print(f"✗ Failed to launch Tauri UI: {e}")
            traceback.print_exc()
            self.tray_icon.showMessage(
                "Swictation",
                f"Failed to launch UI: {e}",
                QSystemTrayIcon.Warning
            )


def main():
    app = SwictationTrayApp(sys.argv)
    sys.exit(app.exec())


if __name__ == '__main__':
    main()