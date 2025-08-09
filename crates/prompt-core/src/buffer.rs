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
}