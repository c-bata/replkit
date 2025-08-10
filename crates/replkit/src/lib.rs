//! # Replkit: Interactive Prompt Library
//!
//! Replkit is a powerful, flexible library for building interactive command-line
//! applications with features like auto-completion, history, and rich text input.
//! It provides a high-level API built on top of solid low-level primitives.
//!
//! ## Quick Start
//!
//! ```rust
//! use replkit::prelude::*;
//!
//! // Create a simple prompt
//! let mut prompt = Prompt::builder()
//!     .with_prefix(">>> ")
//!     .build()
//!     .unwrap();
//!
//! // With completion
//! let mut prompt_with_completion = Prompt::builder()
//!     .with_prefix("$ ")
//!     .with_completer(StaticCompleter::from_strings(vec![
//!         "help", "quit", "status"
//!     ]))
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Architecture
//!
//! Replkit is organized into several layers:
//!
//! - **Low-level primitives** (`replkit-core`): Document, Buffer, KeyParser, Unicode handling
//! - **Platform I/O** (`replkit-io`): Cross-platform terminal input/output implementations
//! - **High-level API** (`replkit`): Prompt, completion, rendering - this crate
//!
//! ## Features
//!
//! - **Flexible completion system**: Support for both static completions and dynamic function-based completers
//! - **Unicode support**: Proper handling of international text and emoji
//! - **Cross-platform**: Works on Windows, macOS, and Linux
//! - **Extensible**: Clean trait-based architecture for customization
//! - **WASM ready**: Core functionality available in WebAssembly environments

// Re-export low-level primitives from replkit-core
pub use replkit_core::{
    // Core text manipulation
    Document, Buffer,
    // Key handling
    Key, KeyEvent, KeyParser, ParserState,
    // Error handling
    error::{BufferError, BufferResult},
    // Unicode utilities
    unicode::{rune_count, display_width, rune_slice, byte_index_from_rune_index, char_at_rune_index},
    // Console trait definitions
    console::{
        ConsoleOutput, ConsoleInput, ConsoleError, ConsoleResult,
        TextStyle, Color, ClearType, OutputCapabilities, BackendType,
        RawModeGuard, SafeTextFilter, SanitizationPolicy,
        EventLoopError, ConsoleCapabilities
    },
};

// Re-export I/O implementations from replkit-io
pub use replkit_io::*;

// High-level components (defined in this crate)
pub mod suggestion;
pub mod completion;
pub mod prompt;

// Re-export high-level components for convenience
pub use suggestion::Suggestion;
pub use completion::{Completor, StaticCompleter};
pub use prompt::{Prompt, PromptBuilder, PromptError, PromptResult};

/// Convenient re-exports for common usage patterns
///
/// Import everything you need with `use replkit::prelude::*;`
pub mod prelude {
    // Core text manipulation types
    pub use replkit_core::{Document, Buffer};

    // Completion system
    pub use crate::{Suggestion, Completor, StaticCompleter};

    // Prompt system
    pub use crate::{Prompt, PromptBuilder, PromptError, PromptResult};

    // Key input handling
    pub use replkit_core::{Key, KeyEvent, KeyParser, ParserState};

    // Error handling
    pub use replkit_core::error::{BufferError, BufferResult};

    // Unicode utilities (commonly used for text processing)
    pub use replkit_core::unicode::{
        rune_count, 
        display_width, 
        rune_slice
    };

    // Console I/O (available from replkit-io)
    pub use replkit_core::console::{
        ConsoleInput, 
        ConsoleOutput, 
        ConsoleError, 
        ConsoleResult,
        TextStyle,
        Color,
        ClearType
    };

    // Common result types for ergonomic error handling
    pub type Result<T> = std::result::Result<T, crate::PromptError>;
}

/// Convenience functions for common use cases
pub mod convenience {
    use crate::prelude::*;

    /// Create a simple prompt with default settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::convenience::simple_prompt;
    ///
    /// let mut prompt = simple_prompt(">>> ");
    /// // Now ready to use: prompt.input().unwrap();
    /// ```
    pub fn simple_prompt(prefix: &str) -> crate::PromptResult<Prompt> {
        Prompt::builder()
            .with_prefix(prefix)
            .build()
    }

    /// Create a prompt with static string completions
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::convenience::prompt_with_completions;
    ///
    /// let mut prompt = prompt_with_completions("$ ", vec![
    ///     "help", "quit", "status", "version"
    /// ]).unwrap();
    /// ```
    pub fn prompt_with_completions<S: Into<String>>(
        prefix: &str, 
        completions: Vec<S>
    ) -> crate::PromptResult<Prompt> {
        let completer = StaticCompleter::from_strings(completions);
        Prompt::builder()
            .with_prefix(prefix)
            .with_completer(completer)
            .build()
    }

    /// Create a prompt with a function-based completer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::convenience::prompt_with_completer;
    ///
    /// let mut prompt = prompt_with_completer("$ ", |document| {
    ///     let word = document.get_word_before_cursor();
    ///     if word.starts_with("git") {
    ///         vec![
    ///             replkit::Suggestion::new("git status", "Show repo status"),
    ///             replkit::Suggestion::new("git commit", "Commit changes"),
    ///         ]
    ///     } else {
    ///         vec![]
    ///     }
    /// }).unwrap();
    /// ```
    pub fn prompt_with_completer<F>(
        prefix: &str,
        completer: F
    ) -> crate::PromptResult<Prompt>
    where
        F: Fn(&Document) -> Vec<Suggestion> + 'static,
    {
        Prompt::builder()
            .with_prefix(prefix)
            .with_completer(completer)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;
        
        // Test that all major types are available
        let _doc = Document::new();
        let _buffer = Buffer::new();
        let _suggestion = Suggestion::new("test", "description");
        let _completer = StaticCompleter::from_strings(vec!["test"]);
        let _prompt = Prompt::builder().build().unwrap();
        
        // Test unicode utilities
        let _count = rune_count("hello");
        let _width = display_width("hello");
        let _slice = rune_slice("hello", 0, 3);
        
        // Test key types
        let _key = Key::Enter;
        let _parser = KeyParser::new();
        
        // Test error types
        let _error = BufferError::invalid_cursor_position(10, 5);
        let _result: Result<String> = Ok("test".to_string());
    }

    #[test]
    fn test_convenience_functions() {
        // Test simple_prompt
        let prompt = convenience::simple_prompt(">>> ");
        assert!(prompt.is_ok());
        assert_eq!(prompt.unwrap().prefix(), ">>> ");

        // Test prompt_with_completions
        let prompt = convenience::prompt_with_completions("$ ", vec!["help", "quit"]);
        assert!(prompt.is_ok());
        let prompt = prompt.unwrap();
        assert_eq!(prompt.prefix(), "$ ");
        assert_eq!(prompt.get_completions().len(), 2);

        // Test prompt_with_completer
        let prompt = convenience::prompt_with_completer("$ ", |_doc| {
            vec![Suggestion::new("test", "Test command")]
        });
        assert!(prompt.is_ok());
        let prompt = prompt.unwrap();
        assert_eq!(prompt.prefix(), "$ ");
        assert_eq!(prompt.get_completions().len(), 1);
    }

    #[test]
    fn test_all_exports_available() {
        // Test that we can access both high-level and low-level APIs
        let _doc = Document::new();
        let _buffer = Buffer::new();
        let _suggestion = Suggestion::new("test", "desc");
        let _completer = StaticCompleter::from_strings(vec!["test"]);
        let _prompt = Prompt::builder().build().unwrap();
        
        // Test that error types are available
        let _buffer_error = BufferError::invalid_cursor_position(1, 0);
        let _prompt_error = PromptError::Interrupted;
    }
}
