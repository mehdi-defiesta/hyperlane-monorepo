#!/bin/bash

# Build script for Hyperlane CLI Tool

echo "🔨 Building Hyperlane CLI Tool..."

# Navigate to the rust workspace root
cd "$(dirname "$0")/../.."

# Build the CLI tool
echo "📦 Compiling with cargo..."
cargo build --release --package hyperlane-cli-tool

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📍 Binary location: ./target/release/hyperlane-cli"
    echo ""
    echo "🚀 Usage examples:"
    echo "  ./target/release/hyperlane-cli send --help"
    echo "  ./target/release/hyperlane-cli search --help"
else
    echo "❌ Build failed!"
    exit 1
fi