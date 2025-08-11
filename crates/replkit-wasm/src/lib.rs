use replkit_core::{
    u32_to_key, Buffer, Document, KeyParser, WasmBufferState, WasmDocumentState, WasmKeyEvent,
    Color, TextStyle, ClearType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::raw::c_void;
use std::ptr;
use std::slice;
use std::str;

// Prompt-related structures for Go bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    pub description: String,
}

// Console output command structures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputCommand {
    WriteText { text: String },
    WriteStyledText { text: String, style: SerializableTextStyle },
    WriteSafeText { text: String },
    MoveCursorTo { row: u16, col: u16 },
    MoveCursorRelative { row_delta: i16, col_delta: i16 },
    Clear { clear_type: SerializableClearType },
    SetStyle { style: SerializableTextStyle },
    ResetStyle,
    Flush,
    SetAlternateScreen { enabled: bool },
    SetCursorVisible { visible: bool },
    GetCursorPosition, // Response will be in separate structure
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTextStyle {
    pub foreground: Option<SerializableColor>,
    pub background: Option<SerializableColor>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SerializableColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
    Ansi256(u8),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SerializableClearType {
    All,
    FromCursor,
    ToCursor,
    CurrentLine,
    FromCursorToEndOfLine,
    FromBeginningOfLineToCursor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputResponse {
    pub success: bool,
    pub error_message: Option<String>,
    pub escape_sequence: Option<String>,
    pub cursor_position: Option<(u16, u16)>,
}

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
        if ALLOCATIONS.is_none() {
            ALLOCATIONS = Some(HashMap::new());
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

// Type conversion helpers
impl From<SerializableColor> for Color {
    fn from(color: SerializableColor) -> Self {
        match color {
            SerializableColor::Black => Color::Black,
            SerializableColor::Red => Color::Red,
            SerializableColor::Green => Color::Green,
            SerializableColor::Yellow => Color::Yellow,
            SerializableColor::Blue => Color::Blue,
            SerializableColor::Magenta => Color::Magenta,
            SerializableColor::Cyan => Color::Cyan,
            SerializableColor::White => Color::White,
            SerializableColor::BrightBlack => Color::BrightBlack,
            SerializableColor::BrightRed => Color::BrightRed,
            SerializableColor::BrightGreen => Color::BrightGreen,
            SerializableColor::BrightYellow => Color::BrightYellow,
            SerializableColor::BrightBlue => Color::BrightBlue,
            SerializableColor::BrightMagenta => Color::BrightMagenta,
            SerializableColor::BrightCyan => Color::BrightCyan,
            SerializableColor::BrightWhite => Color::BrightWhite,
            SerializableColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
            SerializableColor::Ansi256(code) => Color::Ansi256(code),
        }
    }
}

impl From<SerializableTextStyle> for TextStyle {
    fn from(style: SerializableTextStyle) -> Self {
        TextStyle {
            foreground: style.foreground.map(Into::into),
            background: style.background.map(Into::into),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            strikethrough: style.strikethrough,
            dim: style.dim,
            reverse: style.reverse,
        }
    }
}

impl From<SerializableClearType> for ClearType {
    fn from(clear_type: SerializableClearType) -> Self {
        match clear_type {
            SerializableClearType::All => ClearType::All,
            SerializableClearType::FromCursor => ClearType::FromCursor,
            SerializableClearType::ToCursor => ClearType::ToCursor,
            SerializableClearType::CurrentLine => ClearType::CurrentLine,
            SerializableClearType::FromCursorToEndOfLine => ClearType::FromCursorToEndOfLine,
            SerializableClearType::FromBeginningOfLineToCursor => ClearType::FromBeginningOfLineToCursor,
        }
    }
}

// ANSI escape sequence generator
struct AnsiGenerator;

impl AnsiGenerator {
    fn color_to_ansi(color: &Color, is_background: bool) -> String {
        let base = if is_background { 40 } else { 30 };
        let bright_base = if is_background { 100 } else { 90 };
        
        match color {
            Color::Black => format!("\x1b[{}m", base),
            Color::Red => format!("\x1b[{}m", base + 1),
            Color::Green => format!("\x1b[{}m", base + 2),
            Color::Yellow => format!("\x1b[{}m", base + 3),
            Color::Blue => format!("\x1b[{}m", base + 4),
            Color::Magenta => format!("\x1b[{}m", base + 5),
            Color::Cyan => format!("\x1b[{}m", base + 6),
            Color::White => format!("\x1b[{}m", base + 7),
            Color::BrightBlack => format!("\x1b[{}m", bright_base),
            Color::BrightRed => format!("\x1b[{}m", bright_base + 1),
            Color::BrightGreen => format!("\x1b[{}m", bright_base + 2),
            Color::BrightYellow => format!("\x1b[{}m", bright_base + 3),
            Color::BrightBlue => format!("\x1b[{}m", bright_base + 4),
            Color::BrightMagenta => format!("\x1b[{}m", bright_base + 5),
            Color::BrightCyan => format!("\x1b[{}m", bright_base + 6),
            Color::BrightWhite => format!("\x1b[{}m", bright_base + 7),
            Color::Rgb(r, g, b) => {
                if is_background {
                    format!("\x1b[48;2;{};{};{}m", r, g, b)
                } else {
                    format!("\x1b[38;2;{};{};{}m", r, g, b)
                }
            }
            Color::Ansi256(code) => {
                if is_background {
                    format!("\x1b[48;5;{}m", code)
                } else {
                    format!("\x1b[38;5;{}m", code)
                }
            }
        }
    }

    fn style_to_ansi(style: &TextStyle) -> String {
        let mut result = String::new();
        
        if let Some(fg) = &style.foreground {
            result.push_str(&Self::color_to_ansi(fg, false));
        }
        
        if let Some(bg) = &style.background {
            result.push_str(&Self::color_to_ansi(bg, true));
        }
        
        if style.bold {
            result.push_str("\x1b[1m");
        }
        if style.dim {
            result.push_str("\x1b[2m");
        }
        if style.italic {
            result.push_str("\x1b[3m");
        }
        if style.underline {
            result.push_str("\x1b[4m");
        }
        if style.reverse {
            result.push_str("\x1b[7m");
        }
        if style.strikethrough {
            result.push_str("\x1b[9m");
        }
        
        result
    }

    fn clear_to_ansi(clear_type: &ClearType) -> String {
        match clear_type {
            ClearType::All => "\x1b[2J".to_string(),
            ClearType::FromCursor => "\x1b[0J".to_string(),
            ClearType::ToCursor => "\x1b[1J".to_string(),
            ClearType::CurrentLine => "\x1b[2K".to_string(),
            ClearType::FromCursorToEndOfLine => "\x1b[0K".to_string(),
            ClearType::FromBeginningOfLineToCursor => "\x1b[1K".to_string(),
        }
    }

    fn process_command(command: &OutputCommand) -> OutputResponse {
        match command {
            OutputCommand::WriteText { text } => OutputResponse {
                success: true,
                error_message: None,
                escape_sequence: Some(text.clone()),
                cursor_position: None,
            },
            OutputCommand::WriteStyledText { text, style } => {
                let style_ansi = Self::style_to_ansi(&style.clone().into());
                let reset_ansi = "\x1b[0m";
                let full_sequence = format!("{}{}{}", style_ansi, text, reset_ansi);
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(full_sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::WriteSafeText { text } => {
                // Simple sanitization: remove common control sequences
                let safe_text = text
                    .replace('\x1b', "\\e")
                    .replace('\x07', "\\a")
                    .replace('\x08', "\\b");
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(safe_text),
                    cursor_position: None,
                }
            },
            OutputCommand::MoveCursorTo { row, col } => {
                // Convert 0-based to 1-based for ANSI
                let sequence = format!("\x1b[{};{}H", row + 1, col + 1);
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::MoveCursorRelative { row_delta, col_delta } => {
                let mut sequence = String::new();
                
                match *row_delta {
                    d if d > 0 => sequence.push_str(&format!("\x1b[{}B", d)),
                    d if d < 0 => sequence.push_str(&format!("\x1b[{}A", -d)),
                    _ => {}
                }
                
                match *col_delta {
                    d if d > 0 => sequence.push_str(&format!("\x1b[{}C", d)),
                    d if d < 0 => sequence.push_str(&format!("\x1b[{}D", -d)),
                    _ => {}
                }
                
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::Clear { clear_type } => {
                let sequence = Self::clear_to_ansi(&clear_type.clone().into());
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::SetStyle { style } => {
                let sequence = Self::style_to_ansi(&style.clone().into());
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::ResetStyle => OutputResponse {
                success: true,
                error_message: None,
                escape_sequence: Some("\x1b[0m".to_string()),
                cursor_position: None,
            },
            OutputCommand::Flush => OutputResponse {
                success: true,
                error_message: None,
                escape_sequence: Some(String::new()), // No-op for flush
                cursor_position: None,
            },
            OutputCommand::SetAlternateScreen { enabled } => {
                let sequence = if *enabled {
                    "\x1b[?1049h".to_string()
                } else {
                    "\x1b[?1049l".to_string()
                };
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::SetCursorVisible { visible } => {
                let sequence = if *visible {
                    "\x1b[?25h".to_string()
                } else {
                    "\x1b[?25l".to_string()
                };
                OutputResponse {
                    success: true,
                    error_message: None,
                    escape_sequence: Some(sequence),
                    cursor_position: None,
                }
            },
            OutputCommand::GetCursorPosition => OutputResponse {
                success: true,
                error_message: None,
                escape_sequence: Some("\x1b[6n".to_string()), // Query cursor position
                cursor_position: None,
            },
        }
    }
}

// Initialize the global parser storage
fn init_parsers() {
    unsafe {
        if PARSERS.is_none() {
            PARSERS = Some(HashMap::new());
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

#[no_mangle]
pub extern "C" fn feed(parser_id: u32, data_ptr: *const u8, data_len: u32) -> u64 {
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
        if BUFFERS.is_none() {
            BUFFERS = Some(HashMap::new());
        }
    }
}

// Initialize document storage
fn init_documents() {
    unsafe {
        if DOCUMENTS.is_none() {
            DOCUMENTS = Some(HashMap::new());
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

#[no_mangle]
pub extern "C" fn buffer_insert_text(
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

#[no_mangle]
pub extern "C" fn buffer_set_text(buffer_id: u32, text_ptr: *const u8, text_len: u32) -> u32 {
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

#[no_mangle]
pub extern "C" fn buffer_join_next_line(
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

#[no_mangle]
pub extern "C" fn buffer_from_wasm_state(state_ptr: *const u8, state_len: u32) -> u32 {
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

#[no_mangle]
pub extern "C" fn document_with_text(
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

#[no_mangle]
pub extern "C" fn document_with_text_and_key(
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

#[no_mangle]
pub extern "C" fn document_from_wasm_state(state_ptr: *const u8, state_len: u32) -> u32 {
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

// Console Output WASM exports
#[no_mangle]
pub extern "C" fn wasm_output_command(command_ptr: *const u8, command_len: u32) -> u64 {
    let command_json = unsafe {
        let slice = slice::from_raw_parts(command_ptr, command_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => {
                // Return error response for invalid UTF-8
                let error_response = OutputResponse {
                    success: false,
                    error_message: Some("Invalid UTF-8 in command".to_string()),
                    escape_sequence: None,
                    cursor_position: None,
                };
                return serialize_json(&error_response);
            }
        }
    };

    let command = match serde_json::from_str::<OutputCommand>(command_json) {
        Ok(cmd) => cmd,
        Err(e) => {
            // Return error response for invalid JSON
            let error_response = OutputResponse {
                success: false,
                error_message: Some(format!("Invalid JSON: {}", e)),
                escape_sequence: None,
                cursor_position: None,
            };
            return serialize_json(&error_response);
        }
    };

    let response = AnsiGenerator::process_command(&command);
    serialize_json(&response)
}

// Prompt-related WASM exports for Go
#[no_mangle]
pub extern "C" fn wasm_filter_suggestions(
    suggestions_ptr: *const u8,
    suggestions_len: u32,
    prefix_ptr: *const u8,
    prefix_len: u32,
    ignore_case: u32,
) -> u64 {
    let suggestions_json = unsafe {
        let slice = slice::from_raw_parts(suggestions_ptr, suggestions_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let prefix = unsafe {
        let slice = slice::from_raw_parts(prefix_ptr, prefix_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let suggestions: Vec<Suggestion> = match serde_json::from_str(suggestions_json) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let filtered: Vec<Suggestion> = suggestions
        .into_iter()
        .filter(|s| {
            if ignore_case != 0 {
                s.text.to_lowercase().starts_with(&prefix.to_lowercase())
            } else {
                s.text.starts_with(prefix)
            }
        })
        .collect();

    serialize_json(&filtered)
}

// Complete prompt rendering structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptState {
    pub prefix: String,
    pub input_text: String,
    pub cursor_position: usize,
    pub suggestions: Vec<Suggestion>,
    pub selected_suggestion: Option<usize>,
    pub show_suggestions: bool,
    pub window_size: Option<(u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOutput {
    pub success: bool,
    pub error_message: Option<String>,
    pub output_bytes: Option<Vec<u8>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

// WASM function to render complete prompt
#[no_mangle]
pub extern "C" fn wasm_render_prompt(state_ptr: *const u8, state_len: u32) -> u64 {
    let state_json = unsafe {
        let slice = slice::from_raw_parts(state_ptr, state_len as usize);
        match str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => {
                let error_response = RenderOutput {
                    success: false,
                    error_message: Some("Invalid UTF-8 in prompt state".to_string()),
                    output_bytes: None,
                    cursor_row: 0,
                    cursor_col: 0,
                };
                return serialize_json(&error_response);
            }
        }
    };

    let state = match serde_json::from_str::<PromptState>(state_json) {
        Ok(s) => s,
        Err(e) => {
            let error_response = RenderOutput {
                success: false,
                error_message: Some(format!("Invalid JSON: {}", e)),
                output_bytes: None,
                cursor_row: 0,
                cursor_col: 0,
            };
            return serialize_json(&error_response);
        }
    };

    let output = render_complete_prompt(&state);
    serialize_json(&output)
}

fn render_complete_prompt(state: &PromptState) -> RenderOutput {
    let mut output = Vec::new();
    
    // Clear current line and move to beginning
    output.extend_from_slice(b"\r\x1b[2K");
    
    // Render prefix with styling
    output.extend_from_slice(b"\x1b[1m\x1b[34m"); // Bold blue
    output.extend_from_slice(state.prefix.as_bytes());
    output.extend_from_slice(b"\x1b[0m"); // Reset
    
    // Render input text
    output.extend_from_slice(state.input_text.as_bytes());
    
    // Calculate cursor position (prefix length + cursor position in text)
    let cursor_col = state.prefix.len() + state.cursor_position;
    
    // If showing suggestions, render them
    if state.show_suggestions && !state.suggestions.is_empty() {
        // Move to next line for suggestions
        output.extend_from_slice(b"\n");
        
        for (i, suggestion) in state.suggestions.iter().enumerate() {
            // Highlight selected suggestion
            if Some(i) == state.selected_suggestion {
                output.extend_from_slice(b"\x1b[7m"); // Reverse video
            }
            
            // Format suggestion
            let suggestion_line = format!("  {} - {}\n", suggestion.text, suggestion.description);
            output.extend_from_slice(suggestion_line.as_bytes());
            
            if Some(i) == state.selected_suggestion {
                output.extend_from_slice(b"\x1b[0m"); // Reset
            }
        }
        
        // Move cursor back to input line
        let lines_up = state.suggestions.len();
        output.extend_from_slice(format!("\x1b[{}A", lines_up).as_bytes());
    }
    
    // Position cursor at the correct location in the input
    output.extend_from_slice(format!("\r\x1b[{}C", cursor_col).as_bytes());
    
    RenderOutput {
        success: true,
        error_message: None,
        output_bytes: Some(output),
        cursor_row: 0, // We're staying on the same line
        cursor_col: cursor_col as u16,
    }
}
