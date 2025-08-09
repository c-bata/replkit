//! Prompt Core Library
//!
//! This crate provides the core functionality for parsing terminal input and handling
//! key events in interactive prompt applications. It includes comprehensive key definitions
//! and parsing logic that can be used across multiple language bindings.

pub mod key;
pub mod key_parser;
pub mod sequence_matcher;

// Re-export commonly used types for convenience
pub use key::{Key, KeyEvent};
pub use key_parser::{KeyParser, ParserState};
pub use sequence_matcher::{SequenceMatcher, MatchResult, LongestMatchResult};
