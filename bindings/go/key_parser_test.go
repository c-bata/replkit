package keyparsing

import (
	"context"
	"testing"
)

func TestKeyParserCreation(t *testing.T) {
	// This test verifies that the KeyParser struct and its methods are properly defined
	// We can't test the actual WASM functionality without a compiled WASM binary,
	// but we can test the API structure

	ctx := context.Background()

	// Test that we can attempt to create a parser (will fail without WASM binary, but that's expected)
	_, err := NewKeyParser(ctx, []byte{})
	if err == nil {
		t.Error("Expected error when creating parser with empty WASM binary")
	}
}

func TestKeyConstants(t *testing.T) {
	// Test that key constants are properly defined
	keys := []Key{
		ControlA, ControlB, ControlC,
		Up, Down, Left, Right,
		F1, F2, F3,
		Tab, Enter, Escape,
		NotDefined, Ignore,
	}

	// Verify that keys have different values
	keyMap := make(map[Key]bool)
	for _, key := range keys {
		if keyMap[key] {
			t.Errorf("Duplicate key value found: %v", key)
		}
		keyMap[key] = true
	}
}

func TestKeyEventStruct(t *testing.T) {
	// Test that KeyEvent struct is properly defined
	event := KeyEvent{
		Key:      Up,
		RawBytes: []byte{0x1b, 0x5b, 0x41},
		Text:     nil,
	}

	if event.Key != Up {
		t.Errorf("Expected key to be Up, got %v", event.Key)
	}

	if len(event.RawBytes) != 3 {
		t.Errorf("Expected 3 raw bytes, got %d", len(event.RawBytes))
	}
}
