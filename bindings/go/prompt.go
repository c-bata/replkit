package replkit

import (
	"context"
	"fmt"
	"path/filepath"
	"strings"
)

// Document represents the current state of the input text and cursor
type Document struct {
	text           string
	cursorPosition int
}

// NewDocument creates a new Document with the given text and cursor position
func NewDocument(text string, cursorPosition int) *Document {
	return &Document{
		text:           text,
		cursorPosition: cursorPosition,
	}
}

// GetWordBeforeCursor returns the word before the cursor
func (d *Document) GetWordBeforeCursor() string {
	textBeforeCursor := d.text[:d.cursorPosition]
	
	// Find the last space or beginning of string
	words := strings.Fields(textBeforeCursor)
	if len(words) == 0 {
		return ""
	}
	
	// Check if the last character is a space (incomplete word)
	if len(textBeforeCursor) > 0 && textBeforeCursor[len(textBeforeCursor)-1] == ' ' {
		return ""
	}
	
	return words[len(words)-1]
}

// Text returns the full text content
func (d *Document) Text() string {
	return d.text
}

// CursorPosition returns the current cursor position
func (d *Document) CursorPosition() int {
	return d.cursorPosition
}

// Completer is a function that provides suggestions based on the current document
type Completer func(*Document) []Suggestion

// FilterHasPrefix filters suggestions that have the given prefix
func FilterHasPrefix(suggestions []Suggestion, prefix string, ignoreCase bool) []Suggestion {
	var result []Suggestion
	
	for _, s := range suggestions {
		var match bool
		if ignoreCase {
			match = strings.HasPrefix(strings.ToLower(s.Text), strings.ToLower(prefix))
		} else {
			match = strings.HasPrefix(s.Text, prefix)
		}
		
		if match {
			result = append(result, s)
		}
	}
	
	return result
}

// PromptOption represents configuration options for the prompt
type PromptOption func(*PromptConfig) error

// PromptConfig holds the configuration for a prompt session
type PromptConfig struct {
	prefix     string
	completer  Completer
	wasmPath   string
	wasmOutput *WasmOutput
}

// WithPrefix sets the prompt prefix
func WithPrefix(prefix string) PromptOption {
	return func(config *PromptConfig) error {
		config.prefix = prefix
		return nil
	}
}

// WithCompleter sets the completer function
func WithCompleter(completer Completer) PromptOption {
	return func(config *PromptConfig) error {
		config.completer = completer
		return nil
	}
}

// WithWasmPath sets the path to the WASM file
func WithWasmPath(wasmPath string) PromptOption {
	return func(config *PromptConfig) error {
		config.wasmPath = wasmPath
		return nil
	}
}

// Input provides a simple input function similar to go-prompt's Input
func Input(prefix string, completer Completer, opts ...PromptOption) (string, error) {
	ctx := context.Background()
	
	// Default configuration
	config := &PromptConfig{
		prefix:    prefix,
		completer: completer,
		wasmPath:  "wasm/replkit_wasm.wasm", // Default path
	}
	
	// Apply options
	for _, opt := range opts {
		if err := opt(config); err != nil {
			return "", fmt.Errorf("failed to apply option: %w", err)
		}
	}
	
	// Try to find WASM file in common locations
	wasmPath := config.wasmPath
	if !filepath.IsAbs(wasmPath) {
		// Try current directory first
		if _, err := filepath.Abs(wasmPath); err == nil {
			wasmPath, _ = filepath.Abs(wasmPath)
		}
	}
	
	// Initialize WASM output handler
	wasmOutput, err := NewWasmOutput(wasmPath)
	if err != nil {
		return "", fmt.Errorf("failed to initialize WASM output: %w", err)
	}
	defer wasmOutput.Close(ctx)
	
	config.wasmOutput = wasmOutput
	
	// For now, implement a simple input loop
	// This is a basic implementation - in a real scenario, you would:
	// 1. Set terminal to raw mode
	// 2. Read key events
	// 3. Handle completion with Tab key
	// 4. Handle cursor movement, backspace, etc.
	// 5. Render the prompt and suggestions
	
	// Display the prefix
	if err := wasmOutput.WriteText(ctx, prefix); err != nil {
		return "", fmt.Errorf("failed to write prefix: %w", err)
	}
	
	// Simple implementation: read a line from stdin
	var input string
	if _, err := fmt.Scanln(&input); err != nil {
		return "", fmt.Errorf("failed to read input: %w", err)
	}
	
	// Demonstrate completion functionality
	if config.completer != nil {
		document := NewDocument(input, len(input))
		suggestions := config.completer(document)
		
		if len(suggestions) > 0 {
			// Show suggestions using WASM
			fmt.Println("\nSuggestions:")
			for _, suggestion := range suggestions {
				suggestionText := fmt.Sprintf("  %s - %s\n", suggestion.Text, suggestion.Description)
				if err := wasmOutput.WriteStyledText(ctx, suggestionText, BoldWithForeground(BasicColor("Green"))); err != nil {
					return "", fmt.Errorf("failed to write suggestion: %w", err)
				}
			}
		}
	}
	
	return input, nil
}

// SimplePrompt provides a minimal implementation for testing
type SimplePrompt struct {
	prefix     string
	completer  Completer
	wasmOutput *WasmOutput
}

// NewSimplePrompt creates a new simple prompt instance
func NewSimplePrompt(prefix string, completer Completer, wasmPath string) (*SimplePrompt, error) {
	wasmOutput, err := NewWasmOutput(wasmPath)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize WASM output: %w", err)
	}
	
	return &SimplePrompt{
		prefix:     prefix,
		completer:  completer,
		wasmOutput: wasmOutput,
	}, nil
}

// Close releases resources
func (sp *SimplePrompt) Close(ctx context.Context) error {
	if sp.wasmOutput != nil {
		return sp.wasmOutput.Close(ctx)
	}
	return nil
}

// Input reads input with completion support
func (sp *SimplePrompt) Input(ctx context.Context) (string, error) {
	// Write prefix with styling
	prefixStyle := BoldWithForeground(BasicColor("Blue"))
	if err := sp.wasmOutput.WriteStyledText(ctx, sp.prefix, prefixStyle); err != nil {
		return "", fmt.Errorf("failed to write prefix: %w", err)
	}
	
	// Simple input reading (in a real implementation, this would be more sophisticated)
	var input string
	if _, err := fmt.Scanln(&input); err != nil {
		return "", fmt.Errorf("failed to read input: %w", err)
	}
	
	// Test filtering suggestions via WASM
	if sp.completer != nil {
		document := NewDocument(input, len(input))
		allSuggestions := sp.completer(document)
		
		if len(allSuggestions) > 0 {
			prefix := document.GetWordBeforeCursor()
			if prefix != "" {
				// Use WASM to filter suggestions
				filtered, err := sp.wasmOutput.FilterSuggestions(ctx, allSuggestions, prefix, true)
				if err != nil {
					return "", fmt.Errorf("failed to filter suggestions: %w", err)
				}
				
				if len(filtered) > 0 {
					fmt.Printf("\nFiltered suggestions for '%s':\n", prefix)
					for _, suggestion := range filtered {
						suggestionText := fmt.Sprintf("  %s - %s\n", suggestion.Text, suggestion.Description)
						if err := sp.wasmOutput.WriteStyledText(ctx, suggestionText, BoldWithForeground(BasicColor("Green"))); err != nil {
							return "", fmt.Errorf("failed to write suggestion: %w", err)
						}
					}
				}
			}
		}
	}
	
	return input, nil
}