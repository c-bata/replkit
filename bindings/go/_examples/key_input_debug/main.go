package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	keyparsing "github.com/c-bata/replkit/bindings/go"
)

func main() {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	console, err := keyparsing.NewConsoleInput(ctx)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating console input: %v\r\n", err)
		os.Exit(1)
	}
	defer console.Close()

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	if err := console.EnableRawMode(); err != nil {
		fmt.Fprintf(os.Stderr, "Error enabling raw mode: %v\r\n", err)
		os.Exit(1)
	}
	defer console.DisableRawMode()

	fmt.Print("Go Key Input Debug Tool\r\n")
	fmt.Print("Press keys to see events. Press Ctrl+C to exit.\r\n\r\n")

	// Display initial window size
	if size, err := console.GetWindowSize(); err == nil {
		fmt.Printf("[window size] cols=%d, rows=%d\r\n", size.Columns, size.Rows)
	}

	fmt.Print("Ready for input...\r\n")

	// Monitor window size changes in a separate goroutine
	go func() {
		for {
			select {
			case size := <-console.WindowSizeChanges():
				fmt.Printf("[window resized] cols=%d, rows=%d\r\n", size.Columns, size.Rows)
			case <-ctx.Done():
				return
			}
		}
	}()

	for {
		select {
		case <-sigChan:
			fmt.Print("\r\nReceived interrupt signal. Exiting...\r\n")
			cancel()
			return

		default:
			event, err := console.ReadKey(50 * time.Millisecond)
			if err != nil {
				if err == context.Canceled {
					return
				}
				continue
			}

			if event == nil {
				continue
			}

			displayKeyEvent(*event)

			if event.Key == keyparsing.ControlC {
				fmt.Print("Received Ctrl+C, shutting down...\r\n")
				cancel()
				return
			}
		}
	}
}

// Format raw bytes for display
func formatBytes(bytes []byte) string {
	if len(bytes) == 0 {
		return "[]"
	}

	result := "["
	for i, b := range bytes {
		if i > 0 {
			result += " "
		}
		result += fmt.Sprintf("%02x", b)
	}
	result += "]"
	return result
}

// Display key event in simple one-line format
func displayKeyEvent(event keyparsing.KeyEvent) {
	rawBytes := formatBytes(event.RawBytes)
	textPart := ""
	if event.Text != nil && *event.Text != "" {
		textPart = fmt.Sprintf(" | Text: %q", *event.Text)
	}

	fmt.Printf("Key: %s | Raw: %s%s\r\n", event.Key.String(), rawBytes, textPart)
}
