#!/usr/bin/env bash
# Node is a frontend build dependency only; installed swictation is native.
set -euo pipefail
cd "$(dirname "$0")/.."
npm ci --ignore-scripts
npm run build
case "$(uname -s)" in
  Darwin) npm run tauri build -- --bundles app ;;
  Linux) npm run tauri build -- --no-bundle ;;
  *) echo 'Unsupported UI build host' >&2; exit 1 ;;
esac
binary=src-tauri/target/release/swictation-ui
test -x "$binary"
printf 'UI built: %s/%s\n' "$PWD" "$binary"
printf 'Assemble the CLI, daemon, UI and ONNX libraries with scripts/distribution/package.py.\n'
