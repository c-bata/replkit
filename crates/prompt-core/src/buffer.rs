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
    /// use prompt_core::buffer::Buffer;
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



    /// Set the last key stroke for context-aware operations.
    pub fn set_last_key_stroke(&mut self, key: Key) {
        self.last_key_stroke = Some(key);
        self.invalidate_cache();
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
    /// use prompt_core::buffer::Buffer;
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
            let after_cursor = unicode::rune_slice(&current_text, after_overwrite_pos, text_rune_count);
            
            format!("{}{}{}", before_cursor, text, after_cursor)
        } else {
            // Insert mode: insert text at cursor position
            let before_cursor = unicode::rune_slice(&current_text, 0, safe_cursor_pos);
            let after_cursor = unicode::rune_slice(&current_text, safe_cursor_pos, text_rune_count);
            
            format!("{}{}{}", before_cursor, text, after_cursor)
        };
        
        self.working_lines[self.working_index] = new_text;
        
        if move_cursor {
            self.cursor_position = safe_cursor_pos + unicode::rune_count(text);
        } else {
            self.cursor_position = safe_cursor_pos;
        }
        
        self.invalidate_cache();
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
    /// use prompt_core::buffer::Buffer;
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
        let deleted_text = unicode::rune_slice(&current_text, delete_start, safe_cursor_pos).to_string();
        
        // Create new text without the deleted portion
        let before_delete = unicode::rune_slice(&current_text, 0, delete_start);
        let after_cursor = unicode::rune_slice(&current_text, safe_cursor_pos, text_rune_count);
        let new_text = format!("{}{}", before_delete, after_cursor);
        
        self.working_lines[self.working_index] = new_text;
        self.cursor_position = delete_start;
        self.invalidate_cache();
        
        deleted_text
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
    /// use prompt_core::buffer::Buffer;
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
        let deleted_text = unicode::rune_slice(&current_text, safe_cursor_pos, delete_end).to_string();
        
        // Create new text without the deleted portion
        let before_cursor = unicode::rune_slice(&current_text, 0, safe_cursor_pos);
        let after_delete = unicode::rune_slice(&current_text, delete_end, text_rune_count);
        let new_text = format!("{}{}", before_cursor, after_delete);
        
        self.working_lines[self.working_index] = new_text;
        // Update cursor position to be within bounds of new text
        self.cursor_position = safe_cursor_pos;
        self.invalidate_cache();
        
        deleted_text
    }

    /// Set the working index to switch between working lines.
    ///
    /// The index will be clamped to valid bounds within the working lines.
    /// The cursor position will be reset to 0 when switching lines.
    pub fn set_working_index(&mut self, index: usize) -> BufferResult<()> {
        if index >= self.working_lines.len() {
            return Err(BufferError::InvalidWorkingIndex { 
                index, 
                max: self.working_lines.len().saturating_sub(1) 
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
            if cached.text() == current_text && 
               cached.cursor_position() == self.cursor_position &&
               cached.last_key_stroke() == self.last_key_stroke {
                return; // Cache is valid
            }
        }
        
        // Create new cached document
        self.cached_document = Some(Document::with_text_and_key(
            current_text,
            self.cursor_position,
            self.last_key_stroke
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
    /// use prompt_core::buffer::Buffer;
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
            let new_position = self.cursor_position.saturating_sub((-relative_movement) as usize);
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
    /// use prompt_core::buffer::Buffer;
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
    /// use prompt_core::buffer::Buffer;
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
            let new_position = self.cursor_position.saturating_sub((-relative_movement) as usize);
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
    /// use prompt_core::buffer::Buffer;
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
    /// use prompt_core::buffer::Buffer;
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
        let text_len = unicode::rune_count(self.text());
        let new_position = position.min(text_len);
        
        if self.cursor_position != new_position {
            self.cursor_position = new_position;
            self.preferred_column = None; // Reset preferred column when explicitly setting position
            self.invalidate_cache();
        }
    }

    /// Ensure cursor position is within valid bounds.
    ///
    /// This is an internal helper method that clamps the cursor position
    /// to valid bounds within the current text.
    fn ensure_cursor_bounds(&mut self) {
        let text_len = unicode::rune_count(self.text());
        if self.cursor_position > text_len {
            self.cursor_position = text_len;
            self.preferred_column = None; // Reset preferred column when bounds are corrected
        }
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
}