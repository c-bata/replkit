//! Document structure for immutable text analysis and cursor calculations.
//!
//! The Document structure represents immutable text content with cursor position
//! and provides comprehensive text analysis methods. It's designed to be cached
//! and shared safely across operations.

use crate::key::Key;
use crate::unicode;

/// An immutable document representing text content with cursor position.
///
/// Document provides text analysis and cursor calculation methods without
/// modifying the underlying text. This immutability allows for safe caching
/// and sharing of Document instances.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The text content
    text: String,
    /// Cursor position as rune index (not byte index)
    cursor_position: usize,
    /// Last key stroke for context-aware operations
    last_key: Option<Key>,
}

impl Document {
    /// Create a new empty document.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::new();
    /// assert_eq!(doc.text(), "");
    /// assert_eq!(doc.cursor_position(), 0);
    /// ```
    pub fn new() -> Self {
        Document {
            text: String::new(),
            cursor_position: 0,
            last_key: None,
        }
    }

    /// Create a document with specified text and cursor position.
    ///
    /// The cursor position will be clamped to valid bounds within the text.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 5);
    /// assert_eq!(doc.text(), "hello world");
    /// assert_eq!(doc.cursor_position(), 5);
    /// ```
    pub fn with_text(text: String, cursor_position: usize) -> Self {
        let text_len = unicode::rune_count(&text);
        let cursor_position = cursor_position.min(text_len);
        
        Document {
            text,
            cursor_position,
            last_key: None,
        }
    }

    /// Create a document with text, cursor position, and last key stroke.
    pub fn with_text_and_key(text: String, cursor_position: usize, last_key: Option<Key>) -> Self {
        let text_len = unicode::rune_count(&text);
        let cursor_position = cursor_position.min(text_len);
        
        Document {
            text,
            cursor_position,
            last_key,
        }
    }

    /// Get the text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the cursor position as a rune index.
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Get the last key stroke.
    pub fn last_key_stroke(&self) -> Option<Key> {
        self.last_key
    }

    /// Get the display cursor position accounting for Unicode character widths.
    ///
    /// This is important for terminal display where some characters (like CJK)
    /// take up multiple columns.
    pub fn display_cursor_position(&self) -> usize {
        let text_before_cursor = self.text_before_cursor();
        unicode::display_width(text_before_cursor)
    }

    /// Get the text before the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 5);
    /// assert_eq!(doc.text_before_cursor(), "hello");
    /// ```
    pub fn text_before_cursor(&self) -> &str {
        unicode::rune_slice(&self.text, 0, self.cursor_position)
    }

    /// Get the text after the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 5);
    /// assert_eq!(doc.text_after_cursor(), " world");
    /// ```
    pub fn text_after_cursor(&self) -> &str {
        let text_len = unicode::rune_count(&self.text);
        unicode::rune_slice(&self.text, self.cursor_position, text_len)
    }

    /// Get a character relative to the cursor position.
    ///
    /// Returns `None` if the position is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `offset` - Relative offset from cursor (negative for before, positive for after)
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello".to_string(), 2);
    /// assert_eq!(doc.get_char_relative_to_cursor(-1), Some('e'));
    /// assert_eq!(doc.get_char_relative_to_cursor(0), Some('l'));
    /// assert_eq!(doc.get_char_relative_to_cursor(1), Some('l'));
    /// ```
    pub fn get_char_relative_to_cursor(&self, offset: i32) -> Option<char> {
        let target_pos = if offset < 0 {
            self.cursor_position.checked_sub((-offset) as usize)?
        } else {
            self.cursor_position + offset as usize
        };
        
        unicode::char_at_rune_index(&self.text, target_pos)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document() {
        let doc = Document::new();
        assert_eq!(doc.text(), "");
        assert_eq!(doc.cursor_position(), 0);
        assert_eq!(doc.last_key_stroke(), None);
    }

    #[test]
    fn test_with_text() {
        let doc = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.cursor_position(), 5);
    }

    #[test]
    fn test_cursor_position_clamping() {
        let doc = Document::with_text("hello".to_string(), 10);
        assert_eq!(doc.cursor_position(), 5); // Clamped to text length
    }

    #[test]
    fn test_text_before_after_cursor() {
        let doc = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc.text_before_cursor(), "hello");
        assert_eq!(doc.text_after_cursor(), " world");
    }

    #[test]
    fn test_display_cursor_position() {
        let doc = Document::with_text("hello".to_string(), 3);
        assert_eq!(doc.display_cursor_position(), 3);
        
        // Test with wide characters
        let doc_cjk = Document::with_text("こんにちは".to_string(), 2);
        assert_eq!(doc_cjk.display_cursor_position(), 4); // Each char is 2 columns
    }

    #[test]
    fn test_get_char_relative_to_cursor() {
        let doc = Document::with_text("hello".to_string(), 2);
        
        assert_eq!(doc.get_char_relative_to_cursor(-2), Some('h'));
        assert_eq!(doc.get_char_relative_to_cursor(-1), Some('e'));
        assert_eq!(doc.get_char_relative_to_cursor(0), Some('l'));
        assert_eq!(doc.get_char_relative_to_cursor(1), Some('l'));
        assert_eq!(doc.get_char_relative_to_cursor(2), Some('o'));
        assert_eq!(doc.get_char_relative_to_cursor(3), None);
        assert_eq!(doc.get_char_relative_to_cursor(-3), None);
    }

    #[test]
    fn test_unicode_handling() {
        let doc = Document::with_text("こんにちは".to_string(), 3);
        assert_eq!(doc.text_before_cursor(), "こんに");
        assert_eq!(doc.text_after_cursor(), "ちは");
        assert_eq!(doc.get_char_relative_to_cursor(0), Some('ち'));
        assert_eq!(doc.get_char_relative_to_cursor(-1), Some('に'));
    }

    #[test]
    fn test_with_text_and_key() {
        let key = Key::ControlA;
        let doc = Document::with_text_and_key("hello".to_string(), 2, Some(key));
        assert_eq!(doc.text(), "hello");
        assert_eq!(doc.cursor_position(), 2);
        assert_eq!(doc.last_key_stroke(), Some(key));
    }
}