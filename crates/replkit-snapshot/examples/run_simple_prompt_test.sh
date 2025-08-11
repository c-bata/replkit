#!/bin/bash

# Simple prompt example test runner for replkit-snapshot
# This script demonstrates how to test interactive terminal applications
# using replkit-snapshot with the simple_prompt example.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SNAPSHOT_DIR="$SCRIPT_DIR/snapshots"

echo "🚀 Running simple_prompt example test..."
echo ""
echo "This test will:"
echo "1. Start the simple_prompt example"
echo "2. Type 'u' and press TAB to trigger completion"
echo "3. Press Enter to select 'users'"
echo "4. Capture and compare console output snapshots"
echo ""

# Create snapshot directory if it doesn't exist
mkdir -p "$SNAPSHOT_DIR"

# Change to replkit-snapshot directory
cd "$SCRIPT_DIR/.."

echo "📸 Running snapshot test (creating/updating golden files)..."
cargo run -- run \
    --cmd "cargo run --example simple_prompt" \
    --steps "examples/simple_prompt_test.yaml" \
    --compare "$SNAPSHOT_DIR" \
    --update

echo ""
echo "✅ Test completed successfully!"
echo ""
echo "📁 Generated snapshots:"
ls -la "$SNAPSHOT_DIR"/*.golden 2>/dev/null || echo "No golden files found"

echo ""
echo "🔍 To verify the test (run without --update):"
echo "  cargo run -- run --cmd \"cargo run --example simple_prompt\" --steps examples/simple_prompt_test.yaml --compare examples/snapshots"
echo ""
echo "📄 To view snapshot contents:"
echo "  cat examples/snapshots/*.golden"