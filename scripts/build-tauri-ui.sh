#!/bin/bash
set -e

echo "🔨 Building Tauri UI with custom-protocol feature..."

cd "$(dirname "$0")/../tauri-ui"

# Build frontend
echo "📦 Building React frontend..."
npm run build

# Build Tauri backend with custom-protocol feature (REQUIRED for release builds)
echo "🦀 Building Rust backend with custom-protocol..."
cd src-tauri
cargo build --release --features custom-protocol

echo "✅ Build complete!"
echo "Binary: $(pwd)/target/release/swictation-ui"
