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

    /// Get the word before the cursor.
    ///
    /// A word is defined as a sequence of non-whitespace characters.
    /// This method returns the word immediately before the cursor position.
    /// If the cursor is in the middle of a word, it returns the part of the word before the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 11);
    /// assert_eq!(doc.get_word_before_cursor(), "world");
    ///
    /// let doc2 = Document::with_text("hello world".to_string(), 5);
    /// assert_eq!(doc2.get_word_before_cursor(), "hello");
    /// ```
    pub fn get_word_before_cursor(&self) -> &str {
        self.extract_current_word_before_cursor(false, None)
    }

    /// Get the word after the cursor.
    ///
    /// A word is defined as a sequence of non-whitespace characters.
    /// This method returns the word immediately after the cursor position.
    /// If the cursor is in the middle of a word, it returns the part of the word after the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 0);
    /// assert_eq!(doc.get_word_after_cursor(), "hello");
    ///
    /// let doc2 = Document::with_text("hello world".to_string(), 6);
    /// assert_eq!(doc2.get_word_after_cursor(), "world");
    /// ```
    pub fn get_word_after_cursor(&self) -> &str {
        self.extract_current_word_after_cursor(false, None)
    }

    /// Get the word before the cursor, including trailing whitespace.
    ///
    /// This variant includes any whitespace that follows the word.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello  world".to_string(), 7);
    /// assert_eq!(doc.get_word_before_cursor_with_space(), "hello  ");
    /// ```
    pub fn get_word_before_cursor_with_space(&self) -> &str {
        self.extract_current_word_before_cursor(true, None)
    }

    /// Get the word after the cursor, including leading whitespace.
    ///
    /// This variant includes any whitespace that precedes the word.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello  world".to_string(), 5);
    /// assert_eq!(doc.get_word_after_cursor_with_space(), "  world");
    /// ```
    pub fn get_word_after_cursor_with_space(&self) -> &str {
        self.extract_current_word_after_cursor(true, None)
    }

    /// Get the word before the cursor using custom separators.
    ///
    /// Words are separated by any character in the separator string.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello.world/test".to_string(), 16);
    /// assert_eq!(doc.get_word_before_cursor_until_separator("./"), "test");
    /// ```
    pub fn get_word_before_cursor_until_separator(&self, separators: &str) -> &str {
        self.extract_current_word_before_cursor(false, Some(separators))
    }

    /// Get the word after the cursor using custom separators.
    ///
    /// Words are separated by any character in the separator string.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello.world/test".to_string(), 0);
    /// assert_eq!(doc.get_word_after_cursor_until_separator("./"), "hello");
    /// ```
    pub fn get_word_after_cursor_until_separator(&self, separators: &str) -> &str {
        self.extract_current_word_after_cursor(false, Some(separators))
    }

    /// Get the word before the cursor using custom separators, including trailing separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello..world".to_string(), 7);
    /// assert_eq!(doc.get_word_before_cursor_until_separator_with_space("./"), "hello..");
    /// ```
    pub fn get_word_before_cursor_until_separator_with_space(&self, separators: &str) -> &str {
        self.extract_current_word_before_cursor(true, Some(separators))
    }

    /// Get the word after the cursor using custom separators, including leading separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello..world".to_string(), 5);
    /// assert_eq!(doc.get_word_after_cursor_until_separator_with_space("./"), "..world");
    /// ```
    pub fn get_word_after_cursor_until_separator_with_space(&self, separators: &str) -> &str {
        self.extract_current_word_after_cursor(true, Some(separators))
    }

    /// Find the start position of the previous word relative to cursor.
    ///
    /// Returns the number of characters to move left to reach the start of the previous word.
    /// A word is defined as a sequence of non-whitespace characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 11);
    /// assert_eq!(doc.find_start_of_previous_word(), 5); // Move 5 chars left to "world"
    /// ```
    pub fn find_start_of_previous_word(&self) -> usize {
        self.find_word_boundary_before(false, None)
    }

    /// Find the end position of the current word relative to cursor.
    ///
    /// Returns the number of characters to move right to reach the end of the current word.
    /// A word is defined as a sequence of non-whitespace characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello world".to_string(), 0);
    /// assert_eq!(doc.find_end_of_current_word(), 5); // Move 5 chars right to end of "hello"
    /// ```
    pub fn find_end_of_current_word(&self) -> usize {
        self.find_word_boundary_after(false, None)
    }

    /// Find the start position of the previous word, including whitespace.
    ///
    /// This variant includes whitespace when determining word boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello  world".to_string(), 12);
    /// assert_eq!(doc.find_start_of_previous_word_with_space(), 7); // Include spaces
    /// ```
    pub fn find_start_of_previous_word_with_space(&self) -> usize {
        self.find_word_boundary_before(true, None)
    }

    /// Find the end position of the current word, including whitespace.
    ///
    /// This variant includes whitespace when determining word boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello  world".to_string(), 0);
    /// assert_eq!(doc.find_end_of_current_word_with_space(), 7); // Include spaces
    /// ```
    pub fn find_end_of_current_word_with_space(&self) -> usize {
        self.find_word_boundary_after(true, None)
    }

    /// Find the start position of the previous word using custom separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello.world/test".to_string(), 16);
    /// assert_eq!(doc.find_start_of_previous_word_until_separator("./"), 4); // Move to "test"
    /// ```
    pub fn find_start_of_previous_word_until_separator(&self, separators: &str) -> usize {
        self.find_word_boundary_before(false, Some(separators))
    }

    /// Find the end position of the current word using custom separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello.world/test".to_string(), 0);
    /// assert_eq!(doc.find_end_of_current_word_until_separator("./"), 5); // Move to end of "hello"
    /// ```
    pub fn find_end_of_current_word_until_separator(&self, separators: &str) -> usize {
        self.find_word_boundary_after(false, Some(separators))
    }

    /// Find the start position of the previous word using custom separators, including separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello..world".to_string(), 12);
    /// assert_eq!(doc.find_start_of_previous_word_until_separator_with_space("./"), 7); // Include separators
    /// ```
    pub fn find_start_of_previous_word_until_separator_with_space(&self, separators: &str) -> usize {
        self.find_word_boundary_before(true, Some(separators))
    }

    /// Find the end position of the current word using custom separators, including separators.
    ///
    /// # Arguments
    ///
    /// * `separators` - String containing characters to treat as word separators
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("hello..world".to_string(), 0);
    /// assert_eq!(doc.find_end_of_current_word_until_separator_with_space("./"), 7); // Include separators
    /// ```
    pub fn find_end_of_current_word_until_separator_with_space(&self, separators: &str) -> usize {
        self.find_word_boundary_after(true, Some(separators))
    }

    // Helper methods for word operations

    /// Extract the current word before the cursor position.
    /// This handles the case where the cursor is in the middle of a word.
    /// If the cursor is in the middle of a word, it includes the character at the cursor position.
    fn extract_current_word_before_cursor(&self, include_space: bool, separators: Option<&str>) -> &str {
        if self.text.is_empty() || self.cursor_position == 0 {
            return "";
        }

        let chars: Vec<char> = self.text.chars().collect();
        let is_separator = |c: char| {
            if let Some(seps) = separators {
                seps.chars().any(|sep| sep == c)
            } else {
                c.is_whitespace()
            }
        };

        let mut start;
        let end;

        if include_space {
            // For space variants, we want to include the previous word plus any trailing separators
            // First, skip any separators immediately before cursor
            let mut temp_pos = self.cursor_position;
            while temp_pos > 0 && is_separator(chars[temp_pos - 1]) {
                temp_pos -= 1;
            }
            
            // Now find the start of the word before those separators
            while temp_pos > 0 && !is_separator(chars[temp_pos - 1]) {
                temp_pos -= 1;
            }
            
            start = temp_pos;
            // End is at cursor position to include the trailing separators
            end = self.cursor_position;
        } else {
            // For normal variants, check if cursor is at the start of a word
            if self.cursor_position > 0 && is_separator(chars[self.cursor_position - 1]) {
                // Cursor is at the start of a word (after a separator), return empty
                return "";
            }
            
            // Find the start of the current word
            start = {
                let mut pos = self.cursor_position;
                while pos > 0 && !is_separator(chars[pos - 1]) {
                    pos -= 1;
                }
                pos
            };

            // For custom separators, we might need to skip leading whitespace
            // if whitespace is not explicitly included as a separator
            if separators.is_some() {
                while start < self.cursor_position && chars[start].is_whitespace() {
                    start += 1;
                }
            }

            // If we're in the middle of a word (cursor is not at a separator), 
            // include the character at cursor position
            end = if self.cursor_position < chars.len() && !is_separator(chars[self.cursor_position]) {
                self.cursor_position + 1
            } else {
                self.cursor_position
            };
        }

        if start >= end {
            return "";
        }

        unicode::rune_slice(&self.text, start, end)
    }

    /// Extract the current word after the cursor position.
    /// This handles the case where the cursor is in the middle of a word.
    /// If the cursor is in the middle of a word, it starts from the cursor position.
    fn extract_current_word_after_cursor(&self, include_space: bool, separators: Option<&str>) -> &str {
        if self.text.is_empty() {
            return "";
        }

        let chars: Vec<char> = self.text.chars().collect();
        let is_separator = |c: char| {
            if let Some(seps) = separators {
                seps.chars().any(|sep| sep == c)
            } else {
                c.is_whitespace()
            }
        };

        let start;
        let end;

        if include_space {
            // For space variants, we want to include any leading separators plus the next word
            start = self.cursor_position;
            
            // Find the end by first skipping any leading separators, then finding the end of the word
            let mut temp_pos = self.cursor_position;
            while temp_pos < chars.len() && is_separator(chars[temp_pos]) {
                temp_pos += 1;
            }
            
            // Now find the end of the word after those separators
            while temp_pos < chars.len() && !is_separator(chars[temp_pos]) {
                temp_pos += 1;
            }
            
            end = temp_pos;
        } else {
            // For normal variants, handle both separators and non-separators
            if self.cursor_position < chars.len() && is_separator(chars[self.cursor_position]) {
                // If cursor is at a separator, return empty for non-space variant
                return "";
            } else {
                // If cursor is in a word, return from cursor to end of word
                start = self.cursor_position;
                end = {
                    let mut pos = start;
                    while pos < chars.len() && !is_separator(chars[pos]) {
                        pos += 1;
                    }
                    pos
                };
            }
        }

        if start >= end {
            return "";
        }

        unicode::rune_slice(&self.text, start, end)
    }



    /// Find word boundary before cursor position.
    fn find_word_boundary_before(&self, include_space: bool, separators: Option<&str>) -> usize {
        let text_before = self.text_before_cursor();
        if text_before.is_empty() {
            return 0;
        }

        let chars: Vec<char> = text_before.chars().collect();
        let mut pos = chars.len();

        if let Some(seps) = separators {
            let separator_chars: Vec<char> = seps.chars().collect();
            
            // Skip trailing separators/whitespace if not including them
            if !include_space {
                while pos > 0 && separator_chars.contains(&chars[pos - 1]) {
                    pos -= 1;
                }
                
                // If we've skipped all characters and found no word, return 0
                if pos == 0 {
                    return 0;
                }
            }

            // Find start of current word
            let _word_end = pos;
            while pos > 0 && !separator_chars.contains(&chars[pos - 1]) {
                pos -= 1;
            }

            // If including separators, go back to include them
            if include_space {
                while pos > 0 && separator_chars.contains(&chars[pos - 1]) {
                    pos -= 1;
                }
            }

            chars.len() - pos
        } else {
            // Default whitespace-based word boundaries
            
            // Skip trailing whitespace if not including it
            if !include_space {
                while pos > 0 && chars[pos - 1].is_whitespace() {
                    pos -= 1;
                }
                
                // If we've skipped all characters and found no word, return 0
                if pos == 0 {
                    return 0;
                }
            }

            // Find start of current word
            let _word_end = pos;
            while pos > 0 && !chars[pos - 1].is_whitespace() {
                pos -= 1;
            }

            // If including space, go back to include whitespace
            if include_space {
                while pos > 0 && chars[pos - 1].is_whitespace() {
                    pos -= 1;
                }
            }

            chars.len() - pos
        }
    }

    /// Find word boundary after cursor position.
    fn find_word_boundary_after(&self, include_space: bool, separators: Option<&str>) -> usize {
        let text_after = self.text_after_cursor();
        if text_after.is_empty() {
            return 0;
        }

        let chars: Vec<char> = text_after.chars().collect();
        let mut pos = 0;

        if let Some(seps) = separators {
            let separator_chars: Vec<char> = seps.chars().collect();
            
            // Skip leading separators if not including them
            if !include_space {
                while pos < chars.len() && separator_chars.contains(&chars[pos]) {
                    pos += 1;
                }
                
                // If we've skipped all characters and found no word, return 0
                if pos >= chars.len() {
                    return 0;
                }
            }

            // Find end of current word
            let _word_start = pos;
            while pos < chars.len() && !separator_chars.contains(&chars[pos]) {
                pos += 1;
            }

            // If including separators, continue to include them
            if include_space {
                while pos < chars.len() && separator_chars.contains(&chars[pos]) {
                    pos += 1;
                }
            }

            pos
        } else {
            // Default whitespace-based word boundaries
            
            // Skip leading whitespace if not including it
            if !include_space {
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
                
                // If we've skipped all characters and found no word, return 0
                if pos >= chars.len() {
                    return 0;
                }
            }

            // Find end of current word
            let _word_start = pos;
            while pos < chars.len() && !chars[pos].is_whitespace() {
                pos += 1;
            }

            // If including space, continue to include whitespace
            if include_space {
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
            }

            pos
        }
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



    // Word operation tests
    
    #[test]
    fn test_get_word_before_cursor() {
        // Basic word extraction
        let doc = Document::with_text("hello world".to_string(), 11);
        assert_eq!(doc.get_word_before_cursor(), "world");
        
        let doc2 = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc2.get_word_before_cursor(), "hello");
        
        // Cursor in middle of word
        let doc3 = Document::with_text("hello world".to_string(), 8);
        assert_eq!(doc3.get_word_before_cursor(), "wor");
        
        // Multiple spaces
        let doc4 = Document::with_text("hello   world".to_string(), 13);
        assert_eq!(doc4.get_word_before_cursor(), "world");
        
        // Empty cases
        let doc5 = Document::with_text("".to_string(), 0);
        assert_eq!(doc5.get_word_before_cursor(), "");
        
        let doc6 = Document::with_text("hello".to_string(), 0);
        assert_eq!(doc6.get_word_before_cursor(), "");
        
        // Only whitespace
        let doc7 = Document::with_text("   ".to_string(), 3);
        assert_eq!(doc7.get_word_before_cursor(), "");
    }

    #[test]
    fn test_get_word_after_cursor() {
        // Basic word extraction
        let doc = Document::with_text("hello world".to_string(), 0);
        assert_eq!(doc.get_word_after_cursor(), "hello");
        
        let doc2 = Document::with_text("hello world".to_string(), 6);
        assert_eq!(doc2.get_word_after_cursor(), "world");
        
        // Cursor in middle of word
        let doc3 = Document::with_text("hello world".to_string(), 2);
        assert_eq!(doc3.get_word_after_cursor(), "llo");
        
        // Multiple spaces
        let doc4 = Document::with_text("hello   world".to_string(), 0);
        assert_eq!(doc4.get_word_after_cursor(), "hello");
        
        // Empty cases
        let doc5 = Document::with_text("".to_string(), 0);
        assert_eq!(doc5.get_word_after_cursor(), "");
        
        let doc6 = Document::with_text("hello".to_string(), 5);
        assert_eq!(doc6.get_word_after_cursor(), "");
        
        // Only whitespace
        let doc7 = Document::with_text("   ".to_string(), 0);
        assert_eq!(doc7.get_word_after_cursor(), "");
    }

    #[test]
    fn test_get_word_with_space_variants() {
        // Word before cursor with space
        let doc = Document::with_text("hello  world".to_string(), 7);
        assert_eq!(doc.get_word_before_cursor_with_space(), "hello  ");
        
        let doc2 = Document::with_text("hello  world".to_string(), 12);
        assert_eq!(doc2.get_word_before_cursor_with_space(), "world");
        
        // Word after cursor with space
        let doc3 = Document::with_text("hello  world".to_string(), 5);
        assert_eq!(doc3.get_word_after_cursor_with_space(), "  world");
        
        let doc4 = Document::with_text("hello  world".to_string(), 0);
        assert_eq!(doc4.get_word_after_cursor_with_space(), "hello");
        
        // Multiple spaces
        let doc5 = Document::with_text("hello   world   test".to_string(), 8);
        assert_eq!(doc5.get_word_before_cursor_with_space(), "hello   ");
        
        let doc6 = Document::with_text("hello   world   test".to_string(), 5);
        assert_eq!(doc6.get_word_after_cursor_with_space(), "   world");
    }

    #[test]
    fn test_word_operations_with_custom_separators() {
        // Basic separator usage
        let doc = Document::with_text("hello.world/test".to_string(), 16);
        assert_eq!(doc.get_word_before_cursor_until_separator("./"), "test");
        
        let doc2 = Document::with_text("hello.world/test".to_string(), 0);
        assert_eq!(doc2.get_word_after_cursor_until_separator("./"), "hello");
        
        // Multiple separators
        let doc3 = Document::with_text("hello..world".to_string(), 12);
        assert_eq!(doc3.get_word_before_cursor_until_separator("."), "world");
        
        let doc4 = Document::with_text("hello..world".to_string(), 5);
        assert_eq!(doc4.get_word_after_cursor_until_separator("."), "");
        
        // With space variants
        let doc5 = Document::with_text("hello..world".to_string(), 7);
        assert_eq!(doc5.get_word_before_cursor_until_separator_with_space("."), "hello..");
        
        let doc6 = Document::with_text("hello..world".to_string(), 5);
        assert_eq!(doc6.get_word_after_cursor_until_separator_with_space("."), "..world");
        
        // Complex separators
        let doc7 = Document::with_text("path/to\\file:name".to_string(), 17);
        assert_eq!(doc7.get_word_before_cursor_until_separator("/\\:"), "name");
        
        let doc8 = Document::with_text("path/to\\file:name".to_string(), 0);
        assert_eq!(doc8.get_word_after_cursor_until_separator("/\\:"), "path");
    }

    #[test]
    fn test_find_word_boundaries() {
        // Find start of previous word
        let doc = Document::with_text("hello world test".to_string(), 16);
        assert_eq!(doc.find_start_of_previous_word(), 4); // "test" is 4 chars
        
        let doc2 = Document::with_text("hello world test".to_string(), 11);
        assert_eq!(doc2.find_start_of_previous_word(), 5); // "world" is 5 chars
        
        // Find end of current word
        let doc3 = Document::with_text("hello world test".to_string(), 0);
        assert_eq!(doc3.find_end_of_current_word(), 5); // "hello" is 5 chars
        
        let doc4 = Document::with_text("hello world test".to_string(), 6);
        assert_eq!(doc4.find_end_of_current_word(), 5); // "world" is 5 chars
        
        // With spaces
        let doc5 = Document::with_text("hello  world  test".to_string(), 18);
        assert_eq!(doc5.find_start_of_previous_word_with_space(), 6); // "  test" is 6 chars
        
        let doc6 = Document::with_text("hello  world  test".to_string(), 0);
        assert_eq!(doc6.find_end_of_current_word_with_space(), 7); // "hello  " is 7 chars
        
        // Edge cases
        let doc7 = Document::with_text("".to_string(), 0);
        assert_eq!(doc7.find_start_of_previous_word(), 0);
        assert_eq!(doc7.find_end_of_current_word(), 0);
        
        let doc8 = Document::with_text("   ".to_string(), 3);
        assert_eq!(doc8.find_start_of_previous_word(), 0);
        
        let doc9 = Document::with_text("   ".to_string(), 0);
        assert_eq!(doc9.find_end_of_current_word(), 0);
    }

    #[test]
    fn test_find_word_boundaries_with_separators() {
        // Custom separators
        let doc = Document::with_text("hello.world/test".to_string(), 16);
        assert_eq!(doc.find_start_of_previous_word_until_separator("./"), 4); // "test"
        
        let doc2 = Document::with_text("hello.world/test".to_string(), 0);
        assert_eq!(doc2.find_end_of_current_word_until_separator("./"), 5); // "hello"
        
        // With separator spaces
        let doc3 = Document::with_text("hello..world".to_string(), 12);
        assert_eq!(doc3.find_start_of_previous_word_until_separator_with_space("."), 7); // "..world"
        
        let doc4 = Document::with_text("hello..world".to_string(), 0);
        assert_eq!(doc4.find_end_of_current_word_until_separator_with_space("."), 7); // "hello.."
        
        // Multiple separator types
        let doc5 = Document::with_text("a/b\\c:d".to_string(), 7);
        assert_eq!(doc5.find_start_of_previous_word_until_separator("/\\:"), 1); // "d"
        
        let doc6 = Document::with_text("a/b\\c:d".to_string(), 0);
        assert_eq!(doc6.find_end_of_current_word_until_separator("/\\:"), 1); // "a"
    }

    #[test]
    fn test_word_operations_with_unicode() {
        // Japanese text
        let doc = Document::with_text("こんにちは 世界".to_string(), 8);
        assert_eq!(doc.get_word_before_cursor(), "世界");
        assert_eq!(doc.find_start_of_previous_word(), 2); // "世界" is 2 chars
        
        let doc2 = Document::with_text("こんにちは 世界".to_string(), 0);
        assert_eq!(doc2.get_word_after_cursor(), "こんにちは");
        assert_eq!(doc2.find_end_of_current_word(), 5); // "こんにちは" is 5 chars
        
        // Mixed Unicode and ASCII
        let doc3 = Document::with_text("hello 世界 test".to_string(), 15);
        assert_eq!(doc3.get_word_before_cursor(), "test");
        
        let doc4 = Document::with_text("hello 世界 test".to_string(), 6);
        assert_eq!(doc4.get_word_after_cursor(), "世界");
        
        // Emoji
        let doc5 = Document::with_text("hello 🦀 world".to_string(), 14);
        assert_eq!(doc5.get_word_before_cursor(), "world");
        
        let doc6 = Document::with_text("hello 🦀 world".to_string(), 6);
        assert_eq!(doc6.get_word_after_cursor(), "🦀");
        
        // Custom separators with Unicode
        let doc7 = Document::with_text("こんにちは。世界".to_string(), 8);
        assert_eq!(doc7.get_word_before_cursor_until_separator("。"), "世界");
        
        let doc8 = Document::with_text("こんにちは。世界".to_string(), 0);
        assert_eq!(doc8.get_word_after_cursor_until_separator("。"), "こんにちは");
    }

    #[test]
    fn test_word_operations_edge_cases() {
        // Single character words
        let doc = Document::with_text("a b c".to_string(), 5);
        assert_eq!(doc.get_word_before_cursor(), "c");
        assert_eq!(doc.find_start_of_previous_word(), 1);
        
        let doc2 = Document::with_text("a b c".to_string(), 0);
        assert_eq!(doc2.get_word_after_cursor(), "a");
        assert_eq!(doc2.find_end_of_current_word(), 1);
        
        // Cursor at word boundaries
        let doc3 = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc3.get_word_before_cursor(), "hello");
        assert_eq!(doc3.get_word_after_cursor(), "");
        
        let doc4 = Document::with_text("hello world".to_string(), 6);
        assert_eq!(doc4.get_word_before_cursor(), "");
        assert_eq!(doc4.get_word_after_cursor(), "world");
        
        // Consecutive separators
        let doc5 = Document::with_text("hello...world".to_string(), 13);
        assert_eq!(doc5.get_word_before_cursor_until_separator("."), "world");
        
        let doc6 = Document::with_text("hello...world".to_string(), 5);
        assert_eq!(doc6.get_word_after_cursor_until_separator_with_space("."), "...world");
        
        // Only separators
        let doc7 = Document::with_text("...".to_string(), 3);
        assert_eq!(doc7.get_word_before_cursor_until_separator("."), "");
        
        let doc8 = Document::with_text("...".to_string(), 0);
        assert_eq!(doc8.get_word_after_cursor_until_separator("."), "");
        
        // Mixed whitespace and separators
        let doc9 = Document::with_text("hello . world".to_string(), 13);
        assert_eq!(doc9.get_word_before_cursor(), "world");
        assert_eq!(doc9.get_word_before_cursor_until_separator("."), "world");
    }
}