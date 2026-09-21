#!/usr/bin/env python3
"""Exercise the packaged native CLI in a private home without configuring services."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--archive", type=Path, required=True)
parser.add_argument("--allow-unsigned-local", action="store_true")
args = parser.parse_args()
archive = args.archive.resolve()
with archive.open("rb") as source:
    digest = hashlib.file_digest(source, "sha256").hexdigest()
if Path(str(archive) + ".sha256").read_text() != f"{digest}  {archive.name}\n":
    parser.error("archive checksum sidecar mismatch")
with tempfile.TemporaryDirectory(prefix="swictation-release-smoke-") as temporary:
    root = Path(temporary).resolve()
    test_home = root / "home"
    test_home.mkdir(mode=0o700)
    runtime = root / "runtime"
    runtime.mkdir(mode=0o700)
    guards = root / "service-command-guards"
    guards.mkdir()
    service_attempts = root / "service-command-attempts"
    for command in ("systemctl", "launchctl"):
        guard = guards / command
        guard.write_text('#!/bin/sh\nprintf "%s\\n" "$0 $*" >> "$SWICTATION_SMOKE_SERVICE_ATTEMPTS"\nexit 90\n')
        guard.chmod(0o700)
    candidate = root / "swictation"
    with tarfile.open(archive) as bundle:
        member = bundle.getmember("bin/swictation")
        if not member.isfile():
            parser.error("packaged CLI is not a regular file")
        candidate.write_bytes(bundle.extractfile(member).read())
        metadata = json.load(bundle.extractfile("release.json"))
    candidate.chmod(0o700)
    env = dict(os.environ, HOME=str(test_home), PATH=f"{guards}:{os.environ.get('PATH', '')}",
               SWICTATION_SMOKE_SERVICE_ATTEMPTS=str(service_attempts), XDG_DATA_HOME=str(test_home / ".local/share"),
               XDG_CONFIG_HOME=str(test_home / ".config"), XDG_CACHE_HOME=str(test_home / ".cache"),
               XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS="unix:path=" + str(runtime / "absent-bus"),
               XDG_STATE_HOME=str(test_home / ".local/state"), SWICTATION_DISABLE_TRAY="1")
    for key in ("SWICTATION_CONFIG", "SWICTATION_CONFIG_DIR", "SWICTATION_DATA_DIR", "SWICTATION_SOCKET_PATH"):
        env.pop(key, None)
    def run(binary, *arguments):
        return subprocess.check_output([str(binary), *arguments], env=env, stdin=subprocess.DEVNULL, text=True)
    install_args = ["__install", "--archive", str(archive), "--sha256", digest]
    if args.allow_unsigned_local:
        install_args.append("--allow-unsigned-local")
    print(run(candidate, *install_args), end="")
    launcher = test_home / ".local/bin/swictation"
    expected = f"swictation {metadata['version']}"
    if run(launcher, "--version").strip() != expected:
        raise SystemExit("installed command does not report the packaged version")
    print(run(launcher, "uninstall", "--yes"), end="")
    if launcher.exists() or launcher.is_symlink():
        raise SystemExit("uninstall left the managed launcher behind")
    if service_attempts.exists():
        raise SystemExit("isolated install/uninstall unexpectedly attempted service management: " + service_attempts.read_text())
    print(f"Packaged install/version/uninstall smoke passed: {expected}")
