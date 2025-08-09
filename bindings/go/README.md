# Go Key Parser Binding

This package provides Go bindings for the Rust-based key input parser using WebAssembly (WASM).

## Features

- **Cross-platform**: Works on any platform supported by Go and WASM
- **Zero dependencies**: Uses wazero for WASM runtime (pure Go implementation)
- **Memory safe**: Proper memory management between Go and WASM
- **Complete key support**: Handles control characters, arrow keys, function keys, and special sequences
- **Partial sequence handling**: Buffers incomplete escape sequences until complete
- **Go-idiomatic API**: Follows Go conventions for error handling and method naming

## Usage

```go
package main

import (
    "context"
    "fmt"
    "io/ioutil"
    
    keyparsing "github.com/c-bata/prompt/bindings/go"
)

func main() {
    ctx := context.Background()
    
    // Load the WASM binary
    wasmBytes, err := ioutil.ReadFile("wasm/prompt_wasm.wasm")
    if err != nil {
        panic(err)
    }
    
    // Create a new parser
    parser, err := keyparsing.NewKeyParser(ctx, wasmBytes)
    if err != nil {
        panic(err)
    }
    defer parser.Close()
    
    // Parse some input
    events, err := parser.Feed([]byte{0x1b, 0x5b, 0x41}) // Up arrow
    if err != nil {
        panic(err)
    }
    
    for _, event := range events {
        fmt.Printf("Key: %s, Raw: %v\n", event.Key, event.RawBytes)
        if event.Text != nil {
            fmt.Printf("Text: %s\n", *event.Text)
        }
    }
    
    // Handle partial sequences
    events, err = parser.Feed([]byte{0x1b}) // Just ESC
    if err != nil {
        panic(err)
    }
    fmt.Printf("Partial sequence events: %d\n", len(events)) // Should be 0
    
    // Complete the sequence
    events, err = parser.Feed([]byte{0x5b, 0x42}) // [B for Down arrow
    if err != nil {
        panic(err)
    }
    fmt.Printf("Completed sequence: %s\n", events[0].Key) // Should be "Down"
    
    // Reset parser state if needed
    err = parser.Reset()
    if err != nil {
        panic(err)
    }
    
    // Flush any remaining buffered input
    events, err = parser.Flush()
    if err != nil {
        panic(err)
    }
}
```

## API Reference

### Types

#### `Key`
Represents different types of keys that can be parsed. Includes:
- Control characters: `ControlA`, `ControlB`, `ControlC`, etc.
- Arrow keys: `Up`, `Down`, `Left`, `Right`
- Function keys: `F1` through `F24`
- Special keys: `Tab`, `Enter`, `Escape`, `Backspace`, `Delete`
- Modifier combinations: `ShiftUp`, `ControlLeft`, etc.

#### `KeyEvent`
Represents a parsed key event with:
- `Key`: The parsed key type
- `RawBytes`: The original raw bytes that produced this key event
- `Text`: Optional text representation (for printable characters)

### Methods

#### `NewKeyParser(ctx context.Context, wasmBytes []byte) (*KeyParser, error)`
Creates a new KeyParser instance using the provided WASM binary.

#### `Feed(input []byte) ([]KeyEvent, error)`
Processes input bytes and returns parsed key events. Partial sequences are buffered internally.

#### `Flush() ([]KeyEvent, error)`
Processes any remaining buffered input and returns key events. Call this when input is complete.

#### `Reset() error`
Clears the parser state, discarding any buffered partial sequences.

#### `Close() error`
Releases all resources and marks the parser as closed.

## Error Handling

The Go binding follows Go conventions for error handling:
- All methods that can fail return an error as the last return value
- Errors are wrapped with context using `fmt.Errorf` with `%w` verb
- Nil pointer checks are performed to prevent panics
- Parser state is validated before operations

## Testing

Run the test suite:

```bash
go test -v
```

The tests include:
- Unit tests for key constants and structures
- Integration tests with the WASM module
- Error handling validation
- Partial sequence handling
- Memory management verification

## Dependencies

- [wazero](https://github.com/tetratelabs/wazero): WebAssembly runtime for Go (pure Go, no CGO required)