//! Terminal rendering system for interactive prompts
//!
//! This module provides the core rendering functionality for displaying prompts,
//! user input, and completion suggestions in the terminal using actual ConsoleOutput
//! implementations from replkit-io.

use std::io;
use replkit_core::{Document, unicode::display_width};
use crate::{Suggestion, ConsoleOutput, ConsoleError, TextStyle, Color, ClearType};

/// Terminal renderer for interactive prompts
///
/// The Renderer is responsible for displaying prompts, user input, and completion
/// suggestions in the terminal. It uses real ConsoleOutput implementations to
/// provide cross-platform terminal rendering capabilities.
///
/// # Examples
///
/// ```rust
/// use replkit::prelude::*;
/// use replkit_io::MockConsoleOutput;
/// 
/// let console = Box::new(MockConsoleOutput::new());
/// let mut renderer = Renderer::new(console);
/// let document = Document::with_text("hello world".to_string(), 5);
/// 
/// // Render prompt with current document state
/// renderer.render_prompt("$ ", &document).unwrap();
/// ```
pub struct Renderer {
    console: Box<dyn ConsoleOutput>,
    /// Current cursor position (row, col) - 0-based
    cursor_position: (u16, u16),
    /// Last rendered prompt state for cleanup
    last_prompt_lines: u16,
    /// Last rendered completion menu lines for cleanup
    last_completion_lines: u16,
    /// Cached terminal window size (cols, rows)
    terminal_size: (u16, u16),
}

impl Renderer {
    /// Create a new renderer with the given console output
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::prelude::*;
    /// use replkit_io::MockConsoleOutput;
    /// 
    /// let console = Box::new(MockConsoleOutput::new());
    /// let renderer = Renderer::new(console);
    /// ```
    pub fn new(console: Box<dyn ConsoleOutput>) -> Self {
        Self {
            console,
            cursor_position: (0, 0),
            last_prompt_lines: 0,
            last_completion_lines: 0,
            terminal_size: (80, 24), // Default size, will be updated
        }
    }

    /// Initialize the renderer by getting current cursor position
    pub fn initialize(&mut self) -> io::Result<()> {
        // Get current cursor position from terminal
        if let Ok(pos) = self.console.get_cursor_position() {
            self.cursor_position = pos;
        }
        
        Ok(())
    }

    /// Reserve space for completion menu by moving cursor down
    /// Returns the original cursor position
    pub fn reserve_completion_space(&mut self, lines_needed: u16) -> io::Result<(u16, u16)> {
        let original_pos = self.cursor_position;
        
        // Move cursor down to create space for completion menu
        for _ in 0..lines_needed {
            self.console.write_text("\n").map_err(console_error_to_io_error)?;
        }
        
        // Update our tracking of completion lines
        self.last_completion_lines = lines_needed;
        
        Ok(original_pos)
    }

    /// Return to the original prompt position
    pub fn return_to_prompt_position(&mut self, original_pos: (u16, u16)) -> io::Result<()> {
        self.console.move_cursor_to(original_pos.0, original_pos.1).map_err(console_error_to_io_error)?;
        self.cursor_position = original_pos;
        Ok(())
    }

    /// Update terminal size cache
    ///
    /// This should be called when the terminal is resized to ensure proper
    /// text wrapping and cursor positioning.
    pub fn update_terminal_size(&mut self, cols: u16, rows: u16) {
        self.terminal_size = (cols, rows);
    }

    /// Get current terminal size
    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }

    /// Render the prompt with current document state
    ///
    /// This displays the prompt prefix followed by the current text from the document,
    /// with the cursor positioned correctly based on the document's cursor position.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prompt prefix (e.g., "$ ", ">>> ")
    /// * `document` - The current document containing user input and cursor position
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::prelude::*;
    /// use replkit_io::MockConsoleOutput;
    /// 
    /// let console = Box::new(MockConsoleOutput::new());
    /// let mut renderer = Renderer::new(console);
    /// let document = Document::with_text("hello world".to_string(), 5);
    /// 
    /// renderer.render_prompt("$ ", &document).unwrap();
    /// ```
    pub fn render_prompt(&mut self, prefix: &str, document: &Document) -> io::Result<()> {
        // Clear the current line only
        self.console.move_cursor_to(self.cursor_position.0, 0).map_err(console_error_to_io_error)?;
        self.console.clear(ClearType::CurrentLine).map_err(console_error_to_io_error)?;

        // Render prefix with styling
        self.console.set_style(&TextStyle {
            foreground: Some(Color::Green),
            bold: true,
            ..Default::default()
        }).map_err(console_error_to_io_error)?;
        
        self.console.write_text(prefix).map_err(console_error_to_io_error)?;
        self.console.reset_style().map_err(console_error_to_io_error)?;

        // Render the full text
        self.console.write_text(document.text()).map_err(console_error_to_io_error)?;

        // Calculate and move to cursor position
        let prefix_width = display_width(prefix);
        let text_before_cursor = document.text_before_cursor();
        let cursor_col = prefix_width + display_width(text_before_cursor);
        
        // Move cursor to correct position
        self.console.move_cursor_to(self.cursor_position.0, cursor_col as u16).map_err(console_error_to_io_error)?;

        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Render completion suggestions
    ///
    /// Displays a list of completion suggestions below the current prompt.
    /// The suggestions are formatted in a readable manner with proper spacing.
    ///
    /// # Arguments
    ///
    /// * `suggestions` - List of completion suggestions to display
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::prelude::*;
    /// use replkit_io::MockConsoleOutput;
    /// 
    /// let console = Box::new(MockConsoleOutput::new());
    /// let mut renderer = Renderer::new(console);
    /// let suggestions = vec![
    ///     Suggestion::new("help", "Show help information"),
    ///     Suggestion::new("quit", "Exit the application"),
    /// ];
    /// 
    /// renderer.render_completions(&suggestions).unwrap();
    /// ```
    pub fn render_completions(&mut self, suggestions: &[Suggestion]) -> io::Result<()> {
        if suggestions.is_empty() {
            return Ok(());
        }

        // Save current cursor position
        let current_row = self.cursor_position.0;
        
        // Move to next line for completions
        self.console.write_text("\n").map_err(console_error_to_io_error)?;

        // Render suggestions
        let max_suggestions = 10;
        let display_count = suggestions.len().min(max_suggestions);

        for (i, suggestion) in suggestions.iter().take(display_count).enumerate() {
            if i > 0 {
                self.console.write_text("\n").map_err(console_error_to_io_error)?;
            }

            // Style for suggestion text
            self.console.set_style(&TextStyle {
                foreground: Some(Color::Cyan),
                bold: false,
                ..Default::default()
            }).map_err(console_error_to_io_error)?;

            self.console.write_text(&suggestion.text).map_err(console_error_to_io_error)?;
            
            // Add description if available
            if !suggestion.description.is_empty() {
                self.console.set_style(&TextStyle {
                    foreground: Some(Color::BrightBlack),
                    bold: false,
                    ..Default::default()
                }).map_err(console_error_to_io_error)?;
                
                self.console.write_text(&format!(" - {}", suggestion.description))
                    .map_err(console_error_to_io_error)?;
            }

            self.console.reset_style().map_err(console_error_to_io_error)?;
        }

        // Return to original prompt position
        self.console.move_cursor_to(current_row, self.cursor_position.1).map_err(console_error_to_io_error)?;
        
        // Update completion line count for cleanup
        self.last_completion_lines = display_count as u16;
        
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Render completion suggestions with a selected item highlighted
    ///
    /// Displays a list of completion suggestions below the current prompt with the
    /// selected item highlighted using inverse video (background color).
    ///
    /// # Arguments
    ///
    /// * `suggestions` - List of completion suggestions to display
    /// * `selected_index` - Index of the currently selected suggestion to highlight
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::prelude::*;
    /// use replkit_io::MockConsoleOutput;
    /// 
    /// let console = Box::new(MockConsoleOutput::new());
    /// let mut renderer = Renderer::new(console);
    /// let suggestions = vec![
    ///     Suggestion::new("help", "Show help information"),
    ///     Suggestion::new("quit", "Exit the application"),
    /// ];
    /// 
    /// renderer.render_completions_with_selection(&suggestions, 0).unwrap();
    /// ```
    pub fn render_completions_with_selection(&mut self, suggestions: &[Suggestion], selected_index: usize) -> io::Result<()> {
        if suggestions.is_empty() {
            return Ok(());
        }

        // Save current cursor position
        let current_row = self.cursor_position.0;
        
        // Move to next line for completions
        self.console.write_text("\n").map_err(console_error_to_io_error)?;

        // Render suggestions
        let max_suggestions = 10;
        let display_count = suggestions.len().min(max_suggestions);

        for (i, suggestion) in suggestions.iter().take(display_count).enumerate() {
            if i > 0 {
                self.console.write_text("\n").map_err(console_error_to_io_error)?;
            }

            // Check if this is the selected item
            let is_selected = i == selected_index;

            if is_selected {
                // Highlight selected item with background color
                self.console.set_style(&TextStyle {
                    foreground: Some(Color::Black),
                    background: Some(Color::White),
                    bold: true,
                    ..Default::default()
                }).map_err(console_error_to_io_error)?;
            } else {
                // Normal style for non-selected items
                self.console.set_style(&TextStyle {
                    foreground: Some(Color::Cyan),
                    bold: false,
                    ..Default::default()
                }).map_err(console_error_to_io_error)?;
            }

            self.console.write_text(&suggestion.text).map_err(console_error_to_io_error)?;
            
            // Add description if available
            if !suggestion.description.is_empty() {
                if is_selected {
                    self.console.set_style(&TextStyle {
                        foreground: Some(Color::Black),
                        background: Some(Color::White),
                        bold: false,
                        ..Default::default()
                    }).map_err(console_error_to_io_error)?;
                } else {
                    self.console.set_style(&TextStyle {
                        foreground: Some(Color::BrightBlack),
                        bold: false,
                        ..Default::default()
                    }).map_err(console_error_to_io_error)?;
                }

                self.console.write_text(&format!(" - {}", suggestion.description)).map_err(console_error_to_io_error)?;
            }

            // Reset style after each line
            self.console.reset_style().map_err(console_error_to_io_error)?;
        }

        // Show "more suggestions" indicator if needed
        if suggestions.len() > max_suggestions {
            self.console.write_text("\n").map_err(console_error_to_io_error)?;
            self.console.set_style(&TextStyle {
                foreground: Some(Color::BrightBlack),
                bold: false,
                ..Default::default()
            }).map_err(console_error_to_io_error)?;
            
            let more_count = suggestions.len() - max_suggestions;
            self.console.write_text(&format!("... {} more suggestions", more_count)).map_err(console_error_to_io_error)?;
            
            self.console.reset_style().map_err(console_error_to_io_error)?;
        }

        // Return to original prompt position
        self.console.move_cursor_to(current_row, self.cursor_position.1).map_err(console_error_to_io_error)?;
        
        // Update completion line count for cleanup
        let lines_used = display_count as u16 + if suggestions.len() > max_suggestions { 1 } else { 0 };
        self.last_completion_lines = lines_used;
        
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Clear completion suggestions from display
    ///
    /// Removes any currently displayed completion suggestions and restores
    /// the terminal to show only the prompt and user input.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use replkit::prelude::*;
    /// use replkit_io::MockConsoleOutput;
    /// 
    /// let console = Box::new(MockConsoleOutput::new());
    /// let mut renderer = Renderer::new(console);
    /// 
    /// // After showing completions...
    /// renderer.clear_completions().unwrap();
    /// ```
    pub fn clear_completions(&mut self) -> io::Result<()> {
        self.clear_previous_completions()?;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Clear the entire prompt and return to beginning of line
    ///
    /// This is useful when the prompt needs to be completely redrawn or
    /// when exiting the prompt session.
    pub fn clear_prompt(&mut self) -> io::Result<()> {
        self.clear_previous_prompt()?;
        self.clear_previous_completions()?;
        
        // Move to beginning of prompt line - only if we have prompt lines to move back
        if self.last_prompt_lines > 0 {
            let start_row = self.cursor_position.0.saturating_sub(self.last_prompt_lines - 1);
            self.console.move_cursor_to(start_row, 0).map_err(console_error_to_io_error)?;
            self.cursor_position = (start_row, 0);
        } else {
            // No prompt lines, just move to column 0
            self.console.move_cursor_to(self.cursor_position.0, 0).map_err(console_error_to_io_error)?;
            self.cursor_position.1 = 0;
        }
        
        self.last_prompt_lines = 0;
        self.last_completion_lines = 0;
        
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Move cursor to end of current line
    pub fn move_cursor_to_end_of_line(&mut self) -> io::Result<()> {
        // For now, we'll assume cursor is already at the end during normal rendering
        // This could be enhanced to calculate the actual end position
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Write a newline character
    pub fn write_newline(&mut self) -> io::Result<()> {
        self.console.write_text("\n").map_err(console_error_to_io_error)?;
        // Update cursor position to next line, column 0
        self.cursor_position.0 += 1;
        self.cursor_position.1 = 0;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Calculate screen position from character offset
    fn calculate_screen_position(&self, char_offset: usize) -> (u16, u16) {
        let cols = self.terminal_size.0 as usize;
        if cols == 0 {
            return (0, 0);
        }
        
        let row = char_offset / cols;
        let col = char_offset % cols;
        (row as u16, col as u16)
    }

    /// Calculate how many terminal lines the given width will occupy
    fn calculate_line_count(&self, width: usize) -> u16 {
        let cols = self.terminal_size.0 as usize;
        if cols == 0 {
            return 1;
        }
        
        ((width + cols - 1) / cols).max(1) as u16
    }

    /// Clear previously rendered prompt lines
    fn clear_previous_prompt(&mut self) -> io::Result<()> {
        if self.last_prompt_lines == 0 {
            return Ok(());
        }

        // Move to start of prompt
        let start_row = self.cursor_position.0.saturating_sub(self.last_prompt_lines - 1);
        self.console.move_cursor_to(start_row, 0).map_err(console_error_to_io_error)?;

        // Clear each line
        for _ in 0..self.last_prompt_lines {
            self.console.clear(ClearType::CurrentLine).map_err(console_error_to_io_error)?;
            if self.last_prompt_lines > 1 {
                // Move to next line if clearing multiple lines
                self.console.write_text("\r\n").map_err(console_error_to_io_error)?;
            }
        }

        // Return to start position
        self.console.move_cursor_to(start_row, 0).map_err(console_error_to_io_error)?;
        self.last_prompt_lines = 0;
        Ok(())
    }

    /// Clear previously rendered completion lines
    fn clear_previous_completions(&mut self) -> io::Result<()> {
        if self.last_completion_lines == 0 {
            return Ok(());
        }

        // Move to start of completion area
        let completion_start_row = self.cursor_position.0 + self.last_prompt_lines;
        self.console.move_cursor_to(completion_start_row, 0).map_err(console_error_to_io_error)?;

        // Clear each completion line
        for _ in 0..self.last_completion_lines {
            self.console.clear(ClearType::CurrentLine).map_err(console_error_to_io_error)?;
            if self.last_completion_lines > 1 {
                self.console.write_text("\r\n").map_err(console_error_to_io_error)?;
            }
        }

        self.last_completion_lines = 0;
        Ok(())
    }
}

/// Convert ConsoleError to io::Error for compatibility
fn console_error_to_io_error(error: ConsoleError) -> io::Error {
    match error {
        ConsoleError::IoError(msg) => io::Error::new(io::ErrorKind::Other, msg),
        other => io::Error::new(io::ErrorKind::Other, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use replkit_io::mock::MockConsoleOutput;
    use replkit_core::Document;

    #[test]
    fn test_renderer_creation() {
        let console = Box::new(MockConsoleOutput::new());
        let renderer = Renderer::new(console);
        
        assert_eq!(renderer.terminal_size(), (80, 24));
        assert_eq!(renderer.cursor_position, (0, 0));
        assert_eq!(renderer.last_prompt_lines, 0);
        assert_eq!(renderer.last_completion_lines, 0);
    }

    #[test]
    fn test_terminal_size_update() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        
        renderer.update_terminal_size(120, 30);
        assert_eq!(renderer.terminal_size(), (120, 30));
    }

    #[test]
    fn test_calculate_screen_position() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        renderer.update_terminal_size(80, 24);
        
        // Test various positions
        assert_eq!(renderer.calculate_screen_position(0), (0, 0));
        assert_eq!(renderer.calculate_screen_position(40), (0, 40));
        assert_eq!(renderer.calculate_screen_position(80), (1, 0));
        assert_eq!(renderer.calculate_screen_position(120), (1, 40));
    }

    #[test]
    fn test_calculate_line_count() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        renderer.update_terminal_size(80, 24);
        
        assert_eq!(renderer.calculate_line_count(0), 1);
        assert_eq!(renderer.calculate_line_count(40), 1);
        assert_eq!(renderer.calculate_line_count(80), 1);
        assert_eq!(renderer.calculate_line_count(81), 2);
        assert_eq!(renderer.calculate_line_count(160), 2);
        assert_eq!(renderer.calculate_line_count(161), 3);
    }

    #[test]
    fn test_render_prompt_basic() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        let document = Document::with_text("hello world".to_string(), 5);
        
        let result = renderer.render_prompt("$ ", &document);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_completions_empty() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        
        let result = renderer.render_completions(&[]);
        assert!(result.is_ok());
        assert_eq!(renderer.last_completion_lines, 0);
    }

    #[test]
    fn test_render_completions_basic() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        
        let suggestions = vec![
            Suggestion::new("help", "Show help"),
            Suggestion::new("quit", "Exit app"),
        ];
        
        let result = renderer.render_completions(&suggestions);
        assert!(result.is_ok());
        assert_eq!(renderer.last_completion_lines, 2);
    }

    #[test]
    fn test_render_completions_many() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        
        let suggestions: Vec<_> = (0..15)
            .map(|i| Suggestion::new(format!("cmd{}", i), format!("Command {}", i)))
            .collect();
        
        let result = renderer.render_completions(&suggestions);
        assert!(result.is_ok());
        // Should show 10 suggestions + 1 "more" line
        assert_eq!(renderer.last_completion_lines, 11);
    }

    #[test]
    fn test_clear_completions() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        
        // First render some completions
        let suggestions = vec![Suggestion::new("test", "Test command")];
        renderer.render_completions(&suggestions).unwrap();
        assert_eq!(renderer.last_completion_lines, 1);
        
        // Then clear them
        let result = renderer.clear_completions();
        assert!(result.is_ok());
        assert_eq!(renderer.last_completion_lines, 0);
    }

    #[test]
    fn test_clear_prompt() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        let document = Document::with_text("test".to_string(), 2);
        
        // Render prompt
        renderer.render_prompt("$ ", &document).unwrap();
        assert!(renderer.last_prompt_lines > 0);
        
        // Clear prompt
        let result = renderer.clear_prompt();
        assert!(result.is_ok());
        assert_eq!(renderer.last_prompt_lines, 0);
        assert_eq!(renderer.last_completion_lines, 0);
    }
}
