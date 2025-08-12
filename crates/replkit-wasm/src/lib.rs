use replkit_core::{
    u32_to_key, Buffer, Document, KeyParser, WasmBufferState, WasmDocumentState, WasmKeyEvent,
};
use std::collections::HashMap;
use std::os::raw::c_void;
use std::ptr;
use std::slice;
use std::str;

// Global storage for instances
static mut PARSERS: Option<HashMap<u32, KeyParser>> = None;
static mut BUFFERS: Option<HashMap<u32, Buffer>> = None;
static mut DOCUMENTS: Option<HashMap<u32, Document>> = None;
static mut NEXT_ID: u32 = 1;

// Use WASM-compatible allocator
extern "C" {
    fn __wbindgen_malloc(size: usize) -> *mut u8;
    fn __wbindgen_free(ptr: *mut u8, size: usize);
}

// Simple allocation tracking using unsafe static
static mut ALLOCATIONS: Option<HashMap<usize, usize>> = None;

// Helper function to initialize allocations map
fn init_allocations() {
    unsafe {
        let allocations = &raw mut ALLOCATIONS;
        if (*allocations).is_none() {
            *allocations = Some(HashMap::new());
        }
    }
}

// Helper function to allocate and track memory
fn allocate_tracked(size: usize) -> *mut u8 {
    init_allocations();
    let ptr = unsafe { __wbindgen_malloc(size) };
    if !ptr.is_null() {
        unsafe {
            if let Some(ref mut allocations) = ALLOCATIONS {
                allocations.insert(ptr as usize, size);
            }
        }
    }
    ptr
}

// Helper function to free tracked memory
fn free_tracked(ptr: *mut u8) {
    init_allocations();
    let ptr_addr = ptr as usize;
    unsafe {
        if let Some(ref mut allocations) = ALLOCATIONS {
            if let Some(size) = allocations.remove(&ptr_addr) {
                __wbindgen_free(ptr, size);
            }
        }
    }
}

// Helper function to serialize events and return packed pointer/length
fn serialize_events(events: Vec<WasmKeyEvent>) -> u64 {
    // Serialize to JSON
    let json = match serde_json::to_string(&events) {
        Ok(json) => json,
        Err(_) => return 0, // Return null pointer on error
    };

    // Allocate memory for the JSON string
    let json_bytes = json.as_bytes();
    let json_ptr = allocate_tracked(json_bytes.len());

    if json_ptr.is_null() {
        return 0;
    }

    unsafe {
        ptr::copy_nonoverlapping(json_bytes.as_ptr(), json_ptr, json_bytes.len());
    }

    // Return pointer and length as packed u64
    ((json_ptr as u64) << 32) | (json_bytes.len() as u64)
}

// Initialize the global parser storage
fn init_parsers() {
    unsafe {
        let parsers = &raw mut PARSERS;
        if (*parsers).is_none() {
            *parsers = Some(HashMap::new());
        }
    }
}

// WASM-compatible C-style exports
#[no_mangle]
pub extern "C" fn new_parser() -> u32 {
    init_parsers();

    let parser = KeyParser::new();
    let id = unsafe { NEXT_ID };

    unsafe {
        NEXT_ID += 1;
        if let Some(ref mut parsers) = PARSERS {
            parsers.insert(id, parser);
        }
    }

    id
}

/// Feed input data to a parser
///
/// # Safety
/// The caller must ensure that `data_ptr` points to a valid memory region of at least `data_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn feed(parser_id: u32, data_ptr: *const u8, data_len: u32) -> u64 {
    init_parsers();

    let data = unsafe { slice::from_raw_parts(data_ptr, data_len as usize) };

    let events = unsafe {
        if let Some(ref mut parsers) = PARSERS {
            if let Some(parser) = parsers.get_mut(&parser_id) {
                parser.feed(data)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    // Convert to serializable format and return
    let serializable_events: Vec<WasmKeyEvent> =
        events.into_iter().map(WasmKeyEvent::from).collect();
    serialize_events(serializable_events)
}

#[no_mangle]
pub extern "C" fn flush(parser_id: u32) -> u64 {
    init_parsers();

    let events = unsafe {
        if let Some(ref mut parsers) = PARSERS {
            if let Some(parser) = parsers.get_mut(&parser_id) {
                parser.flush()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    // Convert to serializable format and return
    let serializable_events: Vec<WasmKeyEvent> =
        events.into_iter().map(WasmKeyEvent::from).collect();
    serialize_events(serializable_events)
}

#[no_mangle]
pub extern "C" fn reset(parser_id: u32) {
    init_parsers();

    unsafe {
        if let Some(ref mut parsers) = PARSERS {
            if let Some(parser) = parsers.get_mut(&parser_id) {
                parser.reset();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn destroy_parser(parser_id: u32) {
    init_parsers();

    unsafe {
        if let Some(ref mut parsers) = PARSERS {
            parsers.remove(&parser_id);
        }
    }
}

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    unsafe { __wbindgen_malloc(size) as *mut c_void }
}

#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    if !ptr.is_null() {
        free_tracked(ptr as *mut u8);
    }
}

// Initialize buffer storage
fn init_buffers() {
    unsafe {
        let buffers = &raw mut BUFFERS;
        if (*buffers).is_none() {
            *buffers = Some(HashMap::new());
        }
    }
}

// Initialize document storage
fn init_documents() {
    unsafe {
        let documents = &raw mut DOCUMENTS;
        if (*documents).is_none() {
            *documents = Some(HashMap::new());
        }
    }
}

// Helper function to serialize string and return packed pointer/length
fn serialize_string(s: &str) -> u64 {
    let json = match serde_json::to_string(s) {
        Ok(json) => json,
        Err(_) => return 0,
    };

    let json_bytes = json.as_bytes();
    let json_ptr = allocate_tracked(json_bytes.len());

    if json_ptr.is_null() {
        return 0;
    }

    unsafe {
        ptr::copy_nonoverlapping(json_bytes.as_ptr(), json_ptr, json_bytes.len());
    }

    ((json_ptr as u64) << 32) | (json_bytes.len() as u64)
}

// Helper function to serialize any serializable type
fn serialize_json<T: serde::Serialize>(data: &T) -> u64 {
    let json = match serde_json::to_string(data) {
        Ok(json) => json,
        Err(_) => return 0,
    };

    let json_bytes = json.as_bytes();
    let json_ptr = allocate_tracked(json_bytes.len());

    if json_ptr.is_null() {
        return 0;
    }

    unsafe {
        ptr::copy_nonoverlapping(json_bytes.as_ptr(), json_ptr, json_bytes.len());
    }

    ((json_ptr as u64) << 32) | (json_bytes.len() as u64)
}

// Buffer WASM exports
#[no_mangle]
pub extern "C" fn new_buffer() -> u32 {
    init_buffers();

    let buffer = Buffer::new();
    let id = unsafe { NEXT_ID };

    unsafe {
        NEXT_ID += 1;
        if let Some(ref mut buffers) = BUFFERS {
            buffers.insert(id, buffer);
        }
    }

    id
}

/// Insert text into a buffer
///
/// # Safety
/// The caller must ensure that `text_ptr` points to a valid UTF-8 memory region of at least `text_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn buffer_insert_text(
    buffer_id: u32,
    text_ptr: *const u8,
    text_len: u32,
    overwrite: u32,
    move_cursor: u32,
) -> u32 {
    init_buffers();

    let text = unsafe {
        let slice = slice::from_raw_parts(text_ptr, text_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 1, // Error: invalid UTF-8
        }
    };

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.insert_text(text, overwrite != 0, move_cursor != 0);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_delete_before_cursor(buffer_id: u32, count: u32) -> u64 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                let deleted = buffer.delete_before_cursor(count as usize);
                serialize_string(&deleted)
            } else {
                0 // Error: buffer not found
            }
        } else {
            0 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_delete(buffer_id: u32, count: u32) -> u64 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                let deleted = buffer.delete(count as usize);
                serialize_string(&deleted)
            } else {
                0 // Error: buffer not found
            }
        } else {
            0 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_cursor_left(buffer_id: u32, count: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.cursor_left(count as usize);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_cursor_right(buffer_id: u32, count: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.cursor_right(count as usize);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_cursor_up(buffer_id: u32, count: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.cursor_up(count as usize);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_cursor_down(buffer_id: u32, count: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.cursor_down(count as usize);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

/// Set the text content of a buffer
///
/// # Safety
/// The caller must ensure that `text_ptr` points to a valid UTF-8 memory region of at least `text_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn buffer_set_text(buffer_id: u32, text_ptr: *const u8, text_len: u32) -> u32 {
    init_buffers();

    let text = unsafe {
        let slice = slice::from_raw_parts(text_ptr, text_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s.to_string(),
            Err(_) => return 1, // Error: invalid UTF-8
        }
    };

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.set_text(text);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_set_cursor_position(buffer_id: u32, position: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.set_cursor_position(position as usize);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_new_line(buffer_id: u32, copy_margin: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.new_line(copy_margin != 0);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

/// Join the next line to the current line with a separator
///
/// # Safety
/// The caller must ensure that `separator_ptr` points to a valid UTF-8 memory region of at least `separator_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn buffer_join_next_line(
    buffer_id: u32,
    separator_ptr: *const u8,
    separator_len: u32,
) -> u32 {
    init_buffers();

    let separator = unsafe {
        let slice = slice::from_raw_parts(separator_ptr, separator_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 1, // Error: invalid UTF-8
        }
    };

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.join_next_line(separator);
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_swap_characters_before_cursor(buffer_id: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                buffer.swap_characters_before_cursor();
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn buffer_to_wasm_state(buffer_id: u32) -> u64 {
    init_buffers();

    unsafe {
        if let Some(ref buffers) = BUFFERS {
            if let Some(buffer) = buffers.get(&buffer_id) {
                let state = buffer.to_wasm_state();
                serialize_json(&state)
            } else {
                0 // Error: buffer not found
            }
        } else {
            0 // Error: buffers not initialized
        }
    }
}

/// Create a buffer from a serialized WASM state
///
/// # Safety
/// The caller must ensure that `state_ptr` points to a valid UTF-8 JSON memory region of at least `state_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn buffer_from_wasm_state(state_ptr: *const u8, state_len: u32) -> u32 {
    init_buffers();

    let state_json = unsafe {
        let slice = slice::from_raw_parts(state_ptr, state_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 0, // Error: invalid UTF-8
        }
    };

    if let Ok(state) = serde_json::from_str::<WasmBufferState>(state_json) {
        let buffer = Buffer::from_wasm_state(state);
        let id = unsafe { NEXT_ID };

        unsafe {
            NEXT_ID += 1;
            if let Some(ref mut buffers) = BUFFERS {
                buffers.insert(id, buffer);
            }
        }

        id
    } else {
        0 // Error: invalid state
    }
}

#[no_mangle]
pub extern "C" fn buffer_get_document(buffer_id: u32) -> u32 {
    init_buffers();
    init_documents();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if let Some(buffer) = buffers.get_mut(&buffer_id) {
                let document = buffer.document().clone();
                let id = NEXT_ID;
                NEXT_ID += 1;

                if let Some(ref mut documents) = DOCUMENTS {
                    documents.insert(id, document);
                }

                id
            } else {
                0 // Error: buffer not found
            }
        } else {
            0 // Error: buffers not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn destroy_buffer(buffer_id: u32) -> u32 {
    init_buffers();

    unsafe {
        if let Some(ref mut buffers) = BUFFERS {
            if buffers.remove(&buffer_id).is_some() {
                0 // Success
            } else {
                1 // Error: buffer not found
            }
        } else {
            1 // Error: buffers not initialized
        }
    }
}

// Document WASM exports
#[no_mangle]
pub extern "C" fn new_document() -> u32 {
    init_documents();

    let document = Document::new();
    let id = unsafe { NEXT_ID };

    unsafe {
        NEXT_ID += 1;
        if let Some(ref mut documents) = DOCUMENTS {
            documents.insert(id, document);
        }
    }

    id
}

/// Create a document with text
///
/// # Safety
/// The caller must ensure that `text_ptr` points to a valid UTF-8 memory region of at least `text_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn document_with_text(
    text_ptr: *const u8,
    text_len: u32,
    cursor_position: u32,
) -> u32 {
    init_documents();

    let text = unsafe {
        let slice = slice::from_raw_parts(text_ptr, text_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s.to_string(),
            Err(_) => return 0, // Error: invalid UTF-8
        }
    };

    let document = Document::with_text(text, cursor_position as usize);
    let id = unsafe { NEXT_ID };

    unsafe {
        NEXT_ID += 1;
        if let Some(ref mut documents) = DOCUMENTS {
            documents.insert(id, document);
        }
    }

    id
}

/// Create a document with text and optional key
///
/// # Safety
/// The caller must ensure that `text_ptr` points to a valid UTF-8 memory region of at least `text_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn document_with_text_and_key(
    text_ptr: *const u8,
    text_len: u32,
    cursor_position: u32,
    has_key: u32,
    key_value: u32,
) -> u32 {
    init_documents();

    let text = unsafe {
        let slice = slice::from_raw_parts(text_ptr, text_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s.to_string(),
            Err(_) => return 0, // Error: invalid UTF-8
        }
    };

    let key = if has_key != 0 {
        Some(u32_to_key(key_value))
    } else {
        None
    };

    let document = Document::with_text_and_key(text, cursor_position as usize, key);
    let id = unsafe { NEXT_ID };

    unsafe {
        NEXT_ID += 1;
        if let Some(ref mut documents) = DOCUMENTS {
            documents.insert(id, document);
        }
    }

    id
}

#[no_mangle]
pub extern "C" fn document_text_before_cursor(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let text = document.text_before_cursor();
                serialize_string(text)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_text_after_cursor(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let text = document.text_after_cursor();
                serialize_string(text)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_get_word_before_cursor(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let word = document.get_word_before_cursor();
                serialize_string(word)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_get_word_after_cursor(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let word = document.get_word_after_cursor();
                serialize_string(word)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_current_line(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let line = document.current_line();
                serialize_string(&line)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_line_count(document_id: u32) -> u32 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                document.line_count() as u32
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_cursor_position_row(document_id: u32) -> u32 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                document.cursor_position_row() as u32
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_cursor_position_col(document_id: u32) -> u32 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                document.cursor_position_col() as u32
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_display_cursor_position(document_id: u32) -> u32 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                document.display_cursor_position() as u32
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

#[no_mangle]
pub extern "C" fn document_to_wasm_state(document_id: u32) -> u64 {
    init_documents();

    unsafe {
        if let Some(ref documents) = DOCUMENTS {
            if let Some(document) = documents.get(&document_id) {
                let state = document.to_wasm_state();
                serialize_json(&state)
            } else {
                0 // Error: document not found
            }
        } else {
            0 // Error: documents not initialized
        }
    }
}

/// Create a document from a serialized WASM state
///
/// # Safety
/// The caller must ensure that `state_ptr` points to a valid UTF-8 JSON memory region of at least `state_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn document_from_wasm_state(state_ptr: *const u8, state_len: u32) -> u32 {
    init_documents();

    let state_json = unsafe {
        let slice = slice::from_raw_parts(state_ptr, state_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 0, // Error: invalid UTF-8
        }
    };

    if let Ok(state) = serde_json::from_str::<WasmDocumentState>(state_json) {
        let document = Document::from_wasm_state(state);
        let id = unsafe { NEXT_ID };

        unsafe {
            NEXT_ID += 1;
            if let Some(ref mut documents) = DOCUMENTS {
                documents.insert(id, document);
            }
        }

        id
    } else {
        0 // Error: invalid state
    }
}

#[no_mangle]
pub extern "C" fn destroy_document(document_id: u32) -> u32 {
    init_documents();

    unsafe {
        if let Some(ref mut documents) = DOCUMENTS {
            if documents.remove(&document_id).is_some() {
                0 // Success
            } else {
                1 // Error: document not found
            }
        } else {
            1 // Error: documents not initialized
        }
    }
}
