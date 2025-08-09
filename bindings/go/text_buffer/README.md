# Go Text Buffer Bindings

This package provides Go bindings for the Rust-based text buffer and document management system. It enables efficient text editing operations with proper Unicode support through WebAssembly (WASM) integration.

## Features

- **Buffer Management**: Mutable text buffer with editing operations
- **Document Analysis**: Immutable document structure for text analysis
- **Unicode Support**: Proper handling of multi-byte characters, CJK text, and emojis
- **Cursor Operations**: Efficient cursor movement and positioning
- **Multi-line Support**: Line-based operations and navigation
- **Serialization**: State serialization for persistence and inter-process communication
- **WASM Integration**: High-performance Rust backend via WebAssembly

## Installation

```bash
go get github.com/c-bata/prompt/bindings/go/text_buffer
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"
    
    textbuffer "github.com/c-bata/prompt/bindings/go/text_buffer"
)

func main() {
    ctx := context.Background()
    
    // Create text buffer engine
    engine, err := textbuffer.NewTextBufferEngine(ctx)
    if err != nil {
        log.Fatal(err)
    }
    defer engine.Close()
    
    // Create a new buffer
    buffer, err := engine.NewBuffer()
    if err != nil {
        log.Fatal(err)
    }
    defer buffer.Close()
    
    // Insert some text
    err = buffer.InsertText("Hello, World!", false, true)
    if err != nil {
        log.Fatal(err)
    }
    
    // Get current text and cursor position
    text, _ := buffer.Text()
    pos, _ := buffer.CursorPosition()
    
    fmt.Printf("Text: '%s', Cursor: %d\n", text, pos)
    // Output: Text: 'Hello, World!', Cursor: 13
}
```

## API Reference

### TextBufferEngine

The `TextBufferEngine` is the main entry point that manages the WASM runtime and provides factory methods for creating buffers and documents.

```go
// Create a new engine with embedded WASM binary
engine, err := textbuffer.NewTextBufferEngine(ctx)

// Create a new engine with custom WASM binary
engine, err := textbuffer.NewTextBufferEngineWithWasm(ctx, wasmBytes)

// Always close the engine when done
defer engine.Close()
```

### Buffer Operations

The `Buffer` provides mutable text editing operations:

```go
// Create a new buffer
buffer, err := engine.NewBuffer()
defer buffer.Close()

// Text operations
err = buffer.InsertText("Hello", false, true)  // insert, not overwrite, move cursor
err = buffer.SetText("New content")
text, err := buffer.Text()

// Cursor operations
err = buffer.CursorLeft(5)
err = buffer.CursorRight(3)
err = buffer.CursorUp(1)
err = buffer.CursorDown(1)
err = buffer.SetCursorPosition(10)
pos, err := buffer.CursorPosition()

// Deletion operations
deleted, err := buffer.DeleteBeforeCursor(3)  // Delete 3 chars before cursor
deleted, err := buffer.Delete(2)              // Delete 2 chars after cursor

// Advanced operations
err = buffer.NewLine(false)                   // Create new line, don't copy margin
err = buffer.JoinNextLine(" ")               // Join with next line using separator
err = buffer.SwapCharactersBeforeCursor()    // Swap two chars before cursor
```

### Document Analysis

The `Document` provides immutable text analysis operations:

```go
// Create documents
doc, err := engine.NewDocument()
doc, err := engine.NewDocumentWithText("Hello world", 6)
doc, err := engine.NewDocumentWithTextAndKey("Hello", 5, &textbuffer.ControlA)
defer doc.Close()

// Get document from buffer for analysis
doc, err := buffer.Document()
defer doc.Close()

// Text analysis
text, err := doc.Text()
pos, err := doc.CursorPosition()
displayPos, err := doc.DisplayCursorPosition()  // Accounts for Unicode width

textBefore, err := doc.TextBeforeCursor()
textAfter, err := doc.TextAfterCursor()

wordBefore, err := doc.GetWordBeforeCursor()
wordAfter, err := doc.GetWordAfterCursor()

// Multi-line analysis
lineCount, err := doc.LineCount()
currentLine, err := doc.CurrentLine()
row, err := doc.CursorPositionRow()
col, err := doc.CursorPositionCol()
```

### Unicode Support

The library properly handles Unicode text with rune-based indexing:

```go
// Unicode text works seamlessly
err = buffer.InsertText("Hello 世界! 🌍", false, true)

// Cursor positions are in rune indices, not byte indices
pos, err := buffer.CursorPosition()           // Rune index
displayPos, err := buffer.DisplayCursorPosition()  // Display width

// All operations work correctly with Unicode
err = buffer.CursorLeft(2)  // Moves 2 runes, not 2 bytes
```

### State Serialization

Both buffers and documents can be serialized for persistence:

```go
// Serialize buffer state
state, err := buffer.ToWasmState()

// Create buffer from serialized state
newBuffer, err := engine.BufferFromWasmState(state)

// Serialize document state
docState, err := doc.ToWasmState()

// Create document from serialized state
newDoc, err := engine.DocumentFromWasmState(docState)
```

### Error Handling

All operations return errors that should be checked:

```go
if err := buffer.InsertText("text", false, true); err != nil {
    log.Printf("Failed to insert text: %v", err)
    return
}
```

## Key Constants

The package provides constants for all supported keys:

```go
// Control keys
textbuffer.ControlA, textbuffer.ControlB, ..., textbuffer.ControlZ

// Arrow keys
textbuffer.Up, textbuffer.Down, textbuffer.Left, textbuffer.Right

// Function keys
textbuffer.F1, textbuffer.F2, ..., textbuffer.F24

// Special keys
textbuffer.Enter, textbuffer.Tab, textbuffer.Backspace, textbuffer.Delete
textbuffer.Home, textbuffer.End, textbuffer.PageUp, textbuffer.PageDown

// And many more...
```

## Examples

See the `_examples/` directory for comprehensive examples:

- `text_buffer_demo.go`: Complete demonstration of all features
- Run with: `go run _examples/text_buffer_demo.go`

## Performance

The library uses WebAssembly for high-performance text operations while maintaining a clean Go API. The Rust backend handles:

- Efficient Unicode processing
- Optimized cursor movement calculations
- Memory-efficient text storage
- Fast text analysis operations

## Thread Safety

The library is **not** thread-safe. Each `TextBufferEngine`, `Buffer`, and `Document` instance should be used from a single goroutine. Create separate instances for concurrent use.

## Memory Management

- Always call `Close()` on engines, buffers, and documents when done
- The WASM runtime manages its own memory
- Go's garbage collector handles the Go wrapper objects

## Compatibility

- Go 1.21 or later
- Uses `github.com/tetratelabs/wazero` for WASM runtime
- Compatible with the original go-prompt API patterns

## Contributing

This package is part of the larger go-prompt port project. See the main repository for contribution guidelines.

## License

Same license as the main go-prompt project.