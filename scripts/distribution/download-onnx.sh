#!/usr/bin/env bash
set -euo pipefail
if [[ $# != 2 ]]; then
  echo 'usage: download-onnx.sh TARGET DESTINATION' >&2
  exit 1
fi
case "$1" in
  x86_64-unknown-linux-gnu)
    version=1.23.2; distribution=onnxruntime-linux-x64-gpu
    expected_size=240893669
    expected_sha256=2083e361072a79ce16a90dcd5f5cb3ab92574a82a3ce0ac01e5cfa3158176f53 ;;
  aarch64-apple-darwin)
    version=1.22.0; distribution=onnxruntime-osx-arm64
    expected_size=25943843
    expected_sha256=cab6dcbd77e7ec775390e7b73a8939d45fec3379b017c7cb74f5b204c1a1cc07 ;;
  *) echo "unsupported target: $1" >&2; exit 1 ;;
esac
destination=$2
mkdir -p "$destination"
archive="$destination/$distribution-$version.tgz"
curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
  --tlsv1.2 --connect-timeout 15 --max-time 600 --max-filesize "$expected_size" \
  "https://github.com/microsoft/onnxruntime/releases/download/v$version/$distribution-$version.tgz" \
  --output "$archive"
python3 - "$archive" "$expected_size" "$expected_sha256" <<'PY'
import hashlib
from pathlib import Path
import sys

archive = Path(sys.argv[1])
if archive.stat().st_size != int(sys.argv[2]):
    raise SystemExit('ONNX Runtime archive size mismatch')
digest = hashlib.sha256()
with archive.open('rb') as source:
    for chunk in iter(lambda: source.read(1024 * 1024), b''):
        digest.update(chunk)
if digest.hexdigest() != sys.argv[3]:
    raise SystemExit('ONNX Runtime archive SHA-256 mismatch')
PY
tar -xzf "$archive" -C "$destination"
test -d "$destination/$distribution-$version/lib"
printf '%s\n' "$destination/$distribution-$version/lib"
