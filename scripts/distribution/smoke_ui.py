"""Start the real packaged UI before any daemon/database exists."""
import os
from pathlib import Path
import signal
import subprocess


def check_ui_startup(release, home, env, target):
    if target == "aarch64-apple-darwin":
        binary = release / "share/Swictation.app/Contents/MacOS/swictation-ui"
        database = home / "Library/Application Support/swictation/metrics.db"
        command = [str(binary)]
    else:
        binary = release / "bin/swictation-ui"
        database = Path(env["XDG_DATA_HOME"]) / "swictation/metrics.db"
        command = ["dbus-run-session", "--", "xvfb-run", "-a", str(binary)]
    if database.exists():
        raise RuntimeError("UI startup smoke requires an absent metrics database")
    temporary = home / "tmp"
    temporary.mkdir(mode=0o700)
    ui_env = dict(env, SWICTATION_NO_TRAY="1", RUST_BACKTRACE="1",
                  CFFIXED_USER_HOME=str(home), TMPDIR=str(temporary))
    for key in ("SWICTATION_DB_PATH", "SWICTATION_MODEL_PATH"):
        ui_env.pop(key, None)
    if target == "x86_64-unknown-linux-gnu":
        ui_env.update(GDK_BACKEND="x11", XDG_SESSION_TYPE="x11")
        for key in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET"):
            ui_env.pop(key, None)
    log_path = home / "ui-startup.log"
    with log_path.open("wb") as output:
        process = subprocess.Popen(command, env=ui_env, cwd=home,
                                   stdin=subprocess.DEVNULL, stdout=output,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            try:
                code = process.wait(timeout=8)
                return [f"packaged UI exited before a daemon/database existed (exit {code}):\n"
                        + log_path.read_text(errors="replace")]
            except subprocess.TimeoutExpired:
                pass
            log = log_path.read_text(errors="replace")
            if f'Metrics database path: "{database}"' not in log:
                return ["packaged UI did not initialize with its isolated database path:\n" + log]
            if database.exists():
                return ["UI startup created a placeholder metrics database"]
            print("Packaged UI startup passed: no daemon or metrics database required", flush=True)
            return []
        finally:
            # Terminate only this isolated test's process group, including Xvfb.
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                pass
            # A wrapper can exit before its Xvfb/dbus children. Clean up the
            # whole test-owned group even when the immediate child has exited.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=3)
