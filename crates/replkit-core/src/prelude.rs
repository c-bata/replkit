//! Convenient re-exports for low-level primitives
//!
//! This module provides a prelude that re-exports the most commonly used types
//! from the replkit-core crate's low-level primitives. For the complete high-level
//! API, use `replkit::prelude::*` instead.
//!
//! # Examples
//!
//! ```
//! use replkit_core::prelude::*;
//!
//! // Low-level text manipulation
//! let doc = Document::new();
//! let mut buffer = Buffer::new();
//! 
//! // Key parsing
//! let mut parser = KeyParser::new();
//! ```

// Core text manipulation types
pub use crate::document::Document;
pub use crate::buffer::Buffer;

// Key input handling
pub use crate::key::{Key, KeyEvent};
pub use crate::key_parser::{KeyParser, ParserState};

// Error handling
pub use crate::error::{BufferError, BufferResult};

// Unicode utilities (commonly used for text processing)
pub use crate::unicode::{
    rune_count, 
    display_width, 
    rune_slice
};

// Console I/O trait definitions (implementations are in replkit-io)
pub use crate::console::{
    ConsoleInput, 
    ConsoleOutput, 
    ConsoleError, 
    ConsoleResult,
    TextStyle,
    Color,
    ClearType
};

// Common result types for ergonomic error handling
pub type Result<T> = std::result::Result<T, BufferError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // Test that low-level types are available through prelude
        let _doc = Document::new();
        let _buffer = Buffer::new();
        
        // Test unicode utilities
        let text = "hello";
        let _count = rune_count(text);
        let _width = display_width(text);
        let _slice = rune_slice(text, 0, 2);
        
        // Test key types
        let _key = Key::Enter;
        let _parser = KeyParser::new();
        
        // Test error types
        let _error = BufferError::invalid_cursor_position(10, 5);
        let _result: Result<String> = Ok("test".to_string());
    }

    #[test]
    fn test_document_from_prelude() {
        // Test that Document can be used through prelude
        let doc = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.cursor_position(), 5);
    }
}
