#!/bin/bash

# Build script for compiling prompt-wasm to WebAssembly

set -e

echo "Building prompt-wasm for WebAssembly target..."

# Install wasm32-unknown-unknown target if not already installed
rustup target add wasm32-unknown-unknown

# Build the WASM module
cargo build --target wasm32-unknown-unknown --release

# Copy the built WASM file to bindings/go for easy access
mkdir -p ../../bindings/go/wasm
cp ../../target/wasm32-unknown-unknown/release/prompt_wasm.wasm ../../bindings/go/wasm/

echo "WASM build complete! Output: ../../bindings/go/wasm/prompt_wasm.wasm"

# Optional: Use wasm-opt to optimize the WASM file if available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM with wasm-opt..."
    wasm-opt -Os --enable-mutable-globals ../../bindings/go/wasm/prompt_wasm.wasm -o ../../bindings/go/wasm/prompt_wasm.wasm
    echo "WASM optimization complete!"
else
    echo "wasm-opt not found, skipping optimization (install binaryen for optimization)"
fi