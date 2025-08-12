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

use crate::{completion::Completor, renderer::Renderer, Suggestion};
use replkit_core::{error::BufferError, Buffer, Document, Key, KeyParser};
use replkit_io::{ConsoleError, ConsoleInput, ConsoleOutput};

/// Executor is called when user inputs text and presses Enter.
/// Similar to go-prompt's Executor func(string).
pub trait Executor {
    /// Execute the given input string.
    ///
    /// # Arguments
    /// * `input` - The user input string to execute
    ///
    /// # Returns
    /// Returns `Ok(())` to continue the prompt loop, or `Err(PromptError)` to exit.
    fn execute(&mut self, input: &str) -> PromptResult<()>;
}

/// ExitChecker is called after user input to check if prompt must stop and exit the run loop.
/// Similar to go-prompt's ExitChecker func(in string, breakline bool) bool.
pub trait ExitChecker {
    /// Check if the prompt should exit based on the input.
    ///
    /// # Arguments
    /// * `input` - The current input string
    /// * `breakline` - Whether this check is after pressing Enter (true) or during typing (false)
    ///
    /// # Returns
    /// Returns `true` if the prompt should exit, `false` to continue.
    fn should_exit(&self, input: &str, breakline: bool) -> bool;
}

// Implement Executor for closures
impl<F> Executor for F
where
    F: for<'a> FnMut(&'a str) -> PromptResult<()>,
{
    fn execute(&mut self, input: &str) -> PromptResult<()> {
        self(input)
    }
}

// Implement ExitChecker for closures
impl<F> ExitChecker for F
where
    F: Fn(&str, bool) -> bool,
{
    fn should_exit(&self, input: &str, breakline: bool) -> bool {
        self(input, breakline)
    }
}

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
            PromptError::IoError(msg) => write!(f, "I/O error: {msg}"),
            PromptError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {msg}"),
            PromptError::BufferError(err) => write!(f, "Buffer error: {err}"),
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
/// Manages completion suggestions and selection state
#[derive(Debug, Clone)]
pub struct CompletionManager {
    /// Currently available suggestions
    suggestions: Vec<Suggestion>,
    /// Index of selected suggestion (-1 means no selection)
    selected_index: i32,
    /// Whether completion menu is currently visible
    visible: bool,
    /// Maximum number of suggestions to display
    _max_suggestions: usize,
    /// Word separator for completion (go-prompt style)
    word_separator: String,
}

impl CompletionManager {
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            suggestions: Vec::new(),
            selected_index: -1,
            visible: false,
            _max_suggestions: max_suggestions,
            word_separator: " \t\n".to_string(), // Default separators like go-prompt
        }
    }

    /// Get the word separator
    pub fn word_separator(&self) -> &str {
        &self.word_separator
    }

    pub fn reset(&mut self) {
        self.suggestions.clear();
        self.selected_index = -1;
        self.visible = false;
    }

    /// Returns whether the CompletionManager is actively selecting a completion (go-prompt's Completing)
    pub fn completing(&self) -> bool {
        self.selected_index != -1
    }

    pub fn update_suggestions(&mut self, suggestions: Vec<Suggestion>) {
        self.suggestions = suggestions;
        // Don't automatically select first item - wait for Tab key (go-prompt style)
        // Reset selection if suggestions are empty or selection is out of bounds
        if self.suggestions.is_empty() || self.selected_index >= self.suggestions.len() as i32 {
            self.selected_index = -1;
        }
        self.visible = !self.suggestions.is_empty();
    }

    pub fn next(&mut self) {
        if !self.suggestions.is_empty() {
            if self.selected_index == -1 {
                // First Tab press - start selecting from 0 (go-prompt style)
                self.selected_index = 0;
            } else {
                // Cycle through suggestions
                self.selected_index = (self.selected_index + 1) % (self.suggestions.len() as i32);
            }
        }
    }

    pub fn previous(&mut self) {
        if !self.suggestions.is_empty() {
            if self.selected_index == -1 {
                // First BackTab press - start selecting from last item (go-prompt style)
                self.selected_index = (self.suggestions.len() as i32) - 1;
            } else {
                // Cycle through suggestions backwards
                let len = self.suggestions.len() as i32;
                self.selected_index = (self.selected_index - 1 + len) % len;
            }
        }
    }

    pub fn get_selected(&self) -> Option<&Suggestion> {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.suggestions.len() {
            Some(&self.suggestions[self.selected_index as usize])
        } else {
            None
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn suggestions(&self) -> &[Suggestion] {
        &self.suggestions
    }

    pub fn selected_index(&self) -> i32 {
        self.selected_index
    }
}

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
    _key_parser: KeyParser,
    /// Completion manager for handling suggestion navigation
    completion_manager: CompletionManager,
    /// Optional exit checker for determining when to exit the run loop
    exit_checker: Option<Box<dyn ExitChecker>>,
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
        Document::with_text(
            self.buffer.text().to_string(),
            self.buffer.cursor_position(),
        )
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

    /// Apply selected completion if completing (go-prompt default case behavior)
    fn apply_completion_if_completing(&mut self) -> PromptResult<()> {
        if self.completion_manager.completing() {
            if let Some(selected) = self.completion_manager.get_selected() {
                let doc = self.document();
                let word = doc.get_word_before_cursor_until_separator(
                    self.completion_manager.word_separator(),
                );

                if !word.is_empty() {
                    // Delete current word and insert selected completion
                    self.buffer.delete_before_cursor(word.len());
                    self.buffer.insert_text(&selected.text, false, true);
                }
            }
            self.completion_manager.reset();
        }
        Ok(())
    }

    /// Update and render completions (go-prompt style auto-completion)
    fn update_and_render_completions(&mut self) -> PromptResult<()> {
        let suggestions = self.get_completions();
        if !suggestions.is_empty() {
            self.completion_manager.update_suggestions(suggestions);
        } else {
            // Clear completions if no suggestions
            if self.completion_manager.is_visible() {
                self.completion_manager.reset();
            }
        }

        self.render_with_completion_preview()?;
        Ok(())
    }

    /// Render prompt with completion preview (go-prompt style)
    fn render_with_completion_preview(&mut self) -> PromptResult<()> {
        // Always render the prompt first (go-prompt pattern)
        self.renderer
            .render_prompt(&self.prefix, &self.document())?;

        // Render completions if visible
        if self.completion_manager.is_visible() {
            if self.completion_manager.completing() {
                // Show completions with selection
                let selected_idx = self.completion_manager.selected_index();
                self.renderer.render_completions_with_selection(
                    self.completion_manager.suggestions(),
                    selected_idx as usize,
                )?;

                // Render preview of selected completion (go-prompt style)
                if let Some(selected) = self.completion_manager.get_selected() {
                    self.renderer
                        .render_completion_preview(&self.document(), selected)?;
                }
            } else {
                // Show completions without selection
                self.renderer
                    .render_completions(self.completion_manager.suggestions())?;
            }
        }
        Ok(())
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
        self.renderer
            .initialize()
            .map_err(|e| PromptError::IoError(e.to_string()))?;

        // Render initial prompt (go-prompt doesn't show completions initially)
        self.renderer
            .render_prompt(&self.prefix, &self.document())?;

        // Main input loop
        loop {
            match self.input.read_key_timeout(Some(100)) {
                // 100ms timeout
                Ok(Some(key_event)) => {
                    match key_event.key {
                        Key::Enter => {
                            // If completing, apply selected completion first (go-prompt default case)
                            self.apply_completion_if_completing()?;

                            // Re-render the prompt to get correct cursor position
                            self.renderer
                                .render_prompt(&self.prefix, &self.document())?;

                            // Clear completions and move to next line
                            if self.completion_manager.is_visible() {
                                self.completion_manager.reset();
                            }

                            // Write newline (this will clear completion menu automatically)
                            self.renderer.write_newline()?;

                            // Return the entered text
                            return Ok(self.buffer.text().to_string());
                        }
                        Key::ControlC => {
                            if self.completion_manager.is_visible() {
                                self.completion_manager.reset();
                                self.renderer.clear_completions().ok();
                            }
                            return Err(PromptError::Interrupted);
                        }
                        Key::Backspace => {
                            // If completing, apply selected completion first (go-prompt default case)
                            self.apply_completion_if_completing()?;

                            // Delete character before cursor
                            if self.buffer.cursor_position() > 0 {
                                self.buffer.delete_before_cursor(1);
                                // Auto-show completions after backspace (go-prompt style)
                                self.update_and_render_completions()?;
                            }
                        }
                        Key::Tab => {
                            // Tab always calls completion.Next() (go-prompt style)
                            self.completion_manager.next();
                            self.render_with_completion_preview()?;
                        }
                        Key::Up => {
                            // Up key only works when actively completing (go-prompt style)
                            if self.completion_manager.completing() {
                                self.completion_manager.previous();
                                self.render_with_completion_preview()?;
                            }
                        }
                        Key::Down => {
                            // Down key only works when actively completing (go-prompt style)
                            if self.completion_manager.completing() {
                                self.completion_manager.next();
                                self.render_with_completion_preview()?;
                            }
                        }
                        Key::Left => {
                            // If completing, apply selected completion first (go-prompt default case)
                            self.apply_completion_if_completing()?;

                            // Move cursor left
                            if self.buffer.cursor_position() > 0 {
                                self.buffer.cursor_left(1);
                                // Auto-show completions after cursor movement (go-prompt style)
                                self.update_and_render_completions()?;
                            }
                        }
                        Key::Right => {
                            // If completing, apply selected completion first (go-prompt default case)
                            self.apply_completion_if_completing()?;

                            // Move cursor right
                            if self.buffer.cursor_position() < self.buffer.text().len() {
                                self.buffer.cursor_right(1);
                                // Auto-show completions after cursor movement (go-prompt style)
                                self.update_and_render_completions()?;
                            }
                        }
                        _ => {
                            // Handle text input from key events
                            if let Some(text) = &key_event.text {
                                // If completing, apply selected completion first (go-prompt default case)
                                self.apply_completion_if_completing()?;

                                // Insert the new text
                                self.buffer.insert_text(text, false, true);
                                // Auto-show completions after text input (go-prompt style)
                                self.update_and_render_completions()?;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Timeout - continue loop (useful for periodic updates)
                    continue;
                }
                Err(e) => {
                    if self.completion_manager.is_visible() {
                        self.completion_manager.reset();
                        self.renderer.clear_completions().ok();
                    }
                    return Err(PromptError::IoError(format!("Input error: {e}")));
                }
            }
        }
    }

    /// Run the prompt with an executor in a continuous loop
    ///
    /// This method implements the go-prompt style `Run()` functionality where
    /// the executor is called for each line of input. The loop continues until
    /// the exit checker returns true or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `executor` - Function or closure that processes each line of input
    ///
    /// # Behavior
    ///
    /// - Displays the prompt and waits for user input
    /// - When Enter is pressed, calls the executor with the input
    /// - Continues the loop after executor returns
    /// - Checks exit conditions using the configured exit checker
    /// - Handles Ctrl+C as interruption
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
    /// prompt.run(|input| {
    ///     println!("You entered: {}", input);
    ///     if input == "quit" {
    ///         return Err(PromptError::Interrupted);
    ///     }
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn run<E>(&mut self, mut executor: E) -> PromptResult<()>
    where
        E: Executor,
    {
        // Enable raw mode for the entire session
        let _raw_guard = self.input.enable_raw_mode()?;

        // Update terminal size in renderer
        if let Ok((cols, rows)) = self.input.get_window_size() {
            self.renderer.update_terminal_size(cols, rows);
        }

        // Initialize renderer
        self.renderer
            .initialize()
            .map_err(|e| PromptError::IoError(e.to_string()))?;

        loop {
            // Reset buffer for new input
            self.buffer.set_text(String::new());
            self.completion_manager.reset();

            // Render initial prompt
            self.renderer
                .render_prompt(&self.prefix, &self.document())?;

            // Input loop for current line
            let input_result = loop {
                match self.input.read_key_timeout(Some(100)) {
                    Ok(Some(key_event)) => {
                        match key_event.key {
                            Key::Enter => {
                                // If completing, apply selected completion first
                                self.apply_completion_if_completing()?;

                                // Get the final input
                                let input = self.buffer.text().to_string();

                                // Clear completions and move to next line
                                if self.completion_manager.is_visible() {
                                    self.completion_manager.reset();
                                }
                                self.renderer.write_newline()?;

                                // Check exit condition before executing (immediate exit)
                                if let Some(exit_checker) = &self.exit_checker {
                                    if exit_checker.should_exit(&input, false) {
                                        return Ok(());
                                    }
                                }

                                break Ok(input);
                            }
                            Key::ControlC => {
                                if self.completion_manager.is_visible() {
                                    self.completion_manager.reset();
                                    self.renderer.clear_completions().ok();
                                }
                                return Err(PromptError::Interrupted);
                            }
                            Key::ControlD => {
                                // Ctrl+D on empty line exits (go-prompt behavior)
                                if self.buffer.text().is_empty() {
                                    return Ok(());
                                }
                            }
                            Key::Backspace => {
                                self.apply_completion_if_completing()?;
                                if self.buffer.cursor_position() > 0 {
                                    self.buffer.delete_before_cursor(1);
                                    self.update_and_render_completions()?;
                                }
                            }
                            Key::Tab => {
                                self.completion_manager.next();
                                self.render_with_completion_preview()?;
                            }
                            Key::Up => {
                                if self.completion_manager.completing() {
                                    self.completion_manager.previous();
                                    self.render_with_completion_preview()?;
                                }
                            }
                            Key::Down => {
                                if self.completion_manager.completing() {
                                    self.completion_manager.next();
                                    self.render_with_completion_preview()?;
                                }
                            }
                            Key::Left => {
                                self.apply_completion_if_completing()?;
                                if self.buffer.cursor_position() > 0 {
                                    self.buffer.cursor_left(1);
                                    self.update_and_render_completions()?;
                                }
                            }
                            Key::Right => {
                                self.apply_completion_if_completing()?;
                                if self.buffer.cursor_position() < self.buffer.text().len() {
                                    self.buffer.cursor_right(1);
                                    self.update_and_render_completions()?;
                                }
                            }
                            _ => {
                                if let Some(text) = &key_event.text {
                                    self.apply_completion_if_completing()?;
                                    self.buffer.insert_text(text, false, true);
                                    self.update_and_render_completions()?;

                                    // Check exit condition during typing (non-breakline)
                                    if let Some(exit_checker) = &self.exit_checker {
                                        if exit_checker.should_exit(self.buffer.text(), false) {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Timeout - continue loop
                        continue;
                    }
                    Err(e) => {
                        if self.completion_manager.is_visible() {
                            self.completion_manager.reset();
                            self.renderer.clear_completions().ok();
                        }
                        break Err(PromptError::IoError(format!("Input error: {e}")));
                    }
                }
            };

            // Handle the input result
            match input_result {
                Ok(input) => {
                    // Execute the input
                    executor.execute(&input)?;

                    // Check exit condition after execution (breakline = true)
                    if let Some(exit_checker) = &self.exit_checker {
                        if exit_checker.should_exit(&input, true) {
                            return Ok(());
                        }
                    }

                    // Continue to next iteration
                }
                Err(e) => {
                    return Err(e);
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
    exit_checker: Option<Box<dyn ExitChecker>>,
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
            exit_checker: None,
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
        C: Completor + 'static,
    {
        self.completer = Some(Box::new(completer));
        self
    }

    /// Set an exit checker to determine when the prompt should exit
    ///
    /// The exit checker is called in two scenarios:
    /// 1. During typing (breakline = false) - for immediate exit without executing
    /// 2. After pressing Enter (breakline = true) - for exit after execution
    ///
    /// # Arguments
    ///
    /// * `exit_checker` - Any type implementing the `ExitChecker` trait
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit::prelude::*;
    ///
    /// // Exit when user types "quit"
    /// let prompt = PromptBuilder::new()
    ///     .with_exit_checker(|input: &str, _breakline: bool| {
    ///         input == "quit"
    ///     })
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn with_exit_checker<E>(mut self, exit_checker: E) -> Self
    where
        E: ExitChecker + 'static,
    {
        self.exit_checker = Some(Box::new(exit_checker));
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
            _key_parser: KeyParser::new(),
            completion_manager: CompletionManager::new(10),
            exit_checker: self.exit_checker,
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
        let prompt = PromptBuilder::new().with_prefix("$ ").build().unwrap();
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
        // Test error conversion from BufferError
        let buffer_error = BufferError::invalid_cursor_position(10, 5);
        let prompt_error: PromptError = buffer_error.into();
        assert!(matches!(prompt_error, PromptError::BufferError(_)));

        // Test error conversion from std::io::Error
        let io_error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let prompt_error: PromptError = io_error.into();
        assert!(matches!(prompt_error, PromptError::IoError(_)));

        // Test that we can create different error types
        let interrupted = PromptError::Interrupted;
        assert!(matches!(interrupted, PromptError::Interrupted));

        let invalid_config = PromptError::InvalidConfiguration("test".to_string());
        assert!(matches!(
            invalid_config,
            PromptError::InvalidConfiguration(_)
        ));
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

    #[test]
    fn test_executor_trait() {
        // Test that closures implement Executor
        let mut counter = 0;
        let mut executor = |input: &str| -> PromptResult<()> {
            counter += 1;
            if input == "error" {
                Err(PromptError::Interrupted)
            } else {
                Ok(())
            }
        };

        assert!(executor.execute("hello").is_ok());
        assert!(executor.execute("error").is_err());
        assert_eq!(counter, 2);
    }

    #[test]
    fn test_exit_checker_trait() {
        // Test that closures implement ExitChecker
        let exit_checker = |input: &str, breakline: bool| -> bool {
            if breakline {
                input == "quit"
            } else {
                input == "exit"
            }
        };

        assert!(!exit_checker.should_exit("hello", false));
        assert!(!exit_checker.should_exit("hello", true));
        assert!(exit_checker.should_exit("exit", false));
        assert!(exit_checker.should_exit("quit", true));
        assert!(!exit_checker.should_exit("quit", false));
        assert!(!exit_checker.should_exit("exit", true));
    }

    #[test]
    fn test_prompt_builder_with_exit_checker() {
        let prompt = PromptBuilder::new()
            .with_prefix("$ ")
            .with_exit_checker(|input: &str, _breakline: bool| input == "quit")
            .build()
            .unwrap();

        assert_eq!(prompt.prefix(), "$ ");
        // Exit checker is private, so we can't test it directly here
        // but it will be tested in integration tests
    }
}
