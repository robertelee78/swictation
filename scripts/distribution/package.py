#!/usr/bin/env python3
"""Stage, archive, and validate standalone releases (Python is build-time only)."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import plistlib
import re
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin")
REQUIRED = {"bin/swictation", "bin/swictation-daemon", "bin/swictation-ui",
            "share/config.example.toml", "share/models.manifest.json",
            "share/swictation_tray.py", "share/swictation_logo.png", "release.json"}
VERSION = r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
ARCHIVE_LIMIT = 2 * 1024**3
EXPANDED_LIMIT = 4 * 1024**3
ENTRY_LIMIT = 20000


def checked_metadata(value):
    if set(value) != {"schema_version", "version", "target", "source_sha"}:
        raise ValueError("release metadata must have exactly the supported schema fields")
    if value.get("schema_version") != 1 or value.get("target") not in TARGETS:
        raise ValueError("unsupported release schema or target")
    if not re.fullmatch(VERSION, value.get("version", "")):
        raise ValueError("invalid release version")
    if not re.fullmatch(r"[0-9a-f]{40}", value.get("source_sha", "")):
        raise ValueError("source_sha must identify the exact source commit")
    return value


def stage(args):
    version_output = subprocess.check_output([str(args.cli.resolve()), "--version"], text=True).strip()
    if not version_output.startswith("swictation "):
        raise ValueError("release CLI did not identify itself as swictation")
    metadata = checked_metadata({"schema_version": 1, "version": version_output[11:],
                                 "target": args.target, "source_sha": args.source_sha})
    if args.stage_dir.exists():
        raise ValueError("stage directory already exists; use a fresh directory")
    # Validate all required inputs before creating the staging tree.
    inputs = {"bin/swictation": args.cli, "bin/swictation-daemon": args.daemon,
              "bin/swictation-ui": args.ui, "share/config.example.toml": args.config,
              "share/models.manifest.json": args.models_manifest,
              "share/swictation_tray.py": args.tray,
              "share/swictation_logo.png": args.logo}
    for source in inputs.values():
        if not source.is_file():
            raise ValueError(f"missing required release input: {source}")
    json.loads(args.models_manifest.read_text())
    pattern = "*.dylib" if args.target == TARGETS[1] else "*.so*"
    libraries = sorted(args.lib_dir.glob(pattern))
    if not any(path.name.startswith("libonnxruntime.") for path in libraries):
        raise ValueError("ONNX Runtime library is missing")
    if args.target == TARGETS[0]:
        names = {path.name for path in libraries}
        if not {"libonnxruntime_providers_cuda.so", "libonnxruntime_providers_shared.so"} <= names:
            raise ValueError("Linux release must retain the ONNX CUDA/shared providers")
    for library in libraries:
        if not library.is_file():
            raise ValueError(f"missing library symlink target: {library}")
        inputs[f"lib/{library.name}"] = library
    for name, source in inputs.items():
        destination = args.stage_dir / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination, follow_symlinks=True)
        destination.chmod(0o755 if name.startswith("bin/") else 0o644)
    if args.target == TARGETS[1]:
        if not args.app or not (args.app / "Contents/MacOS/swictation-ui").is_file():
            raise ValueError("expected a macOS Swictation.app bundle")
        shutil.copytree(args.app, args.stage_dir / "share/Swictation.app", symlinks=False)
        daemon_app = args.stage_dir / "share/SwictationDaemon.app/Contents"
        (daemon_app / "MacOS").mkdir(parents=True)
        shutil.copy2(args.stage_dir / "bin/swictation-daemon", daemon_app / "MacOS/swictation-daemon")
        info = {"CFBundleName": "SwictationDaemon", "CFBundleDisplayName": "Swictation",
                "CFBundleIdentifier": "com.swictation.daemon", "CFBundleExecutable": "swictation-daemon",
                "CFBundlePackageType": "APPL", "CFBundleVersion": metadata["version"],
                "CFBundleShortVersionString": metadata["version"], "LSMinimumSystemVersion": "14.0",
                "LSUIElement": True, "NSHighResolutionCapable": True,
                "NSMicrophoneUsageDescription": "Swictation uses the microphone to transcribe your speech locally."}
        (daemon_app / "Info.plist").write_bytes(plistlib.dumps(info))
    elif args.app:
        raise ValueError("app bundles are supported only for the macOS target")
    (args.stage_dir / "release.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, sort_keys=True))


def archive(args):
    metadata = checked_metadata(json.loads((args.stage_dir / "release.json").read_text()))
    args.output_dir.mkdir(parents=True, exist_ok=True)
    name = f"swictation-{metadata['version']}-{metadata['target']}.tar.gz"
    destination = args.output_dir / name
    if destination.exists() or Path(str(destination) + ".sha256").exists():
        raise ValueError("refusing to replace an existing release archive")
    with destination.open("xb") as raw, gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as bundle:
            for source in sorted(args.stage_dir.rglob("*")):
                if source.is_symlink() or not (source.is_file() or source.is_dir()):
                    raise ValueError(f"release payload contains a link or special file: {source}")
                info = bundle.gettarinfo(str(source), arcname=source.relative_to(args.stage_dir).as_posix())
                info.uid = info.gid = info.mtime = 0
                info.uname = info.gname = ""
                info.pax_headers = {}
                info.mode = 0o755 if source.is_dir() or source.stat().st_mode & 0o111 else 0o644
                if source.is_file():
                    with source.open("rb") as content:
                        bundle.addfile(info, content)
                else:
                    bundle.addfile(info)
    with destination.open("rb") as source:
        checksum = hashlib.file_digest(source, "sha256").hexdigest()
    Path(str(destination) + ".sha256").write_text(f"{checksum}  {name}\n")
    verify_payload(destination, metadata)
    print(destination)


def verify_payload(path, expected):
    expected_name = f"swictation-{expected['version']}-{expected['target']}.tar.gz"
    if path.name != expected_name:
        raise ValueError("archive filename differs from release identity")
    if path.stat().st_size > ARCHIVE_LIMIT:
        raise ValueError("compressed archive exceeds the native installer limit")
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if Path(str(path) + ".sha256").read_text() != f"{digest}  {path.name}\n":
        raise ValueError("archive checksum differs from its sidecar")
    with tarfile.open(path, "r:gz") as bundle:
        seen = set()
        regular = set()
        total_bytes = 0
        for member in bundle:
            item = PurePosixPath(member.name)
            if (item.is_absolute() or ".." in item.parts or not item.parts
                    or item.as_posix() != member.name.rstrip("/") or "\\" in member.name
                    or item.parts[0] not in {"bin", "lib", "share", "release.json"}):
                raise ValueError(f"unsafe archive path: {member.name}")
            if member.name in seen or not (member.isfile() or member.isdir()):
                raise ValueError(f"duplicate, linked, or special member: {member.name}")
            if member.mode & 0o7000:
                raise ValueError(f"unsafe archive permissions: {member.name}")
            seen.add(member.name)
            total_bytes += member.size
            if len(seen) > ENTRY_LIMIT or total_bytes > EXPANDED_LIMIT:
                raise ValueError("archive exceeds release limits")
            if member.isfile():
                regular.add(member.name)
            if member.name.startswith("bin/") and (not member.isfile() or not member.mode & 0o111):
                raise ValueError(f"release command is not executable: {member.name}")
        if not REQUIRED <= regular or not any(name.startswith("lib/libonnxruntime.") for name in regular):
            raise ValueError(f"release payload is incomplete: {sorted(REQUIRED - regular)}")
        metadata = checked_metadata(json.load(bundle.extractfile("release.json")))
        if metadata != expected:
            raise ValueError("release metadata differs from expected source/version/target")
        json.load(bundle.extractfile("share/models.manifest.json"))
        if expected["target"] == TARGETS[1]:
            for app_name, identifier, executable in [("Swictation.app", "com.swictation.ui", "swictation-ui"),
                                                      ("SwictationDaemon.app", "com.swictation.daemon", "swictation-daemon")]:
                contents = f"share/{app_name}/Contents"
                if not {f"{contents}/Info.plist", f"{contents}/MacOS/{executable}"} <= regular:
                    raise ValueError(f"macOS release is missing the complete {app_name} bundle")
                info = plistlib.load(bundle.extractfile(f"{contents}/Info.plist"))
                if info.get("CFBundleIdentifier") != identifier or info.get("CFBundleExecutable") != executable:
                    raise ValueError(f"incorrect identity for {app_name}")
                if app_name == "SwictationDaemon.app" and not info.get("NSMicrophoneUsageDescription"):
                    raise ValueError("daemon app requires a microphone usage description")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    staging = commands.add_parser("stage")
    staging.add_argument("--target", choices=TARGETS, required=True)
    staging.add_argument("--source-sha", required=True)
    for name in ("cli", "daemon", "ui", "lib-dir", "stage-dir"):
        staging.add_argument(f"--{name}", type=Path, required=True)
    staging.add_argument("--config", type=Path, default=ROOT / "config/config.example.toml")
    staging.add_argument("--models-manifest", type=Path, default=ROOT / "config/models.manifest.json")
    staging.add_argument("--tray", type=Path, default=ROOT / "scripts/ui/swictation_tray.py")
    staging.add_argument("--logo", type=Path, default=ROOT / "scripts/ui/swictation_logo.png")
    staging.add_argument("--app", type=Path)
    staging.set_defaults(run=stage)
    packaging = commands.add_parser("archive")
    packaging.add_argument("--stage-dir", type=Path, required=True)
    packaging.add_argument("--output-dir", type=Path, required=True)
    packaging.set_defaults(run=archive)
    verifying = commands.add_parser("verify")
    verifying.add_argument("--archive", type=Path, required=True)
    verifying.add_argument("--version", required=True)
    verifying.add_argument("--target", choices=TARGETS, required=True)
    verifying.add_argument("--source-sha", required=True)
    verifying.set_defaults(run=lambda args: verify_payload(args.archive, checked_metadata({
        "schema_version": 1, "version": args.version, "target": args.target,
        "source_sha": args.source_sha})))
    args = parser.parse_args()
    try:
        args.run(args)
    except (ValueError, OSError, tarfile.TarError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"release packaging: {error}\n")


if __name__ == "__main__":
    main()
