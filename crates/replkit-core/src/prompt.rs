//! High-level prompt interface with builder pattern
//!
//! This module provides the main `Prompt` struct and `PromptBuilder` for creating
//! interactive command-line prompts. It integrates with the completion system
//! and provides a simple API for creating prompts with various configurations.
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```
//! use replkit_core::prelude::*;
//!
//! let prompt = Prompt::builder()
//!     .with_prefix(">> ")
//!     .build()
//!     .expect("Failed to create prompt");
//! ```
//!
//! ## With completion
//!
//! ```
//! use replkit_core::prelude::*;
//!
//! let completer = StaticCompleter::from_strings(vec!["help", "quit", "status"]);
//! let prompt = Prompt::builder()
//!     .with_prefix("myapp> ")
//!     .with_completer(completer)
//!     .build()
//!     .expect("Failed to create prompt");
//! ```
//!
//! ## With function-based completer
//!
//! ```
//! use replkit_core::prelude::*;
//!
//! let prompt = Prompt::builder()
//!     .with_prefix("$ ")
//!     .with_completer(|document: &Document| {
//!         let word = document.get_word_before_cursor();
//!         if word.starts_with("git") {
//!             vec![
//!                 Suggestion::new("git status", "Show working tree status"),
//!                 Suggestion::new("git commit", "Record changes to repository"),
//!             ]
//!         } else {
//!             vec![]
//!         }
//!     })
//!     .build()
//!     .expect("Failed to create prompt");
//! ```

use crate::{Buffer, Document, Suggestion, completion::Completor, error::BufferError};

/// Error types specific to prompt operations
#[derive(Debug, Clone)]
pub enum PromptError {
    /// User interrupted the prompt (e.g., Ctrl+C)
    Interrupted,
    /// I/O error occurred during prompt operation
    IoError(String),
    /// Invalid prompt configuration
    InvalidConfiguration(String),
    /// Buffer operation failed
    BufferError(BufferError),
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptError::Interrupted => write!(f, "Prompt was interrupted"),
            PromptError::IoError(msg) => write!(f, "I/O error: {}", msg),
            PromptError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            PromptError::BufferError(err) => write!(f, "Buffer error: {}", err),
        }
    }
}

impl std::error::Error for PromptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PromptError::BufferError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<BufferError> for PromptError {
    fn from(err: BufferError) -> Self {
        PromptError::BufferError(err)
    }
}

impl From<std::io::Error> for PromptError {
    fn from(err: std::io::Error) -> Self {
        PromptError::IoError(err.to_string())
    }
}

/// Result type for prompt operations
pub type PromptResult<T> = Result<T, PromptError>;

/// Main prompt interface for interactive input
///
/// The `Prompt` struct provides a high-level interface for creating interactive
/// command-line prompts. It manages the text buffer, handles completion, and
/// will eventually handle the input/output loop.
///
/// Use `Prompt::builder()` to create a new prompt with custom configuration.
///
/// # Examples
///
/// ```
/// use replkit_core::prelude::*;
///
/// let mut prompt = Prompt::builder()
///     .with_prefix(">>> ")
///     .build()
///     .unwrap();
///
/// // The input() method will be implemented in Phase 4
/// // let input = prompt.input().unwrap();
/// ```
pub struct Prompt {
    /// The prefix string shown before user input
    prefix: String,
    /// Optional completion provider
    completer: Option<Box<dyn Completor>>,
    /// Text buffer for managing user input
    buffer: Buffer,
}

impl Prompt {
    /// Create a new prompt builder
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let prompt = Prompt::builder()
    ///     .with_prefix("$ ")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> PromptBuilder {
        PromptBuilder::new()
    }

    /// Get the current prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get the current document state
    pub fn document(&self) -> Document {
        // Create a new document from the buffer's current state
        Document::with_text(self.buffer.text().to_string(), self.buffer.cursor_position())
    }

    /// Get completions for the current document state
    ///
    /// Returns an empty vector if no completer is configured or if the
    /// completer returns no suggestions.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let completer = StaticCompleter::from_strings(vec!["hello", "help"]);
    /// let mut prompt = Prompt::builder()
    ///     .with_completer(completer)
    ///     .build()
    ///     .unwrap();
    ///
    /// // Insert some text to get completions for
    /// prompt.insert_text("he");
    /// let suggestions = prompt.get_completions();
    /// assert_eq!(suggestions.len(), 2);
    /// ```
    pub fn get_completions(&self) -> Vec<Suggestion> {
        match &self.completer {
            Some(completer) => completer.complete(&self.document()),
            None => Vec::new(),
        }
    }

    /// Insert text at the current cursor position
    ///
    /// This is a convenience method for basic text insertion.
    /// For more advanced editing operations, access the buffer directly.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let mut prompt = Prompt::builder().build().unwrap();
    /// prompt.insert_text("hello");
    /// assert_eq!(prompt.document().text(), "hello");
    /// ```
    pub fn insert_text(&mut self, text: &str) -> PromptResult<()> {
        self.buffer.insert_text(text, false, true);
        Ok(())
    }

    /// Clear the current input
    ///
    /// Resets the buffer to empty state with cursor at position 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let mut prompt = Prompt::builder().build().unwrap();
    /// prompt.insert_text("hello");
    /// prompt.clear();
    /// assert_eq!(prompt.document().text(), "");
    /// assert_eq!(prompt.document().cursor_position(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.buffer.set_text(String::new());
    }

    /// Get access to the underlying buffer for advanced operations
    ///
    /// This provides direct access to the text buffer for advanced editing
    /// operations like cursor movement, deletion, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let mut prompt = Prompt::builder().build().unwrap();
    /// prompt.insert_text("hello world");
    /// 
    /// // Use buffer for advanced operations
    /// let buffer = prompt.buffer_mut();
    /// buffer.cursor_left(6); // Move to after "hello"
    /// buffer.insert_text(" beautiful", false, true);
    /// 
    /// assert_eq!(prompt.document().text(), "hello beautiful world");
    /// ```
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Input method placeholder
    ///
    /// This method will be implemented in Phase 4 to handle the interactive
    /// input loop with keyboard handling and rendering.
    ///
    /// # Returns
    ///
    /// The final input string when the user presses Enter.
    ///
    /// # Errors
    ///
    /// Returns `PromptError::Interrupted` if the user cancels (Ctrl+C).
    /// Returns `PromptError::IoError` for I/O related issues.
    pub fn input(&mut self) -> PromptResult<String> {
        // Implementation will be added in Phase 4
        // For now, return a placeholder error
        Err(PromptError::InvalidConfiguration(
            "Input loop not yet implemented - will be added in Phase 4".to_string()
        ))
    }
}

/// Builder for configuring and creating `Prompt` instances
///
/// The `PromptBuilder` uses the builder pattern to provide a fluent API
/// for configuring prompts before creation.
///
/// # Examples
///
/// ```
/// use replkit_core::prelude::*;
///
/// let prompt = PromptBuilder::new()
///     .with_prefix("myapp> ")
///     .with_completer(StaticCompleter::from_strings(vec!["help", "quit"]))
///     .build()
///     .unwrap();
/// ```
pub struct PromptBuilder {
    prefix: String,
    completer: Option<Box<dyn Completor>>,
}

impl PromptBuilder {
    /// Create a new prompt builder with default settings
    ///
    /// Default settings:
    /// - Prefix: "> "
    /// - No completer
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let builder = PromptBuilder::new();
    /// let prompt = builder.build().unwrap();
    /// assert_eq!(prompt.prefix(), "> ");
    /// ```
    pub fn new() -> Self {
        Self {
            prefix: "> ".to_string(),
            completer: None,
        }
    }

    /// Set the prompt prefix
    ///
    /// The prefix is displayed before the user input area.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix string to display
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let prompt = PromptBuilder::new()
    ///     .with_prefix("$ ")
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(prompt.prefix(), "$ ");
    /// ```
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Set a completer using any type that implements `Completor`
    ///
    /// This includes `StaticCompleter`, custom types implementing `Completor`,
    /// and closure functions with the signature `Fn(&Document) -> Vec<Suggestion>`.
    ///
    /// # Arguments
    ///
    /// * `completer` - Any type implementing the `Completor` trait
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// // Using StaticCompleter
    /// let prompt1 = PromptBuilder::new()
    ///     .with_completer(StaticCompleter::from_strings(vec!["help", "quit"]))
    ///     .build()
    ///     .unwrap();
    ///
    /// // Using a closure
    /// let prompt2 = PromptBuilder::new()
    ///     .with_completer(|doc: &Document| {
    ///         vec![Suggestion::new("test", "A test command")]
    ///     })
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_completer<C>(mut self, completer: C) -> Self 
    where 
        C: Completor + 'static
    {
        self.completer = Some(Box::new(completer));
        self
    }

    /// Build the configured prompt
    ///
    /// Creates a new `Prompt` instance with the current configuration.
    ///
    /// # Errors
    ///
    /// Currently always succeeds, but future versions may validate
    /// configuration and return errors for invalid settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::prelude::*;
    ///
    /// let prompt = PromptBuilder::new()
    ///     .with_prefix(">> ")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> PromptResult<Prompt> {
        Ok(Prompt {
            prefix: self.prefix,
            completer: self.completer,
            buffer: Buffer::new(),
        })
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::StaticCompleter;

    #[test]
    fn test_prompt_builder_default() {
        let prompt = PromptBuilder::new().build().unwrap();
        assert_eq!(prompt.prefix(), "> ");
        assert_eq!(prompt.get_completions().len(), 0);
    }

    #[test]
    fn test_prompt_builder_with_prefix() {
        let prompt = PromptBuilder::new()
            .with_prefix("$ ")
            .build()
            .unwrap();
        assert_eq!(prompt.prefix(), "$ ");
    }

    #[test]
    fn test_prompt_builder_with_string_prefix() {
        let prefix = "myapp> ".to_string();
        let prompt = PromptBuilder::new()
            .with_prefix(prefix.clone())
            .build()
            .unwrap();
        assert_eq!(prompt.prefix(), &prefix);
    }

    #[test]
    fn test_prompt_with_static_completer() {
        let completer = StaticCompleter::from_strings(vec!["help", "hello", "history"]);
        let mut prompt = PromptBuilder::new()
            .with_completer(completer)
            .build()
            .unwrap();

        // Test with no input
        let suggestions = prompt.get_completions();
        assert_eq!(suggestions.len(), 3);

        // Test with prefix
        prompt.insert_text("he").unwrap();
        let suggestions = prompt.get_completions();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().any(|s| s.text == "help"));
        assert!(suggestions.iter().any(|s| s.text == "hello"));
    }

    #[test]
    fn test_prompt_with_function_completer() {
        let completer = |document: &Document| -> Vec<Suggestion> {
            let word = document.get_word_before_cursor();
            if word.starts_with("git") {
                vec![
                    Suggestion::new("git status", "Show repository status"),
                    Suggestion::new("git commit", "Commit changes"),
                ]
            } else {
                vec![]
            }
        };

        let mut prompt = PromptBuilder::new()
            .with_completer(completer)
            .build()
            .unwrap();

        // Test with no matching input
        prompt.insert_text("hello").unwrap();
        let suggestions = prompt.get_completions();
        assert_eq!(suggestions.len(), 0);

        // Test with matching input
        prompt.clear();
        prompt.insert_text("git").unwrap();
        let suggestions = prompt.get_completions();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().any(|s| s.text == "git status"));
        assert!(suggestions.iter().any(|s| s.text == "git commit"));
    }

    #[test]
    fn test_prompt_text_operations() {
        let mut prompt = PromptBuilder::new().build().unwrap();

        // Test insert
        prompt.insert_text("hello").unwrap();
        assert_eq!(prompt.document().text(), "hello");
        assert_eq!(prompt.document().cursor_position(), 5);

        // Test additional insert
        prompt.insert_text(" world").unwrap();
        assert_eq!(prompt.document().text(), "hello world");
        assert_eq!(prompt.document().cursor_position(), 11);

        // Test clear
        prompt.clear();
        assert_eq!(prompt.document().text(), "");
        assert_eq!(prompt.document().cursor_position(), 0);
    }

    #[test]
    fn test_prompt_buffer_access() {
        let mut prompt = PromptBuilder::new().build().unwrap();
        
        prompt.insert_text("hello world").unwrap();
        
        // Test direct buffer manipulation
        let buffer = prompt.buffer_mut();
        buffer.cursor_left(6); // Move to after "hello"
        buffer.insert_text(" beautiful", false, true);
        
        assert_eq!(prompt.document().text(), "hello beautiful world");
    }

    #[test]
    fn test_prompt_error_handling() {
        let mut prompt = PromptBuilder::new().build().unwrap();

        // Test that input() returns appropriate error for now
        let result = prompt.input();
        assert!(matches!(result, Err(PromptError::InvalidConfiguration(_))));
    }

    #[test]
    fn test_builder_pattern_chaining() {
        let completer = StaticCompleter::from_strings(vec!["test"]);
        let prompt = PromptBuilder::new()
            .with_prefix(">>> ")
            .with_completer(completer)
            .build()
            .unwrap();

        assert_eq!(prompt.prefix(), ">>> ");
        assert_eq!(prompt.get_completions().len(), 1);
    }

    #[test]
    fn test_prompt_error_display() {
        let error = PromptError::Interrupted;
        assert_eq!(error.to_string(), "Prompt was interrupted");

        let error = PromptError::IoError("connection lost".to_string());
        assert_eq!(error.to_string(), "I/O error: connection lost");

        let error = PromptError::InvalidConfiguration("bad config".to_string());
        assert_eq!(error.to_string(), "Invalid configuration: bad config");
    }

    #[test]
    fn test_prompt_error_conversion() {
        let buffer_error = BufferError::invalid_cursor_position(10, 5);
        let prompt_error: PromptError = buffer_error.into();
        assert!(matches!(prompt_error, PromptError::BufferError(_)));

        let io_error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let prompt_error: PromptError = io_error.into();
        assert!(matches!(prompt_error, PromptError::IoError(_)));
    }
}
