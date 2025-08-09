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

func TestKeyParserMethods(t *testing.T) {
	// Test that all required methods are available on KeyParser
	// This is a compile-time test to ensure the interface is complete

	var parser *KeyParser

	// These should compile without errors
	_ = func() ([]KeyEvent, error) { return parser.Feed([]byte{}) }
	_ = func() ([]KeyEvent, error) { return parser.Flush() }
	_ = func() error { return parser.Reset() }
	_ = func() error { return parser.Close() }
}
func TestKeyStringRepresentation(t *testing.T) {
	// Test that keys have proper string representations
	testCases := []struct {
		key      Key
		expected string
	}{
		{ControlA, "Ctrl+A"},
		{ControlC, "Ctrl+C"},
		{Up, "Up"},
		{Down, "Down"},
		{Left, "Left"},
		{Right, "Right"},
		{F1, "F1"},
		{F12, "F12"},
		{Tab, "Tab"},
		{Enter, "Enter"},
		{Escape, "Escape"},
		{ShiftUp, "Shift+Up"},
		{ControlLeft, "Ctrl+Left"},
		{NotDefined, "NotDefined"},
		{Ignore, "Ignore"},
	}

	for _, tc := range testCases {
		if tc.key.String() != tc.expected {
			t.Errorf("Expected %s.String() to be %q, got %q", tc.expected, tc.expected, tc.key.String())
		}
	}

	// Test unknown key
	unknownKey := Key(999)
	expected := "Key(999)"
	if unknownKey.String() != expected {
		t.Errorf("Expected unknown key string to be %q, got %q", expected, unknownKey.String())
	}
}
func TestKeyParserErrorHandling(t *testing.T) {
	// Test error handling for nil parser
	var parser *KeyParser

	_, err := parser.Feed([]byte{0x03})
	if err == nil {
		t.Error("Expected error when calling Feed on nil parser")
	}

	_, err = parser.Flush()
	if err == nil {
		t.Error("Expected error when calling Flush on nil parser")
	}

	err = parser.Reset()
	if err == nil {
		t.Error("Expected error when calling Reset on nil parser")
	}

	err = parser.Close()
	if err == nil {
		t.Error("Expected error when calling Close on nil parser")
	}
}
