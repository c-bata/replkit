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
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferError::InvalidCursorPosition { position, max } => {
                write!(f, "Invalid cursor position {} (max: {})", position, max)
            }
            BufferError::InvalidWorkingIndex { index, max } => {
                write!(f, "Invalid working index {} (max: {})", index, max)
            }
            BufferError::InvalidRange { start, end } => {
                write!(f, "Invalid range {}..{}", start, end)
            }
            BufferError::UnicodeError(msg) => write!(f, "Unicode error: {}", msg),
            BufferError::EmptyBuffer => write!(f, "Operation not valid on empty buffer"),
        }
    }
}

impl std::error::Error for BufferError {}

/// Result type for buffer operations.
pub type BufferResult<T> = Result<T, BufferError>;