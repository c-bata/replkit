//! Console input/output abstraction traits and types
//!
//! This module provides the core traits and types for cross-platform console I/O,
//! including input handling, output rendering, and terminal state management.

use crate::KeyEvent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Helper trait for testing - allows downcasting to concrete types
pub trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Cross-platform console input interface
pub trait ConsoleInput: Send + Sync + AsAny {
    /// Enable raw terminal mode with automatic restoration
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError>;

    /// Get current terminal window size (columns, rows)
    /// Returns the visible window area (srWindow on Windows), not the buffer size (dwSize)
    /// Values are in character cells, 0-based for API but 1-based for ANSI sequences
    fn get_window_size(&self) -> Result<(u16, u16), ConsoleError>;

    /// Start the event processing loop
    fn start_event_loop(&self) -> Result<(), ConsoleError>;

    /// Stop the event processing loop
    fn stop_event_loop(&self) -> Result<(), ConsoleError>;

    /// Register callback for window resize events
    fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>);

    /// Register callback for key press events
    fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>);

    /// Check if the event loop is currently running
    fn is_running(&self) -> bool;

    /// Get platform-specific capabilities
    fn get_capabilities(&self) -> ConsoleCapabilities;
}

/// Cross-platform console output interface
pub trait ConsoleOutput: Send + Sync + AsAny {
    /// Write text at current cursor position
    fn write_text(&self, text: &str) -> ConsoleResult<()>;

    /// Write text with specific styling
    fn write_styled_text(&self, text: &str, style: &TextStyle) -> ConsoleResult<()>;

    /// Write safe text (control sequences removed/escaped)
    fn write_safe_text(&self, text: &str) -> ConsoleResult<()>;

    /// Move cursor to specific position (0-based coordinates: row, col)
    /// Note: API uses 0-based coordinates, but ANSI sequences use 1-based
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()>;

    /// Move cursor relative to current position
    fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()>;

    /// Clear screen or specific areas
    fn clear(&self, clear_type: ClearType) -> ConsoleResult<()>;

    /// Set text styling for subsequent writes
    fn set_style(&self, style: &TextStyle) -> ConsoleResult<()>;

    /// Reset all styling to default
    fn reset_style(&self) -> ConsoleResult<()>;

    /// Flush buffered output to terminal
    fn flush(&self) -> ConsoleResult<()>;

    /// Enable/disable alternate screen buffer
    fn set_alternate_screen(&self, enabled: bool) -> ConsoleResult<()>;

    /// Show/hide cursor
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()>;

    /// Get current cursor position (row, col)
    /// Returns 0-based coordinates (API convention), converted from 1-based ANSI responses
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)>;

    /// Get output capabilities
    fn get_capabilities(&self) -> OutputCapabilities;
}

/// RAII guard for terminal raw mode with primary restoration responsibility
pub struct RawModeGuard {
    restore_fn: Option<Box<dyn FnOnce() + Send>>,
    platform_info: String,
    is_active: Arc<AtomicBool>,
}

impl RawModeGuard {
    pub fn new<F>(restore_fn: F, platform_info: String) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let is_active = Arc::new(AtomicBool::new(true));
        Self {
            restore_fn: Some(Box::new(restore_fn)),
            platform_info,
            is_active,
        }
    }

    pub fn platform_info(&self) -> &str {
        &self.platform_info
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }

    /// Manually restore terminal mode (prevents automatic restoration on drop)
    pub fn restore(mut self) -> Result<(), ConsoleError> {
        if let Some(restore_fn) = self.restore_fn.take() {
            self.is_active.store(false, Ordering::Relaxed);
            restore_fn();
            Ok(())
        } else {
            Err(ConsoleError::TerminalError("Already restored".to_string()))
        }
    }

    /// Get a weak reference to check if this guard is still active
    pub fn weak_ref(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_active)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if let Some(restore_fn) = self.restore_fn.take() {
            self.is_active.store(false, Ordering::Relaxed);
            restore_fn();
        }
    }
}

/// Platform capabilities and feature support
#[derive(Debug, Clone)]
pub struct ConsoleCapabilities {
    pub supports_raw_mode: bool,
    pub supports_resize_events: bool,
    pub supports_bracketed_paste: bool,
    pub supports_mouse_events: bool,
    pub supports_unicode: bool,
    pub platform_name: String,
    pub backend_type: BackendType,
}

/// Output capabilities and feature support
#[derive(Debug, Clone)]
pub struct OutputCapabilities {
    pub supports_colors: bool,
    /// True color (24-bit RGB) support - determined by runtime detection or environment variables
    /// Windows legacy environments may not support 24-bit color even if the API exists
    pub supports_true_color: bool,
    pub supports_styling: bool,
    pub supports_alternate_screen: bool,
    pub supports_cursor_control: bool,
    pub max_colors: u16,
    pub platform_name: String,
    pub backend_type: BackendType,
}

/// Backend implementation type
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    UnixVt,
    WindowsVt,
    WindowsLegacy,
    WasmBridge,
    Mock,
}

/// Text styling configuration
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
}

/// Color specification for text styling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
    Ansi256(u8),
}

/// Screen clearing options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearType {
    /// Clear entire screen
    All,
    /// Clear from cursor to end of screen
    FromCursor,
    /// Clear from beginning of screen to cursor
    ToCursor,
    /// Clear current line
    CurrentLine,
    /// Clear from cursor to end of line
    FromCursorToEndOfLine,
    /// Clear from beginning of line to cursor
    FromBeginningOfLineToCursor,
}

/// Console operation errors
#[derive(Debug, Clone)]
pub enum ConsoleError {
    /// Platform-specific I/O error
    IoError(String),
    /// Feature not supported on this platform
    UnsupportedFeature { feature: String, platform: String },
    /// Event loop state error
    EventLoopError(EventLoopError),
    /// Terminal setup/teardown error
    TerminalError(String),
    /// Thread management error
    ThreadError(String),
    /// Callback registration error
    CallbackError(String),
    /// WASM bridge communication error
    WasmBridgeError(String),
}

/// Event loop specific errors
#[derive(Debug, Clone)]
pub enum EventLoopError {
    AlreadyRunning,
    NotRunning,
    StartupFailed(String),
    ShutdownTimeout,
    ThreadPanic(String),
}

impl std::fmt::Display for ConsoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsoleError::IoError(msg) => write!(f, "I/O error: {msg}"),
            ConsoleError::UnsupportedFeature { feature, platform } => {
                write!(
                    f,
                    "Feature '{feature}' not supported on platform '{platform}'"
                )
            }
            ConsoleError::EventLoopError(e) => write!(f, "Event loop error: {e:?}"),
            ConsoleError::TerminalError(msg) => write!(f, "Terminal error: {msg}"),
            ConsoleError::ThreadError(msg) => write!(f, "Thread error: {msg}"),
            ConsoleError::CallbackError(msg) => write!(f, "Callback error: {msg}"),
            ConsoleError::WasmBridgeError(msg) => write!(f, "WASM bridge error: {msg}"),
        }
    }
}

impl std::error::Error for ConsoleError {}

/// Result type for console operations
pub type ConsoleResult<T> = Result<T, ConsoleError>;

/// Control sequence sanitization policy for safe text output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SanitizationPolicy {
    /// Remove all control sequences (CSI, OSC, DCS, etc.)
    RemoveAll,
    /// Remove only potentially dangerous sequences (CSI, OSC)
    RemoveDangerous,
    /// Escape control sequences to make them visible
    EscapeAll,
    /// Allow basic formatting but remove dangerous sequences
    AllowBasicFormatting,
}

/// Safe text writer with control sequence filtering
pub struct SafeTextFilter {
    policy: SanitizationPolicy,
    state: FilterState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FilterState {
    Normal,
    Escape,
    Csi,
    OscString,
    DcsString,
}

impl SafeTextFilter {
    pub fn new(policy: SanitizationPolicy) -> Self {
        SafeTextFilter {
            policy,
            state: FilterState::Normal,
        }
    }

    /// Filter text according to the sanitization policy
    pub fn filter(&mut self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());

        for byte in input.bytes() {
            match self.process_byte(byte) {
                FilterAction::Emit(b) => output.push(b as char),
                FilterAction::EmitEscaped(b) => {
                    output.push('\\');
                    output.push('x');
                    output.push_str(&format!("{b:02x}"));
                }
                FilterAction::Skip => {}
            }
        }

        output
    }

    fn process_byte(&mut self, byte: u8) -> FilterAction {
        match self.state {
            FilterState::Normal => {
                match byte {
                    0x1b => {
                        self.state = FilterState::Escape;
                        match self.policy {
                            SanitizationPolicy::EscapeAll => FilterAction::EmitEscaped(byte),
                            _ => FilterAction::Skip,
                        }
                    }
                    0x00..=0x1f | 0x7f => {
                        // Control characters
                        match self.policy {
                            SanitizationPolicy::RemoveAll | SanitizationPolicy::RemoveDangerous => {
                                match byte {
                                    0x09 | 0x0a | 0x0d => FilterAction::Emit(byte), // Tab, LF, CR
                                    _ => FilterAction::Skip,
                                }
                            }
                            SanitizationPolicy::EscapeAll => FilterAction::EmitEscaped(byte),
                            SanitizationPolicy::AllowBasicFormatting => {
                                match byte {
                                    0x07 | 0x08 | 0x09 | 0x0a | 0x0d => FilterAction::Emit(byte), // BEL, BS, Tab, LF, CR
                                    _ => FilterAction::Skip,
                                }
                            }
                        }
                    }
                    _ => FilterAction::Emit(byte),
                }
            }
            FilterState::Escape => match byte {
                b'[' => {
                    self.state = FilterState::Csi;
                    FilterAction::Skip
                }
                b']' => {
                    self.state = FilterState::OscString;
                    FilterAction::Skip
                }
                b'P' => {
                    self.state = FilterState::DcsString;
                    FilterAction::Skip
                }
                _ => {
                    self.state = FilterState::Normal;
                    FilterAction::Skip
                }
            },
            FilterState::Csi => {
                if (0x40..=0x7e).contains(&byte) {
                    // CSI terminator
                    self.state = FilterState::Normal;
                }
                FilterAction::Skip
            }
            FilterState::OscString => {
                if byte == 0x07 || (byte == 0x1b) {
                    // OSC terminator (BEL or ESC)
                    self.state = if byte == 0x1b {
                        FilterState::Escape
                    } else {
                        FilterState::Normal
                    };
                }
                FilterAction::Skip
            }
            FilterState::DcsString => {
                if byte == 0x1b {
                    self.state = FilterState::Escape;
                }
                FilterAction::Skip
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FilterAction {
    Emit(u8),
    EmitEscaped(u8),
    Skip,
}
