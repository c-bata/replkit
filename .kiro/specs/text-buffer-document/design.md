# Design Document

## Overview

The text-buffer-document system provides the core text management infrastructure for the go-prompt port. It consists of two primary components: the Document structure for immutable text analysis and the Buffer structure for mutable text editing operations. This design ensures efficient Unicode handling, WASM compatibility, and performance optimization through strategic caching.

## Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐
│     Buffer      │────│    Document     │
│   (Mutable)     │    │  (Immutable)    │
└─────────────────┘    └─────────────────┘
         │                       │
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│  EditOperations │    │  TextAnalysis   │
│   - Insert      │    │  - WordFind     │
│   - Delete      │    │  - LineCalc     │
│   - Move        │    │  - Position     │
└─────────────────┘    └─────────────────┘
```

### Design Principles

1. **Immutable Document**: Document instances are immutable for safe sharing and caching
2. **Mutable Buffer**: Buffer provides mutable interface while managing Document instances
3. **Unicode-First**: All operations use rune-based indexing for proper Unicode support
4. **WASM-Compatible**: No dependencies that prevent WASM compilation
5. **Performance-Optimized**: Strategic caching and efficient algorithms

## Components and Interfaces

### Document Structure

```rust
pub struct Document {
    text: String,
    cursor_position: usize, // rune index, not byte index
    last_key: Option<Key>,
}

// Document is immutable - all methods return new instances or references
// This allows safe sharing and caching without mutation concerns

impl Document {
    // Core text access
    pub fn new() -> Self
    pub fn with_text(text: String, cursor_position: usize) -> Self
    pub fn text(&self) -> &str
    pub fn cursor_position(&self) -> usize
    pub fn last_key_stroke(&self) -> Option<Key>
    
    // Display and positioning
    pub fn display_cursor_position(&self) -> usize
    pub fn get_char_relative_to_cursor(&self, offset: i32) -> Option<char>
    
    // Text segments
    pub fn text_before_cursor(&self) -> &str
    pub fn text_after_cursor(&self) -> &str
    
    // Word operations
    pub fn get_word_before_cursor(&self) -> &str
    pub fn get_word_after_cursor(&self) -> &str
    pub fn get_word_before_cursor_with_space(&self) -> &str
    pub fn get_word_after_cursor_with_space(&self) -> &str
    pub fn get_word_before_cursor_until_separator(&self, sep: &str) -> &str
    pub fn get_word_after_cursor_until_separator(&self, sep: &str) -> &str
    
    // Word boundary finding
    pub fn find_start_of_previous_word(&self) -> usize
    pub fn find_end_of_current_word(&self) -> usize
    pub fn find_start_of_previous_word_with_space(&self) -> usize
    pub fn find_end_of_current_word_with_space(&self) -> usize
    pub fn find_start_of_previous_word_until_separator(&self, sep: &str) -> usize
    pub fn find_end_of_current_word_until_separator(&self, sep: &str) -> usize
    
    // Line operations
    pub fn current_line_before_cursor(&self) -> &str
    pub fn current_line_after_cursor(&self) -> &str
    pub fn current_line(&self) -> String
    pub fn lines(&self) -> Vec<&str>
    pub fn line_count(&self) -> usize
    
    // Position calculations
    pub fn cursor_position_row(&self) -> usize
    pub fn cursor_position_col(&self) -> usize
    pub fn translate_index_to_position(&self, index: usize) -> (usize, usize)
    pub fn translate_row_col_to_index(&self, row: usize, col: usize) -> usize
    
    // Cursor movement calculations
    pub fn get_cursor_left_position(&self, count: usize) -> i32
    pub fn get_cursor_right_position(&self, count: usize) -> i32
    pub fn get_cursor_up_position(&self, count: usize, preferred_column: Option<usize>) -> i32
    pub fn get_cursor_down_position(&self, count: usize, preferred_column: Option<usize>) -> i32
    
    // Line state
    pub fn on_last_line(&self) -> bool
    pub fn get_end_of_line_position(&self) -> usize
    
    // Internal helpers
    fn line_start_indexes(&self) -> Vec<usize>
    fn find_line_start_index(&self, index: usize) -> (usize, usize)
    fn leading_whitespace_in_current_line(&self) -> &str
}
```

### Buffer Structure

```rust
pub struct Buffer {
    working_lines: Vec<String>,    // Multiple lines for history-like functionality
    working_index: usize,          // Current line index in working_lines
    cursor_position: usize,        // rune index within current line
    cached_document: Option<Document>, // Cached document for performance
    preferred_column: Option<usize>,   // For vertical cursor movement consistency
    last_key_stroke: Option<Key>,      // Track last key for context-aware operations
}

// Buffer provides mutable editing interface while managing Document instances
// The working_lines design allows for undo/redo and multi-line editing support

impl Buffer {
    // Construction
    pub fn new() -> Self
    
    // Core access
    pub fn text(&self) -> &str
    pub fn document(&mut self) -> &Document
    pub fn display_cursor_position(&self) -> usize
    
    // Text modification
    pub fn insert_text(&mut self, text: &str, overwrite: bool, move_cursor: bool)
    pub fn delete_before_cursor(&mut self, count: usize) -> String
    pub fn delete(&mut self, count: usize) -> String
    pub fn new_line(&mut self, copy_margin: bool)
    pub fn join_next_line(&mut self, separator: &str)
    pub fn swap_characters_before_cursor(&mut self)
    
    // Cursor movement
    pub fn cursor_left(&mut self, count: usize)
    pub fn cursor_right(&mut self, count: usize)
    pub fn cursor_up(&mut self, count: usize)
    pub fn cursor_down(&mut self, count: usize)
    
    // State management
    pub fn set_text(&mut self, text: String)
    pub fn set_cursor_position(&mut self, position: usize)
    pub fn set_document(&mut self, document: Document)
    pub fn set_last_key_stroke(&mut self, key: Key)
    
    // Internal helpers
    fn invalidate_cache(&mut self)
    fn ensure_cursor_bounds(&mut self)
}
```

## Data Models

### Error Handling Strategy

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum BufferError {
    InvalidCursorPosition { position: usize, max: usize },
    InvalidWorkingIndex { index: usize, max: usize },
    InvalidRange { start: usize, end: usize },
    UnicodeError(String),
    EmptyBuffer,
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferError::InvalidCursorPosition { position, max } => {
                write!(f, "Invalid cursor position {} (max: {})", position, max)
            }
            BufferError::InvalidWorkingIndex { index, max } => {
                write!(f, "Invalid working index {} (max: {})", index, max)
            }
            BufferError::InvalidRange { start, end } => {
                write!(f, "Invalid range {}..{}", start, end)
            }
            BufferError::UnicodeError(msg) => write!(f, "Unicode error: {}", msg),
            BufferError::EmptyBuffer => write!(f, "Operation not valid on empty buffer"),
        }
    }
}

impl std::error::Error for BufferError {}

pub type BufferResult<T> = Result<T, BufferError>;
```

### Unicode Handling Strategy

The system uses a multi-layered approach to Unicode handling:

1. **Rune-based Indexing**: All cursor positions and text operations use rune indices
2. **Display Width Calculation**: Separate calculation for terminal display width
3. **Grapheme Cluster Awareness**: Future extension point for complex Unicode

```rust
// Unicode utilities module focused on go-prompt compatibility
pub mod unicode {
    use unicode_width::UnicodeWidthStr;
    
    pub fn rune_count(s: &str) -> usize {
        s.chars().count()
    }
    
    pub fn rune_slice(s: &str, start: usize, end: usize) -> &str {
        let start_byte = s.char_indices().nth(start).map(|(i, _)| i).unwrap_or(s.len());
        let end_byte = s.char_indices().nth(end).map(|(i, _)| i).unwrap_or(s.len());
        &s[start_byte..end_byte]
    }
    
    pub fn display_width(s: &str) -> usize {
        s.width()
    }
    
    pub fn char_at_rune_index(s: &str, index: usize) -> Option<char> {
        s.chars().nth(index)
    }
    
    pub fn byte_index_from_rune_index(s: &str, rune_index: usize) -> usize {
        s.char_indices()
            .nth(rune_index)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(s.len())
    }
}
```

### Caching Strategy

```rust
// Document caching in Buffer with invalidation tracking
impl Buffer {
    fn update_cached_document(&mut self) {
        let current_text = self.text().to_string();
        
        // Check if cache is still valid
        if let Some(ref cached) = self.cached_document {
            if cached.text() == current_text && 
               cached.cursor_position() == self.cursor_position &&
               cached.last_key_stroke() == self.last_key_stroke {
                return; // Cache is valid
            }
        }
        
        // Create new cached document
        self.cached_document = Some(Document::with_text_and_key(
            current_text,
            self.cursor_position,
            self.last_key_stroke
        ));
    }
    
    fn invalidate_cache(&mut self) {
        self.cached_document = None;
    }
    
    // Public method to get document with caching
    pub fn document(&mut self) -> &Document {
        self.update_cached_document();
        self.cached_document.as_ref().unwrap()
    }
}
```

### Line Index Caching

```rust
// Efficient line start index calculation with caching
struct LineIndexCache {
    text_hash: u64,
    line_starts: Vec<usize>,
}

impl Document {
    fn line_start_indexes(&self) -> &[usize] {
        // Implementation with caching based on text content hash
    }
}
```

## Error Handling

### Cursor Position Validation

```rust
impl Buffer {
    fn ensure_cursor_bounds(&mut self) {
        let text_len = unicode::rune_count(self.text());
        if self.cursor_position > text_len {
            self.cursor_position = text_len;
        }
    }
}
```

### Graceful Degradation

```rust
// Error handling strategy
pub enum TextError {
    InvalidCursorPosition(usize),
    InvalidRange(usize, usize),
    UnicodeError(String),
}

impl Document {
    pub fn get_char_relative_to_cursor(&self, offset: i32) -> Result<Option<char>, TextError> {
        // Safe implementation with bounds checking
    }
}
```

## Testing Strategy

### Unit Testing Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod document_tests {
        // Basic functionality
        #[test] fn test_empty_document()
        #[test] fn test_text_access()
        #[test] fn test_cursor_positioning()
        
        // Unicode handling
        #[test] fn test_unicode_text()
        #[test] fn test_cjk_characters()
        #[test] fn test_emoji_handling()
        
        // Multi-line operations
        #[test] fn test_line_operations()
        #[test] fn test_position_translation()
        
        // Word operations
        #[test] fn test_word_boundaries()
        #[test] fn test_custom_separators()
    }
    
    mod buffer_tests {
        // Editing operations
        #[test] fn test_text_insertion()
        #[test] fn test_text_deletion()
        #[test] fn test_cursor_movement()
        
        // Advanced operations
        #[test] fn test_line_joining()
        #[test] fn test_character_swapping()
        
        // Caching behavior
        #[test] fn test_document_caching()
        #[test] fn test_cache_invalidation()
    }
    
    mod unicode_tests {
        // Unicode-specific tests
        #[test] fn test_rune_indexing()
        #[test] fn test_display_width()
        #[test] fn test_complex_unicode()
    }
}
```

### Integration Testing

```rust
// Integration tests for WASM compatibility
#[cfg(test)]
mod integration_tests {
    #[test]
    fn test_wasm_serialization() {
        // Test data marshaling across WASM boundary
    }
    
    #[test]
    fn test_large_document_performance() {
        // Performance testing with large documents
    }
}
```

### Property-Based Testing

```rust
// Property-based tests using quickcheck (as suggested in original design)
#[cfg(test)]
mod property_tests {
    use quickcheck::{quickcheck, TestResult};
    use super::*;
    
    #[quickcheck]
    fn cursor_position_always_valid(text: String, pos: usize) -> TestResult {
        if text.len() > 10000 { return TestResult::discard(); }
        
        let doc = Document::with_text(text.clone(), pos);
        TestResult::from_bool(doc.cursor_position() <= unicode::rune_count(&text))
    }
    
    #[quickcheck]
    fn buffer_operations_preserve_invariants(text: String, operations: Vec<u8>) -> TestResult {
        if text.len() > 1000 || operations.len() > 100 { 
            return TestResult::discard(); 
        }
        
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        
        // Apply random operations and verify buffer remains consistent
        for op in operations {
            match op % 4 {
                0 => buffer.cursor_left(1),
                1 => buffer.cursor_right(1),
                2 => { buffer.insert_text("x", false, true); },
                3 => { buffer.delete_before_cursor(1); },
                _ => unreachable!(),
            }
            
            // Verify invariants
            let doc = buffer.document();
            if doc.cursor_position() > unicode::rune_count(doc.text()) {
                return TestResult::failed();
            }
        }
        
        TestResult::passed()
    }
    
    #[quickcheck]
    fn unicode_operations_preserve_validity(text: String) -> bool {
        let doc = Document::with_text(text, 0);
        
        // All text operations should preserve UTF-8 validity
        doc.text_before_cursor().is_empty() || doc.text_before_cursor().chars().count() > 0
    }
}
```

## WASM Integration Considerations

### Memory Management and Serialization for wazero

```rust
// WASM-compatible serialization for wazero runtime (not browser)
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WasmBufferState {
    pub working_lines: Vec<String>,
    pub working_index: usize,
    pub cursor_position: usize,
    pub preferred_column: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WasmDocumentState {
    pub text: String,
    pub cursor_position: usize,
}

// Add to existing wasm.rs file
impl Buffer {
    pub fn to_wasm_state(&self) -> WasmBufferState {
        WasmBufferState {
            working_lines: self.working_lines.clone(),
            working_index: self.working_index,
            cursor_position: self.cursor_position,
            preferred_column: self.preferred_column,
        }
    }
    
    pub fn from_wasm_state(state: WasmBufferState) -> Self {
        let mut buffer = Buffer {
            working_lines: state.working_lines,
            working_index: state.working_index,
            cursor_position: state.cursor_position,
            cached_document: None,
            preferred_column: state.preferred_column,
            last_key_stroke: None,
        };
        buffer.ensure_cursor_bounds();
        buffer
    }
}

impl Document {
    pub fn to_wasm_state(&self) -> WasmDocumentState {
        WasmDocumentState {
            text: self.text.clone(),
            cursor_position: self.cursor_position,
        }
    }
    
    pub fn from_wasm_state(state: WasmDocumentState) -> Self {
        Document::with_text(state.text, state.cursor_position)
    }
}
    
    #[wasm_bindgen]
    pub struct WasmBuffer {
        inner: Buffer,
    }
    
    #[wasm_bindgen]
    impl WasmBuffer {
        #[wasm_bindgen(constructor)]
        pub fn new() -> WasmBuffer {
            WasmBuffer {
                inner: Buffer::new(),
            }
        }
        
        #[wasm_bindgen]
        pub fn text(&self) -> String {
            self.inner.text().to_string()
        }
        
        #[wasm_bindgen]
        pub fn cursor_position(&self) -> usize {
            self.inner.cursor_position()
        }
        
        #[wasm_bindgen]
        pub fn insert_text(&mut self, text: &str, overwrite: bool, move_cursor: bool) -> Result<(), JsValue> {
            self.inner.insert_text(text, overwrite, move_cursor)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        
        #[wasm_bindgen]
        pub fn delete_before_cursor(&mut self, count: usize) -> Result<String, JsValue> {
            self.inner.delete_before_cursor(count)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        
        // Cursor movement methods
        #[wasm_bindgen]
        pub fn cursor_left(&mut self, count: usize) {
            self.inner.cursor_left(count);
        }
        
        #[wasm_bindgen]
        pub fn cursor_right(&mut self, count: usize) {
            self.inner.cursor_right(count);
        }
        
        // Document access for analysis
        #[wasm_bindgen]
        pub fn get_word_before_cursor(&mut self) -> String {
            self.inner.document().get_word_before_cursor().to_string()
        }
        
        #[wasm_bindgen]
        pub fn display_cursor_position(&mut self) -> usize {
            self.inner.document().display_cursor_position()
        }
    }
    
    #[wasm_bindgen]
    pub struct WasmDocument {
        inner: Document,
    }
    
    #[wasm_bindgen]
    impl WasmDocument {
        #[wasm_bindgen(constructor)]
        pub fn new() -> WasmDocument {
            WasmDocument {
                inner: Document::new(),
            }
        }
        
        #[wasm_bindgen]
        pub fn with_text(text: &str, cursor_position: usize) -> WasmDocument {
            WasmDocument {
                inner: Document::with_text(text.to_string(), cursor_position),
            }
        }
        
        #[wasm_bindgen(getter)]
        pub fn text(&self) -> String {
            self.inner.text().to_string()
        }
        
        #[wasm_bindgen(getter)]
        pub fn cursor_position(&self) -> usize {
            self.inner.cursor_position()
        }
        
        #[wasm_bindgen]
        pub fn display_cursor_position(&self) -> usize {
            self.inner.display_cursor_position()
        }
        
        #[wasm_bindgen]
        pub fn text_before_cursor(&self) -> String {
            self.inner.text_before_cursor().to_string()
        }
        
        #[wasm_bindgen]
        pub fn text_after_cursor(&self) -> String {
            self.inner.text_after_cursor().to_string()
        }
    }
}
```

### Data Serialization

```rust
// Efficient serialization for WASM boundary
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct DocumentSnapshot {
    text: String,
    cursor_position: usize,
}

impl From<&Document> for DocumentSnapshot {
    fn from(doc: &Document) -> Self {
        DocumentSnapshot {
            text: doc.text().to_string(),
            cursor_position: doc.cursor_position(),
        }
    }
}
```

## Performance Optimizations

### String Slicing Strategy

```rust
// Efficient string slicing without allocation
impl Document {
    pub fn text_before_cursor(&self) -> &str {
        let byte_index = self.text
            .char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        &self.text[..byte_index]
    }
}
```

### Lazy Computation

```rust
// Lazy computation for expensive operations
pub struct LazyLineIndexes {
    text: String,
    indexes: OnceCell<Vec<usize>>,
}

impl LazyLineIndexes {
    fn compute(&self) -> &Vec<usize> {
        self.indexes.get_or_init(|| {
            // Expensive line index computation
            compute_line_indexes(&self.text)
        })
    }
}
```

## Dependencies and Crate Structure

### External Dependencies

```toml
[dependencies]
# Unicode handling (WASM-compatible)
unicode-width = "0.1"

# Serialization for WASM interop
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
# Property-based testing
quickcheck = "1.0"
quickcheck_macros = "1.0"

[features]
default = []
wasm = []
```

### Module Structure

```
crates/prompt-core/src/
├── lib.rs              # Public API exports
├── key.rs              # Key definitions (already exists)
├── buffer.rs           # Buffer implementation
├── document.rs         # Document implementation
├── unicode.rs          # Unicode utilities
├── error.rs            # Error types
└── wasm.rs             # WASM bindings (feature-gated)
```

## Implementation Phases

This design incorporates lessons learned from the previous buffer port attempt while building on the completed key-input-parser foundation. The phased approach ensures:

1. **Incremental Development**: Each phase builds on the previous
2. **WASM Compatibility**: Early validation of WASM constraints
3. **Unicode Correctness**: Proper handling from the start
4. **Performance**: Caching and optimization built-in
5. **Robustness**: Comprehensive error handling and testing

The design maintains API compatibility with go-prompt while embracing Rust idioms for memory safety and performance. The separation between Document (immutable) and Buffer (mutable) provides clear ownership semantics and enables efficient caching strategies.