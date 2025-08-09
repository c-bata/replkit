//! Text buffer implementation for mutable text editing operations.
//!
//! The Buffer structure provides a mutable interface for text editing operations
//! while managing Document instances for text analysis. It supports multi-line
//! editing, cursor movement, and maintains performance through intelligent caching.

use crate::document::Document;
use crate::error::{BufferError, BufferResult};
use crate::key::Key;
use crate::unicode;

/// A mutable text buffer that manages editing operations.
///
/// Buffer provides the primary interface for text editing operations while
/// maintaining a cached Document instance for efficient text analysis.
/// It supports multi-line editing through a working lines system.
#[derive(Debug, Clone)]
pub struct Buffer {
    /// Multiple lines for history-like functionality and multi-line editing
    working_lines: Vec<String>,
    /// Current line index in working_lines
    working_index: usize,
    /// Cursor position as rune index within current line
    cursor_position: usize,
    /// Cached document for performance optimization
    cached_document: Option<Document>,
    /// Preferred column for vertical cursor movement consistency
    preferred_column: Option<usize>,
    /// Track last key for context-aware operations
    last_key_stroke: Option<Key>,
}

impl Buffer {
    /// Create a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let buffer = Buffer::new();
    /// assert_eq!(buffer.text(), "");
    /// assert_eq!(buffer.cursor_position(), 0);
    /// ```
    pub fn new() -> Self {
        Buffer {
            working_lines: vec![String::new()],
            working_index: 0,
            cursor_position: 0,
            cached_document: None,
            preferred_column: None,
            last_key_stroke: None,
        }
    }

    /// Get the current text content.
    ///
    /// Returns the text of the current working line.
    pub fn text(&self) -> &str {
        &self.working_lines[self.working_index]
    }

    /// Get the current cursor position as a rune index.
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Get the current working index.
    pub fn working_index(&self) -> usize {
        self.working_index
    }

    /// Get the number of working lines.
    pub fn working_lines_count(&self) -> usize {
        self.working_lines.len()
    }

    /// Get a reference to the working lines.
    pub fn working_lines(&self) -> &Vec<String> {
        &self.working_lines
    }

    /// Get the preferred column for vertical cursor movement.
    pub fn preferred_column(&self) -> Option<usize> {
        self.preferred_column
    }

    /// Get the last key stroke.
    pub fn last_key_stroke(&self) -> Option<Key> {
        self.last_key_stroke
    }

    /// Get a reference to the cached Document, creating it if necessary.
    ///
    /// This method provides efficient access to Document functionality
    /// by caching the Document instance when the buffer state hasn't changed.
    pub fn document(&mut self) -> &Document {
        self.update_cached_document();
        self.cached_document.as_ref().unwrap()
    }

    /// Get the display cursor position accounting for Unicode character widths.
    pub fn display_cursor_position(&mut self) -> usize {
        self.document().display_cursor_position()
    }

    /// Set the text content of the current working line.
    ///
    /// This will invalidate the cached document and reset the cursor position
    /// if it's beyond the new text length.
    pub fn set_text(&mut self, text: String) {
        self.working_lines[self.working_index] = text;
        self.ensure_cursor_bounds();
        self.invalidate_cache();
    }

    /// Set the text content with validation.
    ///
    /// This version performs validation on the text before setting it.
    ///
    /// # Arguments
    ///
    /// * `text` - The new text content
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, or a `BufferError` if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// assert!(buffer.set_text_validated("hello world".to_string()).is_ok());
    /// assert_eq!(buffer.text(), "hello world");
    /// ```
    pub fn set_text_validated(&mut self, text: String) -> BufferResult<()> {
        use crate::error::validation;

        // Validate text encoding
        validation::validate_text_encoding(&text)?;

        // Set the text
        self.set_text(text);

        Ok(())
    }

    /// Set the last key stroke for context-aware operations.
    pub fn set_last_key_stroke(&mut self, key: Key) {
        self.last_key_stroke = Some(key);
        self.invalidate_cache();
    }

    /// Set the last key stroke (optional) for context-aware operations.
    pub fn set_last_key_stroke_optional(&mut self, key: Option<Key>) {
        self.last_key_stroke = key;
        self.invalidate_cache();
    }

    /// Set the working lines directly (used for WASM deserialization).
    pub fn set_working_lines(&mut self, lines: Vec<String>) {
        self.working_lines = lines;
        if self.working_lines.is_empty() {
            self.working_lines.push(String::new());
        }
        self.ensure_cursor_bounds();
        self.invalidate_cache();
    }

    /// Set the preferred column for vertical cursor movement.
    pub fn set_preferred_column(&mut self, column: Option<usize>) {
        self.preferred_column = column;
    }

    /// Create a new line at the current cursor position.
    ///
    /// This method inserts a newline character and optionally copies the indentation
    /// (leading whitespace) from the current line to the new line.
    ///
    /// # Arguments
    ///
    /// * `copy_margin` - If true, copy leading whitespace from current line to new line
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("    indented line".to_string());
    /// buffer.set_cursor_position(16);
    ///
    /// // Create new line with indentation copying
    /// buffer.new_line(true);
    /// assert_eq!(buffer.text(), "    indented lin\n    e");
    /// assert_eq!(buffer.cursor_position(), 21);
    /// ```
    pub fn new_line(&mut self, copy_margin: bool) {
        let newline_text = if copy_margin {
            // Get the leading whitespace from the current line
            let doc = self.document();
            let leading_whitespace = doc.leading_whitespace_in_current_line();
            format!("\n{leading_whitespace}")
        } else {
            "\n".to_string()
        };

        // Insert the newline (and optional indentation) at cursor position
        self.insert_text(&newline_text, false, true);
    }

    /// Join the current line with the next line using the specified separator.
    ///
    /// This method merges the current line with the following line, removing the
    /// newline character between them and inserting the specified separator.
    /// Leading whitespace from the next line is trimmed.
    /// If there is no next line, this operation has no effect.
    ///
    /// # Arguments
    ///
    /// * `separator` - The text to insert between the joined lines
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("first line\nsecond line".to_string());
    /// buffer.set_cursor_position(5); // Middle of first line
    ///
    /// // Join lines with a space separator
    /// buffer.join_next_line(" ");
    /// assert_eq!(buffer.text(), "first line second line");
    /// assert_eq!(buffer.cursor_position(), 5); // Cursor position unchanged
    /// ```
    pub fn join_next_line(&mut self, separator: &str) {
        let current_text = self.text().to_string();
        let text_rune_count = unicode::rune_count(&current_text);

        // Find the next newline character after the cursor
        let mut newline_pos = None;
        for (i, ch) in current_text.chars().enumerate().skip(self.cursor_position) {
            if ch == '\n' {
                newline_pos = Some(i);
                break;
            }
        }

        // If no newline found, there's no next line to join
        if let Some(newline_rune_pos) = newline_pos {
            // Convert byte position to rune position
            let newline_rune_index = current_text.chars().take(newline_rune_pos).count();

            // Get the text before the newline
            let before_newline = unicode::rune_slice(&current_text, 0, newline_rune_index);

            // Get the text after the newline and trim leading whitespace
            let after_newline =
                unicode::rune_slice(&current_text, newline_rune_index + 1, text_rune_count);
            let trimmed_after = after_newline.trim_start();

            let new_text = format!("{before_newline}{separator}{trimmed_after}");
            self.working_lines[self.working_index] = new_text;
            self.invalidate_cache();
        }
        // If no newline found, do nothing (no next line to join)
    }

    /// Swap the two characters immediately before the cursor.
    ///
    /// This operation exchanges the character immediately before the cursor with
    /// the character before that. If there are fewer than two characters before
    /// the cursor, this operation has no effect. The cursor position remains unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5); // After "hello"
    ///
    /// // Swap 'l' and 'o'
    /// buffer.swap_characters_before_cursor();
    /// assert_eq!(buffer.text(), "helol world");
    /// assert_eq!(buffer.cursor_position(), 5); // Cursor unchanged
    /// ```
    pub fn swap_characters_before_cursor(&mut self) {
        let current_text = self.text().to_string();
        let cursor_pos = self.cursor_position;

        // Need at least 2 characters before cursor to swap
        if cursor_pos < 2 {
            return;
        }

        // Get the two characters before the cursor
        let char1_pos = cursor_pos - 2;
        let char2_pos = cursor_pos - 1;

        // Extract characters safely using Unicode-aware slicing
        let char1 = unicode::char_at_rune_index(&current_text, char1_pos);
        let char2 = unicode::char_at_rune_index(&current_text, char2_pos);

        if let (Some(ch1), Some(ch2)) = (char1, char2) {
            // Build new text with swapped characters
            let before_swap = unicode::rune_slice(&current_text, 0, char1_pos);
            let after_cursor = unicode::rune_slice(
                &current_text,
                cursor_pos,
                unicode::rune_count(&current_text),
            );

            let new_text = format!("{before_swap}{ch2}{ch1}{after_cursor}");
            self.working_lines[self.working_index] = new_text;
            self.invalidate_cache();
        }
    }

    /// Insert text at the current cursor position.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to insert
    /// * `overwrite` - If true, overwrite existing text; if false, insert text
    /// * `move_cursor` - If true, move cursor after the inserted text
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// // Insert text
    /// buffer.insert_text(" beautiful", false, true);
    /// assert_eq!(buffer.text(), "hello beautiful world");
    /// assert_eq!(buffer.cursor_position(), 15);
    /// ```
    pub fn insert_text(&mut self, text: &str, overwrite: bool, move_cursor: bool) {
        let current_text = self.text().to_string();
        let cursor_pos = self.cursor_position;
        let text_rune_count = unicode::rune_count(&current_text);

        // Ensure cursor is within bounds
        let safe_cursor_pos = cursor_pos.min(text_rune_count);

        let new_text = if overwrite {
            // Overwrite mode: replace characters starting at cursor position
            let before_cursor = unicode::rune_slice(&current_text, 0, safe_cursor_pos);
            let insert_rune_count = unicode::rune_count(text);
            let after_overwrite_pos = (safe_cursor_pos + insert_rune_count).min(text_rune_count);
            let after_cursor =
                unicode::rune_slice(&current_text, after_overwrite_pos, text_rune_count);

            format!("{before_cursor}{text}{after_cursor}")
        } else {
            // Insert mode: insert text at cursor position
            let before_cursor = unicode::rune_slice(&current_text, 0, safe_cursor_pos);
            let after_cursor = unicode::rune_slice(&current_text, safe_cursor_pos, text_rune_count);

            format!("{before_cursor}{text}{after_cursor}")
        };

        self.working_lines[self.working_index] = new_text;

        if move_cursor {
            self.cursor_position = safe_cursor_pos + unicode::rune_count(text);
        } else {
            self.cursor_position = safe_cursor_pos;
        }

        self.invalidate_cache();
    }

    /// Insert text at the current cursor position with validation.
    ///
    /// This version performs validation and returns errors instead of silently
    /// clamping values.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to insert
    /// * `overwrite` - If true, overwrite existing text; if false, insert text
    /// * `move_cursor` - If true, move cursor after the inserted text
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation succeeds, or a `BufferError` if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// assert!(buffer.insert_text_validated(" world", false, true).is_ok());
    /// assert_eq!(buffer.text(), "hello world");
    /// ```
    pub fn insert_text_validated(
        &mut self,
        text: &str,
        overwrite: bool,
        move_cursor: bool,
    ) -> BufferResult<()> {
        use crate::error::validation;

        // Validate text encoding
        validation::validate_text_encoding(text)?;

        // Validate cursor position
        validation::validate_cursor_position(self.cursor_position, self.text())?;

        // Perform the insertion
        self.insert_text(text, overwrite, move_cursor);

        Ok(())
    }

    /// Delete text before the cursor.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of characters to delete
    ///
    /// # Returns
    ///
    /// The deleted text as a string
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// let deleted = buffer.delete_before_cursor(2);
    /// assert_eq!(deleted, "lo");
    /// assert_eq!(buffer.text(), "hel world");
    /// assert_eq!(buffer.cursor_position(), 3);
    /// ```
    pub fn delete_before_cursor(&mut self, count: usize) -> String {
        if count == 0 {
            return String::new();
        }

        let current_text = self.text().to_string();
        let cursor_pos = self.cursor_position;
        let text_rune_count = unicode::rune_count(&current_text);

        // Ensure cursor is within bounds
        let safe_cursor_pos = cursor_pos.min(text_rune_count);

        // Calculate how many characters we can actually delete
        let actual_delete_count = count.min(safe_cursor_pos);

        if actual_delete_count == 0 {
            return String::new();
        }

        let delete_start = safe_cursor_pos - actual_delete_count;

        // Get the text that will be deleted
        let deleted_text =
            unicode::rune_slice(&current_text, delete_start, safe_cursor_pos).to_string();

        // Create new text without the deleted portion
        let before_delete = unicode::rune_slice(&current_text, 0, delete_start);
        let after_cursor = unicode::rune_slice(&current_text, safe_cursor_pos, text_rune_count);
        let new_text = format!("{before_delete}{after_cursor}");

        self.working_lines[self.working_index] = new_text;
        self.cursor_position = delete_start;
        self.invalidate_cache();

        deleted_text
    }

    /// Delete text before the cursor with validation.
    ///
    /// This version performs validation and returns errors for invalid operations.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of characters to delete
    ///
    /// # Returns
    ///
    /// `Ok(deleted_text)` if successful, or a `BufferError` if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello".to_string());
    /// buffer.set_cursor_position(3);
    ///
    /// let result = buffer.delete_before_cursor_validated(2);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), "el");
    /// ```
    pub fn delete_before_cursor_validated(&mut self, count: usize) -> BufferResult<String> {
        use crate::error::validation;

        if count == 0 {
            return Ok(String::new());
        }

        // Validate cursor position
        validation::validate_cursor_position(self.cursor_position, self.text())?;

        // Validate character count
        let available = self.cursor_position;
        validation::validate_character_count(count, available, "delete_before_cursor")?;

        // Perform the deletion
        Ok(self.delete_before_cursor(count))
    }

    /// Delete text after the cursor.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of characters to delete
    ///
    /// # Returns
    ///
    /// The deleted text as a string
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// let deleted = buffer.delete(2);
    /// assert_eq!(deleted, " w");
    /// assert_eq!(buffer.text(), "helloorld");
    /// assert_eq!(buffer.cursor_position(), 5);
    /// ```
    pub fn delete(&mut self, count: usize) -> String {
        if count == 0 {
            return String::new();
        }

        let current_text = self.text().to_string();
        let cursor_pos = self.cursor_position;
        let text_rune_count = unicode::rune_count(&current_text);

        // Ensure cursor is within bounds
        let safe_cursor_pos = cursor_pos.min(text_rune_count);

        // Calculate how many characters we can actually delete
        let remaining_chars = text_rune_count - safe_cursor_pos;
        let actual_delete_count = count.min(remaining_chars);

        if actual_delete_count == 0 {
            return String::new();
        }

        let delete_end = safe_cursor_pos + actual_delete_count;

        // Get the text that will be deleted
        let deleted_text =
            unicode::rune_slice(&current_text, safe_cursor_pos, delete_end).to_string();

        // Create new text without the deleted portion
        let before_cursor = unicode::rune_slice(&current_text, 0, safe_cursor_pos);
        let after_delete = unicode::rune_slice(&current_text, delete_end, text_rune_count);
        let new_text = format!("{before_cursor}{after_delete}");

        self.working_lines[self.working_index] = new_text;
        // Update cursor position to be within bounds of new text
        self.cursor_position = safe_cursor_pos;
        self.invalidate_cache();

        deleted_text
    }

    /// Delete text after the cursor with validation.
    ///
    /// This version performs validation and returns errors for invalid operations.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of characters to delete
    ///
    /// # Returns
    ///
    /// `Ok(deleted_text)` if successful, or a `BufferError` if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello".to_string());
    /// buffer.set_cursor_position(2);
    ///
    /// let result = buffer.delete_validated(2);
    /// assert!(result.is_ok());
    /// assert_eq!(result.unwrap(), "ll");
    /// ```
    pub fn delete_validated(&mut self, count: usize) -> BufferResult<String> {
        use crate::error::validation;

        if count == 0 {
            return Ok(String::new());
        }

        // Validate cursor position
        validation::validate_cursor_position(self.cursor_position, self.text())?;

        // Validate character count
        let text_len = unicode::rune_count(self.text());
        let available = text_len - self.cursor_position;
        validation::validate_character_count(count, available, "delete")?;

        // Perform the deletion
        Ok(self.delete(count))
    }

    /// Set the working index to switch between working lines.
    ///
    /// The index will be clamped to valid bounds within the working lines.
    /// The cursor position will be reset to 0 when switching lines.
    pub fn set_working_index(&mut self, index: usize) -> BufferResult<()> {
        if index >= self.working_lines.len() {
            return Err(BufferError::InvalidWorkingIndex {
                index,
                max: self.working_lines.len().saturating_sub(1),
            });
        }

        self.working_index = index;
        self.cursor_position = 0;
        self.preferred_column = None;
        self.invalidate_cache();
        Ok(())
    }

    /// Add a new working line and optionally switch to it.
    pub fn add_working_line(&mut self, text: String, switch_to: bool) {
        self.working_lines.push(text);
        if switch_to {
            self.working_index = self.working_lines.len() - 1;
            self.cursor_position = 0;
            self.preferred_column = None;
            self.invalidate_cache();
        }
    }

    /// Update the cached document if necessary.
    fn update_cached_document(&mut self) {
        let current_text = self.text().to_string();

        // Check if cache is still valid
        if let Some(ref cached) = self.cached_document {
            if cached.text() == current_text
                && cached.cursor_position() == self.cursor_position
                && cached.last_key_stroke() == self.last_key_stroke
            {
                return; // Cache is valid
            }
        }

        // Create new cached document
        self.cached_document = Some(Document::with_text_and_key(
            current_text,
            self.cursor_position,
            self.last_key_stroke,
        ));
    }

    /// Invalidate the cached document.
    fn invalidate_cache(&mut self) {
        self.cached_document = None;
    }

    /// Move cursor left by the specified number of positions.
    ///
    /// Respects line boundaries and will not move past the start of the current line.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of positions to move left
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// buffer.cursor_left(2);
    /// assert_eq!(buffer.cursor_position(), 3);
    ///
    /// // Cannot move past start of line
    /// buffer.cursor_left(10);
    /// assert_eq!(buffer.cursor_position(), 0);
    /// ```
    pub fn cursor_left(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        let doc = self.document();
        let relative_movement = doc.get_cursor_left_position(count);

        if relative_movement < 0 {
            let new_position = self
                .cursor_position
                .saturating_sub((-relative_movement) as usize);
            self.cursor_position = new_position;
            self.invalidate_cache();
        }
        // If relative_movement is 0, cursor is already at the leftmost valid position
    }

    /// Move cursor right by the specified number of positions.
    ///
    /// Respects line boundaries and will not move past the end of the current line.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of positions to move right
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    /// buffer.set_cursor_position(5);
    ///
    /// buffer.cursor_right(2);
    /// assert_eq!(buffer.cursor_position(), 7);
    ///
    /// // Cannot move past end of line
    /// buffer.cursor_right(10);
    /// assert_eq!(buffer.cursor_position(), 11);
    /// ```
    pub fn cursor_right(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        let doc = self.document();
        let relative_movement = doc.get_cursor_right_position(count);

        if relative_movement > 0 {
            let new_position = self.cursor_position + relative_movement as usize;
            self.cursor_position = new_position;
            self.invalidate_cache();
        }
        // If relative_movement is 0, cursor is already at the rightmost valid position
    }

    /// Move cursor up by the specified number of lines.
    ///
    /// Maintains preferred column for consistent vertical navigation.
    /// If no preferred column is set, uses the current column.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of lines to move up
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("line1\nline2\nline3".to_string());
    /// buffer.set_cursor_position(8); // "li|ne2"
    ///
    /// buffer.cursor_up(1);
    /// assert_eq!(buffer.cursor_position(), 2); // "li|ne1"
    /// ```
    pub fn cursor_up(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        // Set preferred column if not already set
        if self.preferred_column.is_none() {
            let doc = self.document();
            self.preferred_column = Some(doc.cursor_position_col());
        }

        // Get preferred column before borrowing document
        let preferred_column = self.preferred_column;
        let doc = self.document();
        let relative_movement = doc.get_cursor_up_position(count, preferred_column);

        if relative_movement < 0 {
            let new_position = self
                .cursor_position
                .saturating_sub((-relative_movement) as usize);
            self.cursor_position = new_position;
            self.invalidate_cache();
        }
        // If relative_movement is 0, cursor is already at the topmost valid position
    }

    /// Move cursor down by the specified number of lines.
    ///
    /// Maintains preferred column for consistent vertical navigation.
    /// If no preferred column is set, uses the current column.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of lines to move down
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("line1\nline2\nline3".to_string());
    /// buffer.set_cursor_position(2); // "li|ne1"
    ///
    /// buffer.cursor_down(1);
    /// assert_eq!(buffer.cursor_position(), 8); // "li|ne2"
    /// ```
    pub fn cursor_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        // Set preferred column if not already set
        if self.preferred_column.is_none() {
            let doc = self.document();
            self.preferred_column = Some(doc.cursor_position_col());
        }

        // Get preferred column before borrowing document
        let preferred_column = self.preferred_column;
        let doc = self.document();
        let relative_movement = doc.get_cursor_down_position(count, preferred_column);

        if relative_movement > 0 {
            let new_position = self.cursor_position + relative_movement as usize;
            self.cursor_position = new_position;
            self.invalidate_cache();
        }
        // If relative_movement is 0, cursor is already at the bottommost valid position
    }

    /// Set the cursor position with bounds validation and cache invalidation.
    ///
    /// The position will be clamped to valid bounds within the text.
    /// This method also resets the preferred column for vertical movement.
    ///
    /// # Arguments
    ///
    /// * `position` - The new cursor position as a rune index
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello world".to_string());
    ///
    /// buffer.set_cursor_position(5);
    /// assert_eq!(buffer.cursor_position(), 5);
    ///
    /// // Position beyond text length is clamped
    /// buffer.set_cursor_position(100);
    /// assert_eq!(buffer.cursor_position(), 11);
    /// ```
    pub fn set_cursor_position(&mut self, position: usize) {
        use crate::error::validation;

        let _text_len = unicode::rune_count(self.text());
        let new_position = validation::clamp_cursor_position(position, self.text());

        if self.cursor_position != new_position {
            self.cursor_position = new_position;
            self.preferred_column = None; // Reset preferred column when explicitly setting position
            self.invalidate_cache();
        }
    }

    /// Set the cursor position with strict validation (returns error if out of bounds).
    ///
    /// Unlike `set_cursor_position`, this method returns an error if the position
    /// is out of bounds instead of clamping it.
    ///
    /// # Arguments
    ///
    /// * `position` - The new cursor position as a rune index
    ///
    /// # Returns
    ///
    /// `Ok(())` if the position is valid, or a `BufferError` if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello".to_string());
    ///
    /// assert!(buffer.set_cursor_position_strict(3).is_ok());
    /// assert!(buffer.set_cursor_position_strict(10).is_err());
    /// ```
    pub fn set_cursor_position_strict(&mut self, position: usize) -> BufferResult<()> {
        use crate::error::validation;

        validation::validate_cursor_position(position, self.text())?;

        if self.cursor_position != position {
            self.cursor_position = position;
            self.preferred_column = None;
            self.invalidate_cache();
        }

        Ok(())
    }

    /// Ensure cursor position is within valid bounds.
    ///
    /// This is an internal helper method that clamps the cursor position
    /// to valid bounds within the current text.
    fn ensure_cursor_bounds(&mut self) {
        use crate::error::validation;

        let text_len = unicode::rune_count(self.text());
        if self.cursor_position > text_len {
            self.cursor_position =
                validation::clamp_cursor_position(self.cursor_position, self.text());
            self.preferred_column = None; // Reset preferred column when bounds are corrected
        }
    }

    /// Validate the current buffer state.
    ///
    /// This method checks that all internal state is consistent and valid.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the buffer state is valid, or a `BufferError` describing the issue.
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::buffer::Buffer;
    ///
    /// let mut buffer = Buffer::new();
    /// buffer.set_text("hello".to_string());
    /// assert!(buffer.validate_state().is_ok());
    /// ```
    pub fn validate_state(&self) -> BufferResult<()> {
        use crate::error::validation;

        // Validate working index
        validation::validate_working_index(self.working_index, self.working_lines.len())?;

        // Validate cursor position
        validation::validate_cursor_position(self.cursor_position, self.text())?;

        // Validate text encoding for current line
        validation::validate_text_encoding(self.text())?;

        // Validate all working lines
        for (i, line) in self.working_lines.iter().enumerate() {
            validation::validate_text_encoding(line).map_err(|_| {
                BufferError::invalid_text_operation(
                    "validate_working_line",
                    &format!("Invalid text encoding in working line {i}"),
                )
            })?;
        }

        Ok(())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = Buffer::new();
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor_position(), 0);
        assert_eq!(buffer.working_lines.len(), 1);
        assert_eq!(buffer.working_index, 0);
    }

    #[test]
    fn test_set_text() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        assert_eq!(buffer.text(), "hello world");
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_set_cursor_position() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        buffer.set_cursor_position(3);
        assert_eq!(buffer.cursor_position(), 3);

        // Test clamping to bounds
        buffer.set_cursor_position(10);
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_unicode_text() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは".to_string());

        buffer.set_cursor_position(3);
        assert_eq!(buffer.cursor_position(), 3);

        // Test bounds with Unicode
        buffer.set_cursor_position(10);
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_document_caching() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // First access should create cache
        {
            let doc1 = buffer.document();
            assert_eq!(doc1.text(), "hello");
        }

        // Second access should use cache
        {
            let doc2 = buffer.document();
            assert_eq!(doc2.text(), "hello");
        }

        // Changing text should invalidate cache
        buffer.set_text("world".to_string());
        {
            let doc3 = buffer.document();
            assert_eq!(doc3.text(), "world");
        }
    }

    #[test]
    fn test_working_lines_management() {
        let mut buffer = Buffer::new();

        // Initial state
        assert_eq!(buffer.working_lines_count(), 1);
        assert_eq!(buffer.working_index(), 0);
        assert_eq!(buffer.text(), "");

        // Set text on first line
        buffer.set_text("first line".to_string());
        assert_eq!(buffer.text(), "first line");

        // Add second working line
        buffer.add_working_line("second line".to_string(), false);
        assert_eq!(buffer.working_lines_count(), 2);
        assert_eq!(buffer.working_index(), 0); // Still on first line
        assert_eq!(buffer.text(), "first line");

        // Add third line and switch to it
        buffer.add_working_line("third line".to_string(), true);
        assert_eq!(buffer.working_lines_count(), 3);
        assert_eq!(buffer.working_index(), 2);
        assert_eq!(buffer.text(), "third line");
        assert_eq!(buffer.cursor_position(), 0); // Cursor reset when switching
    }

    #[test]
    fn test_set_working_index() {
        let mut buffer = Buffer::new();
        buffer.add_working_line("line 1".to_string(), false);
        buffer.add_working_line("line 2".to_string(), false);

        // Buffer now has: ["", "line 1", "line 2"] at indices [0, 1, 2]
        // Switch to line 1 (index 1)
        assert!(buffer.set_working_index(1).is_ok());
        assert_eq!(buffer.working_index(), 1);
        assert_eq!(buffer.text(), "line 1");
        assert_eq!(buffer.cursor_position(), 0);

        // Switch to line 2 (index 2)
        assert!(buffer.set_working_index(2).is_ok());
        assert_eq!(buffer.working_index(), 2);
        assert_eq!(buffer.text(), "line 2");
        assert_eq!(buffer.cursor_position(), 0);

        // Try invalid index
        let result = buffer.set_working_index(5);
        assert!(result.is_err());
        if let Err(BufferError::InvalidWorkingIndex { index, max }) = result {
            assert_eq!(index, 5);
            assert_eq!(max, 2);
        } else {
            panic!("Expected InvalidWorkingIndex error");
        }
    }

    #[test]
    fn test_error_handling_cursor_position() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // Valid cursor position
        assert!(buffer.set_cursor_position_strict(3).is_ok());
        assert_eq!(buffer.cursor_position(), 3);

        // Invalid cursor position
        let result = buffer.set_cursor_position_strict(10);
        assert!(result.is_err());
        if let Err(BufferError::InvalidCursorPosition { position, max }) = result {
            assert_eq!(position, 10);
            assert_eq!(max, 5);
        } else {
            panic!("Expected InvalidCursorPosition error");
        }

        // Cursor position should remain unchanged after error
        assert_eq!(buffer.cursor_position(), 3);
    }

    #[test]
    fn test_error_handling_text_validation() {
        let mut buffer = Buffer::new();

        // Valid text
        assert!(buffer.set_text_validated("hello world".to_string()).is_ok());
        assert_eq!(buffer.text(), "hello world");

        // Text with null characters should be rejected
        let result = buffer.set_text_validated("hello\0world".to_string());
        assert!(result.is_err());
        if let Err(BufferError::TextEncodingError(_)) = result {
            // Expected
        } else {
            panic!("Expected TextEncodingError");
        }

        // Buffer text should remain unchanged after error
        assert_eq!(buffer.text(), "hello world");
    }

    #[test]
    fn test_error_handling_insert_text() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(3);

        // Valid insertion
        assert!(buffer.insert_text_validated(" world", false, true).is_ok());
        assert_eq!(buffer.text(), "hel worldlo");

        // Reset for next test
        buffer.set_text("hello".to_string());

        // Test with invalid text (null character)
        let result = buffer.insert_text_validated("hello\0world", false, true);
        assert!(result.is_err());
        if let Err(BufferError::TextEncodingError(_)) = result {
            // Expected
        } else {
            panic!("Expected TextEncodingError");
        }

        // Test with cursor position that's invalid - should fail validation
        buffer.cursor_position = 10; // Manually set invalid position
        let result = buffer.insert_text_validated(" world", false, true);
        assert!(result.is_err()); // Should fail because cursor position is invalid
    }

    #[test]
    fn test_error_handling_delete_operations() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Valid deletion before cursor
        let result = buffer.delete_before_cursor_validated(2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "lo");

        // Reset
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Valid deletion after cursor
        let result = buffer.delete_validated(2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " w");

        // Reset
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(2);

        // Try to delete more characters than available before cursor
        let result = buffer.delete_before_cursor_validated(5);
        assert!(result.is_err());
        if let Err(BufferError::InvalidCharacterCount { count, available }) = result {
            assert_eq!(count, 5);
            assert_eq!(available, 2);
        } else {
            panic!("Expected InvalidCharacterCount error");
        }

        // Try to delete more characters than available after cursor
        let result = buffer.delete_validated(10);
        assert!(result.is_err());
        if let Err(BufferError::InvalidCharacterCount { count, available }) = result {
            assert_eq!(count, 10);
            assert_eq!(available, 3);
        } else {
            panic!("Expected InvalidCharacterCount error");
        }
    }

    #[test]
    fn test_buffer_state_validation() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Valid state
        assert!(buffer.validate_state().is_ok());

        // Manually corrupt the state for testing
        buffer.cursor_position = 100; // Invalid cursor position
        let result = buffer.validate_state();
        assert!(result.is_err());

        // Fix the state
        buffer.ensure_cursor_bounds();
        assert!(buffer.validate_state().is_ok());
    }

    #[test]
    fn test_unicode_error_handling() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは".to_string()); // Japanese text

        // Valid operations with Unicode
        buffer.set_cursor_position(3);
        assert!(buffer.validate_state().is_ok());

        let result = buffer.delete_before_cursor_validated(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "に");

        // Insert Unicode text
        assert!(buffer.insert_text_validated("さ", false, true).is_ok());
        assert_eq!(buffer.text(), "こんさちは");
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = Buffer::new();
        buffer.set_text("test".to_string());

        // Test cursor movement with bounds checking
        buffer.set_cursor_position(2);

        // Move cursor beyond bounds should be handled gracefully
        buffer.cursor_right(10);
        assert_eq!(buffer.cursor_position(), 4); // Should be clamped to text end

        buffer.cursor_left(10);
        assert_eq!(buffer.cursor_position(), 0); // Should be clamped to text start
    }

    #[test]
    fn test_error_recovery_scenarios() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());

        // Test that buffer remains in valid state after errors
        buffer.set_cursor_position(5);

        // Attempt invalid operation
        let _ = buffer.set_cursor_position_strict(100);

        // Buffer should still be in valid state
        assert!(buffer.validate_state().is_ok());
        assert_eq!(buffer.cursor_position(), 5); // Should be unchanged

        // Should be able to perform valid operations after error
        buffer.insert_text(" beautiful", false, true);
        assert_eq!(buffer.text(), "hello beautiful world");
    }

    #[test]
    fn test_cursor_position_bounds() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // Valid position
        buffer.set_cursor_position(3);
        assert_eq!(buffer.cursor_position(), 3);

        // Position at end
        buffer.set_cursor_position(5);
        assert_eq!(buffer.cursor_position(), 5);

        // Position beyond end should be clamped
        buffer.set_cursor_position(10);
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_last_key_stroke() {
        let mut buffer = Buffer::new();
        buffer.set_text("test".to_string());

        // Initially no key stroke
        {
            let doc = buffer.document();
            assert_eq!(doc.last_key_stroke(), None);
        }

        // Set key stroke
        use crate::key::Key;
        buffer.set_last_key_stroke(Key::ControlA);
        {
            let doc = buffer.document();
            assert_eq!(doc.last_key_stroke(), Some(Key::ControlA));
        }
    }

    #[test]
    fn test_display_cursor_position() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // For ASCII text, display position should match cursor position
        assert_eq!(buffer.display_cursor_position(), 5);
    }

    #[test]
    fn test_unicode_display_cursor_position() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは".to_string()); // Japanese text with wide characters
        buffer.set_cursor_position(2);

        // Display position should account for wide characters
        let display_pos = buffer.display_cursor_position();
        assert!(display_pos >= 2); // Should be at least the rune position
    }

    #[test]
    fn test_cache_invalidation() {
        let mut buffer = Buffer::new();
        buffer.set_text("initial".to_string());

        // Access document to create cache
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "initial");
        }

        // Changing cursor position should invalidate cache
        buffer.set_cursor_position(3);
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 3);
        }

        // Changing text should invalidate cache
        buffer.set_text("modified".to_string());
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "modified");
        }

        // Setting key stroke should invalidate cache
        use crate::key::Key;
        buffer.set_last_key_stroke(Key::ControlX);
        {
            let doc = buffer.document();
            assert_eq!(doc.last_key_stroke(), Some(Key::ControlX));
        }
    }

    #[test]
    fn test_default_implementation() {
        let buffer = Buffer::default();
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor_position(), 0);
        assert_eq!(buffer.working_index(), 0);
        assert_eq!(buffer.working_lines_count(), 1);
    }

    // Text modification tests

    #[test]
    fn test_insert_text_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Insert text with cursor movement
        buffer.insert_text(" beautiful", false, true);
        assert_eq!(buffer.text(), "hello beautiful world");
        assert_eq!(buffer.cursor_position(), 15);

        // Insert text without cursor movement
        buffer.set_cursor_position(5);
        buffer.insert_text(" amazing", false, false);
        assert_eq!(buffer.text(), "hello amazing beautiful world");
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_insert_text_overwrite_mode() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(6);

        // Overwrite mode
        buffer.insert_text("RUST", true, true);
        assert_eq!(buffer.text(), "hello RUSTd");
        assert_eq!(buffer.cursor_position(), 10);

        // Overwrite at end of text
        buffer.set_cursor_position(11);
        buffer.insert_text("!!!", true, true);
        assert_eq!(buffer.text(), "hello RUSTd!!!");
        assert_eq!(buffer.cursor_position(), 14);
    }

    #[test]
    fn test_insert_text_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは".to_string());
        buffer.set_cursor_position(2);

        // Insert Japanese text
        buffer.insert_text("素晴らしい", false, true);
        assert_eq!(buffer.text(), "こん素晴らしいにちは");
        assert_eq!(buffer.cursor_position(), 7);

        // Insert emoji
        buffer.set_cursor_position(0);
        buffer.insert_text("🦀", false, true);
        assert_eq!(buffer.text(), "🦀こん素晴らしいにちは");
        assert_eq!(buffer.cursor_position(), 1);
    }

    #[test]
    fn test_insert_text_mixed_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("Hello 世界".to_string());
        buffer.set_cursor_position(6);

        // Insert mixed content
        buffer.insert_text("beautiful ", false, true);
        assert_eq!(buffer.text(), "Hello beautiful 世界");
        assert_eq!(buffer.cursor_position(), 16);

        // Insert at various positions
        buffer.set_cursor_position(0);
        buffer.insert_text("🚀 ", false, true);
        assert_eq!(buffer.text(), "🚀 Hello beautiful 世界");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_insert_text_edge_cases() {
        let mut buffer = Buffer::new();

        // Insert into empty buffer
        buffer.insert_text("hello", false, true);
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 5);

        // Insert empty string
        buffer.insert_text("", false, true);
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 5);

        // Insert at out-of-bounds cursor position
        buffer.set_cursor_position(100);
        buffer.insert_text(" world", false, true);
        assert_eq!(buffer.text(), "hello world");
        assert_eq!(buffer.cursor_position(), 11);
    }

    #[test]
    fn test_delete_before_cursor_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Delete 2 characters before cursor
        let deleted = buffer.delete_before_cursor(2);
        assert_eq!(deleted, "lo");
        assert_eq!(buffer.text(), "hel world");
        assert_eq!(buffer.cursor_position(), 3);

        // Delete more characters
        let deleted = buffer.delete_before_cursor(2);
        assert_eq!(deleted, "el");
        assert_eq!(buffer.text(), "h world");
        assert_eq!(buffer.cursor_position(), 1);
    }

    #[test]
    fn test_delete_before_cursor_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは世界".to_string());
        buffer.set_cursor_position(5);

        // Delete Japanese characters
        let deleted = buffer.delete_before_cursor(2);
        assert_eq!(deleted, "ちは");
        assert_eq!(buffer.text(), "こんに世界");
        assert_eq!(buffer.cursor_position(), 3);

        // Delete with emoji
        buffer.set_text("Hello 🦀🚀 World".to_string());
        buffer.set_cursor_position(9);
        let deleted = buffer.delete_before_cursor(3);
        assert_eq!(deleted, "🦀🚀 ");
        assert_eq!(buffer.text(), "Hello World");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_delete_before_cursor_edge_cases() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // Delete from beginning (cursor at 0)
        buffer.set_cursor_position(0);
        let deleted = buffer.delete_before_cursor(5);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 0);

        // Delete more than available
        buffer.set_cursor_position(3);
        let deleted = buffer.delete_before_cursor(10);
        assert_eq!(deleted, "hel");
        assert_eq!(buffer.text(), "lo");
        assert_eq!(buffer.cursor_position(), 0);

        // Delete zero characters
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(3);
        let deleted = buffer.delete_before_cursor(0);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 3);

        // Delete from out-of-bounds cursor
        buffer.set_cursor_position(100);
        let deleted = buffer.delete_before_cursor(2);
        assert_eq!(deleted, "lo");
        assert_eq!(buffer.text(), "hel");
        assert_eq!(buffer.cursor_position(), 3);
    }

    #[test]
    fn test_delete_after_cursor_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Delete 2 characters after cursor
        let deleted = buffer.delete(2);
        assert_eq!(deleted, " w");
        assert_eq!(buffer.text(), "helloorld");
        assert_eq!(buffer.cursor_position(), 5);

        // Delete more characters
        let deleted = buffer.delete(3);
        assert_eq!(deleted, "orl");
        assert_eq!(buffer.text(), "hellod");
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_delete_after_cursor_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは世界".to_string());
        buffer.set_cursor_position(2);

        // Delete Japanese characters
        let deleted = buffer.delete(2);
        assert_eq!(deleted, "にち");
        assert_eq!(buffer.text(), "こんは世界");
        assert_eq!(buffer.cursor_position(), 2);

        // Delete with emoji
        buffer.set_text("Hello 🦀🚀 World".to_string());
        buffer.set_cursor_position(6);
        let deleted = buffer.delete(3);
        assert_eq!(deleted, "🦀🚀 ");
        assert_eq!(buffer.text(), "Hello World");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_delete_after_cursor_edge_cases() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // Delete from end (cursor at end)
        buffer.set_cursor_position(5);
        let deleted = buffer.delete(5);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 5);

        // Delete more than available
        buffer.set_cursor_position(2);
        let deleted = buffer.delete(10);
        assert_eq!(deleted, "llo");
        assert_eq!(buffer.text(), "he");
        assert_eq!(buffer.cursor_position(), 2);

        // Delete zero characters
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(2);
        let deleted = buffer.delete(0);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 2);

        // Delete from out-of-bounds cursor
        buffer.set_cursor_position(100);
        let deleted = buffer.delete(2);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor_position(), 5); // Cursor position clamped to bounds
    }

    #[test]
    fn test_text_modification_cache_invalidation() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());

        // Access document to create cache
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "hello world");
        }

        // Insert text should invalidate cache
        buffer.set_cursor_position(5);
        buffer.insert_text(" beautiful", false, true);
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "hello beautiful world");
            assert_eq!(doc.cursor_position(), 15);
        }

        // Delete before cursor should invalidate cache
        let deleted = buffer.delete_before_cursor(10);
        assert_eq!(deleted, " beautiful");
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "hello world");
            assert_eq!(doc.cursor_position(), 5);
        }

        // Delete after cursor should invalidate cache
        let deleted = buffer.delete(6);
        assert_eq!(deleted, " world");
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "hello");
            assert_eq!(doc.cursor_position(), 5);
        }
    }

    #[test]
    fn test_complex_unicode_editing() {
        let mut buffer = Buffer::new();

        // Start with mixed Unicode content
        buffer.set_text("Hello 世界 🦀 Rust".to_string());
        buffer.set_cursor_position(6);

        // Insert more Unicode
        buffer.insert_text("美しい", false, true);
        assert_eq!(buffer.text(), "Hello 美しい世界 🦀 Rust");
        assert_eq!(buffer.cursor_position(), 9);

        // Reset for next test
        buffer.set_text("Hello 世界 🦀 Rust".to_string());
        buffer.set_cursor_position(6);
        assert_eq!(buffer.text(), "Hello 世界 🦀 Rust");
        assert_eq!(buffer.cursor_position(), 6);

        // Delete emoji
        buffer.set_cursor_position(9);
        let deleted = buffer.delete(2);
        assert_eq!(deleted, "🦀 ");
        assert_eq!(buffer.text(), "Hello 世界 Rust");
        assert_eq!(buffer.cursor_position(), 9);

        // Overwrite with emoji
        buffer.set_cursor_position(7);
        buffer.insert_text("🚀🎉", true, true);
        assert_eq!(buffer.text(), "Hello 世🚀🎉Rust");
        assert_eq!(buffer.cursor_position(), 9);
    }

    // Cursor movement tests

    #[test]
    fn test_cursor_left_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Move left within line
        buffer.cursor_left(2);
        assert_eq!(buffer.cursor_position(), 3);

        // Move to start of line
        buffer.cursor_left(3);
        assert_eq!(buffer.cursor_position(), 0);

        // Cannot move past start
        buffer.cursor_left(5);
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_cursor_left_multiline() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(8); // "li|ne2"

        // Move left within current line
        buffer.cursor_left(1);
        assert_eq!(buffer.cursor_position(), 7); // "l|ine2"

        // Move to start of current line
        buffer.cursor_left(1);
        assert_eq!(buffer.cursor_position(), 6); // "|line2"

        // Cannot move past start of current line (respects line boundaries)
        buffer.cursor_left(5);
        assert_eq!(buffer.cursor_position(), 6); // Still at "|line2"
    }

    #[test]
    fn test_cursor_left_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは世界".to_string());
        buffer.set_cursor_position(5);

        buffer.cursor_left(2);
        assert_eq!(buffer.cursor_position(), 3);

        // Test with emoji
        buffer.set_text("Hello 🦀🚀 World".to_string());
        buffer.set_cursor_position(9); // After emojis
        buffer.cursor_left(3);
        assert_eq!(buffer.cursor_position(), 6); // Before emojis
    }

    #[test]
    fn test_cursor_right_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Move right within line
        buffer.cursor_right(2);
        assert_eq!(buffer.cursor_position(), 7);

        // Move to end of line
        buffer.cursor_right(4);
        assert_eq!(buffer.cursor_position(), 11);

        // Cannot move past end
        buffer.cursor_right(5);
        assert_eq!(buffer.cursor_position(), 11);
    }

    #[test]
    fn test_cursor_right_multiline() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(6); // "|line2"

        // Move right within current line
        buffer.cursor_right(2);
        assert_eq!(buffer.cursor_position(), 8); // "li|ne2"

        // Move to end of current line
        buffer.cursor_right(3);
        assert_eq!(buffer.cursor_position(), 11); // "line2|"

        // Cannot move past end of current line (respects line boundaries)
        buffer.cursor_right(5);
        assert_eq!(buffer.cursor_position(), 11); // Still at "line2|"
    }

    #[test]
    fn test_cursor_right_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは世界".to_string());
        buffer.set_cursor_position(2);

        buffer.cursor_right(2);
        assert_eq!(buffer.cursor_position(), 4);

        // Test with emoji
        buffer.set_text("Hello 🦀🚀 World".to_string());
        buffer.set_cursor_position(6); // Before emojis
        buffer.cursor_right(3);
        assert_eq!(buffer.cursor_position(), 9); // After emojis
    }

    #[test]
    fn test_cursor_up_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(8); // "li|ne2" (line 1, col 2)

        // Move up one line
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 2); // "li|ne1" (line 0, col 2)

        // Cannot move up from first line
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 2); // Still at "li|ne1"
    }

    #[test]
    fn test_cursor_up_preferred_column() {
        let mut buffer = Buffer::new();
        buffer.set_text("short\nlonger line\nshort".to_string());
        buffer.set_cursor_position(9); // "lon|ger line" (line 1, col 3)

        // Move up - should maintain column 3
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 3); // "sho|rt" (line 0, col 3)

        // Move down - should use preferred column
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 9); // "lon|ger line" (line 1, col 3)

        // Move down again - preferred column beyond line length should go to end
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 21); // "short|" (line 2, end) - position 21 not 23
    }

    #[test]
    fn test_cursor_down_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(2); // "li|ne1" (line 0, col 2)

        // Move down one line
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 8); // "li|ne2" (line 1, col 2)

        // Move down another line
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 14); // "li|ne3" (line 2, col 2)

        // Cannot move down from last line
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 14); // Still at "li|ne3"
    }

    #[test]
    fn test_cursor_vertical_movement_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは\n世界テスト\nあいう".to_string());
        buffer.set_cursor_position(7); // "世|界テスト" (line 1, col 1)

        // Move up
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 1); // "こ|んにちは" (line 0, col 1)

        // Move down
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 7); // "世|界テスト" (line 1, col 1)

        // Move down again
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 13); // "あ|いう" (line 2, col 1) - position 13 not 12
    }

    #[test]
    fn test_cursor_movement_zero_count() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        let original_pos = buffer.cursor_position();

        // Zero movement should not change position
        buffer.cursor_left(0);
        assert_eq!(buffer.cursor_position(), original_pos);

        buffer.cursor_right(0);
        assert_eq!(buffer.cursor_position(), original_pos);

        buffer.cursor_up(0);
        assert_eq!(buffer.cursor_position(), original_pos);

        buffer.cursor_down(0);
        assert_eq!(buffer.cursor_position(), original_pos);
    }

    #[test]
    fn test_cursor_movement_single_line() {
        let mut buffer = Buffer::new();
        buffer.set_text("single line text".to_string());
        buffer.set_cursor_position(7);

        // Vertical movement should not change position on single line
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 7);

        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 7);

        // Horizontal movement should work normally
        buffer.cursor_left(3);
        assert_eq!(buffer.cursor_position(), 4);

        buffer.cursor_right(5);
        assert_eq!(buffer.cursor_position(), 9);
    }

    #[test]
    fn test_preferred_column_tracking() {
        let mut buffer = Buffer::new();
        buffer.set_text("short\nvery long line here\nshort".to_string());
        buffer.set_cursor_position(15); // "very long |line here" (line 1, col 10)

        // Initially no preferred column
        assert_eq!(buffer.preferred_column, None);

        // Move up - should set preferred column
        buffer.cursor_up(1);
        assert_eq!(buffer.cursor_position(), 5); // "short|" (line 0, end - col 5)
        assert_eq!(buffer.preferred_column, Some(9)); // Preferred column should be set to 9 (actual column position)

        // Move down - should try to use preferred column
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 15); // "very long |line here" (line 1, col 10)

        // Move down again - preferred column beyond line length
        buffer.cursor_down(1);
        assert_eq!(buffer.cursor_position(), 31); // "short|" (line 2, end)

        // Explicitly setting cursor position should reset preferred column
        buffer.set_cursor_position(10);
        assert_eq!(buffer.preferred_column, None);
    }

    #[test]
    fn test_set_cursor_position_bounds_validation() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());

        // Valid position
        buffer.set_cursor_position(5);
        assert_eq!(buffer.cursor_position(), 5);

        // Position at end
        buffer.set_cursor_position(11);
        assert_eq!(buffer.cursor_position(), 11);

        // Position beyond end should be clamped
        buffer.set_cursor_position(100);
        assert_eq!(buffer.cursor_position(), 11);

        // Setting same position should not invalidate preferred column unnecessarily
        buffer.preferred_column = Some(5);
        buffer.set_cursor_position(11); // Same position
        assert_eq!(buffer.preferred_column, Some(5)); // Should not be reset

        // Setting different position should reset preferred column
        buffer.set_cursor_position(5);
        assert_eq!(buffer.preferred_column, None); // Should be reset
    }

    #[test]
    fn test_cursor_movement_cache_invalidation() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(8);

        // Access document to create cache
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 8);
        }

        // Cursor movement should invalidate cache
        buffer.cursor_left(2);
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 6);
        }

        buffer.cursor_right(1);
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 7);
        }

        buffer.cursor_up(1);
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 1);
        }

        buffer.cursor_down(1);
        {
            let doc = buffer.document();
            assert_eq!(doc.cursor_position(), 7);
        }
    }

    #[test]
    fn test_ensure_cursor_bounds_helper() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());

        // Manually set cursor beyond bounds (simulating internal state corruption)
        buffer.cursor_position = 100;
        buffer.preferred_column = Some(50);

        // ensure_cursor_bounds should fix this
        buffer.ensure_cursor_bounds();
        assert_eq!(buffer.cursor_position(), 5); // Clamped to text length
        assert_eq!(buffer.preferred_column, None); // Reset when bounds corrected

        // Valid cursor position should not be changed
        buffer.cursor_position = 3;
        buffer.preferred_column = Some(10);
        buffer.ensure_cursor_bounds();
        assert_eq!(buffer.cursor_position(), 3); // Unchanged
        assert_eq!(buffer.preferred_column, Some(10)); // Unchanged
    }

    #[test]
    fn test_cursor_movement_empty_buffer() {
        let mut buffer = Buffer::new();
        // Buffer starts empty
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor_position(), 0);

        // All cursor movements should be no-ops on empty buffer
        buffer.cursor_left(5);
        assert_eq!(buffer.cursor_position(), 0);

        buffer.cursor_right(5);
        assert_eq!(buffer.cursor_position(), 0);

        buffer.cursor_up(5);
        assert_eq!(buffer.cursor_position(), 0);

        buffer.cursor_down(5);
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_cursor_movement_complex_scenario() {
        let mut buffer = Buffer::new();
        buffer.set_text("first line\nsecond longer line\nthird".to_string());
        buffer.set_cursor_position(0); // Start at beginning

        // Move right to middle of first line
        buffer.cursor_right(5); // "first| line"
        assert_eq!(buffer.cursor_position(), 5);

        // Move down - should maintain column
        buffer.cursor_down(1); // "secon|d longer line"
        assert_eq!(buffer.cursor_position(), 16);

        // Move right to extend beyond next line length
        buffer.cursor_right(12); // "second longer line|"
        assert_eq!(buffer.cursor_position(), 28);

        // Move down - preferred column should place at end of shorter line
        buffer.cursor_down(1); // "third|"
        assert_eq!(buffer.cursor_position(), 35);

        // Move up - should use preferred column
        buffer.cursor_up(1); // "second longer line|" (back to end)
        assert_eq!(buffer.cursor_position(), 16); // Actual value is 16

        // Move left and then up - should maintain new column
        buffer.cursor_left(5); // "second lon|ger line"
        assert_eq!(buffer.cursor_position(), 11);
        buffer.cursor_up(1); // "first line|" (end of shorter line)
        assert_eq!(buffer.cursor_position(), 5); // Maintains column position
    }

    #[test]
    fn test_combining_characters() {
        let mut buffer = Buffer::new();

        // Text with combining characters (e + combining acute accent)
        buffer.set_text("cafe\u{0301}".to_string()); // café with combining accent
        buffer.set_cursor_position(4);

        // Insert text before combining character
        buffer.insert_text(" au lait", false, true);
        assert_eq!(buffer.text(), "cafe au lait\u{0301}");
        assert_eq!(buffer.cursor_position(), 12);

        // Delete combining character
        let deleted = buffer.delete(1);
        assert_eq!(deleted, "\u{0301}");
        assert_eq!(buffer.text(), "cafe au lait");
        assert_eq!(buffer.cursor_position(), 12);
    }

    #[test]
    fn test_zero_width_characters() {
        let mut buffer = Buffer::new();

        // Text with zero-width space
        buffer.set_text("hello\u{200B}world".to_string());
        buffer.set_cursor_position(5);

        // Delete zero-width character
        let deleted = buffer.delete(1);
        assert_eq!(deleted, "\u{200B}");
        assert_eq!(buffer.text(), "helloworld");
        assert_eq!(buffer.cursor_position(), 5);

        // Insert zero-width character
        buffer.insert_text("\u{200B}", false, true);
        assert_eq!(buffer.text(), "hello\u{200B}world");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_text_modification_bounds_safety() {
        let mut buffer = Buffer::new();
        buffer.set_text("test".to_string());

        // Test various out-of-bounds scenarios
        buffer.set_cursor_position(1000);
        buffer.insert_text("!", false, true);
        assert_eq!(buffer.text(), "test!");
        assert_eq!(buffer.cursor_position(), 5);

        buffer.set_cursor_position(1000);
        let deleted = buffer.delete_before_cursor(2);
        assert_eq!(deleted, "t!");
        assert_eq!(buffer.text(), "tes");
        assert_eq!(buffer.cursor_position(), 3);

        buffer.set_cursor_position(1000);
        let deleted = buffer.delete(5);
        assert_eq!(deleted, "");
        assert_eq!(buffer.text(), "tes");
    }

    // Advanced editing operations tests

    #[test]
    fn test_new_line_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Create new line without copying margin
        buffer.new_line(false);
        assert_eq!(buffer.text(), "hello\n world");
        assert_eq!(buffer.cursor_position(), 6);

        // Create another new line
        buffer.new_line(false);
        assert_eq!(buffer.text(), "hello\n\n world");
        assert_eq!(buffer.cursor_position(), 7);
    }

    #[test]
    fn test_new_line_with_margin_copying() {
        let mut buffer = Buffer::new();
        buffer.set_text("    indented line".to_string());
        buffer.set_cursor_position(17); // End of line

        // Create new line with margin copying
        buffer.new_line(true);
        assert_eq!(buffer.text(), "    indented line\n    ");
        assert_eq!(buffer.cursor_position(), 22);

        // Add text to new line
        buffer.insert_text("more text", false, true);
        assert_eq!(buffer.text(), "    indented line\n    more text");
        assert_eq!(buffer.cursor_position(), 31);
    }

    #[test]
    fn test_new_line_mixed_indentation() {
        let mut buffer = Buffer::new();
        buffer.set_text("  \t  mixed indentation".to_string());
        buffer.set_cursor_position(9); // After "mixe" (position 9 is after the 'e')

        // Create new line with margin copying
        buffer.new_line(true);
        assert_eq!(buffer.text(), "  \t  mixe\n  \t  d indentation");
        assert_eq!(buffer.cursor_position(), 15); // After newline + indentation
    }

    #[test]
    fn test_new_line_no_indentation() {
        let mut buffer = Buffer::new();
        buffer.set_text("no indentation".to_string());
        buffer.set_cursor_position(7);

        // Create new line with margin copying (should not copy anything)
        buffer.new_line(true);
        assert_eq!(buffer.text(), "no inde\nntation");
        assert_eq!(buffer.cursor_position(), 8);
    }

    #[test]
    fn test_new_line_empty_buffer() {
        let mut buffer = Buffer::new();

        // Create new line in empty buffer
        buffer.new_line(false);
        assert_eq!(buffer.text(), "\n");
        assert_eq!(buffer.cursor_position(), 1);

        // Create another new line with margin copying (no margin to copy)
        buffer.new_line(true);
        assert_eq!(buffer.text(), "\n\n");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_new_line_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("    こんにちは世界".to_string());
        buffer.set_cursor_position(7); // After "こんに" (position 7 is after 'に')

        // Create new line with margin copying
        buffer.new_line(true);
        assert_eq!(buffer.text(), "    こんに\n    ちは世界");
        assert_eq!(buffer.cursor_position(), 12); // After newline + 4 spaces
    }

    #[test]
    fn test_join_next_line_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("first line\nsecond line".to_string());
        buffer.set_cursor_position(5); // Middle of first line

        // Join lines with space separator
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "first line second line");
        assert_eq!(buffer.cursor_position(), 5); // Cursor unchanged
    }

    #[test]
    fn test_join_next_line_different_separators() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2".to_string());
        buffer.set_cursor_position(0);

        // Join with custom separator
        buffer.join_next_line(" -> ");
        assert_eq!(buffer.text(), "line1 -> line2");
        assert_eq!(buffer.cursor_position(), 0);

        // Reset for next test
        buffer.set_text("line1\nline2".to_string());
        buffer.set_cursor_position(3);

        // Join with empty separator
        buffer.join_next_line("");
        assert_eq!(buffer.text(), "line1line2");
        assert_eq!(buffer.cursor_position(), 3);
    }

    #[test]
    fn test_join_next_line_multiple_lines() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(2); // In first line

        // Join first and second line
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1 line2\nline3");
        assert_eq!(buffer.cursor_position(), 2);

        // Join the result with third line
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1 line2 line3");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_join_next_line_no_next_line() {
        let mut buffer = Buffer::new();
        buffer.set_text("single line".to_string());
        buffer.set_cursor_position(5);

        // Try to join when there's no next line
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "single line"); // Unchanged
        assert_eq!(buffer.cursor_position(), 5); // Unchanged
    }

    #[test]
    fn test_join_next_line_cursor_after_newline() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\nline2\nline3".to_string());
        buffer.set_cursor_position(6); // Start of second line

        // Join second and third line
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1\nline2 line3");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_join_next_line_empty_lines() {
        let mut buffer = Buffer::new();
        buffer.set_text("line1\n\nline3".to_string());
        buffer.set_cursor_position(3);

        // Join line1 with empty line (empty line gets trimmed)
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1 line3");
        assert_eq!(buffer.cursor_position(), 3);

        // Test with indented empty line
        buffer.set_text("line1\n    \nline3".to_string());
        buffer.set_cursor_position(3);
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1 line3");
        assert_eq!(buffer.cursor_position(), 3);
    }

    #[test]
    fn test_join_next_line_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは\n世界です".to_string());
        buffer.set_cursor_position(2); // After "こん"

        // Join with Unicode separator
        buffer.join_next_line("、");
        assert_eq!(buffer.text(), "こんにちは、世界です");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_swap_characters_before_cursor_basic() {
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5); // After "hello"

        // Swap 'l' and 'o'
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "helol world");
        assert_eq!(buffer.cursor_position(), 5); // Cursor unchanged
    }

    #[test]
    fn test_swap_characters_before_cursor_different_positions() {
        let mut buffer = Buffer::new();
        buffer.set_text("abcdef".to_string());

        // Test swapping at different positions
        buffer.set_cursor_position(2); // After "ab"
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "bacdef");
        assert_eq!(buffer.cursor_position(), 2);

        // Test swapping at end
        buffer.set_cursor_position(6); // After "bacdef"
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "bacdfe");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_swap_characters_before_cursor_insufficient_chars() {
        let mut buffer = Buffer::new();

        // Empty buffer
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor_position(), 0);

        // Single character
        buffer.set_text("a".to_string());
        buffer.set_cursor_position(1);
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "a"); // Unchanged
        assert_eq!(buffer.cursor_position(), 1);

        // Cursor at position 0
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(0);
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "hello"); // Unchanged
        assert_eq!(buffer.cursor_position(), 0);

        // Cursor at position 1 (only one char before)
        buffer.set_cursor_position(1);
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "hello"); // Unchanged
        assert_eq!(buffer.cursor_position(), 1);
    }

    #[test]
    fn test_swap_characters_before_cursor_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("こんにちは".to_string());
        buffer.set_cursor_position(2); // After "こん"

        // Swap Japanese characters
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "んこにちは");
        assert_eq!(buffer.cursor_position(), 2);

        // Test with emoji
        buffer.set_text("🦀🚀hello".to_string());
        buffer.set_cursor_position(2); // After emojis
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "🚀🦀hello");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_swap_characters_before_cursor_mixed_unicode() {
        let mut buffer = Buffer::new();
        buffer.set_text("a🦀hello".to_string());
        buffer.set_cursor_position(2); // After "a🦀"

        // Swap ASCII and emoji
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "🦀ahello");
        assert_eq!(buffer.cursor_position(), 2);

        // Test with combining characters
        buffer.set_text("cafe\u{0301}".to_string()); // café with combining accent
        buffer.set_cursor_position(5); // After "cafe\u{0301}"
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "caf\u{0301}e"); // Swap 'e' and combining accent with 'f'
        assert_eq!(buffer.cursor_position(), 5);
    }

    #[test]
    fn test_advanced_editing_cache_invalidation() {
        let mut buffer = Buffer::new();
        buffer.set_text("    indented line\nsecond line".to_string());
        buffer.set_cursor_position(17);

        // Access document to create cache
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "    indented line\nsecond line");
        }

        // new_line should invalidate cache
        buffer.new_line(true);
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "    indented line\n    \nsecond line");
            assert_eq!(doc.cursor_position(), 22);
        }

        // join_next_line should invalidate cache
        buffer.join_next_line(" ");
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "    indented line\n     second line");
            assert_eq!(doc.cursor_position(), 22);
        }

        // swap_characters_before_cursor should invalidate cache
        buffer.set_cursor_position(6); // After "    in"
        buffer.swap_characters_before_cursor();
        {
            let doc = buffer.document();
            assert_eq!(doc.text(), "    nidented line\n     second line");
            assert_eq!(doc.cursor_position(), 6);
        }
    }

    #[test]
    fn test_advanced_editing_complex_scenario() {
        let mut buffer = Buffer::new();
        buffer.set_text("  def function():\n    pass".to_string());
        buffer.set_cursor_position(17); // End of first line

        // Add new line with indentation
        buffer.new_line(true);
        assert_eq!(buffer.text(), "  def function():\n  \n    pass");
        assert_eq!(buffer.cursor_position(), 20);

        // Add some code
        buffer.insert_text("    return True", false, true);
        assert_eq!(
            buffer.text(),
            "  def function():\n      return True\n    pass"
        );
        assert_eq!(buffer.cursor_position(), 35);

        // Join the return line with pass line
        buffer.join_next_line("; ");
        assert_eq!(buffer.text(), "  def function():\n      return True; pass");
        assert_eq!(buffer.cursor_position(), 35);

        // Fix a typo by swapping characters
        buffer.set_cursor_position(13); // After "functio"
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "  def functoin():\n      return True; pass");
        assert_eq!(buffer.cursor_position(), 13);
    }

    #[test]
    fn test_advanced_editing_edge_cases() {
        let mut buffer = Buffer::new();

        // Test new_line at start of text
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(0);
        buffer.new_line(false);
        assert_eq!(buffer.text(), "\nhello");
        assert_eq!(buffer.cursor_position(), 1);

        // Test join_next_line with cursor at end
        buffer.set_text("line1\nline2".to_string());
        buffer.set_cursor_position(11); // End of text
        buffer.join_next_line(" ");
        assert_eq!(buffer.text(), "line1\nline2"); // No change, no next line

        // Test swap at various edge positions
        buffer.set_text("ab".to_string());
        buffer.set_cursor_position(2);
        buffer.swap_characters_before_cursor();
        assert_eq!(buffer.text(), "ba");
        assert_eq!(buffer.cursor_position(), 2);
    }
}
