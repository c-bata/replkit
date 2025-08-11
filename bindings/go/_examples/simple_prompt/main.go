package main

import (
	"context"
	"fmt"
	"log"
	"path/filepath"

	replkit "github.com/c-bata/replkit/bindings/go"
)

// completer provides the same suggestions as the Rust version
func completer(doc *replkit.Document) []replkit.Suggestion {
	suggestions := []replkit.Suggestion{
		{Text: "users", Description: "Store the username and age"},
		{Text: "articles", Description: "Store the article text posted by user"},
		{Text: "comments", Description: "Store the text commented to articles"},
	}
	
	// Filter based on the word before cursor (case-insensitive prefix matching)
	prefix := doc.GetWordBeforeCursor()
	if prefix == "" {
		return suggestions
	}
	
	return replkit.FilterHasPrefix(suggestions, prefix, true)
}

func main() {
	ctx := context.Background()
	
	fmt.Println("Please select table.")
	
	// Get the WASM file path relative to the example directory
	wasmPath, err := filepath.Abs("../../wasm/replkit_wasm.wasm")
	if err != nil {
		log.Fatalf("Failed to get WASM path: %v", err)
	}
	
	// Create an interactive prompt
	prompt, err := replkit.NewInteractivePrompt(wasmPath, "> ", completer)
	if err != nil {
		log.Fatalf("Failed to create prompt: %v", err)
	}
	defer func() {
		if err := prompt.Close(ctx); err != nil {
			log.Printf("Failed to close prompt: %v", err)
		}
	}()
	
	// Run the interactive prompt - handles raw mode internally
	result, err := prompt.Input(ctx)
	if err != nil {
		log.Fatalf("Failed to get input: %v", err)
	}
	
	fmt.Printf("You selected %s\n", result)
}