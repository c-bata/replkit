package keyparsing

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

// Key represents the different types of keys that can be parsed
type Key int

const (
	// These constants must match the u32 values from the Rust WASM module
	Escape             Key = 0
	ControlA           Key = 1
	ControlB           Key = 2
	ControlC           Key = 3
	ControlD           Key = 4
	ControlE           Key = 5
	ControlF           Key = 6
	ControlG           Key = 7
	ControlH           Key = 8
	ControlI           Key = 9
	ControlJ           Key = 10
	ControlK           Key = 11
	ControlL           Key = 12
	ControlM           Key = 13
	ControlN           Key = 14
	ControlO           Key = 15
	ControlP           Key = 16
	ControlQ           Key = 17
	ControlR           Key = 18
	ControlS           Key = 19
	ControlT           Key = 20
	ControlU           Key = 21
	ControlV           Key = 22
	ControlW           Key = 23
	ControlX           Key = 24
	ControlY           Key = 25
	ControlZ           Key = 26
	ControlSpace       Key = 27
	ControlBackslash   Key = 28
	ControlSquareClose Key = 29
	ControlCircumflex  Key = 30
	ControlUnderscore  Key = 31
	ControlLeft        Key = 32
	ControlRight       Key = 33
	ControlUp          Key = 34
	ControlDown        Key = 35
	Up                 Key = 36
	Down               Key = 37
	Right              Key = 38
	Left               Key = 39
	ShiftLeft          Key = 40
	ShiftUp            Key = 41
	ShiftDown          Key = 42
	ShiftRight         Key = 43
	Home               Key = 44
	End                Key = 45
	Delete             Key = 46
	ShiftDelete        Key = 47
	ControlDelete      Key = 48
	PageUp             Key = 49
	PageDown           Key = 50
	BackTab            Key = 51
	Insert             Key = 52
	Backspace          Key = 53
	Tab                Key = 54
	Enter              Key = 55
	F1                 Key = 56
	F2                 Key = 57
	F3                 Key = 58
	F4                 Key = 59
	F5                 Key = 60
	F6                 Key = 61
	F7                 Key = 62
	F8                 Key = 63
	F9                 Key = 64
	F10                Key = 65
	F11                Key = 66
	F12                Key = 67
	F13                Key = 68
	F14                Key = 69
	F15                Key = 70
	F16                Key = 71
	F17                Key = 72
	F18                Key = 73
	F19                Key = 74
	F20                Key = 75
	F21                Key = 76
	F22                Key = 77
	F23                Key = 78
	F24                Key = 79
	Any                Key = 80
	CPRResponse        Key = 81
	Vt100MouseEvent    Key = 82
	WindowsMouseEvent  Key = 83
	BracketedPaste     Key = 84
	Ignore             Key = 85
	NotDefined         Key = 86
)

// KeyEvent represents a parsed key event
type KeyEvent struct {
	Key      Key     `json:"key"`
	RawBytes []byte  `json:"raw_bytes"`
	Text     *string `json:"text,omitempty"`
}

// KeyParser wraps the WASM-based key parser
type KeyParser struct {
	runtime wazero.Runtime
	module  api.Module
	ctx     context.Context

	// WASM function handles
	newParserFn api.Function
	feedFn      api.Function
	flushFn     api.Function
	resetFn     api.Function
	destroyFn   api.Function

	// Parser instance ID in WASM memory
	parserID uint32
}

// NewKeyParser creates a new KeyParser instance using the provided WASM binary
func NewKeyParser(ctx context.Context, wasmBytes []byte) (*KeyParser, error) {
	// Create a new WASM runtime
	runtime := wazero.NewRuntime(ctx)

	// Instantiate WASI to support basic system calls
	_, err := wasi_snapshot_preview1.Instantiate(ctx, runtime)
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASI: %w", err)
	}

	// Create env module for WASM malloc function
	envBuilder := runtime.NewHostModuleBuilder("env")
	envBuilder.NewFunctionBuilder().
		WithFunc(func(ctx context.Context, size uint32) uint32 {
			// Simple allocator - in production you'd want a proper allocator
			return size // This is a placeholder - the WASM module should handle its own allocation
		}).
		Export("__wbindgen_malloc")

	_, err = envBuilder.Instantiate(ctx)
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate env module: %w", err)
	}

	// Compile and instantiate the WASM module
	compiled, err := runtime.CompileModule(ctx, wasmBytes)
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to compile WASM module: %w", err)
	}

	module, err := runtime.InstantiateModule(ctx, compiled, wazero.NewModuleConfig())
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}

	// Get function handles
	newParserFn := module.ExportedFunction("new_parser")
	if newParserFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'new_parser' function")
	}

	feedFn := module.ExportedFunction("feed")
	if feedFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'feed' function")
	}

	flushFn := module.ExportedFunction("flush")
	if flushFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'flush' function")
	}

	resetFn := module.ExportedFunction("reset")
	if resetFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'reset' function")
	}

	destroyFn := module.ExportedFunction("destroy_parser")
	if destroyFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'destroy_parser' function")
	}

	// Create a new parser instance in WASM
	results, err := newParserFn.Call(ctx)
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to create parser instance: %w", err)
	}

	parserID := uint32(results[0])

	return &KeyParser{
		runtime:     runtime,
		module:      module,
		ctx:         ctx,
		newParserFn: newParserFn,
		feedFn:      feedFn,
		flushFn:     flushFn,
		resetFn:     resetFn,
		destroyFn:   destroyFn,
		parserID:    parserID,
	}, nil
}

// Feed processes input bytes and returns parsed key events
func (p *KeyParser) Feed(input []byte) ([]KeyEvent, error) {
	if len(input) == 0 {
		return nil, nil
	}

	// Allocate memory in WASM for input bytes
	malloc := p.module.ExportedFunction("malloc")
	if malloc == nil {
		return nil, fmt.Errorf("WASM module does not export 'malloc' function")
	}

	results, err := malloc.Call(p.ctx, uint64(len(input)))
	if err != nil {
		return nil, fmt.Errorf("failed to allocate WASM memory: %w", err)
	}

	inputPtr := uint32(results[0])

	// Write input bytes to WASM memory
	if !p.module.Memory().Write(inputPtr, input) {
		return nil, fmt.Errorf("failed to write input to WASM memory")
	}

	// Call the feed function
	results, err = p.feedFn.Call(p.ctx, uint64(p.parserID), uint64(inputPtr), uint64(len(input)))
	if err != nil {
		return nil, fmt.Errorf("failed to call feed function: %w", err)
	}

	// Free the input memory
	free := p.module.ExportedFunction("free")
	if free != nil {
		free.Call(p.ctx, uint64(inputPtr))
	}

	// Parse the result - it's a packed u64 with pointer in high 32 bits and length in low 32 bits
	packed := results[0]
	resultPtr := uint32(packed >> 32)
	resultLen := uint32(packed & 0xFFFFFFFF)

	if resultLen == 0 {
		return nil, nil
	}

	// Read the JSON result from WASM memory
	jsonBytes, ok := p.module.Memory().Read(resultPtr, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from WASM memory")
	}

	// Free the result memory
	if free != nil {
		free.Call(p.ctx, uint64(resultPtr))
	}

	// Parse JSON into KeyEvent slice
	var events []KeyEvent
	if err := json.Unmarshal(jsonBytes, &events); err != nil {
		return nil, fmt.Errorf("failed to parse key events JSON: %w", err)
	}

	return events, nil
}

// Flush processes any remaining buffered input and returns key events
func (p *KeyParser) Flush() ([]KeyEvent, error) {
	results, err := p.flushFn.Call(p.ctx, uint64(p.parserID))
	if err != nil {
		return nil, fmt.Errorf("failed to call flush function: %w", err)
	}

	// Parse the result - it's a packed u64 with pointer in high 32 bits and length in low 32 bits
	packed := results[0]
	resultPtr := uint32(packed >> 32)
	resultLen := uint32(packed & 0xFFFFFFFF)

	if resultLen == 0 {
		return nil, nil
	}

	jsonBytes, ok := p.module.Memory().Read(resultPtr, resultLen)
	if !ok {
		return nil, fmt.Errorf("failed to read result from WASM memory")
	}

	// Free the result memory
	free := p.module.ExportedFunction("free")
	if free != nil {
		free.Call(p.ctx, uint64(resultPtr))
	}

	var events []KeyEvent
	if err := json.Unmarshal(jsonBytes, &events); err != nil {
		return nil, fmt.Errorf("failed to parse key events JSON: %w", err)
	}

	return events, nil
}

// Reset clears the parser state
func (p *KeyParser) Reset() error {
	_, err := p.resetFn.Call(p.ctx, uint64(p.parserID))
	if err != nil {
		return fmt.Errorf("failed to call reset function: %w", err)
	}
	return nil
}

// Close releases all resources
func (p *KeyParser) Close() error {
	if p.destroyFn != nil {
		p.destroyFn.Call(p.ctx, uint64(p.parserID))
	}
	return p.runtime.Close(p.ctx)
}
