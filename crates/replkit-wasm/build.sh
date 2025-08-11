#!/bin/bash

set -e

# Get the directory where this script is located
SCRIPT_DIR=$(cd $(dirname $0); pwd)
# Find the project root (where Cargo.toml is located)
PROJECT_ROOT=$(cd ${SCRIPT_DIR}/../..; pwd)

echo "Script directory: ${SCRIPT_DIR}"
echo "Project root: ${PROJECT_ROOT}"

# Change to the script directory for cargo operations
cd ${SCRIPT_DIR}

# Install wasm32-unknown-unknown target if not already installed
rustup target add wasm32-unknown-unknown

# Build the WASM module
cargo build --target wasm32-unknown-unknown --release

# Create target directories
BINDINGS_WASM_DIR="${PROJECT_ROOT}/bindings/go/wasm"
mkdir -p ${BINDINGS_WASM_DIR}

# Copy the built WASM file to bindings/go for easy access
WASM_SOURCE="${PROJECT_ROOT}/target/wasm32-unknown-unknown/release/replkit_wasm.wasm"
WASM_TARGET="${BINDINGS_WASM_DIR}/replkit_wasm.wasm"

if [ -f "${WASM_SOURCE}" ]; then
    cp "${WASM_SOURCE}" "${WASM_TARGET}"
    echo "WASM build complete! Output: ${WASM_TARGET}"
else
    echo "ERROR: WASM build failed - ${WASM_SOURCE} not found"
    exit 1
fi

# Optional: Use wasm-opt to optimize the WASM file if available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM with wasm-opt..."
    wasm-opt -Os --enable-mutable-globals "${WASM_TARGET}" -o "${WASM_TARGET}"
    echo "WASM optimization complete!"
else
    echo "wasm-opt not found, skipping optimization (install binaryen for optimization)"
fi