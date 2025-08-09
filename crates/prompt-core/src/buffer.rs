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

    /// Set the cursor position.
    ///
    /// The position will be clamped to valid bounds within the text.
    pub fn set_cursor_position(&mut self, position: usize) {
        let text_len = unicode::rune_count(self.text());
        self.cursor_position = position.min(text_len);
        self.invalidate_cache();
    }

    /// Set the last key stroke for context-aware operations.
    pub fn set_last_key_stroke(&mut self, key: Key) {
        self.last_key_stroke = Some(key);
        self.invalidate_cache();
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

    /// Ensure cursor position is within valid bounds.
    fn ensure_cursor_bounds(&mut self) {
        let text_len = unicode::rune_count(self.text());
        if self.cursor_position > text_len {
            self.cursor_position = text_len;
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
}