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
//! use replkit::prelude::*;
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
//! use replkit::prelude::*;
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
//! use replkit::prelude::*;
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

use replkit_core::{Buffer, Document, error::BufferError, Key, KeyParser};
use replkit_io::{ConsoleInput, ConsoleOutput, ConsoleError};
use crate::{Suggestion, completion::Completor, renderer::Renderer};


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

impl From<ConsoleError> for PromptError {
    fn from(err: ConsoleError) -> Self {
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
/// use replkit::prelude::*;
///
/// let mut prompt = Prompt::builder()
///     .with_prefix(">>> ")
///     .build()
///     .unwrap();
///
/// // Now input() method is fully implemented
/// // let input = prompt.input().unwrap();
/// ```
pub struct Prompt {
    /// The prefix string shown before user input
    prefix: String,
    /// Optional completion provider
    completer: Option<Box<dyn Completor>>,
    /// Text buffer for managing user input
    buffer: Buffer,
    /// Terminal renderer for display
    renderer: Renderer,
    /// Console input for keyboard events
    input: Box<dyn ConsoleInput>,
    /// Key parser for processing input
    key_parser: KeyParser,
}

impl Prompt {
    /// Create a new prompt builder
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit::prelude::*;
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
    /// use replkit::prelude::*;
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
    /// use replkit::prelude::*;
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
    /// use replkit::prelude::*;
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
    /// use replkit::prelude::*;
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

    /// Interactive input method with full event loop
    ///
    /// Starts an interactive input session with real-time rendering and completion support.
    /// Handles keyboard events and updates the display accordingly.
    ///
    /// # Keyboard Controls
    /// - **Enter**: Submit input and return the result
    /// - **Ctrl+C**: Cancel input and return Interrupted error
    /// - **Tab**: Show/navigate completions
    /// - **Arrow keys**: Navigate completions (when visible) or move cursor
    /// - **Backspace/Delete**: Edit text
    /// - **Printable characters**: Insert text
    ///
    /// # Returns
    ///
    /// Returns the final input string when the user presses Enter.
    ///
    /// # Errors
    ///
    /// Returns `PromptError::Interrupted` if the user cancels (Ctrl+C).
    /// Returns `PromptError::IoError` for I/O related issues.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use replkit::prelude::*;
    ///
    /// let mut prompt = Prompt::builder()
    ///     .with_prefix(">>> ")
    ///     .build()
    ///     .unwrap();
    ///
    /// match prompt.input() {
    ///     Ok(input) => println!("You entered: {}", input),
    ///     Err(PromptError::Interrupted) => println!("Cancelled"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    pub fn input(&mut self) -> PromptResult<String> {
        // Initialize the input session
        self.buffer.set_text(String::new());
        
        // Enable raw mode first for proper terminal control
        let _raw_guard = self.input.enable_raw_mode()?;
        
        // Update terminal size in renderer
        if let Ok((cols, rows)) = self.input.get_window_size() {
            self.renderer.update_terminal_size(cols, rows);
        }
        
        // Initialize renderer with current cursor position
        self.renderer.initialize().map_err(|e| PromptError::IoError(e.to_string()))?;
        
        // Render initial prompt
        self.renderer.render_prompt(&self.prefix, &self.document())?;
        
        let mut showing_completions = false;
        
        // Main input loop
        loop {
            match self.input.read_key_timeout(Some(100)) { // 100ms timeout
                Ok(Some(key_event)) => {
                    match key_event.key {
                        Key::Enter => {
                            // Clear completions and move to next line
                            if showing_completions {
                                self.renderer.clear_completions().ok();
                                showing_completions = false;
                            }
                            
                            // Move cursor to end of line and add newline
                            self.renderer.move_cursor_to_end_of_line()?;
                            self.renderer.write_newline()?;
                            
                            // Return the entered text
                            return Ok(self.buffer.text().to_string());
                        },
                        Key::ControlC => {
                            if showing_completions {
                                self.renderer.clear_completions().ok();
                            }
                            return Err(PromptError::Interrupted);
                        },
                        Key::Backspace => {
                            // Delete character before cursor
                            if self.buffer.cursor_position() > 0 {
                                self.buffer.delete_before_cursor(1);
                                if showing_completions {
                                    self.renderer.clear_completions().ok();
                                    showing_completions = false;
                                }
                                self.renderer.render_prompt(&self.prefix, &self.document())?;
                            }
                        },
                        Key::Tab => {
                            // Show completions
                            let suggestions = self.get_completions();
                            if !suggestions.is_empty() {
                                self.renderer.render_completions(&suggestions)?;
                                showing_completions = true;
                            }
                        },
                        Key::Left => {
                            // Move cursor left
                            if self.buffer.cursor_position() > 0 {
                                self.buffer.cursor_left(1);
                                self.renderer.render_prompt(&self.prefix, &self.document())?;
                            }
                        },
                        Key::Right => {
                            // Move cursor right
                            if self.buffer.cursor_position() < self.buffer.text().len() {
                                self.buffer.cursor_right(1);
                                self.renderer.render_prompt(&self.prefix, &self.document())?;
                            }
                        },
                        _ => {
                            // Handle text input from key events
                            if let Some(text) = &key_event.text {
                                self.buffer.insert_text(text, false, true);
                                if showing_completions {
                                    self.renderer.clear_completions().ok();
                                    showing_completions = false;
                                }
                                self.renderer.render_prompt(&self.prefix, &self.document())?;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Timeout - continue loop (useful for periodic updates)
                    continue;
                }
                Err(e) => {
                    if showing_completions {
                        self.renderer.clear_completions().ok();
                    }
                    return Err(PromptError::IoError(format!("Input error: {}", e)));
                }
            }
        }
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
/// use replkit::prelude::*;
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
    console_output: Option<Box<dyn ConsoleOutput>>,
    console_input: Option<Box<dyn ConsoleInput>>,
}

impl PromptBuilder {
    /// Create a new prompt builder with default settings
    ///
    /// Default settings:
    /// - Prefix: "> "
    /// - No completer
    /// - Console I/O will be auto-created if not specified
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit::prelude::*;
    ///
    /// let builder = PromptBuilder::new();
    /// let prompt = builder.build().unwrap();
    /// assert_eq!(prompt.prefix(), "> ");
    /// ```
    pub fn new() -> Self {
        Self {
            prefix: "> ".to_string(),
            completer: None,
            console_output: None,
            console_input: None,
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
    /// use replkit::prelude::*;
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
    /// use replkit::prelude::*;
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

    /// Set the console output implementation
    ///
    /// This allows custom console output implementations for testing
    /// or specialized environments.
    ///
    /// # Arguments
    ///
    /// * `output` - Console output implementation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use replkit::prelude::*;
    /// use replkit_io::unix::UnixConsoleOutput;
    ///
    /// let output = UnixConsoleOutput::new().unwrap();
    /// let prompt = PromptBuilder::new()
    ///     .with_console_output(Box::new(output))
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_output(mut self, output: Box<dyn ConsoleOutput>) -> Self {
        self.console_output = Some(output);
        self
    }

    /// Set the console input implementation
    ///
    /// This allows custom console input implementations for testing
    /// or specialized environments.
    ///
    /// # Arguments
    ///
    /// * `input` - Console input implementation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use replkit::prelude::*;
    /// use replkit_io::unix::UnixConsoleInput;
    ///
    /// let input = UnixConsoleInput::new().unwrap();
    /// let prompt = PromptBuilder::new()
    ///     .with_console_input(Box::new(input))
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_console_input(mut self, input: Box<dyn ConsoleInput>) -> Self {
        self.console_input = Some(input);
        self
    }

    /// Set both console input and output using the default platform implementations
    ///
    /// This is a convenience method that automatically creates the appropriate
    /// console implementations for the current platform.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use replkit::prelude::*;
    ///
    /// let prompt = PromptBuilder::new()
    ///     .with_default_console()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_default_console(mut self) -> PromptResult<Self> {
        let output = replkit_io::create_console_output()?;
        let input = replkit_io::create_console_input()?;
        self.console_output = Some(output);
        self.console_input = Some(input);
        Ok(self)
    }

    /// Build the configured prompt
    ///
    /// Creates a new `Prompt` instance with the current configuration.
    /// If console I/O is not specified, default platform implementations will be used.
    ///
    /// # Errors
    ///
    /// Returns an error if console I/O initialization fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use replkit::prelude::*;
    ///
    /// let prompt = PromptBuilder::new()
    ///     .with_prefix(">> ")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn build(self) -> PromptResult<Prompt> {
        // Get or create console output
        let console_output = match self.console_output {
            Some(output) => output,
            None => replkit_io::create_console_output()?,
        };

        // Get or create console input  
        let console_input = match self.console_input {
            Some(input) => input,
            None => replkit_io::create_console_input()?,
        };

        // Create renderer
        let renderer = Renderer::new(console_output);

        Ok(Prompt {
            prefix: self.prefix,
            completer: self.completer,
            buffer: Buffer::new(),
            renderer,
            input: console_input,
            key_parser: KeyParser::new(),
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
