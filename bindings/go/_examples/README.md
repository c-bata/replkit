# Go Key Parser Demo

This directory contains a demonstration of the Go key parser binding.

## Files

- `go_key_demo.go` - Interactive demo application that shows parsed key events in real-time
- `go_key_demo_test.go` - Integration tests for the Go binding
- `go.mod` - Go module configuration with dependencies

## Running the Demo

From the project root directory:

```bash
cd examples
go run go_key_demo.go
```

The demo will:
1. Set the terminal to raw mode
2. Display instructions
3. Show parsed key events as you type
4. Handle special commands:
   - **Ctrl+C**: Exit gracefully
   - **Ctrl+D**: Flush parser buffer (demonstrates handling of incomplete sequences)
   - **Ctrl+R**: Reset parser state

## Testing

Run the integration tests:

```bash
cd examples
go test -v go_key_demo_test.go
```

The tests verify:
- Basic key parsing (Ctrl+C, arrow keys, Tab, etc.)
- Partial sequence handling (ESC followed by sequence completion)
- Parser flush and reset functionality
- Error handling and resource cleanup

## Features Demonstrated

### Raw Terminal Input
The demo uses `golang.org/x/term` to set the terminal to raw mode, allowing it to capture individual keystrokes including:
- Control characters (Ctrl+A, Ctrl+C, etc.)
- Arrow keys and navigation keys
- Function keys (F1-F24)
- Special sequences (mouse events, bracketed paste)

### Go-Idiomatic Error Handling
The demo follows Go conventions:
- All errors are properly checked and handled
- Resources are cleaned up using `defer`
- Graceful shutdown on signals (SIGINT, SIGTERM)

### Integration with go-prompt Style Applications
The demo shows how to integrate the key parser with applications similar to [go-prompt](https://github.com/c-bata/go-prompt):
- Non-blocking input reading using goroutines
- Event-driven key processing
- Proper terminal state management
- Signal handling for graceful shutdown

## Example Output

```
Go Key Parser Demo
==================
Press keys to see parsed events. Press Ctrl+C to exit.
Try arrow keys, function keys, Ctrl combinations, etc.

Key: Up                  Raw: [0x1b 0x5b 0x41]
Key: Down                Raw: [0x1b 0x5b 0x42]
Key: Ctrl+C              Raw: [0x03]

Received Ctrl+C. Exiting gracefully...
```

## Dependencies

- `golang.org/x/term` - Terminal control for raw mode
- `github.com/tetratelabs/wazero` - WebAssembly runtime (via Go binding)

## Notes

- The demo must be run from the project root directory so it can find the WASM binary at `bindings/go/wasm/prompt_wasm.wasm`
- The terminal is automatically restored to its original state on exit
- The demo handles both normal termination (Ctrl+C) and signal-based termination (SIGINT/SIGTERM)