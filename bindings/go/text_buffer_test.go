package replkit

import (
	"context"
	"testing"
)

func TestTextBufferIntegration(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	// Test buffer creation
	buffer, err := parser.NewBuffer()
	if err != nil {
		t.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Test basic text operations
	err = buffer.InsertText("Hello, World!", false, true)
	if err != nil {
		t.Fatalf("Failed to insert text: %v", err)
	}

	text, err := buffer.Text()
	if err != nil {
		t.Fatalf("Failed to get text: %v", err)
	}
	if text != "Hello, World!" {
		t.Errorf("Expected 'Hello, World!', got: %q", text)
	}

	pos, err := buffer.CursorPosition()
	if err != nil {
		t.Fatalf("Failed to get cursor position: %v", err)
	}
	if pos != 13 {
		t.Errorf("Expected cursor position 13, got: %d", pos)
	}
}

func TestBufferCursorMovement(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	buffer, err := parser.NewBuffer()
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

func TestDocumentAnalysis(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	// Create document with text
	doc, err := parser.NewDocumentWithText("Hello world test", 6) // Cursor after "Hello "
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
}

func TestUnicodeHandling(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	buffer, err := parser.NewBuffer()
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
}

func TestStateSerialization(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	// Create and configure buffer
	buffer, err := parser.NewBuffer()
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
	if state.CursorPosition != 7 {
		t.Errorf("Expected cursor position 7, got: %d", state.CursorPosition)
	}

	// Create new buffer from state
	newBuffer, err := parser.BufferFromWasmState(state)
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
	if pos != 7 {
		t.Errorf("Expected cursor position 7, got: %d", pos)
	}
}

func TestMultilineOperations(t *testing.T) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		t.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	buffer, err := parser.NewBuffer()
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

// Benchmark tests
func BenchmarkBufferInsertText(b *testing.B) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		b.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	buffer, err := parser.NewBuffer()
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

func BenchmarkDocumentTextAnalysis(b *testing.B) {
	ctx := context.Background()
	parser, err := New(ctx)
	if err != nil {
		b.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	doc, err := parser.NewDocumentWithText("Hello world test document", 12)
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
