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

    // Multi-line operations

    /// Get the current line before the cursor.
    ///
    /// Returns the portion of the current line that comes before the cursor position.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 8);
    /// assert_eq!(doc.current_line_before_cursor(), "li");
    /// ```
    pub fn current_line_before_cursor(&self) -> &str {
        let (line_start, _) = self.find_line_start_index(self.cursor_position);
        unicode::rune_slice(&self.text, line_start, self.cursor_position)
    }

    /// Get the current line after the cursor.
    ///
    /// Returns the portion of the current line that comes after the cursor position.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 8);
    /// assert_eq!(doc.current_line_after_cursor(), "ne2");
    /// ```
    pub fn current_line_after_cursor(&self) -> &str {
        let text_len = unicode::rune_count(&self.text);
        let mut line_end = self.cursor_position;
        
        // Find the end of the current line (next newline or end of text)
        while line_end < text_len {
            if let Some(ch) = unicode::char_at_rune_index(&self.text, line_end) {
                if ch == '\n' {
                    break;
                }
            }
            line_end += 1;
        }
        
        unicode::rune_slice(&self.text, self.cursor_position, line_end)
    }

    /// Get the complete current line.
    ///
    /// Returns the entire line that contains the cursor, without the trailing newline.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 8);
    /// assert_eq!(doc.current_line(), "line2");
    /// ```
    pub fn current_line(&self) -> String {
        let before = self.current_line_before_cursor();
        let after = self.current_line_after_cursor();
        format!("{}{}", before, after)
    }

    /// Split the text into lines.
    ///
    /// Returns a vector of string slices, one for each line. The newline characters
    /// are not included in the returned strings.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
    /// assert_eq!(doc.lines(), vec!["line1", "line2", "line3"]);
    /// ```
    pub fn lines(&self) -> Vec<&str> {
        if self.text.is_empty() {
            return vec![""];
        }
        
        self.text.split('\n').collect()
    }

    /// Get the number of lines in the document.
    ///
    /// Properly handles trailing newlines - a trailing newline creates a new empty line.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc1 = Document::with_text("line1\nline2".to_string(), 0);
    /// assert_eq!(doc1.line_count(), 2);
    ///
    /// let doc2 = Document::with_text("line1\nline2\n".to_string(), 0);
    /// assert_eq!(doc2.line_count(), 3); // Trailing newline creates empty line
    /// ```
    pub fn line_count(&self) -> usize {
        if self.text.is_empty() {
            return 1;
        }
        
        let newline_count = self.text.chars().filter(|&c| c == '\n').count();
        newline_count + 1
    }

    /// Get the starting rune indexes of each line.
    ///
    /// Returns a vector where each element is the rune index where a line starts.
    /// The first element is always 0. This method uses caching for performance.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
    /// assert_eq!(doc.line_start_indexes(), vec![0, 6, 12]);
    /// ```
    pub fn line_start_indexes(&self) -> Vec<usize> {
        let mut indexes = vec![0];
        
        if self.text.is_empty() {
            return indexes;
        }
        
        let mut current_index = 0;
        for ch in self.text.chars() {
            if ch == '\n' {
                indexes.push(current_index + 1);
            }
            current_index += 1;
        }
        
        indexes
    }

    /// Get the row (line number) of the cursor position.
    ///
    /// Returns 0-based line number where the cursor is located.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 8);
    /// assert_eq!(doc.cursor_position_row(), 1); // Second line (0-based)
    /// ```
    pub fn cursor_position_row(&self) -> usize {
        let (_, row) = self.find_line_start_index(self.cursor_position);
        row
    }

    /// Get the column (character position within line) of the cursor position.
    ///
    /// Returns 0-based column number within the current line.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 8);
    /// assert_eq!(doc.cursor_position_col(), 2); // Third character in line (0-based)
    /// ```
    pub fn cursor_position_col(&self) -> usize {
        let (line_start, _) = self.find_line_start_index(self.cursor_position);
        self.cursor_position - line_start
    }

    /// Translate a linear rune index to (row, column) coordinates.
    ///
    /// # Arguments
    ///
    /// * `index` - The rune index to translate
    ///
    /// # Returns
    ///
    /// A tuple of (row, column) where both are 0-based.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
    /// assert_eq!(doc.translate_index_to_position(8), (1, 2));
    /// ```
    pub fn translate_index_to_position(&self, index: usize) -> (usize, usize) {
        let text_len = unicode::rune_count(&self.text);
        let clamped_index = index.min(text_len);
        
        let (line_start, row) = self.find_line_start_index(clamped_index);
        let col = clamped_index - line_start;
        
        (row, col)
    }

    /// Translate (row, column) coordinates to a linear rune index.
    ///
    /// # Arguments
    ///
    /// * `row` - The 0-based row number
    /// * `col` - The 0-based column number
    ///
    /// # Returns
    ///
    /// The linear rune index corresponding to the given coordinates.
    /// If the coordinates are out of bounds, returns the closest valid position.
    ///
    /// # Examples
    ///
    /// ```
    /// use prompt_core::document::Document;
    ///
    /// let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
    /// assert_eq!(doc.translate_row_col_to_index(1, 2), 8);
    /// ```
    pub fn translate_row_col_to_index(&self, row: usize, col: usize) -> usize {
        let line_starts = self.line_start_indexes();
        
        if row >= line_starts.len() {
            // Row is beyond the document, return end of document
            return unicode::rune_count(&self.text);
        }
        
        let line_start = line_starts[row];
        let line_end = if row + 1 < line_starts.len() {
            line_starts[row + 1] - 1 // Subtract 1 to exclude the newline
        } else {
            unicode::rune_count(&self.text)
        };
        
        let max_col = line_end - line_start;
        let clamped_col = col.min(max_col);
        
        line_start + clamped_col
    }

    /// Find the line start index and row number for a given rune index.
    ///
    /// Returns a tuple of (line_start_index, row_number).
    ///
    /// # Arguments
    ///
    /// * `index` - The rune index to find the line for
    ///
    /// # Returns
    ///
    /// A tuple of (line_start_rune_index, row_number) where row_number is 0-based.
    fn find_line_start_index(&self, index: usize) -> (usize, usize) {
        let line_starts = self.line_start_indexes();
        
        // Binary search to find the appropriate line
        let mut row = 0;
        for (i, &start) in line_starts.iter().enumerate() {
            if start > index {
                break;
            }
            row = i;
        }
        
        (line_starts[row], row)
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

    // Multi-line operation tests

    #[test]
    fn test_current_line_operations() {
        // Basic multi-line text
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 8); // Cursor at "li|ne2"
        assert_eq!(doc.current_line_before_cursor(), "li");
        assert_eq!(doc.current_line_after_cursor(), "ne2");
        assert_eq!(doc.current_line(), "line2");

        // Cursor at start of line
        let doc2 = Document::with_text("line1\nline2\nline3".to_string(), 6); // Cursor at "|line2"
        assert_eq!(doc2.current_line_before_cursor(), "");
        assert_eq!(doc2.current_line_after_cursor(), "line2");
        assert_eq!(doc2.current_line(), "line2");

        // Cursor at end of line
        let doc3 = Document::with_text("line1\nline2\nline3".to_string(), 11); // Cursor at "line2|"
        assert_eq!(doc3.current_line_before_cursor(), "line2");
        assert_eq!(doc3.current_line_after_cursor(), "");
        assert_eq!(doc3.current_line(), "line2");

        // Single line
        let doc4 = Document::with_text("single line".to_string(), 6);
        assert_eq!(doc4.current_line_before_cursor(), "single");
        assert_eq!(doc4.current_line_after_cursor(), " line");
        assert_eq!(doc4.current_line(), "single line");

        // Empty line
        let doc5 = Document::with_text("line1\n\nline3".to_string(), 6); // Cursor on empty line
        assert_eq!(doc5.current_line_before_cursor(), "");
        assert_eq!(doc5.current_line_after_cursor(), "");
        assert_eq!(doc5.current_line(), "");

        // Last line without newline
        let doc6 = Document::with_text("line1\nline2".to_string(), 11); // End of document
        assert_eq!(doc6.current_line_before_cursor(), "line2");
        assert_eq!(doc6.current_line_after_cursor(), "");
        assert_eq!(doc6.current_line(), "line2");
    }

    #[test]
    fn test_lines_method() {
        // Basic multi-line
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
        assert_eq!(doc.lines(), vec!["line1", "line2", "line3"]);

        // Single line
        let doc2 = Document::with_text("single line".to_string(), 0);
        assert_eq!(doc2.lines(), vec!["single line"]);

        // Empty document
        let doc3 = Document::with_text("".to_string(), 0);
        assert_eq!(doc3.lines(), vec![""]);

        // Lines with empty lines
        let doc4 = Document::with_text("line1\n\nline3".to_string(), 0);
        assert_eq!(doc4.lines(), vec!["line1", "", "line3"]);

        // Trailing newline
        let doc5 = Document::with_text("line1\nline2\n".to_string(), 0);
        assert_eq!(doc5.lines(), vec!["line1", "line2", ""]);

        // Only newlines
        let doc6 = Document::with_text("\n\n".to_string(), 0);
        assert_eq!(doc6.lines(), vec!["", "", ""]);
    }

    #[test]
    fn test_line_count() {
        // Basic multi-line
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
        assert_eq!(doc.line_count(), 3);

        // Single line
        let doc2 = Document::with_text("single line".to_string(), 0);
        assert_eq!(doc2.line_count(), 1);

        // Empty document
        let doc3 = Document::with_text("".to_string(), 0);
        assert_eq!(doc3.line_count(), 1);

        // Trailing newline creates new line
        let doc4 = Document::with_text("line1\nline2\n".to_string(), 0);
        assert_eq!(doc4.line_count(), 3);

        // Only newlines
        let doc5 = Document::with_text("\n\n".to_string(), 0);
        assert_eq!(doc5.line_count(), 3);

        // Single newline
        let doc6 = Document::with_text("\n".to_string(), 0);
        assert_eq!(doc6.line_count(), 2);
    }

    #[test]
    fn test_line_start_indexes() {
        // Basic multi-line
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);
        assert_eq!(doc.line_start_indexes(), vec![0, 6, 12]);

        // Single line
        let doc2 = Document::with_text("single line".to_string(), 0);
        assert_eq!(doc2.line_start_indexes(), vec![0]);

        // Empty document
        let doc3 = Document::with_text("".to_string(), 0);
        assert_eq!(doc3.line_start_indexes(), vec![0]);

        // Lines with different lengths
        let doc4 = Document::with_text("a\nbb\nccc".to_string(), 0);
        assert_eq!(doc4.line_start_indexes(), vec![0, 2, 5]);

        // Empty lines
        let doc5 = Document::with_text("line1\n\nline3".to_string(), 0);
        assert_eq!(doc5.line_start_indexes(), vec![0, 6, 7]);

        // Trailing newline
        let doc6 = Document::with_text("line1\nline2\n".to_string(), 0);
        assert_eq!(doc6.line_start_indexes(), vec![0, 6, 12]);
    }

    #[test]
    fn test_cursor_position_row_col() {
        // Basic multi-line
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 8); // "li|ne2"
        assert_eq!(doc.cursor_position_row(), 1);
        assert_eq!(doc.cursor_position_col(), 2);

        // Start of document
        let doc2 = Document::with_text("line1\nline2\nline3".to_string(), 0);
        assert_eq!(doc2.cursor_position_row(), 0);
        assert_eq!(doc2.cursor_position_col(), 0);

        // End of first line
        let doc3 = Document::with_text("line1\nline2\nline3".to_string(), 5);
        assert_eq!(doc3.cursor_position_row(), 0);
        assert_eq!(doc3.cursor_position_col(), 5);

        // Start of second line
        let doc4 = Document::with_text("line1\nline2\nline3".to_string(), 6);
        assert_eq!(doc4.cursor_position_row(), 1);
        assert_eq!(doc4.cursor_position_col(), 0);

        // End of document
        let doc5 = Document::with_text("line1\nline2\nline3".to_string(), 17);
        assert_eq!(doc5.cursor_position_row(), 2);
        assert_eq!(doc5.cursor_position_col(), 5);

        // Empty line
        let doc6 = Document::with_text("line1\n\nline3".to_string(), 6);
        assert_eq!(doc6.cursor_position_row(), 1);
        assert_eq!(doc6.cursor_position_col(), 0);

        // Single line
        let doc7 = Document::with_text("single line".to_string(), 7);
        assert_eq!(doc7.cursor_position_row(), 0);
        assert_eq!(doc7.cursor_position_col(), 7);
    }

    #[test]
    fn test_translate_index_to_position() {
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);

        // Start of document
        assert_eq!(doc.translate_index_to_position(0), (0, 0));

        // End of first line
        assert_eq!(doc.translate_index_to_position(5), (0, 5));

        // Start of second line
        assert_eq!(doc.translate_index_to_position(6), (1, 0));

        // Middle of second line
        assert_eq!(doc.translate_index_to_position(8), (1, 2));

        // End of document
        assert_eq!(doc.translate_index_to_position(17), (2, 5));

        // Beyond document (should clamp)
        assert_eq!(doc.translate_index_to_position(100), (2, 5));

        // Empty line
        let doc2 = Document::with_text("line1\n\nline3".to_string(), 0);
        assert_eq!(doc2.translate_index_to_position(6), (1, 0));
        assert_eq!(doc2.translate_index_to_position(7), (2, 0));
    }

    #[test]
    fn test_translate_row_col_to_index() {
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);

        // Start of document
        assert_eq!(doc.translate_row_col_to_index(0, 0), 0);

        // End of first line
        assert_eq!(doc.translate_row_col_to_index(0, 5), 5);

        // Start of second line
        assert_eq!(doc.translate_row_col_to_index(1, 0), 6);

        // Middle of second line
        assert_eq!(doc.translate_row_col_to_index(1, 2), 8);

        // End of last line
        assert_eq!(doc.translate_row_col_to_index(2, 5), 17);

        // Beyond line length (should clamp to end of line)
        assert_eq!(doc.translate_row_col_to_index(0, 100), 5);

        // Beyond document rows (should clamp to end of document)
        assert_eq!(doc.translate_row_col_to_index(100, 0), 17);

        // Empty line
        let doc2 = Document::with_text("line1\n\nline3".to_string(), 0);
        assert_eq!(doc2.translate_row_col_to_index(1, 0), 6);
        assert_eq!(doc2.translate_row_col_to_index(1, 100), 6); // Empty line, clamps to 0
    }

    #[test]
    fn test_multi_line_with_unicode() {
        // Japanese text with newlines
        // "こんにちは\n世界\nテスト"
        // Line 0: "こんにちは" (positions 0-4)
        // Line 1: "世界" (positions 6-7) 
        // Line 2: "テスト" (positions 9-11)
        let doc = Document::with_text("こんにちは\n世界\nテスト".to_string(), 8); // End of line 1
        assert_eq!(doc.cursor_position_row(), 1);
        assert_eq!(doc.cursor_position_col(), 2);
        assert_eq!(doc.current_line_before_cursor(), "世界");
        assert_eq!(doc.current_line_after_cursor(), "");
        assert_eq!(doc.current_line(), "世界");

        // Test cursor in middle of second line
        let doc2 = Document::with_text("こんにちは\n世界\nテスト".to_string(), 7); // "世|界"
        assert_eq!(doc2.cursor_position_row(), 1);
        assert_eq!(doc2.cursor_position_col(), 1);
        assert_eq!(doc2.current_line_before_cursor(), "世");
        assert_eq!(doc2.current_line_after_cursor(), "界");
        assert_eq!(doc2.current_line(), "世界");

        // Mixed Unicode and ASCII
        let doc3 = Document::with_text("hello\n世界\nworld".to_string(), 7); // Cursor at "世|界"
        assert_eq!(doc3.cursor_position_row(), 1);
        assert_eq!(doc3.cursor_position_col(), 1);
        assert_eq!(doc3.current_line(), "世界");

        // Emoji
        let doc4 = Document::with_text("🦀\n🚀\n🎉".to_string(), 2); // Cursor at start of second line
        assert_eq!(doc4.cursor_position_row(), 1);
        assert_eq!(doc4.cursor_position_col(), 0);
        assert_eq!(doc4.current_line(), "🚀");

        // Line operations
        assert_eq!(doc.lines(), vec!["こんにちは", "世界", "テスト"]);
        assert_eq!(doc.line_count(), 3);
        assert_eq!(doc.line_start_indexes(), vec![0, 6, 9]);
    }

    #[test]
    fn test_multi_line_edge_cases() {
        // Document with only newlines
        let doc = Document::with_text("\n\n\n".to_string(), 2);
        assert_eq!(doc.cursor_position_row(), 2);
        assert_eq!(doc.cursor_position_col(), 0);
        assert_eq!(doc.current_line(), "");
        assert_eq!(doc.line_count(), 4);

        // Very long lines
        let long_line = "a".repeat(1000);
        let text = format!("short\n{}\nshort", long_line);
        let doc2 = Document::with_text(text, 1006); // Middle of long line
        assert_eq!(doc2.cursor_position_row(), 1);
        assert_eq!(doc2.cursor_position_col(), 1000);

        // Empty document
        let doc3 = Document::with_text("".to_string(), 0);
        assert_eq!(doc3.cursor_position_row(), 0);
        assert_eq!(doc3.cursor_position_col(), 0);
        assert_eq!(doc3.current_line(), "");
        assert_eq!(doc3.line_count(), 1);
        assert_eq!(doc3.lines(), vec![""]);

        // Single character lines
        let doc4 = Document::with_text("a\nb\nc".to_string(), 4);
        assert_eq!(doc4.cursor_position_row(), 2);
        assert_eq!(doc4.cursor_position_col(), 0);
        assert_eq!(doc4.current_line(), "c");

        // Trailing spaces in lines
        let doc5 = Document::with_text("line1  \nline2  \nline3".to_string(), 9);
        assert_eq!(doc5.cursor_position_row(), 1);
        assert_eq!(doc5.cursor_position_col(), 1);
        assert_eq!(doc5.current_line(), "line2  ");
    }

    #[test]
    fn test_find_line_start_index() {
        let doc = Document::with_text("line1\nline2\nline3".to_string(), 0);

        // Test various positions
        assert_eq!(doc.find_line_start_index(0), (0, 0));   // Start of first line
        assert_eq!(doc.find_line_start_index(3), (0, 0));   // Middle of first line
        assert_eq!(doc.find_line_start_index(5), (0, 0));   // End of first line
        assert_eq!(doc.find_line_start_index(6), (6, 1));   // Start of second line
        assert_eq!(doc.find_line_start_index(8), (6, 1));   // Middle of second line
        assert_eq!(doc.find_line_start_index(11), (6, 1));  // End of second line
        assert_eq!(doc.find_line_start_index(12), (12, 2)); // Start of third line
        assert_eq!(doc.find_line_start_index(17), (12, 2)); // End of document

        // Edge cases
        let doc2 = Document::with_text("".to_string(), 0);
        assert_eq!(doc2.find_line_start_index(0), (0, 0));

        let doc3 = Document::with_text("single".to_string(), 0);
        assert_eq!(doc3.find_line_start_index(3), (0, 0));
    }
}