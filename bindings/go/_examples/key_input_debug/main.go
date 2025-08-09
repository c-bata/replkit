package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"unicode"

	keyparsing "github.com/c-bata/replkit/bindings/go"
	"golang.org/x/term"
)

func main() {
	ctx := context.Background()

	// Create a new parser using the embedded WASM binary
	parser, err := keyparsing.New(ctx)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating key parser: %v\r\n", err)
		os.Exit(1)
	}
	defer func() {
		if err := parser.Close(); err != nil {
			fmt.Fprintf(os.Stderr, "Error closing parser: %v\r\n", err)
		}
	}()

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// Save original terminal state
	oldState, err := term.MakeRaw(int(os.Stdin.Fd()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error setting raw terminal mode: %v\r\n", err)
		os.Exit(1)
	}
	defer func() {
		if err := term.Restore(int(os.Stdin.Fd()), oldState); err != nil {
			fmt.Fprintf(os.Stderr, "Error restoring terminal: %v\r\n", err)
		}
	}()

	fmt.Print("Go Key Parser Demo\r\n")
	fmt.Print("==================\r\n")
	fmt.Print("Press keys to see parsed events. Press Ctrl+C to exit.\r\n")
	fmt.Print("Try arrow keys, function keys, Ctrl combinations, etc.\r\n\r\n")

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
			fmt.Print("\r\n\r\nReceived interrupt signal. Exiting gracefully...\r\n")
			return

		case err := <-errorChan:
			fmt.Fprintf(os.Stderr, "Input error: %v\r\n", err)
			return

		case inputBytes := <-inputChan:
			// Parse the input bytes
			events, err := parser.Feed(inputBytes)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error parsing input: %v\r\n", err)
				continue
			}

			// Display parsed events
			for _, event := range events {
				displayKeyEvent(event)

				// Handle Ctrl+C for graceful exit
				if event.Key == keyparsing.ControlC {
					fmt.Print("\r\nReceived Ctrl+C. Exiting gracefully...\r\n")
					return
				}

				// Handle Ctrl+D for demonstration of flush
				if event.Key == keyparsing.ControlD {
					fmt.Print("Flushing parser buffer...\r\n")
					flushEvents, flushErr := parser.Flush()
					if flushErr != nil {
						fmt.Fprintf(os.Stderr, "Error flushing parser: %v\r\n", flushErr)
					} else if len(flushEvents) > 0 {
						fmt.Printf("Flushed %d events:\r\n", len(flushEvents))
						for _, flushEvent := range flushEvents {
							displayKeyEvent(flushEvent)
						}
					} else {
						fmt.Print("No buffered events to flush.\r\n")
					}
				}

				// Handle Ctrl+R for demonstration of reset
				if event.Key == keyparsing.ControlR {
					fmt.Print("Resetting parser state...\r\n")
					if resetErr := parser.Reset(); resetErr != nil {
						fmt.Fprintf(os.Stderr, "Error resetting parser: %v\r\n", resetErr)
					} else {
						fmt.Print("Parser state reset successfully.\r\n")
					}
				}
			}
		}
	}
}

// displayKeyEvent formats and displays a key event in a user-friendly way
func displayKeyEvent(event keyparsing.KeyEvent) {
	// Create a more readable format with proper spacing
	fmt.Printf("┌─ Key Event ─────────────────────────────────────────────────────────────────┐\r\n")
	fmt.Printf("│ Key: %-20s", event.Key.String())

	// Display raw bytes in hex format
	fmt.Printf(" Raw: [")
	for i, b := range event.RawBytes {
		if i > 0 {
			fmt.Print(" ")
		}
		fmt.Printf("0x%02x", b)
	}
	fmt.Printf("]")

	// Add padding to align the closing border
	padding := 75 - len(event.Key.String()) - (len(event.RawBytes)*5 + 8) // Approximate calculation
	if padding > 0 {
		fmt.Printf("%*s", padding, "")
	}
	fmt.Printf(" │\r\n")

	// Display text representation if available
	if event.Text != nil && *event.Text != "" {
		fmt.Printf("│ Text: %-10q", *event.Text)

		// Check if it's a printable character
		if len(*event.Text) == 1 {
			r := rune((*event.Text)[0])
			if unicode.IsPrint(r) && event.Key == keyparsing.NotDefined {
				fmt.Printf(" (Printable character)")
			}
		}
		fmt.Printf("%*s │\r\n", 50, "")
	}

	// Add special handling for certain key types
	var description string
	switch event.Key {
	case keyparsing.CPRResponse:
		description = "Cursor Position Report"
	case keyparsing.Vt100MouseEvent:
		description = "VT100 Mouse Event"
	case keyparsing.WindowsMouseEvent:
		description = "Windows Mouse Event"
	case keyparsing.BracketedPaste:
		description = "Bracketed Paste"
	case keyparsing.NotDefined:
		if event.Text != nil && len(*event.Text) == 1 {
			r := rune((*event.Text)[0])
			if unicode.IsPrint(r) {
				description = "Printable character"
			} else {
				description = "Unknown sequence"
			}
		} else {
			description = "Unknown sequence"
		}
	case keyparsing.Ignore:
		description = "Ignored sequence"
	}

	if description != "" {
		fmt.Printf("│ Type: %-20s", description)
		fmt.Printf("%*s │\r\n", 50, "")
	}

	fmt.Printf("└─────────────────────────────────────────────────────────────────────────────┘\r\n\r\n")
}
