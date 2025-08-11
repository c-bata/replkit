package replkit

import (
	"context"
	"io/ioutil"
	"path/filepath"
	"testing"
)

func TestWASMIntegration(t *testing.T) {
	// Load the WASM binary
	wasmPath := filepath.Join("wasm", "replkit_wasm.wasm")
	wasmBytes, err := ioutil.ReadFile(wasmPath)
	if err != nil {
		t.Skipf("WASM binary not found at %s, skipping integration test: %v", wasmPath, err)
		return
	}

	ctx := context.Background()

	// Create a new parser
	parser, err := NewKeyParser(ctx, wasmBytes)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	// Test parsing a simple control character (Ctrl+C)
	events, err := parser.Feed([]byte{0x03})
	if err != nil {
		t.Fatalf("Failed to feed input: %v", err)
	}

	if len(events) != 1 {
		t.Fatalf("Expected 1 event, got %d", len(events))
	}

	if events[0].Key != ControlC {
		t.Errorf("Expected ControlC key, got %v", events[0].Key)
	}

	if len(events[0].RawBytes) != 1 || events[0].RawBytes[0] != 0x03 {
		t.Errorf("Expected raw bytes [0x03], got %v", events[0].RawBytes)
	}

	// Test parsing an arrow key sequence (Up arrow: ESC[A)
	events, err = parser.Feed([]byte{0x1b, 0x5b, 0x41})
	if err != nil {
		t.Fatalf("Failed to feed arrow key input: %v", err)
	}

	if len(events) != 1 {
		t.Fatalf("Expected 1 event for arrow key, got %d", len(events))
	}

	if events[0].Key != Up {
		t.Errorf("Expected Up key, got %v", events[0].Key)
	}

	expectedBytes := []byte{0x1b, 0x5b, 0x41}
	if len(events[0].RawBytes) != len(expectedBytes) {
		t.Errorf("Expected raw bytes length %d, got %d", len(expectedBytes), len(events[0].RawBytes))
	}
	for i, b := range expectedBytes {
		if i >= len(events[0].RawBytes) || events[0].RawBytes[i] != b {
			t.Errorf("Expected raw bytes %v, got %v", expectedBytes, events[0].RawBytes)
			break
		}
	}

	// Test reset functionality
	err = parser.Reset()
	if err != nil {
		t.Fatalf("Failed to reset parser: %v", err)
	}

	// Test flush with empty buffer
	events, err = parser.Flush()
	if err != nil {
		t.Fatalf("Failed to flush parser: %v", err)
	}

	if len(events) != 0 {
		t.Errorf("Expected 0 events after flush, got %d", len(events))
	}

	// Test partial sequence handling
	// Feed partial escape sequence
	events, err = parser.Feed([]byte{0x1b}) // Just ESC
	if err != nil {
		t.Fatalf("Failed to feed partial sequence: %v", err)
	}

	// Should not produce any events yet
	if len(events) != 0 {
		t.Errorf("Expected 0 events for partial sequence, got %d", len(events))
	}

	// Complete the sequence
	events, err = parser.Feed([]byte{0x5b, 0x42}) // [B for Down arrow
	if err != nil {
		t.Fatalf("Failed to complete sequence: %v", err)
	}

	if len(events) != 1 {
		t.Fatalf("Expected 1 event for completed sequence, got %d", len(events))
	}

	if events[0].Key != Down {
		t.Errorf("Expected Down key, got %v", events[0].Key)
	}

	// Test feeding empty input
	events, err = parser.Feed([]byte{})
	if err != nil {
		t.Fatalf("Failed to feed empty input: %v", err)
	}

	if len(events) != 0 {
		t.Errorf("Expected 0 events for empty input, got %d", len(events))
	}

	// Test multiple key events in one feed
	events, err = parser.Feed([]byte{0x03, 0x1b, 0x5b, 0x43}) // Ctrl+C followed by Right arrow
	if err != nil {
		t.Fatalf("Failed to feed multiple keys: %v", err)
	}

	if len(events) != 2 {
		t.Fatalf("Expected 2 events for multiple keys, got %d", len(events))
	}

	if events[0].Key != ControlC {
		t.Errorf("Expected first event to be ControlC, got %v", events[0].Key)
	}

	if events[1].Key != Right {
		t.Errorf("Expected second event to be Right, got %v", events[1].Key)
	}
}
