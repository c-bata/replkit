//! Display rendering and management for REPL output.
//!
//! This module provides the Renderer struct that handles the visual representation
//! of the REPL state, including differential rendering, cursor management, and
//! terminal window handling.

use crate::{
    console::{ClearType, ConsoleOutput, TextStyle},
    unicode, Buffer, ReplError,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Manages the visual representation of the REPL state.
///
/// The Renderer is responsible for efficiently updating the terminal display
/// based on the current buffer state, implementing differential rendering
/// to minimize screen updates and reduce flicker.
pub struct Renderer {
    /// Console output interface for writing to terminal
    output: Box<dyn ConsoleOutput>,
    /// Prompt prefix to display before user input
    prompt: String,
    /// Last cursor position for differential updates
    last_cursor_pos: usize,
    /// Hash of last rendered text for change detection
    last_text_hash: u64,
    /// Last rendered text content for differential comparison
    last_text_content: String,
    /// Current terminal window size (columns, rows)
    window_size: (u16, u16),
    /// Current cursor position on screen (row, col)
    screen_cursor_pos: (u16, u16),
    /// Whether the cursor is currently visible
    cursor_visible: bool,
    /// Current text style
    current_style: TextStyle,
}

/// Result of a rendering operation.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderResult {
    /// Content was updated and rendered
    Updated,
    /// No changes detected, rendering skipped
    NoChange,
    /// Rendering was forced regardless of changes
    Forced,
}

/// Configuration for rendering behavior.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Whether to enable differential rendering optimization
    pub enable_differential_rendering: bool,
    /// Whether to show cursor position
    pub show_cursor: bool,
    /// Maximum line length before wrapping
    pub max_line_length: Option<usize>,
    /// Text style for the prompt
    pub prompt_style: Option<TextStyle>,
    /// Text style for user input
    pub input_style: Option<TextStyle>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            enable_differential_rendering: true,
            show_cursor: true,
            max_line_length: None,
            prompt_style: None,
            input_style: None,
        }
    }
}

impl Renderer {
    /// Create a new renderer with the given console output and prompt.
    ///
    /// # Arguments
    ///
    /// * `output` - Console output interface for writing to terminal
    /// * `prompt` - Prompt prefix to display before user input
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::renderer::Renderer;
    /// use replkit_core::console::ConsoleOutput;
    ///
    /// // This example uses a mock output for testing
    /// # use replkit_core::console::{ConsoleResult, ClearType, TextStyle, OutputCapabilities, BackendType, AsAny};
    /// # struct MockOutput;
    /// # impl ConsoleOutput for MockOutput {
    /// #     fn write_text(&self, _text: &str) -> ConsoleResult<()> { Ok(()) }
    /// #     fn write_styled_text(&self, _text: &str, _style: &TextStyle) -> ConsoleResult<()> { Ok(()) }
    /// #     fn write_safe_text(&self, _text: &str) -> ConsoleResult<()> { Ok(()) }
    /// #     fn move_cursor_to(&self, _row: u16, _col: u16) -> ConsoleResult<()> { Ok(()) }
    /// #     fn move_cursor_relative(&self, _row_delta: i16, _col_delta: i16) -> ConsoleResult<()> { Ok(()) }
    /// #     fn clear(&self, _clear_type: ClearType) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> { Ok(()) }
    /// #     fn reset_style(&self) -> ConsoleResult<()> { Ok(()) }
    /// #     fn flush(&self) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_cursor_visible(&self, _visible: bool) -> ConsoleResult<()> { Ok(()) }
    /// #     fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> { Ok((0, 0)) }
    /// #     fn get_capabilities(&self) -> OutputCapabilities {
    /// #         OutputCapabilities { supports_colors: false, supports_true_color: false, supports_styling: false,
    /// #         supports_alternate_screen: false, supports_cursor_control: false, max_colors: 0,
    /// #         platform_name: "mock".to_string(), backend_type: BackendType::Mock }
    /// #     }
    /// # }
    /// # impl AsAny for MockOutput {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    ///
    /// let output = Box::new(MockOutput);
    /// let renderer = Renderer::new(output, ">>> ".to_string());
    /// ```
    pub fn new(output: Box<dyn ConsoleOutput>, prompt: String) -> Self {
        Renderer {
            output,
            prompt,
            last_cursor_pos: 0,
            last_text_hash: 0,
            last_text_content: String::new(),
            window_size: (80, 24), // Default terminal size
            screen_cursor_pos: (0, 0),
            cursor_visible: true,
            current_style: TextStyle::default(),
        }
    }

    /// Create a new renderer with configuration.
    pub fn with_config(
        output: Box<dyn ConsoleOutput>,
        prompt: String,
        _config: RenderConfig,
    ) -> Self {
        // For now, we'll use the basic constructor and extend this later
        Self::new(output, prompt)
    }

    /// Render the current buffer state to the terminal.
    ///
    /// This method performs differential rendering by comparing the current
    /// buffer state with the last rendered state and only updating changed
    /// portions of the display.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The current buffer state to render
    ///
    /// # Returns
    ///
    /// `Ok(RenderResult)` indicating what rendering action was taken,
    /// or a `ReplError` if rendering failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::{Buffer, renderer::Renderer};
    /// # use replkit_core::console::{ConsoleResult, ClearType, TextStyle, OutputCapabilities, BackendType, AsAny, ConsoleOutput};
    /// # struct MockOutput;
    /// # impl ConsoleOutput for MockOutput {
    /// #     fn write_text(&self, _text: &str) -> ConsoleResult<()> { Ok(()) }
    /// #     fn write_styled_text(&self, _text: &str, _style: &TextStyle) -> ConsoleResult<()> { Ok(()) }
    /// #     fn write_safe_text(&self, _text: &str) -> ConsoleResult<()> { Ok(()) }
    /// #     fn move_cursor_to(&self, _row: u16, _col: u16) -> ConsoleResult<()> { Ok(()) }
    /// #     fn move_cursor_relative(&self, _row_delta: i16, _col_delta: i16) -> ConsoleResult<()> { Ok(()) }
    /// #     fn clear(&self, _clear_type: ClearType) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> { Ok(()) }
    /// #     fn reset_style(&self) -> ConsoleResult<()> { Ok(()) }
    /// #     fn flush(&self) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> { Ok(()) }
    /// #     fn set_cursor_visible(&self, _visible: bool) -> ConsoleResult<()> { Ok(()) }
    /// #     fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> { Ok((0, 0)) }
    /// #     fn get_capabilities(&self) -> OutputCapabilities {
    /// #         OutputCapabilities { supports_colors: false, supports_true_color: false, supports_styling: false,
    /// #         supports_alternate_screen: false, supports_cursor_control: false, max_colors: 0,
    /// #         platform_name: "mock".to_string(), backend_type: BackendType::Mock }
    /// #     }
    /// # }
    /// # impl AsAny for MockOutput {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.insert_text("hello world", false, true);
    ///
    /// let output = Box::new(MockOutput);
    /// let mut renderer = Renderer::new(output, ">>> ".to_string());
    ///
    /// let result = renderer.render(&buffer);
    /// assert!(result.is_ok());
    /// ```
    pub fn render(&mut self, buffer: &Buffer) -> Result<RenderResult, ReplError> {
        let current_text = buffer.text();
        let current_cursor_pos = buffer.cursor_position();

        // Calculate hash for differential rendering
        let current_hash = self.calculate_content_hash(current_text, current_cursor_pos);

        // Check if content has changed
        if current_hash == self.last_text_hash
            && current_cursor_pos == self.last_cursor_pos
            && current_text == self.last_text_content
        {
            return Ok(RenderResult::NoChange);
        }

        // Perform the rendering
        self.render_content(current_text, current_cursor_pos)?;

        // Update tracking state
        self.last_text_hash = current_hash;
        self.last_cursor_pos = current_cursor_pos;
        self.last_text_content = current_text.to_string();

        Ok(RenderResult::Updated)
    }

    /// Force a complete re-render regardless of differential state.
    ///
    /// This method bypasses differential rendering and forces a complete
    /// redraw of the current buffer state. Useful for recovery from
    /// terminal state corruption or after window resizing.
    pub fn force_render(&mut self, buffer: &Buffer) -> Result<RenderResult, ReplError> {
        let current_text = buffer.text();
        let current_cursor_pos = buffer.cursor_position();

        // Clear the line first to ensure clean state
        self.clear_line()?;

        // Perform full rendering
        self.render_content(current_text, current_cursor_pos)?;

        // Update tracking state
        self.last_text_hash = self.calculate_content_hash(current_text, current_cursor_pos);
        self.last_cursor_pos = current_cursor_pos;
        self.last_text_content = current_text.to_string();

        Ok(RenderResult::Forced)
    }

    /// Clear the current line and reset cursor position.
    ///
    /// This method clears the entire current line and positions the cursor
    /// at the beginning of the line, ready for new content.
    pub fn clear_line(&mut self) -> Result<(), ReplError> {
        // Move cursor to beginning of line
        self.output
            .move_cursor_to(self.screen_cursor_pos.0, 0)
            .map_err(|e| ReplError::RenderError(format!("Failed to move cursor: {e}")))?;

        // Clear from cursor to end of line
        self.output
            .clear(ClearType::FromCursorToEndOfLine)
            .map_err(|e| ReplError::RenderError(format!("Failed to clear line: {e}")))?;

        // Reset screen cursor position
        self.screen_cursor_pos.1 = 0;

        // Reset tracking state
        self.last_text_hash = 0;
        self.last_cursor_pos = 0;
        self.last_text_content.clear();

        self.output
            .flush()
            .map_err(|e| ReplError::RenderError(format!("Failed to flush output: {e}")))?;

        Ok(())
    }

    /// Insert a line break and move to the next line.
    ///
    /// This method outputs a newline character and updates the internal
    /// cursor tracking to reflect the new line position.
    pub fn break_line(&mut self) -> Result<(), ReplError> {
        // Write newline
        self.output
            .write_text("\n")
            .map_err(|e| ReplError::RenderError(format!("Failed to write newline: {e}")))?;

        // Update screen cursor position
        self.screen_cursor_pos.0 += 1;
        self.screen_cursor_pos.1 = 0;

        // Reset differential rendering state since we've moved to a new line
        self.last_text_hash = 0;
        self.last_cursor_pos = 0;
        self.last_text_content.clear();

        self.output
            .flush()
            .map_err(|e| ReplError::RenderError(format!("Failed to flush output: {e}")))?;

        Ok(())
    }

    /// Update the terminal window size and adjust display accordingly.
    ///
    /// This method should be called when the terminal window is resized
    /// to ensure proper text wrapping and cursor positioning.
    ///
    /// # Arguments
    ///
    /// * `width` - New terminal width in columns
    /// * `height` - New terminal height in rows
    pub fn update_window_size(&mut self, width: u16, height: u16) {
        self.window_size = (width, height);

        // Reset differential rendering state to force re-render with new dimensions
        self.last_text_hash = 0;
        self.last_cursor_pos = 0;
        self.last_text_content.clear();
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u16, u16) {
        self.window_size
    }

    /// Get the current prompt string.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Set a new prompt string.
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
        // Reset differential state since prompt changed
        self.last_text_hash = 0;
        self.last_text_content.clear();
    }

    /// Get the current screen cursor position.
    pub fn screen_cursor_position(&self) -> (u16, u16) {
        self.screen_cursor_pos
    }

    /// Set cursor visibility.
    pub fn set_cursor_visible(&mut self, visible: bool) -> Result<(), ReplError> {
        if self.cursor_visible != visible {
            self.output.set_cursor_visible(visible).map_err(|e| {
                ReplError::RenderError(format!("Failed to set cursor visibility: {e}"))
            })?;
            self.cursor_visible = visible;
        }
        Ok(())
    }

    /// Check if cursor is currently visible.
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Calculate display width of text accounting for Unicode characters.
    ///
    /// This method calculates how many terminal columns the given text
    /// will occupy, taking into account wide Unicode characters.
    fn calculate_display_width(&self, text: &str) -> usize {
        unicode::display_width(text)
    }

    /// Calculate content hash for differential rendering.
    fn calculate_content_hash(&self, text: &str, cursor_pos: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        cursor_pos.hash(&mut hasher);
        self.prompt.hash(&mut hasher);
        hasher.finish()
    }

    /// Perform the actual rendering of content to the terminal.
    fn render_content(&mut self, text: &str, cursor_pos: usize) -> Result<(), ReplError> {
        // Move to beginning of current line
        self.output
            .move_cursor_to(self.screen_cursor_pos.0, 0)
            .map_err(|e| {
                ReplError::RenderError(format!("Failed to move cursor to line start: {e}"))
            })?;

        // Clear the line to ensure clean rendering
        self.output
            .clear(ClearType::FromCursorToEndOfLine)
            .map_err(|e| ReplError::RenderError(format!("Failed to clear line: {e}")))?;

        // Render prompt
        self.output
            .write_text(&self.prompt)
            .map_err(|e| ReplError::RenderError(format!("Failed to write prompt: {e}")))?;

        // Calculate prompt display width for cursor positioning
        let prompt_width = self.calculate_display_width(&self.prompt);

        // Handle line wrapping if text is too long
        let (display_text, _wrapped_lines) = self.handle_line_wrapping(text)?;

        // Render the text content
        self.output
            .write_safe_text(&display_text)
            .map_err(|e| ReplError::RenderError(format!("Failed to write text: {e}")))?;

        // Calculate and set cursor position
        let cursor_display_pos = self.calculate_cursor_display_position(text, cursor_pos)?;
        let final_cursor_col = prompt_width + cursor_display_pos;

        // Handle cursor positioning with line wrapping
        let (cursor_row, cursor_col) = if final_cursor_col >= self.window_size.0 as usize {
            // Cursor would wrap to next line
            let wrapped_row =
                self.screen_cursor_pos.0 + (final_cursor_col / self.window_size.0 as usize) as u16;
            let wrapped_col = final_cursor_col % self.window_size.0 as usize;
            (wrapped_row, wrapped_col as u16)
        } else {
            (self.screen_cursor_pos.0, final_cursor_col as u16)
        };

        // Position cursor at the correct location
        self.output
            .move_cursor_to(cursor_row, cursor_col)
            .map_err(|e| ReplError::RenderError(format!("Failed to position cursor: {e}")))?;

        // Update screen cursor tracking
        self.screen_cursor_pos = (cursor_row, cursor_col);

        // Flush output to ensure immediate display
        self.output
            .flush()
            .map_err(|e| ReplError::RenderError(format!("Failed to flush output: {e}")))?;

        Ok(())
    }

    /// Handle line wrapping for text longer than terminal width.
    fn handle_line_wrapping(&self, text: &str) -> Result<(String, Vec<String>), ReplError> {
        let prompt_width = self.calculate_display_width(&self.prompt);
        let available_width = self.window_size.0 as usize;

        if available_width <= prompt_width {
            // Terminal too narrow, just return the text as-is
            return Ok((text.to_string(), vec![]));
        }

        let text_width = available_width - prompt_width;
        let text_display_width = self.calculate_display_width(text);

        if text_display_width <= text_width {
            // Text fits on one line
            Ok((text.to_string(), vec![]))
        } else {
            // Text needs wrapping - for now, we'll implement simple character-based wrapping
            // A more sophisticated implementation would handle word boundaries
            let mut wrapped_lines = Vec::new();
            let mut current_line = String::new();
            let mut current_width = 0;

            for ch in text.chars() {
                let char_width = unicode::display_width(&ch.to_string());

                if current_width + char_width > text_width && !current_line.is_empty() {
                    wrapped_lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0;
                }

                current_line.push(ch);
                current_width += char_width;
            }

            if !current_line.is_empty() {
                wrapped_lines.push(current_line);
            }

            // For now, return just the first line and track wrapped lines
            let display_text = wrapped_lines.first().unwrap_or(&String::new()).clone();
            Ok((display_text, wrapped_lines))
        }
    }

    /// Calculate the display position of the cursor within the text.
    fn calculate_cursor_display_position(
        &self,
        text: &str,
        cursor_pos: usize,
    ) -> Result<usize, ReplError> {
        if cursor_pos == 0 {
            return Ok(0);
        }

        let text_rune_count = unicode::rune_count(text);
        let safe_cursor_pos = cursor_pos.min(text_rune_count);

        // Get the text up to the cursor position
        let text_before_cursor = unicode::rune_slice(text, 0, safe_cursor_pos);

        // Calculate display width of text before cursor
        Ok(self.calculate_display_width(text_before_cursor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::{AsAny, BackendType, ConsoleOutput, ConsoleResult, OutputCapabilities};
    use std::sync::{Arc, Mutex};

    // Mock console output for testing
    #[derive(Debug)]
    struct MockConsoleOutput {
        operations: Arc<Mutex<Vec<String>>>,
        cursor_pos: Arc<Mutex<(u16, u16)>>,
        cursor_visible: Arc<Mutex<bool>>,
    }

    impl MockConsoleOutput {
        fn new() -> Self {
            MockConsoleOutput {
                operations: Arc::new(Mutex::new(Vec::new())),
                cursor_pos: Arc::new(Mutex::new((0, 0))),
                cursor_visible: Arc::new(Mutex::new(true)),
            }
        }

        fn get_operations(&self) -> Vec<String> {
            self.operations.lock().unwrap().clone()
        }

        fn clear_operations(&self) {
            self.operations.lock().unwrap().clear();
        }
    }

    impl ConsoleOutput for MockConsoleOutput {
        fn write_text(&self, text: &str) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push(format!("write_text: {}", text));
            Ok(())
        }

        fn write_styled_text(&self, text: &str, _style: &TextStyle) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push(format!("write_styled_text: {}", text));
            Ok(())
        }

        fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push(format!("write_safe_text: {}", text));
            Ok(())
        }

        fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
            *self.cursor_pos.lock().unwrap() = (row, col);
            self.operations
                .lock()
                .unwrap()
                .push(format!("move_cursor_to: ({}, {})", row, col));
            Ok(())
        }

        fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
            self.operations.lock().unwrap().push(format!(
                "move_cursor_relative: ({}, {})",
                row_delta, col_delta
            ));
            Ok(())
        }

        fn clear(&self, clear_type: ClearType) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push(format!("clear: {:?}", clear_type));
            Ok(())
        }

        fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push("set_style".to_string());
            Ok(())
        }

        fn reset_style(&self) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push("reset_style".to_string());
            Ok(())
        }

        fn flush(&self) -> ConsoleResult<()> {
            self.operations.lock().unwrap().push("flush".to_string());
            Ok(())
        }

        fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> {
            self.operations
                .lock()
                .unwrap()
                .push("set_alternate_screen".to_string());
            Ok(())
        }

        fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
            *self.cursor_visible.lock().unwrap() = visible;
            self.operations
                .lock()
                .unwrap()
                .push(format!("set_cursor_visible: {}", visible));
            Ok(())
        }

        fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
            Ok(*self.cursor_pos.lock().unwrap())
        }

        fn get_capabilities(&self) -> OutputCapabilities {
            OutputCapabilities {
                supports_colors: true,
                supports_true_color: true,
                supports_styling: true,
                supports_alternate_screen: true,
                supports_cursor_control: true,
                max_colors: 256,
                platform_name: "mock".to_string(),
                backend_type: BackendType::Mock,
            }
        }
    }

    impl AsAny for MockConsoleOutput {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_renderer_new() {
        let mock_output = MockConsoleOutput::new();
        let renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        assert_eq!(renderer.prompt(), ">>> ");
        assert_eq!(renderer.window_size(), (80, 24));
        assert_eq!(renderer.screen_cursor_position(), (0, 0));
        assert!(renderer.is_cursor_visible());
    }

    #[test]
    fn test_renderer_render_empty_buffer() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let buffer = Buffer::new();
        let result = renderer.render(&buffer);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RenderResult::Updated);

        let ops = operations.lock().unwrap();
        assert!(ops.iter().any(|op| op.contains("write_text: >>> ")));
        assert!(ops.iter().any(|op| op.contains("flush")));
    }

    #[test]
    fn test_renderer_render_with_text() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let mut buffer = Buffer::new();
        buffer.insert_text("hello world", false, true);

        let result = renderer.render(&buffer);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RenderResult::Updated);

        let ops = operations.lock().unwrap();
        assert!(ops.iter().any(|op| op.contains("write_text: >>> ")));
        assert!(ops
            .iter()
            .any(|op| op.contains("write_safe_text: hello world")));
        assert!(ops.iter().any(|op| op.contains("flush")));
    }

    #[test]
    fn test_renderer_differential_rendering() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let mut buffer = Buffer::new();
        buffer.insert_text("hello", false, true);

        // First render
        let result1 = renderer.render(&buffer);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), RenderResult::Updated);

        let ops_count_after_first = operations.lock().unwrap().len();
        assert!(ops_count_after_first > 0);

        // Second render with same content - should be skipped
        let result2 = renderer.render(&buffer);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), RenderResult::NoChange);

        let ops_count_after_second = operations.lock().unwrap().len();
        assert_eq!(ops_count_after_first, ops_count_after_second);

        // Third render with different content - should update
        buffer.insert_text(" world", false, true);
        let result3 = renderer.render(&buffer);
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), RenderResult::Updated);

        let ops_count_after_third = operations.lock().unwrap().len();
        assert!(ops_count_after_third > ops_count_after_second);
    }

    #[test]
    fn test_renderer_force_render() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let mut buffer = Buffer::new();
        buffer.insert_text("test", false, true);

        // First render
        let result1 = renderer.render(&buffer);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), RenderResult::Updated);

        operations.lock().unwrap().clear();

        // Force render with same content - should render anyway
        let result2 = renderer.force_render(&buffer);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), RenderResult::Forced);

        let ops = operations.lock().unwrap();
        assert!(!ops.is_empty());
        assert!(ops.iter().any(|op| op.contains("clear")));
    }

    #[test]
    fn test_renderer_clear_line() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let result = renderer.clear_line();
        assert!(result.is_ok());

        let ops = operations.lock().unwrap();
        assert!(ops.iter().any(|op| op.contains("move_cursor_to: (0, 0)")));
        assert!(ops
            .iter()
            .any(|op| op.contains("clear: FromCursorToEndOfLine")));
        assert!(ops.iter().any(|op| op.contains("flush")));
    }

    #[test]
    fn test_renderer_break_line() {
        let mock_output = MockConsoleOutput::new();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let result = renderer.break_line();
        assert!(result.is_ok());

        let ops = operations.lock().unwrap();
        assert!(ops.iter().any(|op| op.contains("write_text: \n")));
        assert!(ops.iter().any(|op| op.contains("flush")));

        // Check that screen cursor position was updated
        assert_eq!(renderer.screen_cursor_position(), (1, 0));
    }

    #[test]
    fn test_renderer_update_window_size() {
        let mock_output = MockConsoleOutput::new();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        assert_eq!(renderer.window_size(), (80, 24));

        renderer.update_window_size(120, 30);
        assert_eq!(renderer.window_size(), (120, 30));
    }

    #[test]
    fn test_renderer_set_prompt() {
        let mock_output = MockConsoleOutput::new();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        assert_eq!(renderer.prompt(), ">>> ");

        renderer.set_prompt("$ ".to_string());
        assert_eq!(renderer.prompt(), "$ ");
    }

    #[test]
    fn test_renderer_cursor_visibility() {
        let mock_output = MockConsoleOutput::new();
        let cursor_visible = mock_output.cursor_visible.clone();
        let operations = mock_output.operations.clone();
        let mut renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        assert!(renderer.is_cursor_visible());
        assert!(*cursor_visible.lock().unwrap());

        let result = renderer.set_cursor_visible(false);
        assert!(result.is_ok());
        assert!(!renderer.is_cursor_visible());
        assert!(!*cursor_visible.lock().unwrap());

        let ops = operations.lock().unwrap();
        assert!(ops
            .iter()
            .any(|op| op.contains("set_cursor_visible: false")));
    }

    #[test]
    fn test_renderer_calculate_display_width() {
        let mock_output = MockConsoleOutput::new();
        let renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        assert_eq!(renderer.calculate_display_width("hello"), 5);
        assert_eq!(renderer.calculate_display_width(""), 0);
        // Note: Unicode width testing depends on the unicode module implementation
    }

    #[test]
    fn test_renderer_calculate_content_hash() {
        let mock_output = MockConsoleOutput::new();
        let renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let hash1 = renderer.calculate_content_hash("hello", 5);
        let hash2 = renderer.calculate_content_hash("hello", 5);
        let hash3 = renderer.calculate_content_hash("hello", 3);
        let hash4 = renderer.calculate_content_hash("world", 5);

        assert_eq!(hash1, hash2); // Same content should have same hash
        assert_ne!(hash1, hash3); // Different cursor position should have different hash
        assert_ne!(hash1, hash4); // Different text should have different hash
    }

    #[test]
    fn test_renderer_handle_line_wrapping_short_text() {
        let mock_output = MockConsoleOutput::new();
        let renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let result = renderer.handle_line_wrapping("hello");
        assert!(result.is_ok());

        let (display_text, wrapped_lines) = result.unwrap();
        assert_eq!(display_text, "hello");
        assert!(wrapped_lines.is_empty());
    }

    #[test]
    fn test_renderer_calculate_cursor_display_position() {
        let mock_output = MockConsoleOutput::new();
        let renderer = Renderer::new(Box::new(mock_output), ">>> ".to_string());

        let result1 = renderer.calculate_cursor_display_position("hello world", 0);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0);

        let result2 = renderer.calculate_cursor_display_position("hello world", 5);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 5);

        let result3 = renderer.calculate_cursor_display_position("hello world", 11);
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), 11);

        // Test cursor position beyond text length
        let result4 = renderer.calculate_cursor_display_position("hello", 10);
        assert!(result4.is_ok());
        assert_eq!(result4.unwrap(), 5); // Should be clamped to text length
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert!(config.enable_differential_rendering);
        assert!(config.show_cursor);
        assert!(config.max_line_length.is_none());
        assert!(config.prompt_style.is_none());
        assert!(config.input_style.is_none());
    }

    #[test]
    fn test_renderer_with_config() {
        let mock_output = MockConsoleOutput::new();
        let config = RenderConfig::default();
        let renderer = Renderer::with_config(Box::new(mock_output), ">>> ".to_string(), config);

        assert_eq!(renderer.prompt(), ">>> ");
        assert_eq!(renderer.window_size(), (80, 24));
    }

    #[test]
    fn test_render_result_debug() {
        assert_eq!(format!("{:?}", RenderResult::Updated), "Updated");
        assert_eq!(format!("{:?}", RenderResult::NoChange), "NoChange");
        assert_eq!(format!("{:?}", RenderResult::Forced), "Forced");
    }

    #[test]
    fn test_render_result_equality() {
        assert_eq!(RenderResult::Updated, RenderResult::Updated);
        assert_eq!(RenderResult::NoChange, RenderResult::NoChange);
        assert_eq!(RenderResult::Forced, RenderResult::Forced);
        assert_ne!(RenderResult::Updated, RenderResult::NoChange);
    }
}
