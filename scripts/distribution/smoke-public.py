#!/usr/bin/env python3
"""Test the public native bootstrap and update path in an isolated user profile."""
import argparse
import os
from pathlib import Path
import platform
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--version", required=True, help="Expected current stable version")
args = parser.parse_args()

with tempfile.TemporaryDirectory(prefix="swictation-public-proof-") as temporary:
    root = Path(temporary).resolve()
    home = root / "home"
    home.mkdir(mode=0o700)
    runtime = root / "runtime"
    runtime.mkdir(mode=0o700)
    guards = root / "guards"
    guards.mkdir(mode=0o700)
    attempts = root / "service-attempts"
    for name in ("launchctl", "systemctl"):
        script = guards / name
        script.write_text('#!/bin/sh\nprintf attempted >> "$SWICTATION_PROOF_ATTEMPTS"\nexit 91\n')
        script.chmod(0o700)
    if platform.system() == "Darwin":
        data = home / "Library/Application Support/swictation"
        config_directory = data
    elif platform.system() == "Linux":
        data = home / ".local/share/swictation"
        config_directory = home / ".config/swictation"
    else:
        parser.error("Public proof supports Linux and macOS only")
    config_directory.mkdir(parents=True, mode=0o700)
    config = config_directory / "config.toml"
    config.write_text("num_threads = 7\n")
    models = data / "models"
    models.mkdir(parents=True, mode=0o700)
    sentinel = models / "preserved-test-file"
    sentinel.write_text("keep this user data\n")
    env = dict(os.environ, HOME=str(home),
        PATH=f"{home}/.local/bin:{guards}:{os.environ['PATH']}",
        SWICTATION_PROOF_ATTEMPTS=str(attempts),
        XDG_DATA_HOME=str(home / ".local/share"), XDG_CONFIG_HOME=str(home / ".config"),
        XDG_CACHE_HOME=str(home / ".cache"), XDG_STATE_HOME=str(home / ".local/state"),
        XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS=f"unix:path={runtime}/absent-bus",
        SWICTATION_DISABLE_TRAY="1")
    for name in ("SWICTATION_CONFIG", "SWICTATION_CONFIG_DIR", "SWICTATION_DATA_DIR", "SWICTATION_SOCKET_PATH"):
        env.pop(name, None)
    installer = root / "install.sh"
    subprocess.run(["curl", "-fLsS", "https://github.com/robertelee78/swictation/releases/latest/download/install.sh",
                    "-o", str(installer)], check=True)
    subprocess.run(["sh", str(installer)], env=env, stdin=subprocess.DEVNULL, check=True)
    cli = home / ".local/bin/swictation"

    def run(*arguments):
        print("PUBLIC TEST:", "swictation", *arguments, flush=True)
        output = subprocess.check_output([str(cli), *arguments], env=env,
                                         stdin=subprocess.DEVNULL, text=True)
        print(output, end="", flush=True)
        return output

    expected = f"swictation {args.version}"
    if run("--version").strip() != expected:
        raise SystemExit("Public bootstrap installed an unexpected version")
    run("setup", "--list")
    run("update", "--check")
    run("update", "--force")
    if run("--version").strip() != expected:
        raise SystemExit("Public update installed an unexpected version")
    run("uninstall", "--yes")
    if cli.exists() or cli.is_symlink():
        raise SystemExit("Uninstall left the native launcher behind")
    if config.read_text() != "num_threads = 7\n" or sentinel.read_text() != "keep this user data\n":
        raise SystemExit("Public lifecycle changed existing user data")
    if attempts.exists():
        raise SystemExit("Public lifecycle unexpectedly attempted service management")
    print("PASS: public bootstrap, installed CLI, update check, same-version update, uninstall and user data preservation")
