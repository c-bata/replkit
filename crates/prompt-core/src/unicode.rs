//! Unicode utilities for text buffer operations.
//!
//! This module provides Unicode-aware string operations that are essential for
//! proper text editing in international contexts. All operations use rune-based
//! indexing (character count) rather than byte indexing for correct Unicode handling.

use unicode_width::UnicodeWidthStr;

/// Count the number of Unicode characters (runes) in a string.
///
/// This is different from byte length and is used for cursor positioning.
///
/// # Examples
///
/// ```
/// use prompt_core::unicode::rune_count;
///
/// assert_eq!(rune_count("hello"), 5);
/// assert_eq!(rune_count("こんにちは"), 5); // Japanese characters
/// assert_eq!(rune_count("🦀🚀"), 2); // Emoji
/// ```
pub fn rune_count(s: &str) -> usize {
    s.chars().count()
}

/// Get the display width of a string, accounting for wide characters.
///
/// This is important for terminal display where some characters (like CJK)
/// take up two columns.
///
/// # Examples
///
/// ```
/// use prompt_core::unicode::display_width;
///
/// assert_eq!(display_width("hello"), 5);
/// assert_eq!(display_width("こんにちは"), 10); // Each Japanese char is 2 columns
/// ```
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Extract a substring by rune indices (not byte indices).
///
/// This is safe for Unicode strings and will not panic on character boundaries.
///
/// # Arguments
///
/// * `s` - The input string
/// * `start` - Starting rune index (inclusive)
/// * `end` - Ending rune index (exclusive)
///
/// # Examples
///
/// ```
/// use prompt_core::unicode::rune_slice;
///
/// assert_eq!(rune_slice("hello", 1, 4), "ell");
/// assert_eq!(rune_slice("こんにちは", 1, 3), "んに");
/// ```
pub fn rune_slice(s: &str, start: usize, end: usize) -> &str {
    let start_byte = s
        .char_indices()
        .nth(start)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let end_byte = s
        .char_indices()
        .nth(end)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[start_byte..end_byte]
}

/// Get the character at a specific rune index.
///
/// Returns `None` if the index is out of bounds.
///
/// # Examples
///
/// ```
/// use prompt_core::unicode::char_at_rune_index;
///
/// assert_eq!(char_at_rune_index("hello", 1), Some('e'));
/// assert_eq!(char_at_rune_index("こんにちは", 1), Some('ん'));
/// assert_eq!(char_at_rune_index("hello", 10), None);
/// ```
pub fn char_at_rune_index(s: &str, index: usize) -> Option<char> {
    s.chars().nth(index)
}

/// Convert a rune index to a byte index.
///
/// This is useful when you need to slice the string using byte indices
/// but have a rune-based position.
///
/// # Examples
///
/// ```
/// use prompt_core::unicode::byte_index_from_rune_index;
///
/// assert_eq!(byte_index_from_rune_index("hello", 2), 2);
/// assert_eq!(byte_index_from_rune_index("こんにちは", 2), 6); // Each char is 3 bytes
/// ```
pub fn byte_index_from_rune_index(s: &str, rune_index: usize) -> usize {
    s.char_indices()
        .nth(rune_index)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_count() {
        assert_eq!(rune_count(""), 0);
        assert_eq!(rune_count("hello"), 5);
        assert_eq!(rune_count("こんにちは"), 5);
        assert_eq!(rune_count("🦀🚀"), 2);
        assert_eq!(rune_count("café"), 4);
    }

    #[test]
    fn test_display_width() {
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("こんにちは"), 10);
        assert_eq!(display_width("🦀"), 2); // Emoji are typically 2 columns
    }

    #[test]
    fn test_rune_slice() {
        assert_eq!(rune_slice("hello", 0, 5), "hello");
        assert_eq!(rune_slice("hello", 1, 4), "ell");
        assert_eq!(rune_slice("hello", 0, 0), "");
        assert_eq!(rune_slice("こんにちは", 1, 3), "んに");
        assert_eq!(rune_slice("hello", 10, 20), ""); // Out of bounds
    }

    #[test]
    fn test_char_at_rune_index() {
        assert_eq!(char_at_rune_index("hello", 0), Some('h'));
        assert_eq!(char_at_rune_index("hello", 4), Some('o'));
        assert_eq!(char_at_rune_index("hello", 5), None);
        assert_eq!(char_at_rune_index("こんにちは", 1), Some('ん'));
        assert_eq!(char_at_rune_index("🦀🚀", 1), Some('🚀'));
    }

    #[test]
    fn test_byte_index_from_rune_index() {
        assert_eq!(byte_index_from_rune_index("hello", 0), 0);
        assert_eq!(byte_index_from_rune_index("hello", 2), 2);
        assert_eq!(byte_index_from_rune_index("hello", 5), 5);
        assert_eq!(byte_index_from_rune_index("こんにちは", 0), 0);
        assert_eq!(byte_index_from_rune_index("こんにちは", 1), 3);
        assert_eq!(byte_index_from_rune_index("こんにちは", 2), 6);
    }
}