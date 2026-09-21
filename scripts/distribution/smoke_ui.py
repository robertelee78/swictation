"""Start the real packaged UI before any daemon/database exists."""
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import subprocess
import time


def _stop_macos_app(process):
    """Stop the direct app process without signaling OS-managed helpers."""
    try:
        process.terminate()
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        try:
            process.kill()
        except ProcessLookupError:
            pass
        process.wait(timeout=3)


def _stop_process_group(process):
    """Give every test-owned child time to clean up, even if its wrapper exits."""
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        process.wait(timeout=3)
        return
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        process.poll()  # Reap the wrapper without mistaking its exit for cleanup.
        try:
            os.killpg(process.pid, 0)
        except ProcessLookupError:
            break
        time.sleep(0.05)
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    process.wait(timeout=3)


def _portal_mounted(runtime):
    """Read mount metadata without stat'ing a disconnected FUSE mount."""
    mounted = False
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        fields = line.split()
        mountpoint = re.sub(r"\\([0-7]{3})", lambda match: chr(int(match[1], 8)), fields[4])
        if mountpoint != str(runtime / "doc"):
            continue
        filesystem = fields[fields.index("-") + 1]
        if filesystem != "fuse.portal":
            raise RuntimeError(f"Refusing to unmount unexpected {filesystem} at {mountpoint}")
        mounted = True
    return mounted


def _cleanup_portal_mount(runtime):
    """Unmount only a portal under the private runtime this helper created."""
    metadata = runtime.lstat()
    if (not runtime.is_absolute() or runtime.resolve(strict=True) != runtime
            or not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != os.getuid()
            or metadata.st_mode & 0o077):
        raise RuntimeError(f"Refusing portal cleanup outside a private runtime: {runtime}")
    if not _portal_mounted(runtime):
        return
    unmount = shutil.which("fusermount3") or shutil.which("fusermount")
    if not unmount:
        raise RuntimeError("Private portal mount remains but fusermount3/fusermount is unavailable")
    diagnostic = ""
    for options in (("-u",), ("-u", "-z")):
        # Recheck the exact mount before each operation, including lazy detach
        # of a busy/disconnected portal after its process group has stopped.
        if not _portal_mounted(runtime):
            return
        try:
            result = subprocess.run([unmount, *options, "--", str(runtime / "doc")],
                                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, timeout=5)
            diagnostic = f"exit {result.returncode}: {result.stdout.strip()}"
        except subprocess.TimeoutExpired:
            diagnostic = "unmount timed out"
        if not _portal_mounted(runtime):
            return
    raise RuntimeError(f"Private portal mount remains at {runtime / 'doc'}: {diagnostic}")


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
    runtime = None
    ui_env = dict(env, SWICTATION_NO_TRAY="1", RUST_BACKTRACE="1",
                  CFFIXED_USER_HOME=str(home), TMPDIR=str(temporary))
    for key in ("SWICTATION_DB_PATH", "SWICTATION_MODEL_PATH"):
        ui_env.pop(key, None)
    if target == "x86_64-unknown-linux-gnu":
        # Never clean a caller's runtime mount. This exclusive directory is
        # owned by this UI smoke, including any D-Bus-activated document portal.
        runtime = temporary / "ui-runtime"
        runtime.mkdir(mode=0o700)
        ui_env.update(GDK_BACKEND="x11", XDG_SESSION_TYPE="x11",
                      XDG_RUNTIME_DIR=str(runtime))
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
            try:
                if target == "aarch64-apple-darwin":
                    _stop_macos_app(process)
                else:
                    _stop_process_group(process)
            finally:
                if runtime is not None:
                    _cleanup_portal_mount(runtime)
