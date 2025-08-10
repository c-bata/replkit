//! Convenient re-exports for common usage patterns
//!
//! This module provides a prelude that re-exports the most commonly used types
//! and traits from the replkit-core crate. Users can import everything they need
//! with a single `use replkit::prelude::*;` statement.
//!
//! # Examples
//!
//! ```
//! use replkit_core::prelude::*;
//!
//! // Now you can use Document, Buffer, Suggestion, etc. directly
//! let doc = Document::new();
//! let suggestion = Suggestion::new("test", "A test suggestion");
//! ```

// Core text manipulation types
pub use crate::document::Document;
pub use crate::buffer::Buffer;

// Completion system
pub use crate::suggestion::Suggestion;
pub use crate::completion::{Completor, StaticCompleter};

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

// Re-export traits and types that will be implemented in future tasks
// These will be uncommented as they are implemented:

// Prompt system (to be implemented in Phase 2)
// pub use crate::prompt::{Prompt, PromptBuilder};

// Rendering system (to be implemented in Phase 3) 
// pub use crate::renderer::Renderer;

// Console I/O (available from existing console module)
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
        // Test that key types are available through prelude
        let _doc = Document::new();
        let _buffer = Buffer::new();
        let _suggestion = Suggestion::new("test", "description");
        
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
    fn test_suggestion_from_prelude() {
        // Test that Suggestion can be created using types from prelude
        let suggestion = Suggestion::new("users", "Store user data");
        assert_eq!(suggestion.text, "users");
        assert_eq!(suggestion.description, "Store user data");
    }

    #[test]
    fn test_document_from_prelude() {
        // Test that Document can be used through prelude
        let doc = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.cursor_position(), 5);
    }

    #[test]
    fn test_completion_from_prelude() {
        // Test that Completor trait and StaticCompleter are available through prelude
        let completer = StaticCompleter::from_strings(vec!["hello", "help", "history"]);
        let doc = Document::with_text("he".to_string(), 2);
        let suggestions = completer.complete(&doc);
        assert_eq!(suggestions.len(), 2);
        
        // Test function-based completer
        let func_completer = |document: &Document| -> Vec<Suggestion> {
            if document.text().starts_with("test") {
                vec![Suggestion::new("testing", "Run tests")]
            } else {
                vec![]
            }
        };
        
        let test_doc = Document::with_text("test".to_string(), 4);
        let test_suggestions = func_completer.complete(&test_doc);
        assert_eq!(test_suggestions.len(), 1);
        assert_eq!(test_suggestions[0].text, "testing");
    }
}
