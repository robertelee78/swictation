#!/bin/bash
# Pre-push verification script
# Runs the same checks as GitHub Actions locally to catch errors before pushing

set -e

# Find git root directory
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$REPO_ROOT"

echo "🔍 Running pre-push verification from $REPO_ROOT..."
echo ""

# 1. TypeScript build
echo "📦 Building TypeScript..."
cd "$REPO_ROOT/tauri-ui"
npm run build
echo "✅ TypeScript build passed"
echo ""

# 2. Rust build with strict warnings (same as GitHub Actions)
echo "🦀 Building Rust with RUSTFLAGS=-D warnings..."
cd "$REPO_ROOT/tauri-ui/src-tauri"
export RUSTFLAGS="-D warnings"
cargo build
echo "✅ Rust build passed (zero warnings)"
echo ""

echo "✨ All checks passed! Safe to push."
