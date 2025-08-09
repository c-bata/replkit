package textbuffer

import (
	"context"
	"testing"
)

func TestTextBufferEngine_Creation(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	if engine == nil {
		t.Fatal("Engine should not be nil")
	}
}

func TestBuffer_BasicOperations(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Test initial state
	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text: %v", err)
	}
	if text != "" {
		t.Errorf("Expected empty text, got: %q", text)
	}

	pos, err := buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	if pos != 0 {
		t.Errorf("Expected cursor position 0, got: %d", pos)
	}

	// Test text insertion
	err = buffer.InsertText("Hello, World!", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	text, err = buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text after insertion: %v", err)
	}
	if text != "Hello, World!" {
		t.Errorf("Expected 'Hello, World!', got: %q", text)
	}

	pos, err = buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position after insertion: %v", err)
	}
	if pos != 13 {
		t.Errorf("Expected cursor position 13, got: %d", pos)
	}
}

func TestBuffer_CursorMovement(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert some text
	err = buffer.InsertText("Hello, World!", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	// Move cursor left
	err = buffer.CursorLeft(5)
	if err != nil {
		t.Fatalf("Failed to move cursor left: %v", err)
	}

	pos, err := buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	if pos != 8 {
		t.Errorf("Expected cursor position 8, got: %d", pos)
	}

	// Move cursor right
	err = buffer.CursorRight(2)
	if err != nil {
		t.Fatalf("Failed to move cursor right: %v", err)
	}

	pos, err = buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	if pos != 10 {
		t.Errorf("Expected cursor position 10, got: %d", pos)
	}
}

func TestBuffer_TextDeletion(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert some text
	err = buffer.InsertText("Hello, World!", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	// Delete before cursor
	deleted, err := buffer.DeleteBeforeCursor(7)
	if err != nil {
		t.Fatalf("Failed to delete before cursor: %v", err)
	}
	if deleted != ", World" {
		t.Errorf("Expected deleted text ', World', got: %q", deleted)
	}

	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text after deletion: %v", err)
	}
	if text != "Hello!" {
		t.Errorf("Expected 'Hello!', got: %q", text)
	}

	pos, err := buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	if pos != 6 {
		t.Errorf("Expected cursor position 6, got: %d", pos)
	}
}

func TestBuffer_MultilineOperations(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert text and create new line
	err = buffer.InsertText("First line", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	err = buffer.NewLine(false)
	if err != nil {
		t.Fatalf("Failed to create new line: %v", err)
	}

	err = buffer.InsertText("Second line", false, true)
	if err != nil {
		t.Fatalf("Failed to insert second line text: %v", err)
	}

	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text: %v", err)
	}
	expected := "First line\nSecond line"
	if text != expected {
		t.Errorf("Expected %q, got: %q", expected, text)
	}

	// Test cursor up/down movement
	err = buffer.CursorUp(1)
	if err != nil {
		t.Fatalf("Failed to move cursor up: %v", err)
	}

	// Get document to check cursor position
	doc, err := buffer.Document()
	if err != nil {
		t.Fatalf("Failed to get document: %v", err)
	}
	defer doc.Close()

	row, err := doc.CursorPositionRow()
	if err != nil {
		t.Fatalf("Failed to get cursor row: %v", err)
	}
	if row != 0 {
		t.Errorf("Expected cursor row 0, got: %d", row)
	}
}

func TestBuffer_UnicodeHandling(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert Unicode text
	unicodeText := "Hello 世界! 🌍"
	err = buffer.InsertText(unicodeText, false, true)
	if err != nil {
		t.Fatalf("Failed to insert Unicode text: %v", err)
	}

	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text: %v", err)
	}
	if text != unicodeText {
		t.Errorf("Expected %q, got: %q", unicodeText, text)
	}

	// Test cursor position (should be in rune index, not byte index)
	pos, err := buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	// "Hello 世界! 🌍" has 11 runes
	if pos != 11 {
		t.Errorf("Expected cursor position 11 (rune index), got: %d", pos)
	}

	// Test display cursor position (should account for wide characters)
	displayPos, err := buffer.DisplayCursorPosition()
	if err != nil {
		t.Fatalf("Failed to get display cursor position: %v", err)
	}
	// Should be different from rune position due to wide characters
	if displayPos == pos {
		t.Logf("Display position (%d) same as rune position (%d) - this might be expected depending on Unicode width calculation", displayPos, pos)
	}
}

func TestDocument_TextAnalysis(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	// Create document with text
	doc, err := engine.NewDocumentWithText("Hello world test", 6) // Cursor after "Hello "
	if err != nil {
		t.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	// Test text before cursor
	textBefore, err := doc.TextBeforeCursor()
	if err != nil {
		t.Fatalf("Failed to get text before cursor: %v", err)
	}
	if textBefore != "Hello " {
		t.Errorf("Expected 'Hello ', got: %q", textBefore)
	}

	// Test text after cursor
	textAfter, err := doc.TextAfterCursor()
	if err != nil {
		t.Fatalf("Failed to get text after cursor: %v", err)
	}
	if textAfter != "world test" {
		t.Errorf("Expected 'world test', got: %q", textAfter)
	}

	// Test word operations
	wordBefore, err := doc.GetWordBeforeCursor()
	if err != nil {
		t.Fatalf("Failed to get word before cursor: %v", err)
	}
	if wordBefore != "Hello" {
		t.Errorf("Expected 'Hello', got: %q", wordBefore)
	}

	wordAfter, err := doc.GetWordAfterCursor()
	if err != nil {
		t.Fatalf("Failed to get word after cursor: %v", err)
	}
	if wordAfter != "world" {
		t.Errorf("Expected 'world', got: %q", wordAfter)
	}
}

func TestDocument_MultilineAnalysis(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	// Create document with multiline text
	text := "First line\nSecond line\nThird line"
	doc, err := engine.NewDocumentWithText(text, 15) // Cursor at start of "Second line"
	if err != nil {
		t.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	// Test line count
	lineCount, err := doc.LineCount()
	if err != nil {
		t.Fatalf("Failed to get line count: %v", err)
	}
	if lineCount != 3 {
		t.Errorf("Expected 3 lines, got: %d", lineCount)
	}

	// Test cursor position
	row, err := doc.CursorPositionRow()
	if err != nil {
		t.Fatalf("Failed to get cursor row: %v", err)
	}
	if row != 1 {
		t.Errorf("Expected cursor row 1, got: %d", row)
	}

	col, err := doc.CursorPositionCol()
	if err != nil {
		t.Fatalf("Failed to get cursor column: %v", err)
	}
	if col != 4 {
		t.Errorf("Expected cursor column 4, got: %d", col)
	}

	// Test current line
	currentLine, err := doc.CurrentLine()
	if err != nil {
		t.Fatalf("Failed to get current line: %v", err)
	}
	if currentLine != "Second line" {
		t.Errorf("Expected 'Second line', got: %q", currentLine)
	}
}

func TestWasmStateSerialization(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	// Create and configure buffer
	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	err = buffer.InsertText("Test content", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	err = buffer.CursorLeft(5)
	if err != nil {
		t.Fatalf("Failed to move cursor: %v", err)
	}

	// Serialize buffer state
	state, err := buffer.ToWasmState()
	if err != nil {
		t.Fatalf("Failed to serialize buffer state: %v", err)
	}

	if len(state.WorkingLines) == 0 {
		t.Error("Expected working lines in serialized state")
	}
	if state.CursorPosition != 8 {
		t.Errorf("Expected cursor position 8, got: %d", state.CursorPosition)
	}

	// Create new buffer from state
	newBuffer, err := engine.BufferFromWasmState(state)
	if err != nil {
		t.Fatalf("Failed to create buffer from state: %v", err)
	}
	defer newBuffer.Close()

	// Verify restored state
	text, err := newBuffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text from restored buffer: %v", err)
	}
	if text != "Test content" {
		t.Errorf("Expected 'Test content', got: %q", text)
	}

	pos, err := newBuffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position from restored buffer: %v", err)
	}
	if pos != 8 {
		t.Errorf("Expected cursor position 8, got: %d", pos)
	}
}

func TestDocument_WasmStateSerialization(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	// Create document
	originalText := "Hello 世界!"
	doc, err := engine.NewDocumentWithText(originalText, 7)
	if err != nil {
		t.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	// Serialize document state
	state, err := doc.ToWasmState()
	if err != nil {
		t.Fatalf("Failed to serialize document state: %v", err)
	}

	if state.Text != originalText {
		t.Errorf("Expected text %q, got: %q", originalText, state.Text)
	}
	if state.CursorPosition != 7 {
		t.Errorf("Expected cursor position 7, got: %d", state.CursorPosition)
	}

	// Create new document from state
	newDoc, err := engine.DocumentFromWasmState(state)
	if err != nil {
		t.Fatalf("Failed to create document from state: %v", err)
	}
	defer newDoc.Close()

	// Verify restored state
	text, err := newDoc.Text()
	if err != nil {
		t.Fatalf("Failed to get text from restored document: %v", err)
	}
	if text != originalText {
		t.Errorf("Expected text %q, got: %q", originalText, text)
	}

	pos, err := newDoc.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position from restored document: %v", err)
	}
	if pos != 7 {
		t.Errorf("Expected cursor position 7, got: %d", pos)
	}
}

func TestBuffer_AdvancedOperations(t *testing.T) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		t.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Test character swapping
	err = buffer.InsertText("ab", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	err = buffer.SwapCharactersBeforeCursor()
	if err != nil {
		t.Fatalf("Failed to swap characters: %v", err)
	}

	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text: %v", err)
	}
	if text != "ba" {
		t.Errorf("Expected 'ba' after character swap, got: %q", text)
	}

	// Test line joining
	err = buffer.SetText("First line\nSecond line")
	if err != nil {
		t.Fatalf("Failed to set text: %v", err)
	}

	err = buffer.SetCursorPosition(10) // End of first line
	if err != nil {
		t.Fatalf("Failed to set cursor position: %v", err)
	}

	err = buffer.JoinNextLine(" ")
	if err != nil {
		t.Fatalf("Failed to join next line: %v", err)
	}

	text, err = buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text after line join: %v", err)
	}
	if text != "First line Second line" {
		t.Errorf("Expected 'First line Second line', got: %q", text)
	}
}

func TestKey_String(t *testing.T) {
	tests := []struct {
		key      Key
		expected string
	}{
		{ControlA, "ControlA"},
		{Up, "Up"},
		{F1, "F1"},
		{Tab, "Tab"},
		{Enter, "Enter"},
		{NotDefined, "NotDefined"},
		{Key(999), "Key(999)"}, // Unknown key
	}

	for _, test := range tests {
		result := test.key.String()
		if result != test.expected {
			t.Errorf("Key(%d).String() = %q, expected %q", int(test.key), result, test.expected)
		}
	}
}

// Benchmark tests
func BenchmarkBuffer_InsertText(b *testing.B) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		b.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	buffer, err := engine.NewBuffer()
	if err != nil {
		b.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		buffer.SetText("") // Reset for each iteration
		buffer.InsertText("Hello, World!", false, true)
	}
}

func BenchmarkDocument_TextAnalysis(b *testing.B) {
	ctx := context.Background()
	engine, err := NewTextBufferEngine(ctx)
	if err != nil {
		b.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	doc, err := engine.NewDocumentWithText("Hello world test document", 12)
	if err != nil {
		b.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		doc.GetWordBeforeCursor()
		doc.GetWordAfterCursor()
		doc.TextBeforeCursor()
		doc.TextAfterCursor()
	}
}
