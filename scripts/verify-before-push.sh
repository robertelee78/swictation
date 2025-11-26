#!/bin/bash
# Pre-push verification script
# Runs the same checks as GitHub Actions locally to catch errors before pushing

set -e

echo "🔍 Running pre-push verification..."
echo ""

# 1. TypeScript build
echo "📦 Building TypeScript..."
cd tauri-ui
npm run build
echo "✅ TypeScript build passed"
echo ""

# 2. Rust build with strict warnings (same as GitHub Actions)
echo "🦀 Building Rust with RUSTFLAGS=-D warnings..."
cd src-tauri
export RUSTFLAGS="-D warnings"
cargo build
echo "✅ Rust build passed (zero warnings)"
echo ""

echo "✨ All checks passed! Safe to push."
