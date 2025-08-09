use prompt_core::{KeyParser, WasmKeyEvent};
use serde_json;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::ptr;
use std::slice;


// Global storage for parser instances
static mut PARSERS: Option<HashMap<u32, KeyParser>> = None;
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
    let serializable_events: Vec<WasmKeyEvent> = events.into_iter().map(WasmKeyEvent::from).collect();
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
    let serializable_events: Vec<WasmKeyEvent> = events.into_iter().map(WasmKeyEvent::from).collect();
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