package replkit

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
)

// PromptState represents the complete state needed for rendering
type PromptState struct {
	Prefix              string        `json:"prefix"`
	InputText           string        `json:"input_text"`
	CursorPosition      int           `json:"cursor_position"`
	Suggestions         []Suggestion  `json:"suggestions"`
	SelectedSuggestion  *int          `json:"selected_suggestion,omitempty"`
	ShowSuggestions     bool          `json:"show_suggestions"`
	WindowSize          *[2]uint16    `json:"window_size,omitempty"`
}

// RenderOutput represents the rendered output from WASM
type RenderOutput struct {
	Success      bool    `json:"success"`
	ErrorMessage *string `json:"error_message,omitempty"`
	OutputBytes  []byte  `json:"output_bytes,omitempty"`
	CursorRow    uint16  `json:"cursor_row"`
	CursorCol    uint16  `json:"cursor_col"`
}

// SimpleRenderer provides simple prompt rendering via WASM
type SimpleRenderer struct {
	wasmOutput *WasmOutput
}

// NewSimpleRenderer creates a new renderer
func NewSimpleRenderer(wasmPath string) (*SimpleRenderer, error) {
	wasmOutput, err := NewWasmOutput(wasmPath)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize WASM: %w", err)
	}
	
	return &SimpleRenderer{
		wasmOutput: wasmOutput,
	}, nil
}

// Close releases resources
func (sr *SimpleRenderer) Close(ctx context.Context) error {
	if sr.wasmOutput != nil {
		return sr.wasmOutput.Close(ctx)
	}
	return nil
}

// RenderPrompt renders the complete prompt and returns the output bytes
func (sr *SimpleRenderer) RenderPrompt(ctx context.Context, state *PromptState) (*RenderOutput, error) {
	// Serialize prompt state to JSON
	stateJSON, err := json.Marshal(state)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal prompt state: %w", err)
	}
	
	// Get WASM functions
	mallocFn := sr.wasmOutput.module.ExportedFunction("malloc")
	freeFn := sr.wasmOutput.module.ExportedFunction("free")
	renderFn := sr.wasmOutput.module.ExportedFunction("wasm_render_prompt")
	
	if mallocFn == nil || freeFn == nil || renderFn == nil {
		return nil, fmt.Errorf("required WASM functions not found")
	}
	
	// Allocate memory for state JSON
	stateResults, err := mallocFn.Call(ctx, uint64(len(stateJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to allocate WASM memory: %w", err)
	}
	statePtr := uint32(stateResults[0])
	
	// Write state JSON to WASM memory
	if !sr.wasmOutput.module.Memory().Write(statePtr, stateJSON) {
		freeFn.Call(ctx, uint64(statePtr))
		return nil, fmt.Errorf("failed to write state to WASM memory")
	}
	
	// Call wasm_render_prompt
	results, err := renderFn.Call(ctx, uint64(statePtr), uint64(len(stateJSON)))
	if err != nil {
		freeFn.Call(ctx, uint64(statePtr))
		return nil, fmt.Errorf("failed to call wasm_render_prompt: %w", err)
	}
	
	// Free state memory
	freeFn.Call(ctx, uint64(statePtr))
	
	// Parse result (packed pointer and length)
	packedResult := results[0]
	resultPtr := uint32(packedResult >> 32)
	resultLen := uint32(packedResult & 0xFFFFFFFF)
	
	if resultPtr == 0 {
		return nil, fmt.Errorf("WASM function returned null pointer")
	}
	
	// Read result JSON from WASM memory
	resultJSON, ok := sr.wasmOutput.module.Memory().Read(resultPtr, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from WASM memory")
	}
	
	// Free result memory
	freeFn.Call(ctx, uint64(resultPtr))
	
	// Parse result JSON
	var output RenderOutput
	if err := json.Unmarshal(resultJSON, &output); err != nil {
		return nil, fmt.Errorf("failed to parse render output JSON: %w", err)
	}
	
	return &output, nil
}

// WriteBytes writes raw bytes to stdout
func (sr *SimpleRenderer) WriteBytes(bytes []byte) error {
	_, err := os.Stdout.Write(bytes)
	return err
}

// InteractivePrompt provides a complete interactive prompt experience
type InteractivePrompt struct {
	renderer     *SimpleRenderer
	consoleInput *ConsoleInput
	prefix       string
	completer    Completer
	state        *PromptState
}

// NewInteractivePrompt creates a new interactive prompt
func NewInteractivePrompt(wasmPath, prefix string, completer Completer) (*InteractivePrompt, error) {
	ctx := context.Background()
	
	renderer, err := NewSimpleRenderer(wasmPath)
	if err != nil {
		return nil, err
	}
	
	consoleInput, err := NewConsoleInput(ctx)
	if err != nil {
		renderer.Close(ctx)
		return nil, fmt.Errorf("failed to create console input: %w", err)
	}
	
	state := &PromptState{
		Prefix:          prefix,
		InputText:       "",
		CursorPosition:  0,
		Suggestions:     []Suggestion{},
		ShowSuggestions: false,
	}
	
	return &InteractivePrompt{
		renderer:     renderer,
		consoleInput: consoleInput,
		prefix:       prefix,
		completer:    completer,
		state:        state,
	}, nil
}

// Close releases resources
func (ip *InteractivePrompt) Close(ctx context.Context) error {
	var err error
	if ip.consoleInput != nil {
		if closeErr := ip.consoleInput.Close(); closeErr != nil {
			err = closeErr
		}
	}
	if ip.renderer != nil {
		if closeErr := ip.renderer.Close(ctx); closeErr != nil {
			err = closeErr
		}
	}
	return err
}

// Input runs the interactive prompt and returns the final input
func (ip *InteractivePrompt) Input(ctx context.Context) (string, error) {
	// Enable raw mode
	if err := ip.consoleInput.EnableRawMode(); err != nil {
		return "", fmt.Errorf("failed to enable raw mode: %w", err)
	}
	defer ip.consoleInput.DisableRawMode()
	
	// Start input processing
	if err := ip.consoleInput.Start(); err != nil {
		return "", fmt.Errorf("failed to start console input: %w", err)
	}
	defer ip.consoleInput.Stop()
	
	// Initial render
	ip.updateSuggestions()
	if err := ip.render(ctx); err != nil {
		return "", err
	}
	
	// Main input loop
	for {
		keyEvent, err := ip.consoleInput.ReadKey()
		if err != nil {
			return "", fmt.Errorf("failed to read key: %w", err)
		}
		
		// Handle key events
		if handled, result := ip.handleKeyEvent(keyEvent, ctx); handled {
			if result != nil {
				return *result, nil
			}
		}
	}
}

// handleKeyEvent processes a key event and returns whether it was handled and optional result
func (ip *InteractivePrompt) handleKeyEvent(event KeyEvent, ctx context.Context) (bool, *string) {
	switch event.Key {
	case Enter, ControlJ, ControlM:
		// Submit current input
		result := ip.state.InputText
		fmt.Print("\n")
		return true, &result
		
	case ControlC:
		// Cancel
		fmt.Print("\n")
		result := ""
		return true, &result
		
	case ControlD:
		if ip.state.InputText == "" {
			// EOF on empty input
			fmt.Print("\n")
			result := ""
			return true, &result
		}
		
	case Backspace, ControlH:
		// Delete character before cursor
		if ip.state.CursorPosition > 0 {
			text := ip.state.InputText
			newText := text[:ip.state.CursorPosition-1] + text[ip.state.CursorPosition:]
			ip.state.InputText = newText
			ip.state.CursorPosition--
			ip.updateSuggestions()
			ip.render(ctx)
		}
		return true, nil
		
	case Delete:
		// Delete character after cursor
		if ip.state.CursorPosition < len(ip.state.InputText) {
			text := ip.state.InputText
			newText := text[:ip.state.CursorPosition] + text[ip.state.CursorPosition+1:]
			ip.state.InputText = newText
			ip.updateSuggestions()
			ip.render(ctx)
		}
		return true, nil
		
	case Left, ControlB:
		// Move cursor left
		if ip.state.CursorPosition > 0 {
			ip.state.CursorPosition--
			ip.render(ctx)
		}
		return true, nil
		
	case Right, ControlF:
		// Move cursor right
		if ip.state.CursorPosition < len(ip.state.InputText) {
			ip.state.CursorPosition++
			ip.render(ctx)
		}
		return true, nil
		
	case NotDefined:
		// Regular character input
		if event.Text != "" {
			// Insert character at cursor position
			text := ip.state.InputText
			newText := text[:ip.state.CursorPosition] + event.Text + text[ip.state.CursorPosition:]
			ip.state.InputText = newText
			ip.state.CursorPosition += len(event.Text)
			ip.updateSuggestions()
			ip.render(ctx)
		}
		return true, nil
	}
	
	return false, nil
}

// updateSuggestions updates the suggestions based on current input
func (ip *InteractivePrompt) updateSuggestions() {
	if ip.completer != nil {
		doc := NewDocument(ip.state.InputText, ip.state.CursorPosition)
		suggestions := ip.completer(doc)
		ip.state.Suggestions = suggestions
		ip.state.ShowSuggestions = len(suggestions) > 0
		if len(suggestions) > 0 {
			selectedIndex := 0
			ip.state.SelectedSuggestion = &selectedIndex
		} else {
			ip.state.SelectedSuggestion = nil
		}
	}
}

// render renders the current prompt state
func (ip *InteractivePrompt) render(ctx context.Context) error {
	output, err := ip.renderer.RenderPrompt(ctx, ip.state)
	if err != nil {
		return err
	}
	
	if !output.Success {
		return fmt.Errorf("render failed: %s", *output.ErrorMessage)
	}
	
	return ip.renderer.WriteBytes(output.OutputBytes)
}

// GetInput returns the current input text
func (ip *InteractivePrompt) GetInput() string {
	return ip.state.InputText
}

// GetSuggestions returns current suggestions
func (ip *InteractivePrompt) GetSuggestions() []Suggestion {
	return ip.state.Suggestions
}