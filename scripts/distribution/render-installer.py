#!/usr/bin/env python3
"""Build-time renderer; the installed bootstrap needs only standard shell tools."""
import argparse
import hashlib
import os
from pathlib import Path
import re
from package import ARCHIVE_LIMIT, TARGETS, VERSION

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--version", required=True)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--distribution-dir", type=Path, required=True)
args = parser.parse_args()
if not re.fullmatch(VERSION, args.version):
    parser.error("invalid release version")
team = os.environ.get("APPLE_TEAM_ID", "")
if not re.fullmatch(r"[A-Z0-9]{10}", team):
    parser.error("APPLE_TEAM_ID must pin the ten-character release signing team")
template = Path(__file__).with_name("install.sh.in").read_text()
rendered = template.replace("@SWICTATION_RELEASE_VERSION@", args.version)
rendered = rendered.replace("@SWICTATION_APPLE_TEAM_ID@", team)
for target, prefix in zip(TARGETS, ("LINUX", "MACOS")):
    archive = args.distribution_dir / f"swictation-{args.version}-{target}.tar.gz"
    size = archive.stat().st_size
    if not 0 < size <= ARCHIVE_LIMIT:
        parser.error(f"invalid release archive size for {target}")
    with archive.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if Path(str(archive) + ".sha256").read_text() != f"{digest}  {archive.name}\n":
        parser.error(f"checksum sidecar mismatch for {target}")
    rendered = rendered.replace(f"@SWICTATION_{prefix}_SHA256@", digest)
    rendered = rendered.replace(f"@SWICTATION_{prefix}_SIZE@", str(size))
if "@SWICTATION_" in rendered:
    parser.error("unresolved installer template value")
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(rendered)
args.output.chmod(0o755)
