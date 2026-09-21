#!/usr/bin/env python3
"""Exercise packaged installation and daemon selection and UI startup without services or inference."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
from smoke_ui import check_ui_startup


def check_model_selection(daemon, test_home, env, target):
    """Empty markers prove selection/file checks only; they are not usable weights."""
    if target == "aarch64-apple-darwin":
        config_dir = test_home / "Library/Application Support/swictation"
        positive = ("auto-coreml-only", "auto", "coreml", True, "CoreML")
    elif target == "x86_64-unknown-linux-gnu":
        config_dir = Path(env["XDG_CONFIG_HOME"]) / "swictation"
        positive = ("auto-cpu-onnx", "auto", "onnx", True, "CPU")
    else:
        raise SystemExit(f"Unsupported packaged daemon smoke target: {target}")
    config_dir.mkdir(parents=True, exist_ok=True)
    cases = [positive,
             ("explicit-onnx-missing", "0.6b-cpu", "coreml", False, "CPU"),
             ("auto-all-missing", "auto", None, False, None)]
    if target == "x86_64-unknown-linux-gnu":
        cases.append(("auto-low-free-vram", "auto", "onnx", True, "CPU"))
    failures = []
    for name, override, marker, succeeds, backend in cases:
        fixture = test_home / "model-selection-fixtures" / name
        paths = {"stt_0_6b_model_path": fixture / "onnx-0.6b",
                 "stt_1_1b_model_path": fixture / "onnx-1.1b",
                 "stt_coreml_model_path": fixture / "coreml",
                 "vad_model_path": fixture / "vad/silero_vad.onnx",
                 "socket_path": fixture / "unused.sock"}
        fixture.mkdir(parents=True)
        if marker == "coreml":
            (paths["stt_coreml_model_path"] / "encoder.mlmodelc").mkdir(parents=True)
        elif marker == "onnx":
            paths["stt_0_6b_model_path"].mkdir()
            (paths["stt_0_6b_model_path"] / "encoder.onnx").touch()
        config = {**{key: str(value) for key, value in paths.items()},
                  "stt_model_override": override}
        (config_dir / "config.toml").write_text(
            "".join(f"{key} = {json.dumps(value)}\n" for key, value in config.items()))
        case_env = dict(env)
        case_env.pop("SWICTATION_SMOKE_VRAM", None)
        if name == "auto-low-free-vram":
            case_env["SWICTATION_SMOKE_VRAM"] = "8192, 2048"
        result = subprocess.run([str(daemon), "--dry-run"], env=case_env, cwd=fixture,
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, timeout=45)
        output = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", result.stdout)
        problems = []
        if (result.returncode == 0) != succeeds:
            problems.append(f"expected {'success' if succeeds else 'failure'}, got exit {result.returncode}")
        if backend is not None:
            if not re.search(rf"Would load:[^\n]*\b{backend}\b", output):
                problems.append(f"selected model diagnostic does not name {backend}")
            selected_path = paths["stt_coreml_model_path" if backend == "CoreML" else "stt_0_6b_model_path"]
            if f"Path: {selected_path}" not in output:
                problems.append("selected model diagnostic does not report the fixture path")
        expected_message = "Dry-run complete" if succeeds else "Model files not found"
        if expected_message.lower() not in output.lower():
            problems.append(f"missing diagnostic: {expected_message}")
        if problems:
            failures.append(f"{name}: {'; '.join(problems)}\n{output}")
        else:
            print(f"Packaged daemon selection/file-presence check passed: {name}", flush=True)
    return failures


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
    # Keep Linux auto selection deterministic even on a GPU-equipped test host.
    nvidia_guard = guards / "nvidia-smi"
    nvidia_guard.write_text('#!/bin/sh\n[ -n "$SWICTATION_SMOKE_VRAM" ] || exit 1\nprintf "%s\\n" "$SWICTATION_SMOKE_VRAM"\n')
    nvidia_guard.chmod(0o700)
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
               XDG_STATE_HOME=str(test_home / ".local/state"), SWICTATION_DISABLE_TRAY="1",
               RUST_LOG="info", NO_COLOR="1")
    for key in ("SWICTATION_CONFIG", "SWICTATION_CONFIG_DIR", "SWICTATION_DATA_DIR",
                "SWICTATION_SOCKET_PATH", "SWICTATION_MODEL_PATH"):
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
    try:
        daemon = launcher.resolve(strict=True).parent / "swictation-daemon"
        failures = check_model_selection(daemon, test_home, env, metadata["target"])
        failures.extend(check_ui_startup(daemon.parent.parent, test_home, env, metadata["target"]))
    finally:
        print(run(launcher, "uninstall", "--yes"), end="")
    if launcher.exists() or launcher.is_symlink():
        raise SystemExit("uninstall left the managed launcher behind")
    if service_attempts.exists():
        raise SystemExit("isolated install/uninstall unexpectedly attempted service management: " + service_attempts.read_text())
    if failures:
        raise SystemExit("Packaged lifecycle smoke failed:\n" + "\n".join(failures))
    print(f"Packaged install/version/daemon-selection/empty-profile UI/uninstall smoke passed: {expected}")
