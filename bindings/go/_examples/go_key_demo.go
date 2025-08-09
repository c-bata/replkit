package main

import (
	"context"
	"fmt"
	"io/ioutil"
	"os"
	"os/signal"
	"syscall"

	keyparsing "github.com/c-bata/prompt/bindings/go"
	"golang.org/x/term"
)

func main() {
	ctx := context.Background()

	// Load the WASM binary
	wasmPath := "bindings/go/wasm/prompt_wasm.wasm"
	wasmBytes, err := ioutil.ReadFile(wasmPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading WASM binary from %s: %v\n", wasmPath, err)
		fmt.Fprintf(os.Stderr, "Make sure to run this from the project root directory\n")
		os.Exit(1)
	}

	// Create a new parser
	parser, err := keyparsing.NewKeyParser(ctx, wasmBytes)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating key parser: %v\n", err)
		os.Exit(1)
	}
	defer func() {
		if err := parser.Close(); err != nil {
			fmt.Fprintf(os.Stderr, "Error closing parser: %v\n", err)
		}
	}()

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// Save original terminal state
	oldState, err := term.MakeRaw(int(os.Stdin.Fd()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error setting raw terminal mode: %v\n", err)
		os.Exit(1)
	}
	defer func() {
		if err := term.Restore(int(os.Stdin.Fd()), oldState); err != nil {
			fmt.Fprintf(os.Stderr, "Error restoring terminal: %v\n", err)
		}
	}()

	fmt.Print("Go Key Parser Demo\n")
	fmt.Print("==================\n")
	fmt.Print("Press keys to see parsed events. Press Ctrl+C to exit.\n")
	fmt.Print("Try arrow keys, function keys, Ctrl combinations, etc.\n\n")

	// Channel for input bytes
	inputChan := make(chan []byte, 100)
	errorChan := make(chan error, 1)

	// Start input reading goroutine
	go func() {
		buffer := make([]byte, 256)
		for {
			n, err := os.Stdin.Read(buffer)
			if err != nil {
				errorChan <- fmt.Errorf("error reading from stdin: %w", err)
				return
			}
			if n > 0 {
				// Make a copy of the data since buffer is reused
				data := make([]byte, n)
				copy(data, buffer[:n])
				inputChan <- data
			}
		}
	}()

	// Main event loop
	for {
		select {
		case <-sigChan:
			fmt.Print("\n\nReceived interrupt signal. Exiting gracefully...\n")
			return

		case err := <-errorChan:
			fmt.Fprintf(os.Stderr, "Input error: %v\n", err)
			return

		case inputBytes := <-inputChan:
			// Parse the input bytes
			events, err := parser.Feed(inputBytes)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error parsing input: %v\n", err)
				continue
			}

			// Display parsed events
			for _, event := range events {
				displayKeyEvent(event)

				// Handle Ctrl+C for graceful exit
				if event.Key == keyparsing.ControlC {
					fmt.Print("\nReceived Ctrl+C. Exiting gracefully...\n")
					return
				}

				// Handle Ctrl+D for demonstration of flush
				if event.Key == keyparsing.ControlD {
					fmt.Print("Flushing parser buffer...\n")
					flushEvents, flushErr := parser.Flush()
					if flushErr != nil {
						fmt.Fprintf(os.Stderr, "Error flushing parser: %v\n", flushErr)
					} else if len(flushEvents) > 0 {
						fmt.Printf("Flushed %d events:\n", len(flushEvents))
						for _, flushEvent := range flushEvents {
							displayKeyEvent(flushEvent)
						}
					} else {
						fmt.Print("No buffered events to flush.\n")
					}
				}

				// Handle Ctrl+R for demonstration of reset
				if event.Key == keyparsing.ControlR {
					fmt.Print("Resetting parser state...\n")
					if resetErr := parser.Reset(); resetErr != nil {
						fmt.Fprintf(os.Stderr, "Error resetting parser: %v\n", resetErr)
					} else {
						fmt.Print("Parser state reset successfully.\n")
					}
				}
			}
		}
	}
}

// displayKeyEvent formats and displays a key event in a user-friendly way
func displayKeyEvent(event keyparsing.KeyEvent) {
	fmt.Printf("Key: %-20s", event.Key.String())

	// Display raw bytes in hex format
	fmt.Printf(" Raw: [")
	for i, b := range event.RawBytes {
		if i > 0 {
			fmt.Print(" ")
		}
		fmt.Printf("0x%02x", b)
	}
	fmt.Print("]")

	// Display text representation if available
	if event.Text != nil && *event.Text != "" {
		fmt.Printf(" Text: %q", *event.Text)
	}

	// Add special handling for certain key types
	switch event.Key {
	case keyparsing.CPRResponse:
		fmt.Print(" (Cursor Position Report)")
	case keyparsing.Vt100MouseEvent:
		fmt.Print(" (VT100 Mouse Event)")
	case keyparsing.WindowsMouseEvent:
		fmt.Print(" (Windows Mouse Event)")
	case keyparsing.BracketedPaste:
		fmt.Print(" (Bracketed Paste)")
	case keyparsing.NotDefined:
		fmt.Print(" (Unknown sequence)")
	case keyparsing.Ignore:
		fmt.Print(" (Ignored sequence)")
	}

	fmt.Print("\n")
}
