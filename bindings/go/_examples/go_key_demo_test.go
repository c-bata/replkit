package main

import (
	"context"
	"testing"

	keyparsing "github.com/c-bata/prompt/bindings/go"
)

func TestGoKeyDemoIntegration(t *testing.T) {
	ctx := context.Background()

	// Create a new parser using the embedded WASM binary
	parser, err := keyparsing.New(ctx)
	if err != nil {
		t.Fatalf("Error creating key parser: %v", err)
	}
	defer func() {
		if err := parser.Close(); err != nil {
			t.Errorf("Error closing parser: %v", err)
		}
	}()

	// Test basic key parsing
	testCases := []struct {
		name     string
		input    []byte
		expected keyparsing.Key
	}{
		{
			name:     "Ctrl+C",
			input:    []byte{0x03},
			expected: keyparsing.ControlC,
		},
		{
			name:     "Up Arrow",
			input:    []byte{0x1b, 0x5b, 0x41},
			expected: keyparsing.Up,
		},
		{
			name:     "Down Arrow",
			input:    []byte{0x1b, 0x5b, 0x42},
			expected: keyparsing.Down,
		},
		{
			name:     "Tab",
			input:    []byte{0x09},
			expected: keyparsing.Tab,
		},
		{
			name:     "Enter (Ctrl+M)",
			input:    []byte{0x0d},
			expected: keyparsing.ControlM,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			// Reset parser state before each test
			if err := parser.Reset(); err != nil {
				t.Fatalf("Error resetting parser: %v", err)
			}

			// Parse the input
			events, err := parser.Feed(tc.input)
			if err != nil {
				t.Fatalf("Error parsing input: %v", err)
			}

			// Verify we got exactly one event
			if len(events) != 1 {
				t.Fatalf("Expected 1 event, got %d", len(events))
			}

			// Verify the key matches
			if events[0].Key != tc.expected {
				t.Errorf("Expected key %s, got %s", tc.expected, events[0].Key)
			}

			// Verify raw bytes match
			if len(events[0].RawBytes) != len(tc.input) {
				t.Errorf("Expected %d raw bytes, got %d", len(tc.input), len(events[0].RawBytes))
			}
			for i, b := range tc.input {
				if i >= len(events[0].RawBytes) || events[0].RawBytes[i] != b {
					t.Errorf("Raw bytes mismatch at index %d: expected 0x%02x, got 0x%02x", i, b, events[0].RawBytes[i])
				}
			}
		})
	}
}

func TestPartialSequenceHandling(t *testing.T) {
	ctx := context.Background()

	// Create a new parser using the embedded WASM binary
	parser, err := keyparsing.New(ctx)
	if err != nil {
		t.Fatalf("Error creating key parser: %v", err)
	}
	defer parser.Close()

	// Test partial sequence handling
	// Send ESC first (partial sequence)
	events, err := parser.Feed([]byte{0x1b})
	if err != nil {
		t.Fatalf("Error feeding partial sequence: %v", err)
	}

	// Should have no events yet (partial sequence)
	if len(events) != 0 {
		t.Errorf("Expected 0 events for partial sequence, got %d", len(events))
	}

	// Complete the sequence with [A (Up arrow)
	events, err = parser.Feed([]byte{0x5b, 0x41})
	if err != nil {
		t.Fatalf("Error feeding sequence completion: %v", err)
	}

	// Should now have one Up arrow event
	if len(events) != 1 {
		t.Fatalf("Expected 1 event after completing sequence, got %d", len(events))
	}

	if events[0].Key != keyparsing.Up {
		t.Errorf("Expected Up arrow, got %s", events[0].Key)
	}
}

func TestFlushAndReset(t *testing.T) {
	ctx := context.Background()

	// Create a new parser using the embedded WASM binary
	parser, err := keyparsing.New(ctx)
	if err != nil {
		t.Fatalf("Error creating key parser: %v", err)
	}
	defer parser.Close()

	// Send a partial sequence
	events, err := parser.Feed([]byte{0x1b})
	if err != nil {
		t.Fatalf("Error feeding partial sequence: %v", err)
	}

	if len(events) != 0 {
		t.Errorf("Expected 0 events for partial sequence, got %d", len(events))
	}

	// Test flush - should handle the incomplete sequence
	flushEvents, err := parser.Flush()
	if err != nil {
		t.Fatalf("Error flushing parser: %v", err)
	}

	// Should get an Escape key event
	if len(flushEvents) != 1 {
		t.Errorf("Expected 1 event after flush, got %d", len(flushEvents))
	} else if flushEvents[0].Key != keyparsing.Escape {
		t.Errorf("Expected Escape key after flush, got %s", flushEvents[0].Key)
	}

	// Test reset
	if err := parser.Reset(); err != nil {
		t.Fatalf("Error resetting parser: %v", err)
	}

	// After reset, flush should return no events
	flushEvents, err = parser.Flush()
	if err != nil {
		t.Fatalf("Error flushing after reset: %v", err)
	}

	if len(flushEvents) != 0 {
		t.Errorf("Expected 0 events after reset and flush, got %d", len(flushEvents))
	}
}
