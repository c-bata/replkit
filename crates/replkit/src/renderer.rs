//! Terminal rendering system for interactive prompts
//!
//! This module provides the core rendering functionality for displaying prompts,
//! user input, and completion suggestions in the terminal using actual ConsoleOutput
//! implementations from replkit-io.

use crate::{ClearType, Color, ConsoleError, ConsoleOutput, Suggestion, TextStyle};
use replkit_core::{unicode::display_width, Document};
use std::io;

/// Formatted suggestion for display
#[derive(Debug, Clone)]
struct FormattedSuggestion {
    text: String,
    description: String,
}

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
    /// Previous cursor position in character units (go-prompt style)
    previous_cursor: usize,
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
            previous_cursor: 0,
            last_prompt_lines: 0,
            last_completion_lines: 0,
            terminal_size: (80, 24), // Default size, will be updated
        }
    }

    /// Initialize the renderer (go-prompt style - no cursor position query)
    pub fn initialize(&mut self) -> io::Result<()> {
        // Reset previous cursor position
        self.previous_cursor = 0;
        Ok(())
    }

    /// Reserve space for completion menu by moving cursor down (go-prompt style)
    pub fn reserve_completion_space(&mut self, lines_needed: u16) -> io::Result<()> {
        // Move cursor down to create space for completion menu
        for _ in 0..lines_needed {
            self.console
                .write_text("\n")
                .map_err(console_error_to_io_error)?;
        }

        // Update our tracking of completion lines
        self.last_completion_lines = lines_needed;
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
    /// Uses go-prompt compatible rendering approach with proper cursor management.
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
        // Move from previous cursor position to beginning of line (go-prompt style)
        self.move_cursor(self.previous_cursor, 0)?;

        // Hide cursor during rendering (go-prompt pattern)
        self.console
            .set_cursor_visible(false)
            .map_err(console_error_to_io_error)?;

        // Render prefix with styling (go-prompt uses SetColor)
        self.console
            .set_style(&TextStyle {
                foreground: Some(Color::Green),
                bold: true,
                ..Default::default()
            })
            .map_err(console_error_to_io_error)?;

        self.console
            .write_text(prefix)
            .map_err(console_error_to_io_error)?;
        
        // Reset to default color (go-prompt pattern)
        self.console
            .reset_style()
            .map_err(console_error_to_io_error)?;

        // Render the full text with input styling
        self.console
            .set_style(&TextStyle {
                foreground: Some(Color::White),
                ..Default::default()
            })
            .map_err(console_error_to_io_error)?;

        let line = document.text();
        self.console
            .write_text(line)
            .map_err(console_error_to_io_error)?;
        
        self.console
            .reset_style()
            .map_err(console_error_to_io_error)?;

        // Calculate cursor position (go-prompt style)
        let prefix_width = display_width(prefix);
        let line_width = display_width(line);
        let mut cursor = prefix_width + line_width;
        
        // Handle line wrapping like go-prompt
        self.handle_line_wrap(cursor)?;

        // Clear from cursor down (go-prompt's EraseDown)
        self.console
            .clear(ClearType::FromCursor)
            .map_err(console_error_to_io_error)?;

        // Move cursor to correct position using backward movement (go-prompt pattern)
        let text_after_cursor_width = display_width(document.text_after_cursor());
        cursor = self.backward(cursor, text_after_cursor_width)?;

        // Show cursor and flush
        self.console
            .set_cursor_visible(true)
            .map_err(console_error_to_io_error)?;
        
        self.console.flush().map_err(console_error_to_io_error)?;
        
        // Update previous cursor for next render (go-prompt style)
        self.previous_cursor = cursor;
        
        // Update prompt line tracking for cleanup
        self.last_prompt_lines = 1; // Basic prompt is always 1 line
        Ok(())
    }

    /// Render completion suggestions (go-prompt style)
    pub fn render_completions(&mut self, suggestions: &[Suggestion]) -> io::Result<()> {
        if suggestions.is_empty() {
            return Ok(());
        }

        let max_suggestions = 10;
        let window_height = suggestions.len().min(max_suggestions);
        let display_suggestions = &suggestions[..window_height];

        // Format suggestions with consistent width (go-prompt style)
        let available_width = self.terminal_size.0.saturating_sub(3) as usize; // -3 for prefix + scrollbar
        let formatted = self.format_suggestions_for_display(display_suggestions, available_width);

        // Prepare area (go-prompt's prepareArea pattern)
        for _ in 0..window_height {
            self.console
                .write_text("\n")
                .map_err(console_error_to_io_error)?;
        }
        
        // Move cursor back up to start rendering completions
        for _ in 0..window_height {
            self.console
                .move_cursor_relative(-1, 0)
                .map_err(console_error_to_io_error)?;
        }

        // Render each formatted completion line
        for formatted_suggestion in formatted.iter() {
            // Move down one line for each suggestion
            self.console
                .move_cursor_relative(1, 0)
                .map_err(console_error_to_io_error)?;

            // Set completion styling with background
            self.console
                .set_style(&TextStyle {
                    foreground: Some(Color::White),
                    background: Some(Color::Cyan),
                    ..Default::default()
                })
                .map_err(console_error_to_io_error)?;

            // Write the formatted text (already padded to consistent width)
            self.console
                .write_text(&formatted_suggestion.text)
                .map_err(console_error_to_io_error)?;

            // Write the formatted description (already padded)
            if !formatted_suggestion.description.is_empty() {
                self.console
                    .set_style(&TextStyle {
                        foreground: Some(Color::BrightBlack),
                        background: Some(Color::Cyan),
                        ..Default::default()
                    })
                    .map_err(console_error_to_io_error)?;

                self.console
                    .write_text(&formatted_suggestion.description)
                    .map_err(console_error_to_io_error)?;
            }

            // Add scrollbar space with background
            self.console
                .set_style(&TextStyle {
                    background: Some(Color::BrightBlack),
                    ..Default::default()
                })
                .map_err(console_error_to_io_error)?;

            self.console
                .write_text(" ")
                .map_err(console_error_to_io_error)?;

            self.console
                .reset_style()
                .map_err(console_error_to_io_error)?;

            // Move cursor back to beginning of line
            let total_width = display_width(&formatted_suggestion.text) + 
                display_width(&formatted_suggestion.description) + 1; // +1 for scrollbar
            self.console
                .move_cursor_relative(0, -(total_width as i16))
                .map_err(console_error_to_io_error)?;
        }

        // Move cursor back up to original position (go-prompt pattern)
        self.console
            .move_cursor_relative(-(window_height as i16), 0)
            .map_err(console_error_to_io_error)?;

        self.last_completion_lines = window_height as u16;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Render completion suggestions with a selected item highlighted (go-prompt style)
    pub fn render_completions_with_selection(
        &mut self,
        suggestions: &[Suggestion],
        selected_index: usize,
    ) -> io::Result<()> {
        if suggestions.is_empty() {
            return Ok(());
        }

        let max_display = 10;
        let window_height = suggestions.len().min(max_display);
        let display_suggestions = &suggestions[..window_height];

        // Format suggestions with consistent width (go-prompt style)
        let available_width = self.terminal_size.0.saturating_sub(3) as usize; // -3 for prefix + scrollbar
        let formatted = self.format_suggestions_for_display(display_suggestions, available_width);

        // Prepare area (go-prompt's prepareArea pattern)
        for _ in 0..window_height {
            self.console
                .write_text("\n")
                .map_err(console_error_to_io_error)?;
        }
        
        // Move cursor back up to start rendering completions
        for _ in 0..window_height {
            self.console
                .move_cursor_relative(-1, 0)
                .map_err(console_error_to_io_error)?;
        }

        // Render each formatted completion line
        for (i, formatted_suggestion) in formatted.iter().enumerate() {
            // Move down one line for each suggestion
            self.console
                .move_cursor_relative(1, 0)
                .map_err(console_error_to_io_error)?;

            let is_selected = i == selected_index;
            
            // Set styling based on selection for text
            if is_selected {
                self.console
                    .set_style(&TextStyle {
                        foreground: Some(Color::Black),
                        background: Some(Color::White),
                        bold: true,
                        ..Default::default()
                    })
                    .map_err(console_error_to_io_error)?;
            } else {
                self.console
                    .set_style(&TextStyle {
                        foreground: Some(Color::White),
                        background: Some(Color::Cyan),
                        ..Default::default()
                    })
                    .map_err(console_error_to_io_error)?;
            }

            // Write the formatted text (already padded to consistent width)
            self.console
                .write_text(&formatted_suggestion.text)
                .map_err(console_error_to_io_error)?;

            // Write the formatted description (already padded)
            if !formatted_suggestion.description.is_empty() {
                if is_selected {
                    self.console
                        .set_style(&TextStyle {
                            foreground: Some(Color::Black),
                            background: Some(Color::White),
                            ..Default::default()
                        })
                        .map_err(console_error_to_io_error)?;
                } else {
                    self.console
                        .set_style(&TextStyle {
                            foreground: Some(Color::BrightBlack),
                            background: Some(Color::Cyan),
                            ..Default::default()
                        })
                        .map_err(console_error_to_io_error)?;
                }

                self.console
                    .write_text(&formatted_suggestion.description)
                    .map_err(console_error_to_io_error)?;
            }

            // Add scrollbar space with appropriate background
            if is_selected {
                self.console
                    .set_style(&TextStyle {
                        background: Some(Color::White),
                        ..Default::default()
                    })
                    .map_err(console_error_to_io_error)?;
            } else {
                self.console
                    .set_style(&TextStyle {
                        background: Some(Color::BrightBlack),
                        ..Default::default()
                    })
                    .map_err(console_error_to_io_error)?;
            }

            self.console
                .write_text(" ")
                .map_err(console_error_to_io_error)?;

            self.console
                .reset_style()
                .map_err(console_error_to_io_error)?;

            // Move cursor back to beginning of line
            let total_width = display_width(&formatted_suggestion.text) + 
                display_width(&formatted_suggestion.description) + 1; // +1 for scrollbar
            self.console
                .move_cursor_relative(0, -(total_width as i16))
                .map_err(console_error_to_io_error)?;
        }

        // Move cursor back up to original position (go-prompt pattern)
        self.console
            .move_cursor_relative(-(window_height as i16), 0)
            .map_err(console_error_to_io_error)?;

        self.last_completion_lines = window_height as u16;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Clear completion suggestions from display (simplified)
    pub fn clear_completions(&mut self) -> io::Result<()> {
        // Simple implementation - just reset the counter
        self.last_completion_lines = 0;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Clear the entire prompt and return to beginning of line (simplified)
    pub fn clear_prompt(&mut self) -> io::Result<()> {
        // Simple implementation - just reset counters
        self.last_prompt_lines = 0;
        self.last_completion_lines = 0;
        self.previous_cursor = 0;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Move cursor to end of current line
    pub fn move_cursor_to_end_of_line(&mut self) -> io::Result<()> {
        // In go-prompt style, we don't need to do anything special here
        // The cursor should already be at the correct position
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Write a newline character
    pub fn write_newline(&mut self) -> io::Result<()> {
        self.console
            .write_text("\n")
            .map_err(console_error_to_io_error)?;
        // Reset previous cursor position
        self.previous_cursor = 0;
        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    // Temporarily removed complex methods to focus on basic functionality

    /// Render completion preview (go-prompt style)
    pub fn render_completion_preview(&mut self, document: &Document, suggestion: &Suggestion) -> io::Result<()> {
        // Get the word that would be replaced using separator-based logic
        let word = document.get_word_before_cursor_until_separator(" \t\n");
        let word_width = display_width(word);
        
        // Move cursor back to start of word
        if word_width > 0 {
            let cursor = self.previous_cursor;
            self.previous_cursor = self.backward(cursor, word_width)?;
        }

        // Render suggestion with preview styling (go-prompt previewSuggestionTextColor)
        self.console
            .set_style(&TextStyle {
                foreground: Some(Color::BrightBlack),
                ..Default::default()
            })
            .map_err(console_error_to_io_error)?;

        self.console
            .write_text(&suggestion.text)
            .map_err(console_error_to_io_error)?;

        self.console
            .reset_style()
            .map_err(console_error_to_io_error)?;

        // Update cursor position
        self.previous_cursor += display_width(&suggestion.text);

        // Render rest of the line
        let rest = document.text_after_cursor();
        if !rest.is_empty() {
            self.console
                .write_text(rest)
                .map_err(console_error_to_io_error)?;

            // Handle line wrapping for the complete line
            let total_width = display_width(&suggestion.text) + display_width(rest);
            self.handle_line_wrap(total_width)?;

            // Move cursor back to correct position
            self.previous_cursor = self.backward(self.previous_cursor, display_width(rest))?;
        }

        self.console.flush().map_err(console_error_to_io_error)?;
        Ok(())
    }

    /// Convert character position to screen coordinates (go-prompt's toPos)
    fn to_pos(&self, cursor: usize) -> (usize, usize) {
        let col = self.terminal_size.0 as usize;
        if col == 0 {
            return (0, 0);
        }
        (cursor % col, cursor / col)
    }

    /// Calculate how many terminal lines the given width will occupy
    fn calculate_line_count(&self, width: usize) -> u16 {
        let cols = self.terminal_size.0 as usize;
        if cols == 0 {
            return 1;
        }

        width.div_ceil(cols).max(1) as u16
    }

    /// Move cursor from one character position to another (go-prompt's move)
    fn move_cursor(&mut self, from: usize, to: usize) -> io::Result<usize> {
        let (from_x, from_y) = self.to_pos(from);
        let (to_x, to_y) = self.to_pos(to);

        // Move vertically first
        if from_y > to_y {
            self.console
                .move_cursor_relative(-((from_y - to_y) as i16), 0)
                .map_err(console_error_to_io_error)?;
        } else if to_y > from_y {
            self.console
                .move_cursor_relative((to_y - from_y) as i16, 0)
                .map_err(console_error_to_io_error)?;
        }

        // Move horizontally
        if from_x > to_x {
            self.console
                .move_cursor_relative(0, -((from_x - to_x) as i16))
                .map_err(console_error_to_io_error)?;
        } else if to_x > from_x {
            self.console
                .move_cursor_relative(0, (to_x - from_x) as i16)
                .map_err(console_error_to_io_error)?;
        }

        Ok(to)
    }

    /// Move cursor backward by n characters (go-prompt's backward)
    fn backward(&mut self, from: usize, n: usize) -> io::Result<usize> {
        self.move_cursor(from, from.saturating_sub(n))
    }

    // Cursor movement methods temporarily removed

    /// Handle line wrapping like go-prompt's lineWrap function
    fn handle_line_wrap(&mut self, cursor_pos: usize) -> io::Result<()> {
        let cols = self.terminal_size.0 as usize;
        if cols == 0 {
            return Ok(());
        }

        // Check if we need to handle line wrapping
        if cursor_pos > 0 && cursor_pos % cols == 0 {
            // On Unix systems (not Windows), go-prompt adds a newline at column boundaries
            #[cfg(not(target_os = "windows"))]
            {
                self.console
                    .write_text("\n")
                    .map_err(console_error_to_io_error)?;
            }
        }
        Ok(())
    }

    /// Format suggestions for consistent display width (go-prompt style)
    fn format_suggestions_for_display(&self, suggestions: &[Suggestion], max_width: usize) -> Vec<FormattedSuggestion> {
        if suggestions.is_empty() {
            return Vec::new();
        }

        // Find maximum text width
        let max_text_width = suggestions
            .iter()
            .map(|s| display_width(&s.text))
            .max()
            .unwrap_or(0)
            .min(max_width / 2); // Don't use more than half width for text

        // Find maximum description width
        let remaining_width = max_width.saturating_sub(max_text_width + 1); // -1 for space
        let max_desc_width = suggestions
            .iter()
            .map(|s| display_width(&s.description))
            .max()
            .unwrap_or(0)
            .min(remaining_width);

        let mut formatted = Vec::new();
        
        for suggestion in suggestions {
            // Format text with consistent width
            let mut text = suggestion.text.clone();
            let text_width = display_width(&text);
            
            if text_width > max_text_width {
                // Truncate if too long
                text = self.truncate_text(&text, max_text_width);
            }
            
            // Pad to consistent width
            let padding_needed = max_text_width.saturating_sub(display_width(&text));
            text.push_str(&" ".repeat(padding_needed));

            // Format description with consistent width
            let mut description = String::new();
            if !suggestion.description.is_empty() && max_desc_width > 0 {
                description = format!(" {}", suggestion.description);
                let desc_width = display_width(&description);
                
                if desc_width > max_desc_width {
                    // Truncate if too long
                    description = self.truncate_text(&description, max_desc_width);
                }
                
                // Pad to consistent width
                let desc_padding = max_desc_width.saturating_sub(display_width(&description));
                description.push_str(&" ".repeat(desc_padding));
            } else if max_desc_width > 0 {
                // Empty description, but pad to consistent width
                description = " ".repeat(max_desc_width);
            }

            formatted.push(FormattedSuggestion { text, description });
        }

        formatted
    }

    /// Truncate text to fit within specified width (go-prompt style)
    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        if max_width <= 3 {
            return "...".to_string();
        }
        
        let mut result = String::new();
        let mut current_width = 0;
        
        for ch in text.chars() {
            let ch_width = display_width(&ch.to_string());
            if current_width + ch_width > max_width - 3 {
                result.push_str("...");
                break;
            }
            result.push(ch);
            current_width += ch_width;
        }
        
        result
    }

    // Clear methods temporarily simplified
}

/// Convert ConsoleError to io::Error for compatibility
fn console_error_to_io_error(error: ConsoleError) -> io::Error {
    match error {
        ConsoleError::IoError(msg) => io::Error::other(msg),
        other => io::Error::other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use replkit_core::Document;
    use replkit_io::mock::MockConsoleOutput;

    #[test]
    fn test_renderer_creation() {
        let console = Box::new(MockConsoleOutput::new());
        let renderer = Renderer::new(console);

        assert_eq!(renderer.terminal_size(), (80, 24));
        assert_eq!(renderer.previous_cursor, 0);
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
    fn test_to_pos() {
        let console = Box::new(MockConsoleOutput::new());
        let mut renderer = Renderer::new(console);
        renderer.update_terminal_size(80, 24);

        // Test various positions
        assert_eq!(renderer.to_pos(0), (0, 0));
        assert_eq!(renderer.to_pos(40), (40, 0));
        assert_eq!(renderer.to_pos(80), (0, 1));
        assert_eq!(renderer.to_pos(120), (40, 1));
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
