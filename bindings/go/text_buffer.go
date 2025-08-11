package replkit

import (
	"encoding/json"
	"fmt"
)

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

// Buffer represents a mutable text buffer with editing capabilities
type Buffer struct {
	parser   *KeyParser
	bufferID uint32
}

// NewBuffer creates a new Buffer instance using the existing KeyParser's WASM runtime
func (p *KeyParser) NewBuffer() (*Buffer, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	newBufferFn := p.module.ExportedFunction("new_buffer")
	if newBufferFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'new_buffer' function")
	}

	results, err := newBufferFn.Call(p.ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create buffer: %w", err)
	}

	bufferID := uint32(results[0])
	return &Buffer{
		parser:   p,
		bufferID: bufferID,
	}, nil
}

// Text returns the current text content of the buffer
func (b *Buffer) Text() (string, error) {
	if b == nil || b.parser == nil || b.parser.module == nil {
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
	if b == nil || b.parser == nil || b.parser.module == nil {
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
	if b == nil || b.parser == nil || b.parser.module == nil {
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
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	insertTextFn := b.parser.module.ExportedFunction("buffer_insert_text")
	if insertTextFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_insert_text' function")
	}

	textPtr, err := b.parser.allocateString(text)
	if err != nil {
		return err
	}
	defer b.parser.freeMemory(textPtr)

	overwriteFlag := uint64(0)
	if overwrite {
		overwriteFlag = 1
	}
	moveCursorFlag := uint64(0)
	if moveCursor {
		moveCursorFlag = 1
	}

	_, err = insertTextFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(textPtr), uint64(len(text)), overwriteFlag, moveCursorFlag)
	if err != nil {
		return fmt.Errorf("failed to insert text: %w", err)
	}

	return nil
}

// DeleteBeforeCursor deletes count characters before the cursor and returns the deleted text
func (b *Buffer) DeleteBeforeCursor(count int) (string, error) {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return "", fmt.Errorf("buffer is nil or closed")
	}

	deleteBeforeFn := b.parser.module.ExportedFunction("buffer_delete_before_cursor")
	if deleteBeforeFn == nil {
		return "", fmt.Errorf("WASM module does not export 'buffer_delete_before_cursor' function")
	}

	results, err := deleteBeforeFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return "", fmt.Errorf("failed to delete before cursor: %w", err)
	}

	var deletedText string
	if err := b.parser.readJSONResult(results[0], &deletedText); err != nil {
		return "", err
	}

	return deletedText, nil
}

// Delete deletes count characters after the cursor and returns the deleted text
func (b *Buffer) Delete(count int) (string, error) {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return "", fmt.Errorf("buffer is nil or closed")
	}

	deleteFn := b.parser.module.ExportedFunction("buffer_delete")
	if deleteFn == nil {
		return "", fmt.Errorf("WASM module does not export 'buffer_delete' function")
	}

	results, err := deleteFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return "", fmt.Errorf("failed to delete: %w", err)
	}

	var deletedText string
	if err := b.parser.readJSONResult(results[0], &deletedText); err != nil {
		return "", err
	}

	return deletedText, nil
}

// CursorLeft moves the cursor left by count positions
func (b *Buffer) CursorLeft(count int) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	cursorLeftFn := b.parser.module.ExportedFunction("buffer_cursor_left")
	if cursorLeftFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_cursor_left' function")
	}

	_, err := cursorLeftFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor left: %w", err)
	}

	return nil
}

// CursorRight moves the cursor right by count positions
func (b *Buffer) CursorRight(count int) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	cursorRightFn := b.parser.module.ExportedFunction("buffer_cursor_right")
	if cursorRightFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_cursor_right' function")
	}

	_, err := cursorRightFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor right: %w", err)
	}

	return nil
}

// CursorUp moves the cursor up by count lines
func (b *Buffer) CursorUp(count int) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	cursorUpFn := b.parser.module.ExportedFunction("buffer_cursor_up")
	if cursorUpFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_cursor_up' function")
	}

	_, err := cursorUpFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor up: %w", err)
	}

	return nil
}

// CursorDown moves the cursor down by count lines
func (b *Buffer) CursorDown(count int) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	cursorDownFn := b.parser.module.ExportedFunction("buffer_cursor_down")
	if cursorDownFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_cursor_down' function")
	}

	_, err := cursorDownFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(count))
	if err != nil {
		return fmt.Errorf("failed to move cursor down: %w", err)
	}

	return nil
}

// SetText sets the buffer text and resets cursor position
func (b *Buffer) SetText(text string) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	setTextFn := b.parser.module.ExportedFunction("buffer_set_text")
	if setTextFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_set_text' function")
	}

	textPtr, err := b.parser.allocateString(text)
	if err != nil {
		return err
	}
	defer b.parser.freeMemory(textPtr)

	_, err = setTextFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(textPtr), uint64(len(text)))
	if err != nil {
		return fmt.Errorf("failed to set text: %w", err)
	}

	return nil
}

// SetCursorPosition sets the cursor position to the specified rune index
func (b *Buffer) SetCursorPosition(position int) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	setCursorPosFn := b.parser.module.ExportedFunction("buffer_set_cursor_position")
	if setCursorPosFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_set_cursor_position' function")
	}

	_, err := setCursorPosFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(position))
	if err != nil {
		return fmt.Errorf("failed to set cursor position: %w", err)
	}

	return nil
}

// NewLine creates a new line at the cursor position
func (b *Buffer) NewLine(copyMargin bool) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	newLineFn := b.parser.module.ExportedFunction("buffer_new_line")
	if newLineFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_new_line' function")
	}

	copyMarginFlag := uint64(0)
	if copyMargin {
		copyMarginFlag = 1
	}

	_, err := newLineFn.Call(b.parser.ctx, uint64(b.bufferID), copyMarginFlag)
	if err != nil {
		return fmt.Errorf("failed to create new line: %w", err)
	}

	return nil
}

// JoinNextLine joins the current line with the next line using the specified separator
func (b *Buffer) JoinNextLine(separator string) error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	joinNextLineFn := b.parser.module.ExportedFunction("buffer_join_next_line")
	if joinNextLineFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_join_next_line' function")
	}

	sepPtr, err := b.parser.allocateString(separator)
	if err != nil {
		return err
	}
	defer b.parser.freeMemory(sepPtr)

	_, err = joinNextLineFn.Call(b.parser.ctx, uint64(b.bufferID), uint64(sepPtr), uint64(len(separator)))
	if err != nil {
		return fmt.Errorf("failed to join next line: %w", err)
	}

	return nil
}

// SwapCharactersBeforeCursor swaps the two characters before the cursor
func (b *Buffer) SwapCharactersBeforeCursor() error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return fmt.Errorf("buffer is nil or closed")
	}

	swapCharsFn := b.parser.module.ExportedFunction("buffer_swap_characters_before_cursor")
	if swapCharsFn == nil {
		return fmt.Errorf("WASM module does not export 'buffer_swap_characters_before_cursor' function")
	}

	_, err := swapCharsFn.Call(b.parser.ctx, uint64(b.bufferID))
	if err != nil {
		return fmt.Errorf("failed to swap characters: %w", err)
	}

	return nil
}

// Document returns the current Document for text analysis operations
func (b *Buffer) Document() (*WasmDocument, error) {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return nil, fmt.Errorf("buffer is nil or closed")
	}

	getDocumentFn := b.parser.module.ExportedFunction("buffer_get_document")
	if getDocumentFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'buffer_get_document' function")
	}

	results, err := getDocumentFn.Call(b.parser.ctx, uint64(b.bufferID))
	if err != nil {
		return nil, fmt.Errorf("failed to get document: %w", err)
	}

	documentID := uint32(results[0])
	return &WasmDocument{
		parser:     b.parser,
		documentID: documentID,
	}, nil
}

// ToWasmState serializes the buffer state for WASM interop
func (b *Buffer) ToWasmState() (*WasmBufferState, error) {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return nil, fmt.Errorf("buffer is nil or closed")
	}

	toWasmStateFn := b.parser.module.ExportedFunction("buffer_to_wasm_state")
	if toWasmStateFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'buffer_to_wasm_state' function")
	}

	results, err := toWasmStateFn.Call(b.parser.ctx, uint64(b.bufferID))
	if err != nil {
		return nil, fmt.Errorf("failed to serialize buffer state: %w", err)
	}

	var state WasmBufferState
	if err := b.parser.readJSONResult(results[0], &state); err != nil {
		return nil, err
	}

	return &state, nil
}

// BufferFromWasmState creates a new Buffer from serialized state
func (p *KeyParser) BufferFromWasmState(state *WasmBufferState) (*Buffer, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	fromWasmStateFn := p.module.ExportedFunction("buffer_from_wasm_state")
	if fromWasmStateFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'buffer_from_wasm_state' function")
	}

	stateJSON, err := json.Marshal(state)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal buffer state: %w", err)
	}

	statePtr, err := p.allocateString(string(stateJSON))
	if err != nil {
		return nil, err
	}
	defer p.freeMemory(statePtr)

	results, err := fromWasmStateFn.Call(p.ctx, uint64(statePtr), uint64(len(stateJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to create buffer from state: %w", err)
	}

	bufferID := uint32(results[0])
	return &Buffer{
		parser:   p,
		bufferID: bufferID,
	}, nil
}

// Close releases the buffer resources
func (b *Buffer) Close() error {
	if b == nil || b.parser == nil || b.parser.module == nil {
		return nil // Already closed
	}

	destroyBufferFn := b.parser.module.ExportedFunction("destroy_buffer")
	if destroyBufferFn == nil {
		return fmt.Errorf("WASM module does not export 'destroy_buffer' function")
	}

	_, err := destroyBufferFn.Call(b.parser.ctx, uint64(b.bufferID))
	if err != nil {
		return fmt.Errorf("failed to destroy buffer: %w", err)
	}

	// Mark as closed
	b.parser = nil
	return nil
}

// WasmDocument represents an immutable text document with cursor position for analysis
type WasmDocument struct {
	parser     *KeyParser
	documentID uint32
}

// NewDocument creates a new empty WasmDocument
func (p *KeyParser) NewDocument() (*WasmDocument, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	newDocumentFn := p.module.ExportedFunction("new_document")
	if newDocumentFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'new_document' function")
	}

	results, err := newDocumentFn.Call(p.ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create document: %w", err)
	}

	documentID := uint32(results[0])
	return &WasmDocument{
		parser:     p,
		documentID: documentID,
	}, nil
}

// NewDocumentWithText creates a new Document with the specified text and cursor position
func (p *KeyParser) NewDocumentWithText(text string, cursorPosition int) (*WasmDocument, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	docWithTextFn := p.module.ExportedFunction("document_with_text")
	if docWithTextFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'document_with_text' function")
	}

	textPtr, err := p.allocateString(text)
	if err != nil {
		return nil, err
	}
	defer p.freeMemory(textPtr)

	results, err := docWithTextFn.Call(p.ctx, uint64(textPtr), uint64(len(text)), uint64(cursorPosition))
	if err != nil {
		return nil, fmt.Errorf("failed to create document with text: %w", err)
	}

	documentID := uint32(results[0])
	return &WasmDocument{
		parser:     p,
		documentID: documentID,
	}, nil
}

// NewDocumentWithTextAndKey creates a new Document with text, cursor position, and last key
func (p *KeyParser) NewDocumentWithTextAndKey(text string, cursorPosition int, lastKey *Key) (*WasmDocument, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	docWithTextAndKeyFn := p.module.ExportedFunction("document_with_text_and_key")
	if docWithTextAndKeyFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'document_with_text_and_key' function")
	}

	textPtr, err := p.allocateString(text)
	if err != nil {
		return nil, err
	}
	defer p.freeMemory(textPtr)

	keyValue := uint64(0)
	hasKey := uint64(0)
	if lastKey != nil {
		keyValue = uint64(*lastKey)
		hasKey = 1
	}

	results, err := docWithTextAndKeyFn.Call(p.ctx, uint64(textPtr), uint64(len(text)), uint64(cursorPosition), hasKey, keyValue)
	if err != nil {
		return nil, fmt.Errorf("failed to create document with text and key: %w", err)
	}

	documentID := uint32(results[0])
	return &WasmDocument{
		parser:     p,
		documentID: documentID,
	}, nil
}

// Text returns the document text
func (d *WasmDocument) Text() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
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
func (d *WasmDocument) CursorPosition() (int, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	state, err := d.ToWasmState()
	if err != nil {
		return 0, err
	}

	return state.CursorPosition, nil
}

// DisplayCursorPosition returns the display cursor position accounting for Unicode width
func (d *WasmDocument) DisplayCursorPosition() (int, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	displayCursorPosFn := d.parser.module.ExportedFunction("document_display_cursor_position")
	if displayCursorPosFn == nil {
		return 0, fmt.Errorf("WASM module does not export 'document_display_cursor_position' function")
	}

	results, err := displayCursorPosFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get display cursor position: %w", err)
	}

	return int(results[0]), nil
}

// TextBeforeCursor returns the text before the cursor
func (d *WasmDocument) TextBeforeCursor() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	textBeforeCursorFn := d.parser.module.ExportedFunction("document_text_before_cursor")
	if textBeforeCursorFn == nil {
		return "", fmt.Errorf("WASM module does not export 'document_text_before_cursor' function")
	}

	results, err := textBeforeCursorFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get text before cursor: %w", err)
	}

	var text string
	if err := d.parser.readJSONResult(results[0], &text); err != nil {
		return "", err
	}

	return text, nil
}

// TextAfterCursor returns the text after the cursor
func (d *WasmDocument) TextAfterCursor() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	textAfterCursorFn := d.parser.module.ExportedFunction("document_text_after_cursor")
	if textAfterCursorFn == nil {
		return "", fmt.Errorf("WASM module does not export 'document_text_after_cursor' function")
	}

	results, err := textAfterCursorFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get text after cursor: %w", err)
	}

	var text string
	if err := d.parser.readJSONResult(results[0], &text); err != nil {
		return "", err
	}

	return text, nil
}

// GetWordBeforeCursor returns the word before the cursor
func (d *WasmDocument) GetWordBeforeCursor() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	getWordBeforeFn := d.parser.module.ExportedFunction("document_get_word_before_cursor")
	if getWordBeforeFn == nil {
		return "", fmt.Errorf("WASM module does not export 'document_get_word_before_cursor' function")
	}

	results, err := getWordBeforeFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get word before cursor: %w", err)
	}

	var word string
	if err := d.parser.readJSONResult(results[0], &word); err != nil {
		return "", err
	}

	return word, nil
}

// GetWordAfterCursor returns the word after the cursor
func (d *WasmDocument) GetWordAfterCursor() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	getWordAfterFn := d.parser.module.ExportedFunction("document_get_word_after_cursor")
	if getWordAfterFn == nil {
		return "", fmt.Errorf("WASM module does not export 'document_get_word_after_cursor' function")
	}

	results, err := getWordAfterFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get word after cursor: %w", err)
	}

	var word string
	if err := d.parser.readJSONResult(results[0], &word); err != nil {
		return "", err
	}

	return word, nil
}

// CurrentLine returns the current line text
func (d *WasmDocument) CurrentLine() (string, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return "", fmt.Errorf("document is nil or closed")
	}

	currentLineFn := d.parser.module.ExportedFunction("document_current_line")
	if currentLineFn == nil {
		return "", fmt.Errorf("WASM module does not export 'document_current_line' function")
	}

	results, err := currentLineFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return "", fmt.Errorf("failed to get current line: %w", err)
	}

	var line string
	if err := d.parser.readJSONResult(results[0], &line); err != nil {
		return "", err
	}

	return line, nil
}

// LineCount returns the number of lines in the document
func (d *WasmDocument) LineCount() (int, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	lineCountFn := d.parser.module.ExportedFunction("document_line_count")
	if lineCountFn == nil {
		return 0, fmt.Errorf("WASM module does not export 'document_line_count' function")
	}

	results, err := lineCountFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get line count: %w", err)
	}

	return int(results[0]), nil
}

// CursorPositionRow returns the cursor row (0-based)
func (d *WasmDocument) CursorPositionRow() (int, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	cursorRowFn := d.parser.module.ExportedFunction("document_cursor_position_row")
	if cursorRowFn == nil {
		return 0, fmt.Errorf("WASM module does not export 'document_cursor_position_row' function")
	}

	results, err := cursorRowFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get cursor row: %w", err)
	}

	return int(results[0]), nil
}

// CursorPositionCol returns the cursor column (0-based)
func (d *WasmDocument) CursorPositionCol() (int, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return 0, fmt.Errorf("document is nil or closed")
	}

	cursorColFn := d.parser.module.ExportedFunction("document_cursor_position_col")
	if cursorColFn == nil {
		return 0, fmt.Errorf("WASM module does not export 'document_cursor_position_col' function")
	}

	results, err := cursorColFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return 0, fmt.Errorf("failed to get cursor column: %w", err)
	}

	return int(results[0]), nil
}

// ToWasmState serializes the document state for WASM interop
func (d *WasmDocument) ToWasmState() (*WasmDocumentState, error) {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return nil, fmt.Errorf("document is nil or closed")
	}

	toWasmStateFn := d.parser.module.ExportedFunction("document_to_wasm_state")
	if toWasmStateFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'document_to_wasm_state' function")
	}

	results, err := toWasmStateFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return nil, fmt.Errorf("failed to serialize document state: %w", err)
	}

	var state WasmDocumentState
	if err := d.parser.readJSONResult(results[0], &state); err != nil {
		return nil, err
	}

	return &state, nil
}

// DocumentFromWasmState creates a new Document from serialized state
func (p *KeyParser) DocumentFromWasmState(state *WasmDocumentState) (*WasmDocument, error) {
	if p == nil || p.module == nil {
		return nil, fmt.Errorf("parser is nil or closed")
	}

	fromWasmStateFn := p.module.ExportedFunction("document_from_wasm_state")
	if fromWasmStateFn == nil {
		return nil, fmt.Errorf("WASM module does not export 'document_from_wasm_state' function")
	}

	stateJSON, err := json.Marshal(state)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal document state: %w", err)
	}

	statePtr, err := p.allocateString(string(stateJSON))
	if err != nil {
		return nil, err
	}
	defer p.freeMemory(statePtr)

	results, err := fromWasmStateFn.Call(p.ctx, uint64(statePtr), uint64(len(stateJSON)))
	if err != nil {
		return nil, fmt.Errorf("failed to create document from state: %w", err)
	}

	documentID := uint32(results[0])
	return &WasmDocument{
		parser:     p,
		documentID: documentID,
	}, nil
}

// Close releases the document resources
func (d *WasmDocument) Close() error {
	if d == nil || d.parser == nil || d.parser.module == nil {
		return nil // Already closed
	}

	destroyDocumentFn := d.parser.module.ExportedFunction("destroy_document")
	if destroyDocumentFn == nil {
		return fmt.Errorf("WASM module does not export 'destroy_document' function")
	}

	_, err := destroyDocumentFn.Call(d.parser.ctx, uint64(d.documentID))
	if err != nil {
		return fmt.Errorf("failed to destroy document: %w", err)
	}

	// Mark as closed
	d.parser = nil
	return nil
}
