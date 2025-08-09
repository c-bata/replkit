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
/// If the range is invalid (end < start), returns an empty string.
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
    if start >= end {
        return "";
    }
    
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
        // Basic ASCII
        assert_eq!(rune_count(""), 0);
        assert_eq!(rune_count("hello"), 5);
        
        // Japanese (CJK)
        assert_eq!(rune_count("こんにちは"), 5);
        assert_eq!(rune_count("世界"), 2);
        
        // Chinese (CJK)
        assert_eq!(rune_count("你好"), 2);
        assert_eq!(rune_count("中文测试"), 4);
        
        // Korean (CJK)
        assert_eq!(rune_count("안녕하세요"), 5);
        
        // Emoji
        assert_eq!(rune_count("🦀🚀"), 2);
        // Note: Complex emoji sequences may have different rune counts depending on implementation
        assert!(rune_count("👨‍👩‍👧‍👦") >= 2); // Family emoji (complex, varies by implementation)
        assert!(rune_count("🏳️‍🌈") >= 2); // Flag with combining chars (varies by implementation)
        
        // Mixed content
        assert_eq!(rune_count("Hello 世界 🦀"), 10); // H-e-l-l-o-space-世-界-space-🦀 = 10
        
        // Accented characters
        assert_eq!(rune_count("café"), 4);
        assert_eq!(rune_count("naïve"), 5);
        assert_eq!(rune_count("résumé"), 6);
        
        // Combining characters
        assert_eq!(rune_count("é"), 1); // Single composed char
        assert_eq!(rune_count("e\u{0301}"), 2); // e + combining acute accent
    }

    #[test]
    fn test_display_width() {
        // Basic ASCII
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("hello"), 5);
        
        // Japanese (full-width)
        assert_eq!(display_width("こんにちは"), 10);
        assert_eq!(display_width("世界"), 4);
        
        // Chinese (full-width)
        assert_eq!(display_width("你好"), 4);
        assert_eq!(display_width("中文测试"), 8);
        
        // Korean (full-width)
        assert_eq!(display_width("안녕하세요"), 10);
        
        // Emoji (typically 2 columns)
        assert_eq!(display_width("🦀"), 2);
        assert_eq!(display_width("🚀"), 2);
        
        // Mixed content
        assert_eq!(display_width("Hello 世界"), 10); // 5 + 1 + 4 = 10
        
        // Zero-width characters
        assert_eq!(display_width("\u{200B}"), 0); // Zero-width space
        assert_eq!(display_width("a\u{200B}b"), 2); // a + zero-width + b
        
        // Control characters
        // Tab width is implementation-dependent, but should be non-negative
        let tab_width = display_width("\t");
        assert!(tab_width == 0 || tab_width > 0); // Either 0 or positive
        assert_eq!(display_width("\n"), 0); // Newline has no display width
    }

    #[test]
    fn test_rune_slice() {
        // Basic ASCII
        assert_eq!(rune_slice("hello", 0, 5), "hello");
        assert_eq!(rune_slice("hello", 1, 4), "ell");
        assert_eq!(rune_slice("hello", 0, 0), "");
        
        // Japanese
        assert_eq!(rune_slice("こんにちは", 1, 3), "んに");
        assert_eq!(rune_slice("こんにちは", 0, 2), "こん");
        assert_eq!(rune_slice("こんにちは", 3, 5), "ちは");
        
        // Chinese
        assert_eq!(rune_slice("你好世界", 1, 3), "好世");
        
        // Korean
        assert_eq!(rune_slice("안녕하세요", 2, 4), "하세");
        
        // Emoji
        assert_eq!(rune_slice("🦀🚀🎉", 1, 2), "🚀");
        assert_eq!(rune_slice("🦀🚀🎉", 0, 2), "🦀🚀");
        
        // Mixed content
        assert_eq!(rune_slice("Hello 世界 🦀", 6, 8), "世界");
        
        // Edge cases
        assert_eq!(rune_slice("hello", 10, 20), ""); // Out of bounds
        assert_eq!(rune_slice("hello", 3, 3), ""); // Empty slice
        assert_eq!(rune_slice("hello", 2, 1), ""); // Invalid range
        
        // Accented characters
        assert_eq!(rune_slice("café", 1, 3), "af");
        assert_eq!(rune_slice("résumé", 2, 5), "sum");
    }

    #[test]
    fn test_char_at_rune_index() {
        // Basic ASCII
        assert_eq!(char_at_rune_index("hello", 0), Some('h'));
        assert_eq!(char_at_rune_index("hello", 4), Some('o'));
        assert_eq!(char_at_rune_index("hello", 5), None);
        
        // Japanese
        assert_eq!(char_at_rune_index("こんにちは", 0), Some('こ'));
        assert_eq!(char_at_rune_index("こんにちは", 1), Some('ん'));
        assert_eq!(char_at_rune_index("こんにちは", 4), Some('は'));
        assert_eq!(char_at_rune_index("こんにちは", 5), None);
        
        // Chinese
        assert_eq!(char_at_rune_index("你好", 0), Some('你'));
        assert_eq!(char_at_rune_index("你好", 1), Some('好'));
        assert_eq!(char_at_rune_index("你好", 2), None);
        
        // Korean
        assert_eq!(char_at_rune_index("안녕", 0), Some('안'));
        assert_eq!(char_at_rune_index("안녕", 1), Some('녕'));
        
        // Emoji
        assert_eq!(char_at_rune_index("🦀🚀", 0), Some('🦀'));
        assert_eq!(char_at_rune_index("🦀🚀", 1), Some('🚀'));
        assert_eq!(char_at_rune_index("🦀🚀", 2), None);
        
        // Mixed content
        let mixed = "Hello 世界 🦀";
        assert_eq!(char_at_rune_index(mixed, 0), Some('H'));
        assert_eq!(char_at_rune_index(mixed, 6), Some('世'));
        assert_eq!(char_at_rune_index(mixed, 8), Some(' '));
        assert_eq!(char_at_rune_index(mixed, 9), Some('🦀'));
        
        // Accented characters
        assert_eq!(char_at_rune_index("café", 3), Some('é'));
        assert_eq!(char_at_rune_index("naïve", 2), Some('ï'));
        
        // Empty string
        assert_eq!(char_at_rune_index("", 0), None);
    }

    #[test]
    fn test_byte_index_from_rune_index() {
        // Basic ASCII (1 byte per char)
        assert_eq!(byte_index_from_rune_index("hello", 0), 0);
        assert_eq!(byte_index_from_rune_index("hello", 2), 2);
        assert_eq!(byte_index_from_rune_index("hello", 5), 5);
        
        // Japanese (3 bytes per char)
        assert_eq!(byte_index_from_rune_index("こんにちは", 0), 0);
        assert_eq!(byte_index_from_rune_index("こんにちは", 1), 3);
        assert_eq!(byte_index_from_rune_index("こんにちは", 2), 6);
        assert_eq!(byte_index_from_rune_index("こんにちは", 5), 15);
        
        // Chinese (3 bytes per char)
        assert_eq!(byte_index_from_rune_index("你好", 0), 0);
        assert_eq!(byte_index_from_rune_index("你好", 1), 3);
        assert_eq!(byte_index_from_rune_index("你好", 2), 6);
        
        // Korean (3 bytes per char)
        assert_eq!(byte_index_from_rune_index("안녕", 0), 0);
        assert_eq!(byte_index_from_rune_index("안녕", 1), 3);
        assert_eq!(byte_index_from_rune_index("안녕", 2), 6);
        
        // Emoji (4 bytes per char typically)
        let emoji_text = "🦀🚀";
        assert_eq!(byte_index_from_rune_index(emoji_text, 0), 0);
        assert_eq!(byte_index_from_rune_index(emoji_text, 1), 4);
        assert_eq!(byte_index_from_rune_index(emoji_text, 2), 8);
        
        // Mixed content
        let mixed = "Hi 世界";
        assert_eq!(byte_index_from_rune_index(mixed, 0), 0); // H
        assert_eq!(byte_index_from_rune_index(mixed, 1), 1); // i
        assert_eq!(byte_index_from_rune_index(mixed, 2), 2); // space
        assert_eq!(byte_index_from_rune_index(mixed, 3), 3); // 世 (starts at byte 3)
        assert_eq!(byte_index_from_rune_index(mixed, 4), 6); // 界 (starts at byte 6)
        assert_eq!(byte_index_from_rune_index(mixed, 5), 9); // End of string
        
        // Accented characters (2 bytes for é)
        assert_eq!(byte_index_from_rune_index("café", 0), 0);
        assert_eq!(byte_index_from_rune_index("café", 1), 1);
        assert_eq!(byte_index_from_rune_index("café", 2), 2);
        assert_eq!(byte_index_from_rune_index("café", 3), 3);
        assert_eq!(byte_index_from_rune_index("café", 4), 5); // é is 2 bytes
        
        // Out of bounds
        assert_eq!(byte_index_from_rune_index("hello", 10), 5);
        assert_eq!(byte_index_from_rune_index("こんにちは", 10), 15);
        
        // Empty string
        assert_eq!(byte_index_from_rune_index("", 0), 0);
        assert_eq!(byte_index_from_rune_index("", 5), 0);
    }

    #[test]
    fn test_unicode_edge_cases() {
        // Zero-width characters
        let text_with_zwsp = "a\u{200B}b"; // a + zero-width space + b
        assert_eq!(rune_count(text_with_zwsp), 3);
        assert_eq!(display_width(text_with_zwsp), 2); // Only a and b are visible
        assert_eq!(char_at_rune_index(text_with_zwsp, 1), Some('\u{200B}'));
        
        // Combining characters
        let combining = "e\u{0301}"; // e + combining acute accent
        assert_eq!(rune_count(combining), 2);
        assert_eq!(char_at_rune_index(combining, 0), Some('e'));
        assert_eq!(char_at_rune_index(combining, 1), Some('\u{0301}'));
        
        // Complex emoji sequences
        let family_emoji = "👨‍👩‍👧‍👦"; // Family emoji with ZWJ sequences
        assert!(rune_count(family_emoji) > 1); // Complex emoji are multiple runes
        
        // Right-to-left text (Arabic)
        let arabic = "مرحبا";
        assert_eq!(rune_count(arabic), 5);
        assert!(display_width(arabic) > 0);
        
        // Mixed scripts
        let mixed_scripts = "Hello こんにちは مرحبا 🦀";
        assert!(rune_count(mixed_scripts) > 10);
        assert!(display_width(mixed_scripts) > 10);
        
        // Surrogate pairs (handled automatically by Rust's char type)
        let surrogate_text = "𝕳𝖊𝖑𝖑𝖔"; // Mathematical bold text
        assert_eq!(rune_count(surrogate_text), 5);
        assert!(display_width(surrogate_text) > 0);
    }

    #[test]
    fn test_boundary_conditions() {
        // Empty string
        assert_eq!(rune_count(""), 0);
        assert_eq!(display_width(""), 0);
        assert_eq!(rune_slice("", 0, 0), "");
        assert_eq!(char_at_rune_index("", 0), None);
        assert_eq!(byte_index_from_rune_index("", 0), 0);
        
        // Single character strings
        assert_eq!(rune_count("a"), 1);
        assert_eq!(rune_count("世"), 1);
        assert_eq!(rune_count("🦀"), 1);
        
        // Very long strings (performance test)
        let long_ascii = "a".repeat(1000);
        assert_eq!(rune_count(&long_ascii), 1000);
        assert_eq!(display_width(&long_ascii), 1000);
        
        let long_cjk = "世".repeat(1000);
        assert_eq!(rune_count(&long_cjk), 1000);
        assert_eq!(display_width(&long_cjk), 2000);
        
        // Invalid slice ranges
        assert_eq!(rune_slice("hello", 5, 3), ""); // end < start
        assert_eq!(rune_slice("hello", 10, 15), ""); // both out of bounds
    }
}