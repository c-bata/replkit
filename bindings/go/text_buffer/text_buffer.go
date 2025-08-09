package textbuffer

import (
	"context"
	_ "embed"
	"encoding/json"
	"fmt"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

//go:embed prompt_wasm.wasm
var embeddedWasm []byte

// Key represents the different types of keys that can be parsed.
// These constants must match the u32 values from the Rust WASM module.
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

// String returns the string representation of the Key
func (k Key) String() string {
	switch k {
	case Escape:
		return "Escape"
	case ControlA:
		return "ControlA"
	case ControlB:
		return "ControlB"
	case ControlC:
		return "ControlC"
	case ControlD:
		return "ControlD"
	case ControlE:
		return "ControlE"
	case ControlF:
		return "ControlF"
	case ControlG:
		return "ControlG"
	case ControlH:
		return "ControlH"
	case ControlI:
		return "ControlI"
	case ControlJ:
		return "ControlJ"
	case ControlK:
		return "ControlK"
	case ControlL:
		return "ControlL"
	case ControlM:
		return "ControlM"
	case ControlN:
		return "ControlN"
	case ControlO:
		return "ControlO"
	case ControlP:
		return "ControlP"
	case ControlQ:
		return "ControlQ"
	case ControlR:
		return "ControlR"
	case ControlS:
		return "ControlS"
	case ControlT:
		return "ControlT"
	case ControlU:
		return "ControlU"
	case ControlV:
		return "ControlV"
	case ControlW:
		return "ControlW"
	case ControlX:
		return "ControlX"
	case ControlY:
		return "ControlY"
	case ControlZ:
		return "ControlZ"
	case ControlSpace:
		return "ControlSpace"
	case ControlBackslash:
		return "ControlBackslash"
	case ControlSquareClose:
		return "ControlSquareClose"
	case ControlCircumflex:
		return "ControlCircumflex"
	case ControlUnderscore:
		return "ControlUnderscore"
	case ControlLeft:
		return "ControlLeft"
	case ControlRight:
		return "ControlRight"
	case ControlUp:
		return "ControlUp"
	case ControlDown:
		return "ControlDown"
	case Up:
		return "Up"
	case Down:
		return "Down"
	case Right:
		return "Right"
	case Left:
		return "Left"
	case ShiftLeft:
		return "ShiftLeft"
	case ShiftUp:
		return "ShiftUp"
	case ShiftDown:
		return "ShiftDown"
	case ShiftRight:
		return "ShiftRight"
	case Home:
		return "Home"
	case End:
		return "End"
	case Delete:
		return "Delete"
	case ShiftDelete:
		return "ShiftDelete"
	case ControlDelete:
		return "ControlDelete"
	case PageUp:
		return "PageUp"
	case PageDown:
		return "PageDown"
	case BackTab:
		return "BackTab"
	case Insert:
		return "Insert"
	case Backspace:
		return "Backspace"
	case Tab:
		return "Tab"
	case Enter:
		return "Enter"
	case F1:
		return "F1"
	case F2:
		return "F2"
	case F3:
		return "F3"
	case F4:
		return "F4"
	case F5:
		return "F5"
	case F6:
		return "F6"
	case F7:
		return "F7"
	case F8:
		return "F8"
	case F9:
		return "F9"
	case F10:
		return "F10"
	case F11:
		return "F11"
	case F12:
		return "F12"
	case F13:
		return "F13"
	case F14:
		return "F14"
	case F15:
		return "F15"
	case F16:
		return "F16"
	case F17:
		return "F17"
	case F18:
		return "F18"
	case F19:
		return "F19"
	case F20:
		return "F20"
	case F21:
		return "F21"
	case F22:
		return "F22"
	case F23:
		return "F23"
	case F24:
		return "F24"
	case Any:
		return "Any"
	case CPRResponse:
		return "CPRResponse"
	case Vt100MouseEvent:
		return "Vt100MouseEvent"
	case WindowsMouseEvent:
		return "WindowsMouseEvent"
	case BracketedPaste:
		return "BracketedPaste"
	case Ignore:
		return "Ignore"
	case NotDefined:
		return "NotDefined"
	default:
		return fmt.Sprintf("Key(%d)", int(k))
	}
}

// WasmBufferState represents the serializable state of a Buffer for WASM interop
type WasmBufferState struct {
	WorkingLines    []string `json:"working_lines"`
	WorkingIndex    int      `json:"working_index"`
	CursorPosition  int      `json:"cursor_position"`
	PreferredColumn *int     `json:"preferred_column,omitempty"`
	LastKeyStroke   *int     `json:"last_key_stroke,omitempty"`
}

// WasmDocumentState represents the serializable state of a Document for WASM interop
type WasmDocumentState struct {
	Text           string `json:"text"`
	CursorPosition int    `json:"cursor_position"`
	LastKey        *int   `json:"last_key,omitempty"`
}

// TextBufferEngine wraps the WASM-based text buffer functionality
type TextBufferEngine struct {
	runtime wazero.Runtime
	module  api.Module
	ctx     context.Context

	// WASM function handles for Buffer operations
	newBufferFn           api.Function
	bufferInsertTextFn    api.Function
	bufferDeleteBeforeFn  api.Function
	bufferDeleteFn        api.Function
	bufferCursorLeftFn    api.Function
	bufferCursorRightFn   api.Function
	bufferCursorUpFn      api.Function
	bufferCursorDownFn    api.Function
	bufferSetTextFn       api.Function
	bufferSetCursorPosFn  api.Function
	bufferNewLineFn       api.Function
	bufferJoinNextLineFn  api.Function
	bufferSwapCharsFn     api.Function
	bufferToWasmStateFn   api.Function
	bufferFromWasmStateFn api.Function
	bufferGetDocumentFn   api.Function
	destroyBufferFn       api.Function

	// WASM function handles for Document operations
	newDocumentFn         api.Function
	docWithTextFn         api.Function
	docWithTextAndKeyFn   api.Function
	docTextBeforeCursorFn api.Function
	docTextAfterCursorFn  api.Function
	docGetWordBeforeFn    api.Function
	docGetWordAfterFn     api.Function
	docCurrentLineFn      api.Function
	docLineCountFn        api.Function
	docCursorRowFn        api.Function
	docCursorColFn        api.Function
	docDisplayCursorPosFn api.Function
	docToWasmStateFn      api.Function
	docFromWasmStateFn    api.Function
	destroyDocumentFn     api.Function
}

// NewTextBufferEngine creates a new TextBufferEngine instance using the embedded WASM binary.
func NewTextBufferEngine(ctx context.Context) (*TextBufferEngine, error) {
	return NewTextBufferEngineWithWasm(ctx, embeddedWasm)
}

// NewTextBufferEngineWithWasm creates a new TextBufferEngine instance using the provided WASM binary.
func NewTextBufferEngineWithWasm(ctx context.Context, wasmBytes []byte) (*TextBufferEngine, error) {
	if len(wasmBytes) == 0 {
		return nil, fmt.Errorf("WASM binary cannot be empty")
	}

	// Create a new WASM runtime
	runtime := wazero.NewRuntime(ctx)

	// Instantiate WASI to support basic system calls
	_, err := wasi_snapshot_preview1.Instantiate(ctx, runtime)
	if err != nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASI: %w", err)
	}

	// Create env module for WASM malloc/free functions
	envBuilder := runtime.NewHostModuleBuilder("env")
	envBuilder.NewFunctionBuilder().
		WithFunc(func(ctx context.Context, size uint32) uint32 {
			return size // Placeholder - WASM module handles its own allocation
		}).
		Export("__wbindgen_malloc")
	envBuilder.NewFunctionBuilder().
		WithFunc(func(ctx context.Context, ptr uint32, size uint32) {
			// No-op since WASM has its own memory management
		}).
		Export("__wbindgen_free")

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

	engine := &TextBufferEngine{
		runtime: runtime,
		module:  module,
		ctx:     ctx,
	}

	// Get Buffer function handles
	if engine.newBufferFn = module.ExportedFunction("new_buffer"); engine.newBufferFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'new_buffer' function")
	}
	if engine.bufferInsertTextFn = module.ExportedFunction("buffer_insert_text"); engine.bufferInsertTextFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_insert_text' function")
	}
	if engine.bufferDeleteBeforeFn = module.ExportedFunction("buffer_delete_before_cursor"); engine.bufferDeleteBeforeFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_delete_before_cursor' function")
	}
	if engine.bufferDeleteFn = module.ExportedFunction("buffer_delete"); engine.bufferDeleteFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_delete' function")
	}
	if engine.bufferCursorLeftFn = module.ExportedFunction("buffer_cursor_left"); engine.bufferCursorLeftFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_cursor_left' function")
	}
	if engine.bufferCursorRightFn = module.ExportedFunction("buffer_cursor_right"); engine.bufferCursorRightFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_cursor_right' function")
	}
	if engine.bufferCursorUpFn = module.ExportedFunction("buffer_cursor_up"); engine.bufferCursorUpFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_cursor_up' function")
	}
	if engine.bufferCursorDownFn = module.ExportedFunction("buffer_cursor_down"); engine.bufferCursorDownFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_cursor_down' function")
	}
	if engine.bufferSetTextFn = module.ExportedFunction("buffer_set_text"); engine.bufferSetTextFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_set_text' function")
	}
	if engine.bufferSetCursorPosFn = module.ExportedFunction("buffer_set_cursor_position"); engine.bufferSetCursorPosFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_set_cursor_position' function")
	}
	if engine.bufferNewLineFn = module.ExportedFunction("buffer_new_line"); engine.bufferNewLineFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_new_line' function")
	}
	if engine.bufferJoinNextLineFn = module.ExportedFunction("buffer_join_next_line"); engine.bufferJoinNextLineFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_join_next_line' function")
	}
	if engine.bufferSwapCharsFn = module.ExportedFunction("buffer_swap_characters_before_cursor"); engine.bufferSwapCharsFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_swap_characters_before_cursor' function")
	}
	if engine.bufferToWasmStateFn = module.ExportedFunction("buffer_to_wasm_state"); engine.bufferToWasmStateFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_to_wasm_state' function")
	}
	if engine.bufferFromWasmStateFn = module.ExportedFunction("buffer_from_wasm_state"); engine.bufferFromWasmStateFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_from_wasm_state' function")
	}
	if engine.bufferGetDocumentFn = module.ExportedFunction("buffer_get_document"); engine.bufferGetDocumentFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'buffer_get_document' function")
	}
	if engine.destroyBufferFn = module.ExportedFunction("destroy_buffer"); engine.destroyBufferFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'destroy_buffer' function")
	}

	// Get Document function handles
	if engine.newDocumentFn = module.ExportedFunction("new_document"); engine.newDocumentFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'new_document' function")
	}
	if engine.docWithTextFn = module.ExportedFunction("document_with_text"); engine.docWithTextFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_with_text' function")
	}
	if engine.docWithTextAndKeyFn = module.ExportedFunction("document_with_text_and_key"); engine.docWithTextAndKeyFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_with_text_and_key' function")
	}
	if engine.docTextBeforeCursorFn = module.ExportedFunction("document_text_before_cursor"); engine.docTextBeforeCursorFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_text_before_cursor' function")
	}
	if engine.docTextAfterCursorFn = module.ExportedFunction("document_text_after_cursor"); engine.docTextAfterCursorFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_text_after_cursor' function")
	}
	if engine.docGetWordBeforeFn = module.ExportedFunction("document_get_word_before_cursor"); engine.docGetWordBeforeFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_get_word_before_cursor' function")
	}
	if engine.docGetWordAfterFn = module.ExportedFunction("document_get_word_after_cursor"); engine.docGetWordAfterFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_get_word_after_cursor' function")
	}
	if engine.docCurrentLineFn = module.ExportedFunction("document_current_line"); engine.docCurrentLineFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_current_line' function")
	}
	if engine.docLineCountFn = module.ExportedFunction("document_line_count"); engine.docLineCountFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_line_count' function")
	}
	if engine.docCursorRowFn = module.ExportedFunction("document_cursor_position_row"); engine.docCursorRowFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_cursor_position_row' function")
	}
	if engine.docCursorColFn = module.ExportedFunction("document_cursor_position_col"); engine.docCursorColFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_cursor_position_col' function")
	}
	if engine.docDisplayCursorPosFn = module.ExportedFunction("document_display_cursor_position"); engine.docDisplayCursorPosFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_display_cursor_position' function")
	}
	if engine.docToWasmStateFn = module.ExportedFunction("document_to_wasm_state"); engine.docToWasmStateFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_to_wasm_state' function")
	}
	if engine.docFromWasmStateFn = module.ExportedFunction("document_from_wasm_state"); engine.docFromWasmStateFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'document_from_wasm_state' function")
	}
	if engine.destroyDocumentFn = module.ExportedFunction("destroy_document"); engine.destroyDocumentFn == nil {
		runtime.Close(ctx)
		return nil, fmt.Errorf("WASM module does not export 'destroy_document' function")
	}

	return engine, nil
}

// Close releases all resources and marks the engine as closed.
func (e *TextBufferEngine) Close() error {
	if e == nil {
		return fmt.Errorf("engine is nil")
	}
	if e.module == nil {
		return nil // Already closed
	}

	err := e.runtime.Close(e.ctx)

	// Mark as closed to prevent further use
	e.module = nil
	e.runtime = nil

	return err
}

// Helper function to allocate memory in WASM and write string data
func (e *TextBufferEngine) allocateString(s string) (uint32, error) {
	if len(s) == 0 {
		return 0, nil
	}

	malloc := e.module.ExportedFunction("malloc")
	if malloc == nil {
		return 0, fmt.Errorf("WASM module does not export 'malloc' function")
	}

	results, err := malloc.Call(e.ctx, uint64(len(s)))
	if err != nil {
		return 0, fmt.Errorf("failed to allocate WASM memory: %w", err)
	}

	ptr := uint32(results[0])
	if !e.module.Memory().Write(ptr, []byte(s)) {
		return 0, fmt.Errorf("failed to write string to WASM memory")
	}

	return ptr, nil
}

// Helper function to read string from WASM memory
func (e *TextBufferEngine) readString(ptr uint32, length uint32) (string, error) {
	if length == 0 {
		return "", nil
	}

	bytes, ok := e.module.Memory().Read(ptr, length)
	if !ok {
		return "", fmt.Errorf("failed to read string from WASM memory")
	}

	return string(bytes), nil
}

// Helper function to free WASM memory
func (e *TextBufferEngine) freeMemory(ptr uint32) {
	if ptr == 0 {
		return
	}
	free := e.module.ExportedFunction("free")
	if free != nil {
		free.Call(e.ctx, uint64(ptr))
	}
}

// Helper function to read JSON result from WASM
func (e *TextBufferEngine) readJSONResult(packed uint64, target interface{}) error {
	resultPtr := uint32(packed >> 32)
	resultLen := uint32(packed & 0xFFFFFFFF)

	if resultLen == 0 {
		return fmt.Errorf("empty result from WASM")
	}

	jsonBytes, ok := e.module.Memory().Read(resultPtr, resultLen)
	if !ok {
		return fmt.Errorf("failed to read result from WASM memory")
	}

	defer e.freeMemory(resultPtr)

	return json.Unmarshal(jsonBytes, target)
}

// Buffer represents a mutable text buffer with editing capabilities
type Buffer struct {
	engine   *TextBufferEngine
	bufferID uint32
}

// NewBuffer creates a new Buffer instance
func (e *TextBufferEngine) NewBuffer() (*Buffer, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	results, err := e.newBufferFn.Call(e.ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create buffer: %w", err)
	}

	bufferID := uint32(results[0])
	return &Buffer{
		engine:   e,
		bufferID: bufferID,
	}, nil
}

// Text returns the current text content of the buffer
func (b *Buffer) Text() (string, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return "", fmt.Errorf("buffer is nil or closed")
	}

	// Get the buffer's current document and extract text from it
	doc, err := b.Document()
	if err != nil {
		return "", err
	}
	defer doc.Close()

	return doc.Text()
}

// CursorPosition returns the current cursor position in rune index
func (b *Buffer) CursorPosition() (int, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return 0, fmt.Errorf("buffer is nil or closed")
	}

	doc, err := b.Document()
	if err != nil {
		return 0, err
	}
	defer doc.Close()

	return doc.CursorPosition()
}

// DisplayCursorPosition returns the display cursor position accounting for Unicode width
func (b *Buffer) DisplayCursorPosition() (int, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return 0, fmt.Errorf("buffer is nil or closed")
	}

	doc, err := b.Document()
	if err != nil {
		return 0, err
	}
	defer doc.Close()

	return doc.DisplayCursorPosition()
}

// InsertText inserts text at the current cursor position
func (b *Buffer) InsertText(text string, overwrite bool, moveCursor bool) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	textPtr, err := b.engine.allocateString(text)
	if err != nil {
		return err
	}
	defer b.engine.freeMemory(textPtr)

	overwriteFlag := uint64(0)
	if overwrite {
		overwriteFlag = 1
	}
	moveCursorFlag := uint64(0)
	if moveCursor {
		moveCursorFlag = 1
	}

	_, err = b.engine.bufferInsertTextFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(textPtr), uint64(len(text)), overwriteFlag, moveCursorFlag)
	if err != nil {
		return fmt.Errorf("failed to insert text: %w", err)
	}

	return nil
}

// DeleteBeforeCursor deletes count characters before the cursor and returns the deleted text
func (b *Buffer) DeleteBeforeCursor(count int) (string, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return "", fmt.Errorf("buffer is nil or closed")
	}

	results, err := b.engine.bufferDeleteBeforeFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return "", fmt.Errorf("failed to delete before cursor: %w", err)
	}

	var deletedText string
	if err := b.engine.readJSONResult(results[0], &deletedText); err != nil {
		return "", err
	}

	return deletedText, nil
}

// Delete deletes count characters after the cursor and returns the deleted text
func (b *Buffer) Delete(count int) (string, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return "", fmt.Errorf("buffer is nil or closed")
	}

	results, err := b.engine.bufferDeleteFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return "", fmt.Errorf("failed to delete: %w", err)
	}

	var deletedText string
	if err := b.engine.readJSONResult(results[0], &deletedText); err != nil {
		return "", err
	}

	return deletedText, nil
}

// CursorLeft moves the cursor left by count positions
func (b *Buffer) CursorLeft(count int) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferCursorLeftFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor left: %w", err)
	}

	return nil
}

// CursorRight moves the cursor right by count positions
func (b *Buffer) CursorRight(count int) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferCursorRightFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor right: %w", err)
	}

	return nil
}

// CursorUp moves the cursor up by count lines
func (b *Buffer) CursorUp(count int) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferCursorUpFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor up: %w", err)
	}

	return nil
}

// CursorDown moves the cursor down by count lines
func (b *Buffer) CursorDown(count int) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferCursorDownFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor down: %w", err)
	}

	return nil
}

// SetText sets the buffer text and resets cursor position
func (b *Buffer) SetText(text string) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	textPtr, err := b.engine.allocateString(text)
	if err != nil {
		return err
	}
	defer b.engine.freeMemory(textPtr)

	_, err = b.engine.bufferSetTextFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(textPtr), uint64(len(text)))
	if err != nil {
		return fmt.Errorf("failed to set text: %w", err)
	}

	return nil
}

// SetCursorPosition sets the cursor position to the specified rune index
func (b *Buffer) SetCursorPosition(position int) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferSetCursorPosFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(position))
	if err != nil {
		return fmt.Errorf("failed to set cursor position: %w", err)
	}

	return nil
}

// NewLine creates a new line at the cursor position
func (b *Buffer) NewLine(copyMargin bool) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	copyMarginFlag := uint64(0)
	if copyMargin {
		copyMarginFlag = 1
	}

	_, err := b.engine.bufferNewLineFn.Call(b.engine.ctx, uint64(b.bufferID), copyMarginFlag)
	if err != nil {
		return fmt.Errorf("failed to create new line: %w", err)
	}

	return nil
}

// JoinNextLine joins the current line with the next line using the specified separator
func (b *Buffer) JoinNextLine(separator string) error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	sepPtr, err := b.engine.allocateString(separator)
	if err != nil {
		return err
	}
	defer b.engine.freeMemory(sepPtr)

	_, err = b.engine.bufferJoinNextLineFn.Call(b.engine.ctx, uint64(b.bufferID), uint64(sepPtr), uint64(len(separator)))
	if err != nil {
		return fmt.Errorf("failed to join next line: %w", err)
	}

	return nil
}

// SwapCharactersBeforeCursor swaps the two characters before the cursor
func (b *Buffer) SwapCharactersBeforeCursor() error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	_, err := b.engine.bufferSwapCharsFn.Call(b.engine.ctx, uint64(b.bufferID))
	if err != nil {
		return fmt.Errorf("failed to swap characters: %w", err)
	}

	return nil
}

// Document returns the current Document for text analysis operations
func (b *Buffer) Document() (*Document, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return nil, fmt.Errorf("buffer is nil or closed")
	}

	results, err := b.engine.bufferGetDocumentFn.Call(b.engine.ctx, uint64(b.bufferID))
	if err != nil {
		return nil, fmt.Errorf("failed to get document: %w", err)
	}

	documentID := uint32(results[0])
	return &Document{
		engine:     b.engine,
		documentID: documentID,
	}, nil
}

// ToWasmState serializes the buffer state for WASM interop
func (b *Buffer) ToWasmState() (*WasmBufferState, error) {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return nil, fmt.Errorf("buffer is nil or closed")
	}

	results, err := b.engine.bufferToWasmStateFn.Call(b.engine.ctx, uint64(b.bufferID))
	if err != nil {
		return nil, fmt.Errorf("failed to serialize buffer state: %w", err)
	}

	var state WasmBufferState
	if err := b.engine.readJSONResult(results[0], &state); err != nil {
		return nil, err
	}

	return &state, nil
}

// FromWasmState creates a new Buffer from serialized state
func (e *TextBufferEngine) BufferFromWasmState(state *WasmBufferState) (*Buffer, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	stateJSON, err := json.Marshal(state)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal buffer state: %w", err)
	}

	statePtr, err := e.allocateString(string(stateJSON))
	if err != nil {
		return nil, err
	}
	defer e.freeMemory(statePtr)

	results, err := e.bufferFromWasmStateFn.Call(e.ctx, uint64(statePtr), uint64(len(stateJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to create buffer from state: %w", err)
	}

	bufferID := uint32(results[0])
	return &Buffer{
		engine:   e,
		bufferID: bufferID,
	}, nil
}

// Close releases the buffer resources
func (b *Buffer) Close() error {
	if b == nil || b.engine == nil || b.engine.module == nil {
		return nil // Already closed
	}

	_, err := b.engine.destroyBufferFn.Call(b.engine.ctx, uint64(b.bufferID))
	if err != nil {
		return fmt.Errorf("failed to destroy buffer: %w", err)
	}

	// Mark as closed
	b.engine = nil
	return nil
}

// Document represents an immutable text document with cursor position for analysis
type Document struct {
	engine     *TextBufferEngine
	documentID uint32
}

// NewDocument creates a new empty Document
func (e *TextBufferEngine) NewDocument() (*Document, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	results, err := e.newDocumentFn.Call(e.ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create document: %w", err)
	}

	documentID := uint32(results[0])
	return &Document{
		engine:     e,
		documentID: documentID,
	}, nil
}

// NewDocumentWithText creates a new Document with the specified text and cursor position
func (e *TextBufferEngine) NewDocumentWithText(text string, cursorPosition int) (*Document, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	textPtr, err := e.allocateString(text)
	if err != nil {
		return nil, err
	}
	defer e.freeMemory(textPtr)

	results, err := e.docWithTextFn.Call(e.ctx, uint64(textPtr), uint64(len(text)), uint64(cursorPosition))
	if err != nil {
		return nil, fmt.Errorf("failed to create document with text: %w", err)
	}

	documentID := uint32(results[0])
	return &Document{
		engine:     e,
		documentID: documentID,
	}, nil
}

// NewDocumentWithTextAndKey creates a new Document with text, cursor position, and last key
func (e *TextBufferEngine) NewDocumentWithTextAndKey(text string, cursorPosition int, lastKey *Key) (*Document, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	textPtr, err := e.allocateString(text)
	if err != nil {
		return nil, err
	}
	defer e.freeMemory(textPtr)

	keyValue := uint64(0)
	hasKey := uint64(0)
	if lastKey != nil {
		keyValue = uint64(*lastKey)
		hasKey = 1
	}

	results, err := e.docWithTextAndKeyFn.Call(e.ctx, uint64(textPtr), uint64(len(text)), uint64(cursorPosition), hasKey, keyValue)
	if err != nil {
		return nil, fmt.Errorf("failed to create document with text and key: %w", err)
	}

	documentID := uint32(results[0])
	return &Document{
		engine:     e,
		documentID: documentID,
	}, nil
}

// Text returns the document text
func (d *Document) Text() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	// Get the document state and extract text
	state, err := d.ToWasmState()
	if err != nil {
		return "", err
	}

	return state.Text, nil
}

// CursorPosition returns the cursor position in rune index
func (d *Document) CursorPosition() (int, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	state, err := d.ToWasmState()
	if err != nil {
		return 0, err
	}

	return state.CursorPosition, nil
}

// DisplayCursorPosition returns the display cursor position accounting for Unicode width
func (d *Document) DisplayCursorPosition() (int, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docDisplayCursorPosFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get display cursor position: %w", err)
	}

	return int(results[0]), nil
}

// TextBeforeCursor returns the text before the cursor
func (d *Document) TextBeforeCursor() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docTextBeforeCursorFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get text before cursor: %w", err)
	}

	var text string
	if err := d.engine.readJSONResult(results[0], &text); err != nil {
		return "", err
	}

	return text, nil
}

// TextAfterCursor returns the text after the cursor
func (d *Document) TextAfterCursor() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docTextAfterCursorFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get text after cursor: %w", err)
	}

	var text string
	if err := d.engine.readJSONResult(results[0], &text); err != nil {
		return "", err
	}

	return text, nil
}

// GetWordBeforeCursor returns the word before the cursor
func (d *Document) GetWordBeforeCursor() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docGetWordBeforeFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get word before cursor: %w", err)
	}

	var word string
	if err := d.engine.readJSONResult(results[0], &word); err != nil {
		return "", err
	}

	return word, nil
}

// GetWordAfterCursor returns the word after the cursor
func (d *Document) GetWordAfterCursor() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docGetWordAfterFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get word after cursor: %w", err)
	}

	var word string
	if err := d.engine.readJSONResult(results[0], &word); err != nil {
		return "", err
	}

	return word, nil
}

// CurrentLine returns the current line text
func (d *Document) CurrentLine() (string, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docCurrentLineFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get current line: %w", err)
	}

	var line string
	if err := d.engine.readJSONResult(results[0], &line); err != nil {
		return "", err
	}

	return line, nil
}

// LineCount returns the number of lines in the document
func (d *Document) LineCount() (int, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docLineCountFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get line count: %w", err)
	}

	return int(results[0]), nil
}

// CursorPositionRow returns the cursor row (0-based)
func (d *Document) CursorPositionRow() (int, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docCursorRowFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get cursor row: %w", err)
	}

	return int(results[0]), nil
}

// CursorPositionCol returns the cursor column (0-based)
func (d *Document) CursorPositionCol() (int, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docCursorColFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get cursor column: %w", err)
	}

	return int(results[0]), nil
}

// ToWasmState serializes the document state for WASM interop
func (d *Document) ToWasmState() (*WasmDocumentState, error) {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return nil, fmt.Errorf("document is nil or closed")
	}

	results, err := d.engine.docToWasmStateFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return nil, fmt.Errorf("failed to serialize document state: %w", err)
	}

	var state WasmDocumentState
	if err := d.engine.readJSONResult(results[0], &state); err != nil {
		return nil, err
	}

	return &state, nil
}

// DocumentFromWasmState creates a new Document from serialized state
func (e *TextBufferEngine) DocumentFromWasmState(state *WasmDocumentState) (*Document, error) {
	if e == nil || e.module == nil {
		return nil, fmt.Errorf("engine is nil or closed")
	}

	stateJSON, err := json.Marshal(state)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal document state: %w", err)
	}

	statePtr, err := e.allocateString(string(stateJSON))
	if err != nil {
		return nil, err
	}
	defer e.freeMemory(statePtr)

	results, err := e.docFromWasmStateFn.Call(e.ctx, uint64(statePtr), uint64(len(stateJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to create document from state: %w", err)
	}

	documentID := uint32(results[0])
	return &Document{
		engine:     e,
		documentID: documentID,
	}, nil
}

// Close releases the document resources
func (d *Document) Close() error {
	if d == nil || d.engine == nil || d.engine.module == nil {
		return nil // Already closed
	}

	_, err := d.engine.destroyDocumentFn.Call(d.engine.ctx, uint64(d.documentID))
	if err != nil {
		return fmt.Errorf("failed to destroy document: %w", err)
	}

	// Mark as closed
	d.engine = nil
	return nil
}
