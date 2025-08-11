package replkit

import (
	"context"
	"encoding/json"
	"fmt"
	"os"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// OutputCommand represents console output commands to send to WASM
type OutputCommand struct {
	Type      string                 `json:"type"`
	Text      string                 `json:"text,omitempty"`
	Style     *SerializableTextStyle `json:"style,omitempty"`
	Row       *uint16                `json:"row,omitempty"`
	Col       *uint16                `json:"col,omitempty"`
	RowDelta  *int16                 `json:"row_delta,omitempty"`
	ColDelta  *int16                 `json:"col_delta,omitempty"`
	ClearType *string                `json:"clear_type,omitempty"`
	Enabled   *bool                  `json:"enabled,omitempty"`
	Visible   *bool                  `json:"visible,omitempty"`
}

// SerializableTextStyle represents text styling options
type SerializableTextStyle struct {
	Foreground    SerializableColor `json:"foreground,omitempty"`
	Background    SerializableColor `json:"background,omitempty"`
	Bold          bool              `json:"bold"`
	Italic        bool              `json:"italic"`
	Underline     bool              `json:"underline"`
	Strikethrough bool              `json:"strikethrough"`
	Dim           bool              `json:"dim"`
	Reverse       bool              `json:"reverse"`
}

// SerializableColor represents color values - using interface for Rust enum compatibility
type SerializableColor interface {
	json.Marshaler
}

// Basic color types
type BasicColorType string

const (
	ColorBlack         BasicColorType = "Black"
	ColorRed           BasicColorType = "Red"
	ColorGreen         BasicColorType = "Green"
	ColorYellow        BasicColorType = "Yellow"
	ColorBlue          BasicColorType = "Blue"
	ColorMagenta       BasicColorType = "Magenta"
	ColorCyan          BasicColorType = "Cyan"
	ColorWhite         BasicColorType = "White"
	ColorBrightBlack   BasicColorType = "BrightBlack"
	ColorBrightRed     BasicColorType = "BrightRed"
	ColorBrightGreen   BasicColorType = "BrightGreen"
	ColorBrightYellow  BasicColorType = "BrightYellow"
	ColorBrightBlue    BasicColorType = "BrightBlue"
	ColorBrightMagenta BasicColorType = "BrightMagenta"
	ColorBrightCyan    BasicColorType = "BrightCyan"
	ColorBrightWhite   BasicColorType = "BrightWhite"
)

func (b BasicColorType) MarshalJSON() ([]byte, error) {
	return json.Marshal(string(b))
}

// RGB color type
type RgbColor struct {
	R, G, B uint8
}

func (r RgbColor) MarshalJSON() ([]byte, error) {
	return json.Marshal(struct {
		Rgb [3]uint8 `json:"Rgb"`
	}{
		Rgb: [3]uint8{r.R, r.G, r.B},
	})
}

// ANSI 256 color type
type Ansi256Color struct {
	Code uint8
}

func (a Ansi256Color) MarshalJSON() ([]byte, error) {
	return json.Marshal(struct {
		Ansi256 uint8 `json:"Ansi256"`
	}{
		Ansi256: a.Code,
	})
}

// OutputResponse represents the response from WASM output processing
type OutputResponse struct {
	Success        bool     `json:"success"`
	ErrorMessage   *string  `json:"error_message,omitempty"`
	EscapeSequence *string  `json:"escape_sequence,omitempty"`
	CursorPosition *[2]uint16 `json:"cursor_position,omitempty"`
}

// Suggestion represents a completion suggestion for the prompt
type Suggestion struct {
	Text        string `json:"text"`
	Description string `json:"description"`
}

// WasmOutput provides console output functionality via WASM
type WasmOutput struct {
	runtime wazero.Runtime
	module  api.Module
}

// NewWasmOutput creates a new WASM-based console output handler
func NewWasmOutput(wasmPath string) (*WasmOutput, error) {
	ctx := context.Background()
	
	// Create a new WASM runtime
	runtime := wazero.NewRuntime(ctx)
	
	// Read WASM binary first to get memory requirements
	wasmBytes, err := os.ReadFile(wasmPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read WASM file: %w", err)
	}
	
	// Compile the WASM module to analyze it
	module, err := runtime.CompileModule(ctx, wasmBytes)
	if err != nil {
		return nil, fmt.Errorf("failed to compile WASM module: %w", err)
	}
	
	// Create env module with proper memory allocation functions
	builder := runtime.NewHostModuleBuilder("env")
	
	var wasmMemory api.Memory
	
	builder.NewFunctionBuilder().
		WithFunc(func(ctx context.Context, m api.Module, size uint32) uint32 {
			// Get WASM memory and allocate 
			if wasmMemory == nil {
				wasmMemory = m.Memory()
			}
			// Simple bump allocator - start from page 1 (64KB)
			// In a real implementation, this would be more sophisticated
			currentSize := wasmMemory.Size()
			if currentSize == 0 {
				return 0
			}
			// Return a valid memory offset (simplified)
			return uint32(65536) // Start after first page
		}).
		Export("__wbindgen_malloc")
	
	builder.NewFunctionBuilder().
		WithFunc(func(ctx context.Context, m api.Module, ptr uint32, size uint32) {
			// Free implementation - no-op for now since we have a simple allocator
		}).
		Export("__wbindgen_free")
	
	_, err = builder.Instantiate(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create env module: %w", err)
	}
	
	// Instantiate the compiled WASM module
	wasmModule, err := runtime.InstantiateModule(ctx, module, wazero.NewModuleConfig())
	if err != nil {
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}
	
	return &WasmOutput{
		runtime: runtime,
		module:  wasmModule,
	}, nil
}

// Close releases WASM runtime resources
func (w *WasmOutput) Close(ctx context.Context) error {
	if w.module != nil {
		if err := w.module.Close(ctx); err != nil {
			return err
		}
	}
	if w.runtime != nil {
		return w.runtime.Close(ctx)
	}
	return nil
}

// ProcessCommand sends a command to WASM and returns the escape sequence
func (w *WasmOutput) ProcessCommand(ctx context.Context, cmd OutputCommand) (*OutputResponse, error) {
	// Serialize command to JSON
	cmdJSON, err := json.Marshal(cmd)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal command: %w", err)
	}
	
	// Get WASM functions
	mallocFn := w.module.ExportedFunction("malloc")
	freeFn := w.module.ExportedFunction("free")
	outputCmdFn := w.module.ExportedFunction("wasm_output_command")
	
	if mallocFn == nil || freeFn == nil || outputCmdFn == nil {
		return nil, fmt.Errorf("required WASM functions not found")
	}
	
	// Allocate memory for command JSON in WASM
	cmdResults, err := mallocFn.Call(ctx, uint64(len(cmdJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to allocate WASM memory: %w", err)
	}
	cmdPtr := uint32(cmdResults[0])
	
	// Write command JSON to WASM memory
	if !w.module.Memory().Write(cmdPtr, cmdJSON) {
		freeFn.Call(ctx, uint64(cmdPtr))
		return nil, fmt.Errorf("failed to write command to WASM memory")
	}
	
	// Call wasm_output_command
	results, err := outputCmdFn.Call(ctx, uint64(cmdPtr), uint64(len(cmdJSON)))
	if err != nil {
		freeFn.Call(ctx, uint64(cmdPtr))
		return nil, fmt.Errorf("failed to call wasm_output_command: %w", err)
	}
	
	// Free command memory
	freeFn.Call(ctx, uint64(cmdPtr))
	
	// Parse result (packed pointer and length)
	packedResult := results[0]
	responsePtr := uint32(packedResult >> 32)
	responseLen := uint32(packedResult & 0xFFFFFFFF)
	
	if responsePtr == 0 {
		return nil, fmt.Errorf("WASM function returned null pointer")
	}
	
	// Read response JSON from WASM memory
	responseJSON, ok := w.module.Memory().Read(responsePtr, responseLen)
	if !ok {
		return nil, fmt.Errorf("failed to read response from WASM memory")
	}
	
	// Free response memory
	freeFn.Call(ctx, uint64(responsePtr))
	
	// Parse response JSON
	var response OutputResponse
	if err := json.Unmarshal(responseJSON, &response); err != nil {
		return nil, fmt.Errorf("failed to parse response JSON: %w", err)
	}
	
	return &response, nil
}

// WriteText outputs plain text to the console
func (w *WasmOutput) WriteText(ctx context.Context, text string) error {
	cmd := OutputCommand{Type: "WriteText", Text: text}
	response, err := w.ProcessCommand(ctx, cmd)
	if err != nil {
		return err
	}
	
	if !response.Success {
		return fmt.Errorf("WASM command failed: %s", *response.ErrorMessage)
	}
	
	if response.EscapeSequence != nil {
		fmt.Print(*response.EscapeSequence)
	}
	
	return nil
}

// WriteStyledText outputs text with styling to the console
func (w *WasmOutput) WriteStyledText(ctx context.Context, text string, style *SerializableTextStyle) error {
	cmd := OutputCommand{Type: "WriteStyledText", Text: text, Style: style}
	response, err := w.ProcessCommand(ctx, cmd)
	if err != nil {
		return err
	}
	
	if !response.Success {
		return fmt.Errorf("WASM command failed: %s", *response.ErrorMessage)
	}
	
	if response.EscapeSequence != nil {
		fmt.Print(*response.EscapeSequence)
	}
	
	return nil
}

// MoveCursorTo moves the cursor to a specific position
func (w *WasmOutput) MoveCursorTo(ctx context.Context, row, col uint16) error {
	cmd := OutputCommand{Type: "MoveCursorTo", Row: &row, Col: &col}
	response, err := w.ProcessCommand(ctx, cmd)
	if err != nil {
		return err
	}
	
	if !response.Success {
		return fmt.Errorf("WASM command failed: %s", *response.ErrorMessage)
	}
	
	if response.EscapeSequence != nil {
		fmt.Print(*response.EscapeSequence)
	}
	
	return nil
}

// Clear clears the screen or parts of it
func (w *WasmOutput) Clear(ctx context.Context, clearType string) error {
	cmd := OutputCommand{Type: "Clear", ClearType: &clearType}
	response, err := w.ProcessCommand(ctx, cmd)
	if err != nil {
		return err
	}
	
	if !response.Success {
		return fmt.Errorf("WASM command failed: %s", *response.ErrorMessage)
	}
	
	if response.EscapeSequence != nil {
		fmt.Print(*response.EscapeSequence)
	}
	
	return nil
}

// ResetStyle resets text styling to default
func (w *WasmOutput) ResetStyle(ctx context.Context) error {
	cmd := OutputCommand{Type: "ResetStyle"}
	response, err := w.ProcessCommand(ctx, cmd)
	if err != nil {
		return err
	}
	
	if !response.Success {
		return fmt.Errorf("WASM command failed: %s", *response.ErrorMessage)
	}
	
	if response.EscapeSequence != nil {
		fmt.Print(*response.EscapeSequence)
	}
	
	return nil
}

// FilterSuggestions filters suggestions based on prefix using WASM
func (w *WasmOutput) FilterSuggestions(ctx context.Context, suggestions []Suggestion, prefix string, ignoreCase bool) ([]Suggestion, error) {
	// Serialize suggestions to JSON
	suggestionsJSON, err := json.Marshal(suggestions)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal suggestions: %w", err)
	}
	
	// Get WASM functions
	mallocFn := w.module.ExportedFunction("malloc")
	freeFn := w.module.ExportedFunction("free")
	filterFn := w.module.ExportedFunction("wasm_filter_suggestions")
	
	if mallocFn == nil || freeFn == nil || filterFn == nil {
		return nil, fmt.Errorf("required WASM functions not found")
	}
	
	// Allocate memory for suggestions JSON
	suggestionsResults, err := mallocFn.Call(ctx, uint64(len(suggestionsJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to allocate WASM memory for suggestions: %w", err)
	}
	suggestionsPtr := uint32(suggestionsResults[0])
	
	// Allocate memory for prefix
	prefixBytes := []byte(prefix)
	prefixResults, err := mallocFn.Call(ctx, uint64(len(prefixBytes)))
	if err != nil {
		freeFn.Call(ctx, uint64(suggestionsPtr))
		return nil, fmt.Errorf("failed to allocate WASM memory for prefix: %w", err)
	}
	prefixPtr := uint32(prefixResults[0])
	
	// Write data to WASM memory
	if !w.module.Memory().Write(suggestionsPtr, suggestionsJSON) {
		freeFn.Call(ctx, uint64(suggestionsPtr))
		freeFn.Call(ctx, uint64(prefixPtr))
		return nil, fmt.Errorf("failed to write suggestions to WASM memory")
	}
	
	if !w.module.Memory().Write(prefixPtr, prefixBytes) {
		freeFn.Call(ctx, uint64(suggestionsPtr))
		freeFn.Call(ctx, uint64(prefixPtr))
		return nil, fmt.Errorf("failed to write prefix to WASM memory")
	}
	
	// Set ignore case flag
	ignoreCaseFlag := uint64(0)
	if ignoreCase {
		ignoreCaseFlag = 1
	}
	
	// Call wasm_filter_suggestions
	results, err := filterFn.Call(ctx, uint64(suggestionsPtr), uint64(len(suggestionsJSON)), uint64(prefixPtr), uint64(len(prefixBytes)), ignoreCaseFlag)
	if err != nil {
		freeFn.Call(ctx, uint64(suggestionsPtr))
		freeFn.Call(ctx, uint64(prefixPtr))
		return nil, fmt.Errorf("failed to call wasm_filter_suggestions: %w", err)
	}
	
	// Free input memory
	freeFn.Call(ctx, uint64(suggestionsPtr))
	freeFn.Call(ctx, uint64(prefixPtr))
	
	// Parse result
	packedResult := results[0]
	resultPtr := uint32(packedResult >> 32)
	resultLen := uint32(packedResult & 0xFFFFFFFF)
	
	if resultPtr == 0 {
		return nil, fmt.Errorf("WASM function returned null pointer")
	}
	
	// Read result JSON from WASM memory
	resultJSON, ok := w.module.Memory().Read(resultPtr, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from WASM memory")
	}
	
	// Free result memory
	freeFn.Call(ctx, uint64(resultPtr))
	
	// Parse result JSON
	var filtered []Suggestion
	if err := json.Unmarshal(resultJSON, &filtered); err != nil {
		return nil, fmt.Errorf("failed to parse filtered suggestions JSON: %w", err)
	}
	
	return filtered, nil
}

// Helper functions for creating common colors
func RGB(r, g, b uint8) SerializableColor {
	return RgbColor{R: r, G: g, B: b}
}

func Ansi256(code uint8) SerializableColor {
	return Ansi256Color{Code: code}
}

func BasicColor(color string) SerializableColor {
	return BasicColorType(color)
}

// Helper functions for creating styles
func Bold() *SerializableTextStyle {
	return &SerializableTextStyle{Bold: true}
}

func BoldWithForeground(color SerializableColor) *SerializableTextStyle {
	return &SerializableTextStyle{Bold: true, Foreground: color}
}