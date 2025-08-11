package main

import (
	"context"
	"fmt"
	"log"
	"path/filepath"

	replkit "github.com/c-bata/replkit/bindings/go"
)

func main() {
	ctx := context.Background()
	
	// Get the WASM file path
	wasmPath, err := filepath.Abs("../../wasm/replkit_wasm.wasm")
	if err != nil {
		log.Fatalf("Failed to get WASM path: %v", err)
	}
	
	// Initialize WASM output handler
	wasmOutput, err := replkit.NewWasmOutput(wasmPath)
	if err != nil {
		log.Fatalf("Failed to initialize WASM output: %v", err)
	}
	defer wasmOutput.Close(ctx)
	
	// Test basic text output
	fmt.Println("Testing WASM output functionality:")
	
	// Test plain text
	if err := wasmOutput.WriteText(ctx, "Hello, World!\n"); err != nil {
		log.Fatalf("Failed to write text: %v", err)
	}
	
	// Test styled text
	redBold := &replkit.SerializableTextStyle{
		Foreground: replkit.BasicColor("Red"),
		Bold:       true,
	}
	if err := wasmOutput.WriteStyledText(ctx, "This is red and bold text!\n", redBold); err != nil {
		log.Fatalf("Failed to write styled text: %v", err)
	}
	
	// Test cursor movement
	if err := wasmOutput.MoveCursorTo(ctx, 10, 20); err != nil {
		log.Fatalf("Failed to move cursor: %v", err)
	}
	
	if err := wasmOutput.WriteText(ctx, "Text at position (10, 20)\n"); err != nil {
		log.Fatalf("Failed to write positioned text: %v", err)
	}
	
	// Test clear screen
	if err := wasmOutput.Clear(ctx, "All"); err != nil {
		log.Fatalf("Failed to clear screen: %v", err)
	}
	
	// Test suggestion filtering
	fmt.Println("Testing suggestion filtering:")
	
	suggestions := []replkit.Suggestion{
		{Text: "users", Description: "Store the username and age"},
		{Text: "articles", Description: "Store the article text posted by user"},
		{Text: "comments", Description: "Store the text commented to articles"},
		{Text: "uploads", Description: "Handle file uploads"},
	}
	
	filtered, err := wasmOutput.FilterSuggestions(ctx, suggestions, "u", true)
	if err != nil {
		log.Fatalf("Failed to filter suggestions: %v", err)
	}
	
	fmt.Printf("Suggestions starting with 'u': %d found\n", len(filtered))
	for _, s := range filtered {
		fmt.Printf("  %s - %s\n", s.Text, s.Description)
	}
	
	fmt.Println("WASM test completed successfully!")
}