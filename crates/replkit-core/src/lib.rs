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

// Console I/O abstraction
pub mod console;

// REPL engine
pub mod repl;

// Key handling
pub mod key_handler;

// Rendering
pub mod renderer;

// Event loop
pub mod event_loop;

// Platform factory
pub mod platform;

// Re-export commonly used types for convenience
pub use key::{Key, KeyEvent};
pub use key_parser::{KeyParser, ParserState};
pub use sequence_matcher::{LongestMatchResult, MatchResult, SequenceMatcher};
pub use wasm::{key_to_u32, u32_to_key, WasmKeyEvent, WasmKeyParser};

// Re-export WASM serialization types when wasm feature is enabled
#[cfg(feature = "wasm")]
pub use wasm::{WasmBufferState, WasmDocumentState};

// Re-export text buffer types
pub use buffer::Buffer;
pub use document::Document;
pub use error::{BufferError, BufferResult};
pub use unicode::{
    byte_index_from_rune_index, char_at_rune_index, display_width, rune_count, rune_slice,
};

// Re-export console types
pub use console::{
    AsAny, BackendType, ClearType, Color, ConsoleCapabilities, ConsoleError, ConsoleInput,
    ConsoleOutput, ConsoleResult, EventLoopError, OutputCapabilities, RawModeGuard, SafeTextFilter,
    SanitizationPolicy, TextStyle,
};

// Re-export REPL types
pub use repl::{KeyAction, KeyBinding, ReplConfig, ReplEngine, ReplError};

// Re-export key handler types
pub use key_handler::{KeyHandler, KeyResult};

// Re-export renderer types
pub use renderer::{RenderConfig, RenderResult, Renderer};

// Re-export event loop types
pub use event_loop::{EventLoop, ReplEvent};

// Re-export platform factory types
pub use platform::{
    create_native_console_io, create_native_factory, NativePlatformFactory, PlatformFactory,
};
