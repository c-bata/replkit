#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"

mkdir -p "$BIN_DIR"

cd "$PROJECT_ROOT"
cargo build --example simple_prompt
cp target/debug/examples/simple_prompt "$BIN_DIR/rust_simple_prompt"

cd "$PROJECT_ROOT/references/go-prompt/_example/simple-echo"
go build -o "$BIN_DIR/go_simple_echo" main.go

chmod +x "$BIN_DIR/rust_simple_prompt" "$BIN_DIR/go_simple_echo"