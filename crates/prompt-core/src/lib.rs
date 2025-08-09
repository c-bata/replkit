//! Prompt Core Library
//!
//! This crate provides the core functionality for parsing terminal input, handling
//! key events, and managing text buffers in interactive prompt applications. It includes
//! comprehensive key definitions, parsing logic, and text editing capabilities that can
//! be used across multiple language bindings.

pub mod key;
pub mod key_parser;
pub mod sequence_matcher;
pub mod wasm;

// Text buffer and document modules
pub mod buffer;
pub mod document;
pub mod error;
pub mod unicode;

// Re-export commonly used types for convenience
pub use key::{Key, KeyEvent};
pub use key_parser::{KeyParser, ParserState};
pub use sequence_matcher::{SequenceMatcher, MatchResult, LongestMatchResult};
pub use wasm::{WasmKeyEvent, WasmKeyParser, key_to_u32, u32_to_key};

// Re-export WASM serialization types when wasm feature is enabled
#[cfg(feature = "wasm")]
pub use wasm::{WasmBufferState, WasmDocumentState};

// Re-export text buffer types
pub use buffer::Buffer;
pub use document::Document;
pub use error::{BufferError, BufferResult};
pub use unicode::{rune_count, display_width, rune_slice, char_at_rune_index, byte_index_from_rune_index};
