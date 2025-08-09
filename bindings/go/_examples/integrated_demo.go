package main

import (
	"context"
	"fmt"
	"log"

	keyparsing "github.com/c-bata/prompt/bindings/go"
)

func main() {
	fmt.Println("=== Integrated Go Prompt Demo ===")

	ctx := context.Background()

	// Create parser (which now includes text buffer functionality)
	parser, err := keyparsing.New(ctx)
	if err != nil {
		log.Fatalf("Failed to create parser: %v", err)
	}
	defer parser.Close()

	fmt.Println("✓ Parser created successfully")

	// Demonstrate key parsing
	fmt.Println("\n--- Key Parsing ---")
	demonstrateKeyParsing(parser)

	// Demonstrate text buffer operations
	fmt.Println("\n--- Text Buffer Operations ---")
	demonstrateTextBuffer(parser)

	// Demonstrate document analysis
	fmt.Println("\n--- Document Analysis ---")
	demonstrateDocumentAnalysis(parser)

	// Demonstrate Unicode handling
	fmt.Println("\n--- Unicode Handling ---")
	demonstrateUnicode(parser)

	fmt.Println("\n=== Demo completed successfully! ===")
}

func demonstrateKeyParsing(parser *keyparsing.KeyParser) {
	// Test key parsing with arrow key sequence
	upArrowBytes := []byte{0x1b, 0x5b, 0x41} // ESC [ A
	events, err := parser.Feed(upArrowBytes)
	if err != nil {
		log.Fatalf("Failed to parse key: %v", err)
	}

	if len(events) > 0 {
		fmt.Printf("Parsed key: %s (raw bytes: %v)\n", events[0].Key, events[0].RawBytes)
	}

	// Test with Ctrl+C
	ctrlCBytes := []byte{0x03}
	events, err = parser.Feed(ctrlCBytes)
	if err != nil {
		log.Fatalf("Failed to parse key: %v", err)
	}

	if len(events) > 0 {
		fmt.Printf("Parsed key: %s (raw bytes: %v)\n", events[0].Key, events[0].RawBytes)
	}
}

func demonstrateTextBuffer(parser *keyparsing.KeyParser) {
	buffer, err := parser.NewBuffer()
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
}

func demonstrateDocumentAnalysis(parser *keyparsing.KeyParser) {
	// Create a document with some text for analysis
	text := "The quick brown fox jumps over the lazy dog"
	doc, err := parser.NewDocumentWithText(text, 16) // Cursor after "fox"
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
	multiDoc, err := parser.NewDocumentWithText(multilineText, 25) // Cursor in second line
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

func demonstrateUnicode(parser *keyparsing.KeyParser) {
	buffer, err := parser.NewBuffer()
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
