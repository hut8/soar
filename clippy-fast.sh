#!/bin/bash
# Fast clippy script for development
# This skips dependencies and only checks the library, making it much faster

set -e

echo "Running fast clippy (library only, no dependencies)..."
cargo clippy --lib --no-deps -- -D warnings

echo ""
echo "Fast clippy completed! For full checks, run: cargo clippy --all-targets --all-features"
