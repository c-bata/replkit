package main

import (
	"context"
	"fmt"
	"log"

	textbuffer "github.com/c-bata/prompt/bindings/go/text_buffer"
)

func main() {
	fmt.Println("=== Go Text Buffer Demo ===")

	ctx := context.Background()

	// Create text buffer engine
	engine, err := textbuffer.NewTextBufferEngine(ctx)
	if err != nil {
		log.Fatalf("Failed to create text buffer engine: %v", err)
	}
	defer engine.Close()

	fmt.Println("✓ Text buffer engine created successfully")

	// Demonstrate Buffer operations
	fmt.Println("\n--- Buffer Operations ---")
	demonstrateBuffer(engine)

	// Demonstrate Document operations
	fmt.Println("\n--- Document Operations ---")
	demonstrateDocument(engine)

	// Demonstrate Unicode handling
	fmt.Println("\n--- Unicode Handling ---")
	demonstrateUnicode(engine)

	// Demonstrate serialization
	fmt.Println("\n--- State Serialization ---")
	demonstrateSerialization(engine)

	fmt.Println("\n=== Demo completed successfully! ===")
}

func demonstrateBuffer(engine *textbuffer.TextBufferEngine) {
	buffer, err := engine.NewBuffer()
	if err != nil {
		log.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert some text
	err = buffer.InsertText("Hello, World!", false, true)
	if err != nil {
		log.Fatalf("Failed to insert text: %v", err)
	}

	text, _ := buffer.Text()
	pos, _ := buffer.CursorPosition()
	fmt.Printf("After insertion: '%s' (cursor at %d)\n", text, pos)

	// Move cursor and insert more text
	err = buffer.CursorLeft(6) // Move to before "World"
	if err != nil {
		log.Fatalf("Failed to move cursor: %v", err)
	}

	err = buffer.InsertText("Beautiful ", false, true)
	if err != nil {
		log.Fatalf("Failed to insert text: %v", err)
	}

	text, _ = buffer.Text()
	pos, _ = buffer.CursorPosition()
	fmt.Printf("After insertion: '%s' (cursor at %d)\n", text, pos)

	// Delete some text
	deleted, err := buffer.DeleteBeforeCursor(10)
	if err != nil {
		log.Fatalf("Failed to delete text: %v", err)
	}

	text, _ = buffer.Text()
	fmt.Printf("After deletion: '%s' (deleted: '%s')\n", text, deleted)

	// Create multiline content
	err = buffer.SetText("First line")
	if err != nil {
		log.Fatalf("Failed to set text: %v", err)
	}

	err = buffer.NewLine(false)
	if err != nil {
		log.Fatalf("Failed to create new line: %v", err)
	}

	err = buffer.InsertText("Second line", false, true)
	if err != nil {
		log.Fatalf("Failed to insert second line: %v", err)
	}

	text, _ = buffer.Text()
	fmt.Printf("Multiline text: %q\n", text)

	// Test cursor movement in multiline text
	err = buffer.CursorUp(1)
	if err != nil {
		log.Fatalf("Failed to move cursor up: %v", err)
	}

	doc, err := buffer.Document()
	if err != nil {
		log.Fatalf("Failed to get document: %v", err)
	}
	defer doc.Close()

	row, _ := doc.CursorPositionRow()
	col, _ := doc.CursorPositionCol()
	fmt.Printf("Cursor position after moving up: row %d, col %d\n", row, col)
}

func demonstrateDocument(engine *textbuffer.TextBufferEngine) {
	// Create a document with some text for analysis
	text := "The quick brown fox jumps over the lazy dog"
	doc, err := engine.NewDocumentWithText(text, 16) // Cursor after "fox"
	if err != nil {
		log.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	docText, _ := doc.Text()
	pos, _ := doc.CursorPosition()
	fmt.Printf("Document text: '%s' (cursor at %d)\n", docText, pos)

	// Analyze text around cursor
	textBefore, _ := doc.TextBeforeCursor()
	textAfter, _ := doc.TextAfterCursor()
	fmt.Printf("Text before cursor: '%s'\n", textBefore)
	fmt.Printf("Text after cursor: '%s'\n", textAfter)

	// Word analysis
	wordBefore, _ := doc.GetWordBeforeCursor()
	wordAfter, _ := doc.GetWordAfterCursor()
	fmt.Printf("Word before cursor: '%s'\n", wordBefore)
	fmt.Printf("Word after cursor: '%s'\n", wordAfter)

	// Test with multiline document
	multilineText := "First line of text\nSecond line here\nThird and final line"
	multiDoc, err := engine.NewDocumentWithText(multilineText, 25) // Cursor in second line
	if err != nil {
		log.Fatalf("Failed to create multiline document: %v", err)
	}
	defer multiDoc.Close()

	lineCount, _ := multiDoc.LineCount()
	row, _ := multiDoc.CursorPositionRow()
	col, _ := multiDoc.CursorPositionCol()
	currentLine, _ := multiDoc.CurrentLine()

	fmt.Printf("Multiline document: %d lines\n", lineCount)
	fmt.Printf("Cursor at row %d, col %d\n", row, col)
	fmt.Printf("Current line: '%s'\n", currentLine)
}

func demonstrateUnicode(engine *textbuffer.TextBufferEngine) {
	buffer, err := engine.NewBuffer()
	if err != nil {
		log.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Insert Unicode text with various character types
	unicodeText := "Hello 世界! 🌍 Testing 测试 🚀"
	err = buffer.InsertText(unicodeText, false, true)
	if err != nil {
		log.Fatalf("Failed to insert Unicode text: %v", err)
	}

	text, _ := buffer.Text()
	pos, _ := buffer.CursorPosition()
	displayPos, _ := buffer.DisplayCursorPosition()

	fmt.Printf("Unicode text: '%s'\n", text)
	fmt.Printf("Cursor position (rune index): %d\n", pos)
	fmt.Printf("Display cursor position: %d\n", displayPos)

	// Test cursor movement with Unicode
	err = buffer.CursorLeft(5) // Move back 5 runes
	if err != nil {
		log.Fatalf("Failed to move cursor: %v", err)
	}

	pos, _ = buffer.CursorPosition()
	displayPos, _ = buffer.DisplayCursorPosition()
	fmt.Printf("After moving left 5 runes: pos %d, display %d\n", pos, displayPos)

	// Get document for text analysis
	doc, err := buffer.Document()
	if err != nil {
		log.Fatalf("Failed to get document: %v", err)
	}
	defer doc.Close()

	textBefore, _ := doc.TextBeforeCursor()
	textAfter, _ := doc.TextAfterCursor()
	fmt.Printf("Text before cursor: '%s'\n", textBefore)
	fmt.Printf("Text after cursor: '%s'\n", textAfter)
}

func demonstrateSerialization(engine *textbuffer.TextBufferEngine) {
	// Create and configure a buffer
	buffer, err := engine.NewBuffer()
	if err != nil {
		log.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	err = buffer.InsertText("Serialization test content", false, true)
	if err != nil {
		log.Fatalf("Failed to insert text: %v", err)
	}

	err = buffer.CursorLeft(8) // Move cursor back
	if err != nil {
		log.Fatalf("Failed to move cursor: %v", err)
	}

	originalText, _ := buffer.Text()
	originalPos, _ := buffer.CursorPosition()
	fmt.Printf("Original buffer: '%s' (cursor at %d)\n", originalText, originalPos)

	// Serialize buffer state
	state, err := buffer.ToWasmState()
	if err != nil {
		log.Fatalf("Failed to serialize buffer state: %v", err)
	}

	fmt.Printf("Serialized state: %d working lines, cursor at %d\n",
		len(state.WorkingLines), state.CursorPosition)

	// Create new buffer from serialized state
	newBuffer, err := engine.BufferFromWasmState(state)
	if err != nil {
		log.Fatalf("Failed to create buffer from state: %v", err)
	}
	defer newBuffer.Close()

	restoredText, _ := newBuffer.Text()
	restoredPos, _ := newBuffer.CursorPosition()
	fmt.Printf("Restored buffer: '%s' (cursor at %d)\n", restoredText, restoredPos)

	// Verify they match
	if originalText == restoredText && originalPos == restoredPos {
		fmt.Println("✓ Serialization/deserialization successful!")
	} else {
		fmt.Println("✗ Serialization/deserialization failed!")
	}

	// Test document serialization
	doc, err := engine.NewDocumentWithText("Document serialization test 文档", 15)
	if err != nil {
		log.Fatalf("Failed to create document: %v", err)
	}
	defer doc.Close()

	docState, err := doc.ToWasmState()
	if err != nil {
		log.Fatalf("Failed to serialize document state: %v", err)
	}

	newDoc, err := engine.DocumentFromWasmState(docState)
	if err != nil {
		log.Fatalf("Failed to create document from state: %v", err)
	}
	defer newDoc.Close()

	originalDocText, _ := doc.Text()
	restoredDocText, _ := newDoc.Text()
	originalDocPos, _ := doc.CursorPosition()
	restoredDocPos, _ := newDoc.CursorPosition()

	fmt.Printf("Document serialization: '%s' -> '%s'\n", originalDocText, restoredDocText)
	fmt.Printf("Document cursor: %d -> %d\n", originalDocPos, restoredDocPos)

	if originalDocText == restoredDocText && originalDocPos == restoredDocPos {
		fmt.Println("✓ Document serialization successful!")
	} else {
		fmt.Println("✗ Document serialization failed!")
	}
}

// Helper function to demonstrate advanced editing operations
func demonstrateAdvancedEditing(engine *textbuffer.TextBufferEngine) {
	fmt.Println("\n--- Advanced Editing Operations ---")

	buffer, err := engine.NewBuffer()
	if err != nil {
		log.Fatalf("Failed to create buffer: %v", err)
	}
	defer buffer.Close()

	// Character swapping
	err = buffer.InsertText("ab", false, true)
	if err != nil {
		log.Fatalf("Failed to insert text: %v", err)
	}

	fmt.Printf("Before swap: '%s'\n", mustGetText(buffer))

	err = buffer.SwapCharactersBeforeCursor()
	if err != nil {
		log.Fatalf("Failed to swap characters: %v", err)
	}

	fmt.Printf("After swap: '%s'\n", mustGetText(buffer))

	// Line joining
	err = buffer.SetText("First line\nSecond line\nThird line")
	if err != nil {
		log.Fatalf("Failed to set text: %v", err)
	}

	err = buffer.SetCursorPosition(10) // End of first line
	if err != nil {
		log.Fatalf("Failed to set cursor position: %v", err)
	}

	fmt.Printf("Before join: %q\n", mustGetText(buffer))

	err = buffer.JoinNextLine(" ")
	if err != nil {
		log.Fatalf("Failed to join lines: %v", err)
	}

	fmt.Printf("After join: %q\n", mustGetText(buffer))
}

// Helper function to get text without error handling for cleaner demo code
func mustGetText(buffer *textbuffer.Buffer) string {
	text, err := buffer.Text()
	if err != nil {
		log.Fatalf("Failed to get text: %v", err)
	}
	return text
}
