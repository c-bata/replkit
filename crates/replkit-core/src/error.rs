//! Error types for text buffer and document operations.

use std::fmt;

/// Errors that can occur during buffer and document operations.
#[derive(Debug, Clone, PartialEq)]
pub enum BufferError {
    /// Invalid cursor position specified.
    InvalidCursorPosition { position: usize, max: usize },
    /// Invalid working index specified.
    InvalidWorkingIndex { index: usize, max: usize },
    /// Invalid range specified.
    InvalidRange { start: usize, end: usize },
    /// Unicode-related error.
    UnicodeError(String),
    /// Operation not valid on empty buffer.
    EmptyBuffer,
    /// Invalid text operation parameters.
    InvalidTextOperation { operation: String, reason: String },
    /// Bounds check failed for operation.
    BoundsCheckFailed {
        operation: String,
        position: usize,
        bounds: (usize, usize),
    },
    /// Invalid character count for operation.
    InvalidCharacterCount { count: usize, available: usize },
    /// Text encoding error.
    TextEncodingError(String),
    /// Invalid line number.
    InvalidLineNumber { line: usize, max_line: usize },
    /// Invalid column position.
    InvalidColumnPosition { column: usize, max_column: usize },
    /// Operation would result in invalid state.
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferError::InvalidCursorPosition { position, max } => {
                write!(f, "Invalid cursor position {position} (max: {max})")
            }
            BufferError::InvalidWorkingIndex { index, max } => {
                write!(f, "Invalid working index {index} (max: {max})")
            }
            BufferError::InvalidRange { start, end } => {
                write!(f, "Invalid range {start}..{end}")
            }
            BufferError::UnicodeError(msg) => write!(f, "Unicode error: {msg}"),
            BufferError::EmptyBuffer => write!(f, "Operation not valid on empty buffer"),
            BufferError::InvalidTextOperation { operation, reason } => {
                write!(f, "Invalid text operation '{operation}': {reason}")
            }
            BufferError::BoundsCheckFailed {
                operation,
                position,
                bounds,
            } => {
                write!(
                    f,
                    "Bounds check failed for '{}': position {} not in range {}..{}",
                    operation, position, bounds.0, bounds.1
                )
            }
            BufferError::InvalidCharacterCount { count, available } => {
                write!(
                    f,
                    "Invalid character count {count}: only {available} available"
                )
            }
            BufferError::TextEncodingError(msg) => write!(f, "Text encoding error: {msg}"),
            BufferError::InvalidLineNumber { line, max_line } => {
                write!(f, "Invalid line number {line} (max: {max_line})")
            }
            BufferError::InvalidColumnPosition { column, max_column } => {
                write!(f, "Invalid column position {column} (max: {max_column})")
            }
            BufferError::InvalidStateTransition { from, to, reason } => {
                write!(
                    f,
                    "Invalid state transition from '{from}' to '{to}': {reason}"
                )
            }
        }
    }
}

impl std::error::Error for BufferError {}

impl BufferError {
    /// Create an invalid cursor position error.
    pub fn invalid_cursor_position(position: usize, max: usize) -> Self {
        BufferError::InvalidCursorPosition { position, max }
    }

    /// Create an invalid working index error.
    pub fn invalid_working_index(index: usize, max: usize) -> Self {
        BufferError::InvalidWorkingIndex { index, max }
    }

    /// Create an invalid range error.
    pub fn invalid_range(start: usize, end: usize) -> Self {
        BufferError::InvalidRange { start, end }
    }

    /// Create a bounds check failed error.
    pub fn bounds_check_failed(operation: &str, position: usize, bounds: (usize, usize)) -> Self {
        BufferError::BoundsCheckFailed {
            operation: operation.to_string(),
            position,
            bounds,
        }
    }

    /// Create an invalid character count error.
    pub fn invalid_character_count(count: usize, available: usize) -> Self {
        BufferError::InvalidCharacterCount { count, available }
    }

    /// Create an invalid text operation error.
    pub fn invalid_text_operation(operation: &str, reason: &str) -> Self {
        BufferError::InvalidTextOperation {
            operation: operation.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create a Unicode error.
    pub fn unicode_error(msg: &str) -> Self {
        BufferError::UnicodeError(msg.to_string())
    }

    /// Create a text encoding error.
    pub fn text_encoding_error(msg: &str) -> Self {
        BufferError::TextEncodingError(msg.to_string())
    }

    /// Create an invalid line number error.
    pub fn invalid_line_number(line: usize, max_line: usize) -> Self {
        BufferError::InvalidLineNumber { line, max_line }
    }

    /// Create an invalid column position error.
    pub fn invalid_column_position(column: usize, max_column: usize) -> Self {
        BufferError::InvalidColumnPosition { column, max_column }
    }

    /// Create an invalid state transition error.
    pub fn invalid_state_transition(from: &str, to: &str, reason: &str) -> Self {
        BufferError::InvalidStateTransition {
            from: from.to_string(),
            to: to.to_string(),
            reason: reason.to_string(),
        }
    }
}

/// Result type for buffer operations.
pub type BufferResult<T> = Result<T, BufferError>;

/// Validation utilities for buffer operations.
pub mod validation {
    use super::{BufferError, BufferResult};
    use crate::unicode;

    /// Validate cursor position is within text bounds.
    pub fn validate_cursor_position(position: usize, text: &str) -> BufferResult<()> {
        let text_len = unicode::rune_count(text);
        if position > text_len {
            Err(BufferError::invalid_cursor_position(position, text_len))
        } else {
            Ok(())
        }
    }

    /// Validate working index is within bounds.
    pub fn validate_working_index(index: usize, max_index: usize) -> BufferResult<()> {
        if index >= max_index {
            Err(BufferError::invalid_working_index(
                index,
                max_index.saturating_sub(1),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate range is valid.
    pub fn validate_range(start: usize, end: usize) -> BufferResult<()> {
        if start > end {
            Err(BufferError::invalid_range(start, end))
        } else {
            Ok(())
        }
    }

    /// Validate range is within text bounds.
    pub fn validate_range_bounds(start: usize, end: usize, text: &str) -> BufferResult<()> {
        validate_range(start, end)?;
        let text_len = unicode::rune_count(text);
        if end > text_len {
            Err(BufferError::bounds_check_failed(
                "range_bounds",
                end,
                (0, text_len),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate character count for deletion operations.
    pub fn validate_character_count(
        count: usize,
        available: usize,
        _operation: &str,
    ) -> BufferResult<usize> {
        if count == 0 {
            return Ok(0);
        }
        if count > available {
            Err(BufferError::invalid_character_count(count, available))
        } else {
            Ok(count)
        }
    }

    /// Validate text is valid UTF-8.
    pub fn validate_text_encoding(text: &str) -> BufferResult<()> {
        // Rust strings are always valid UTF-8, but we can check for other issues
        if text.chars().any(|c| c == '\0') {
            Err(BufferError::text_encoding_error(
                "Text contains null characters",
            ))
        } else {
            Ok(())
        }
    }

    /// Validate line number is within document bounds.
    pub fn validate_line_number(line: usize, line_count: usize) -> BufferResult<()> {
        if line >= line_count {
            Err(BufferError::invalid_line_number(
                line,
                line_count.saturating_sub(1),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate column position is within line bounds.
    pub fn validate_column_position(column: usize, line_text: &str) -> BufferResult<()> {
        let line_len = unicode::rune_count(line_text);
        if column > line_len {
            Err(BufferError::invalid_column_position(column, line_len))
        } else {
            Ok(())
        }
    }

    /// Clamp cursor position to valid bounds.
    pub fn clamp_cursor_position(position: usize, text: &str) -> usize {
        let text_len = unicode::rune_count(text);
        position.min(text_len)
    }

    /// Clamp working index to valid bounds.
    pub fn clamp_working_index(index: usize, max_index: usize) -> usize {
        if max_index == 0 {
            0
        } else {
            index.min(max_index - 1)
        }
    }

    /// Clamp character count to available characters.
    pub fn clamp_character_count(count: usize, available: usize) -> usize {
        count.min(available)
    }
}
#[cfg(test)]
mod tests {
    use super::validation::*;
    use super::*;

    #[test]
    fn test_buffer_error_creation() {
        let err = BufferError::invalid_cursor_position(10, 5);
        assert_eq!(
            err,
            BufferError::InvalidCursorPosition {
                position: 10,
                max: 5
            }
        );

        let err = BufferError::invalid_working_index(3, 2);
        assert_eq!(err, BufferError::InvalidWorkingIndex { index: 3, max: 2 });

        let err = BufferError::bounds_check_failed("test_op", 15, (0, 10));
        assert_eq!(
            err,
            BufferError::BoundsCheckFailed {
                operation: "test_op".to_string(),
                position: 15,
                bounds: (0, 10)
            }
        );
    }

    #[test]
    fn test_buffer_error_display() {
        let err = BufferError::invalid_cursor_position(10, 5);
        assert_eq!(err.to_string(), "Invalid cursor position 10 (max: 5)");

        let err = BufferError::invalid_character_count(5, 3);
        assert_eq!(
            err.to_string(),
            "Invalid character count 5: only 3 available"
        );

        let err = BufferError::bounds_check_failed("test_op", 15, (0, 10));
        assert_eq!(
            err.to_string(),
            "Bounds check failed for 'test_op': position 15 not in range 0..10"
        );
    }

    #[test]
    fn test_validate_cursor_position() {
        // Valid positions
        assert!(validate_cursor_position(0, "hello").is_ok());
        assert!(validate_cursor_position(3, "hello").is_ok());
        assert!(validate_cursor_position(5, "hello").is_ok());

        // Invalid position
        let result = validate_cursor_position(10, "hello");
        assert!(result.is_err());
        if let Err(BufferError::InvalidCursorPosition { position, max }) = result {
            assert_eq!(position, 10);
            assert_eq!(max, 5);
        } else {
            panic!("Expected InvalidCursorPosition error");
        }
    }

    #[test]
    fn test_validate_working_index() {
        // Valid indices
        assert!(validate_working_index(0, 3).is_ok());
        assert!(validate_working_index(2, 3).is_ok());

        // Invalid index
        let result = validate_working_index(3, 3);
        assert!(result.is_err());
        if let Err(BufferError::InvalidWorkingIndex { index, max }) = result {
            assert_eq!(index, 3);
            assert_eq!(max, 2);
        } else {
            panic!("Expected InvalidWorkingIndex error");
        }
    }

    #[test]
    fn test_validate_range() {
        // Valid ranges
        assert!(validate_range(0, 5).is_ok());
        assert!(validate_range(2, 2).is_ok());

        // Invalid range
        let result = validate_range(5, 2);
        assert!(result.is_err());
        if let Err(BufferError::InvalidRange { start, end }) = result {
            assert_eq!(start, 5);
            assert_eq!(end, 2);
        } else {
            panic!("Expected InvalidRange error");
        }
    }

    #[test]
    fn test_validate_range_bounds() {
        let text = "hello world";

        // Valid range within bounds
        assert!(validate_range_bounds(0, 5, text).is_ok());
        assert!(validate_range_bounds(6, 11, text).is_ok());

        // Invalid range (start > end)
        let result = validate_range_bounds(5, 2, text);
        assert!(result.is_err());

        // Range exceeds text bounds
        let result = validate_range_bounds(0, 20, text);
        assert!(result.is_err());
        if let Err(BufferError::BoundsCheckFailed {
            operation,
            position,
            bounds,
        }) = result
        {
            assert_eq!(operation, "range_bounds");
            assert_eq!(position, 20);
            assert_eq!(bounds, (0, 11));
        } else {
            panic!("Expected BoundsCheckFailed error");
        }
    }

    #[test]
    fn test_validate_character_count() {
        // Valid counts
        assert_eq!(validate_character_count(0, 5, "test").unwrap(), 0);
        assert_eq!(validate_character_count(3, 5, "test").unwrap(), 3);
        assert_eq!(validate_character_count(5, 5, "test").unwrap(), 5);

        // Invalid count
        let result = validate_character_count(10, 5, "test");
        assert!(result.is_err());
        if let Err(BufferError::InvalidCharacterCount { count, available }) = result {
            assert_eq!(count, 10);
            assert_eq!(available, 5);
        } else {
            panic!("Expected InvalidCharacterCount error");
        }
    }

    #[test]
    fn test_validate_text_encoding() {
        // Valid text
        assert!(validate_text_encoding("hello world").is_ok());
        assert!(validate_text_encoding("こんにちは").is_ok());
        assert!(validate_text_encoding("Hello 👋 World").is_ok());

        // Text with null characters
        let result = validate_text_encoding("hello\0world");
        assert!(result.is_err());
        if let Err(BufferError::TextEncodingError(_)) = result {
            // Expected
        } else {
            panic!("Expected TextEncodingError");
        }
    }

    #[test]
    fn test_validate_line_number() {
        // Valid line numbers
        assert!(validate_line_number(0, 5).is_ok());
        assert!(validate_line_number(4, 5).is_ok());

        // Invalid line number
        let result = validate_line_number(5, 5);
        assert!(result.is_err());
        if let Err(BufferError::InvalidLineNumber { line, max_line }) = result {
            assert_eq!(line, 5);
            assert_eq!(max_line, 4);
        } else {
            panic!("Expected InvalidLineNumber error");
        }
    }

    #[test]
    fn test_validate_column_position() {
        let line_text = "hello world";

        // Valid column positions
        assert!(validate_column_position(0, line_text).is_ok());
        assert!(validate_column_position(5, line_text).is_ok());
        assert!(validate_column_position(11, line_text).is_ok());

        // Invalid column position
        let result = validate_column_position(15, line_text);
        assert!(result.is_err());
        if let Err(BufferError::InvalidColumnPosition { column, max_column }) = result {
            assert_eq!(column, 15);
            assert_eq!(max_column, 11);
        } else {
            panic!("Expected InvalidColumnPosition error");
        }
    }

    #[test]
    fn test_clamp_functions() {
        // Test cursor position clamping
        assert_eq!(clamp_cursor_position(3, "hello"), 3);
        assert_eq!(clamp_cursor_position(10, "hello"), 5);
        assert_eq!(clamp_cursor_position(0, ""), 0);

        // Test working index clamping
        assert_eq!(clamp_working_index(2, 5), 2);
        assert_eq!(clamp_working_index(10, 5), 4);
        assert_eq!(clamp_working_index(5, 0), 0);

        // Test character count clamping
        assert_eq!(clamp_character_count(3, 5), 3);
        assert_eq!(clamp_character_count(10, 5), 5);
        assert_eq!(clamp_character_count(0, 5), 0);
    }

    #[test]
    fn test_unicode_validation() {
        // Test with various Unicode characters
        let unicode_text = "Hello 👋 こんにちは 🌍";

        assert!(validate_text_encoding(unicode_text).is_ok());
        assert!(validate_cursor_position(8, unicode_text).is_ok());

        // Test clamping with Unicode
        let clamped = clamp_cursor_position(100, unicode_text);
        assert_eq!(clamped, crate::unicode::rune_count(unicode_text));
    }

    #[test]
    fn test_error_chaining() {
        // Test that validation functions can be chained
        let text = "hello world";
        let result = validate_text_encoding(text)
            .and_then(|_| validate_cursor_position(5, text))
            .and_then(|_| validate_range_bounds(0, 5, text));

        assert!(result.is_ok());

        // Test error propagation in chain
        let result = validate_text_encoding(text)
            .and_then(|_| validate_cursor_position(20, text))
            .and_then(|_| validate_range_bounds(0, 5, text));

        assert!(result.is_err());
    }
}
