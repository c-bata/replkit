# Design Document

## Overview

The console-input-output system provides a robust, cross-platform abstraction for terminal input/output operations. The design emphasizes safety, performance, and compatibility across diverse environments including native platforms (Unix, Windows) and constrained environments (WASM). The architecture separates platform-specific implementations from the common interface while ensuring consistent behavior and optimal performance on each target platform.

The system includes both ConsoleInput for handling keyboard input and events, and ConsoleOutput for efficient terminal rendering and cursor control, providing a complete terminal I/O solution.

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                  Language Bindings                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Rust Native │  │ Go Bindings │  │ Python (PyO3)       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                 ConsoleInput Trait                          │
│                 (replkit-core)                               │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│              Platform Implementations                       │
│                    (replkit-io)                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Unix I/O    │  │ Windows VT  │  │ Windows Legacy      │ │
│  │ (termios)   │  │ (VT seq)    │  │ (Win32 Console)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    OS/Platform APIs                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ libc        │  │ Win32 API   │  │ WASM Host Bridge    │ │
│  │ termios     │  │ Console API │  │ (Serialization)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **Platform Abstraction**: Common trait interface with platform-specific implementations
2. **Safety First**: RAII guards for terminal state management
3. **Performance**: Non-blocking I/O with efficient kernel primitives
4. **Compatibility**: Support for legacy systems and constrained environments
5. **Extensibility**: Plugin architecture for new platforms and features

## Components and Interfaces

### Core Trait Definitions

```rust
// In replkit-core/src/console.rs
use crate::{Key, KeyEvent, KeyModifiers};
use std::sync::{Arc, Mutex};

/// Helper trait for testing - allows downcasting to concrete types
pub trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    UnixVt,
    WindowsVt,
    WindowsLegacy,
    WasmBridge,
    Mock,
}

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

#[derive(Debug, Clone, PartialEq)]
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

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            foreground: None,
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            reverse: false,
        }
    }
}

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
                    output.push_str(&format!("{:02x}", b));
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
            FilterState::Escape => {
                match byte {
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
                }
            }
            FilterState::Csi => {
                if byte >= 0x40 && byte <= 0x7e {
                    // CSI terminator
                    self.state = FilterState::Normal;
                }
                FilterAction::Skip
            }
            FilterState::OscString => {
                if byte == 0x07 || (byte == 0x1b) {
                    // OSC terminator (BEL or ESC)
                    self.state = if byte == 0x1b { FilterState::Escape } else { FilterState::Normal };
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
```

### Error Handling

```rust
// In replkit-core/src/console.rs
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
            ConsoleError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ConsoleError::UnsupportedFeature { feature, platform } => {
                write!(f, "Feature '{}' not supported on platform '{}'", feature, platform)
            }
            ConsoleError::EventLoopError(e) => write!(f, "Event loop error: {:?}", e),
            ConsoleError::TerminalError(msg) => write!(f, "Terminal error: {}", msg),
            ConsoleError::ThreadError(msg) => write!(f, "Thread error: {}", msg),
            ConsoleError::CallbackError(msg) => write!(f, "Callback error: {}", msg),
            ConsoleError::WasmBridgeError(msg) => write!(f, "WASM bridge error: {}", msg),
        }
    }
}

impl std::error::Error for ConsoleError {}

pub type ConsoleResult<T> = Result<T, ConsoleError>;
```

### Platform Factory

```rust
// In replkit-io/src/lib.rs
use replkit_core::console::{ConsoleInput, ConsoleOutput, ConsoleResult};

pub fn create_console_io() -> ConsoleResult<(Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>)> {
    let input = create_console_input()?;
    let output = create_console_output()?;
    Ok((input, output))
}

pub fn create_console_input() -> ConsoleResult<Box<dyn ConsoleInput>> {
    #[cfg(unix)]
    {
        Ok(Box::new(unix::UnixConsoleInput::new()?))
    }
    
    #[cfg(windows)]
    {
        // Try VT mode first, fall back to legacy
        match windows::WindowsVtConsoleInput::new() {
            Ok(vt_input) => Ok(Box::new(vt_input)),
            Err(_) => Ok(Box::new(windows::WindowsLegacyConsoleInput::new()?)),
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Box::new(wasm::WasmBridgeConsoleInput::new()?))
    }
    
    #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
    {
        Err(ConsoleError::UnsupportedFeature {
            feature: "console input".to_string(),
            platform: std::env::consts::OS.to_string(),
        })
    }
}

pub fn create_console_output() -> ConsoleResult<Box<dyn ConsoleOutput>> {
    #[cfg(unix)]
    {
        Ok(Box::new(unix::UnixConsoleOutput::new()?))
    }
    
    #[cfg(windows)]
    {
        // Try VT mode first, fall back to legacy
        match windows::WindowsVtConsoleOutput::new() {
            Ok(vt_output) => Ok(Box::new(vt_output)),
            Err(_) => Ok(Box::new(windows::WindowsLegacyConsoleOutput::new()?)),
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Box::new(wasm::WasmBridgeConsoleOutput::new()?))
    }
    
    #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
    {
        Err(ConsoleError::UnsupportedFeature {
            feature: "console output".to_string(),
            platform: std::env::consts::OS.to_string(),
        })
    }
}

pub fn create_mock_console_io() -> (Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>) {
    (
        Box::new(mock::MockConsoleInput::new()),
        Box::new(mock::MockConsoleOutput::new()),
    )
}
```

## Platform-Specific Implementations

### Unix Implementation

```rust
// In replkit-io/src/unix.rs
use replkit_core::console::*;
use replkit_core::{KeyParser, KeyEvent};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::os::unix::io::RawFd;

// Event loop state machine for proper synchronization
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum EventLoopState {
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Stopping = 3,
}

// Separate inner state for safe thread sharing
struct UnixConsoleInputInner {
    // Terminal state
    stdin_fd: RawFd,
    
    // Event loop management
    event_loop_state: AtomicU8,
    wake_pipe: Mutex<Option<(RawFd, RawFd)>>, // (read_fd, write_fd)
    event_thread: Mutex<Option<JoinHandle<()>>>,
    
    // Key parsing
    key_parser: Mutex<KeyParser>,
    
    // Callbacks
    resize_callback: Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>,
    key_callback: Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>,
    
    // Window size tracking
    last_window_size: Mutex<Option<(u16, u16)>>,
}

pub struct UnixConsoleInput {
    inner: Arc<UnixConsoleInputInner>,
}

impl UnixConsoleInput {
    pub fn new() -> ConsoleResult<Self> {
        let stdin_fd = libc::STDIN_FILENO;
        
        // Verify we have a TTY
        if unsafe { libc::isatty(stdin_fd) } == 0 {
            return Err(ConsoleError::TerminalError(
                "stdin is not a TTY".to_string()
            ));
        }
        
        let inner = Arc::new(UnixConsoleInputInner {
            stdin_fd,
            event_loop_state: AtomicU8::new(EventLoopState::Stopped as u8),
            wake_pipe: Mutex::new(None),
            event_thread: Mutex::new(None),
            key_parser: Mutex::new(KeyParser::new()),
            resize_callback: Mutex::new(None),
            key_callback: Mutex::new(None),
            last_window_size: Mutex::new(None),
        });
        
        Ok(UnixConsoleInput { inner })
    }
    
    fn setup_raw_mode(&self) -> ConsoleResult<libc::termios> {
        let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
        
        // Get current terminal attributes
        if unsafe { libc::tcgetattr(self.inner.stdin_fd, &mut termios) } != 0 {
            return Err(ConsoleError::IoError(
                "Failed to get terminal attributes".to_string()
            ));
        }
        
        // Save original settings for return
        let original_termios = termios;
        
        // Configure raw mode
        termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        termios.c_iflag &= !(libc::IXON | libc::ICRNL);
        termios.c_cc[libc::VMIN] = 0;
        termios.c_cc[libc::VTIME] = 0;
        
        // Apply new settings
        if unsafe { libc::tcsetattr(self.inner.stdin_fd, libc::TCSANOW, &termios) } != 0 {
            return Err(ConsoleError::IoError(
                "Failed to set terminal attributes".to_string()
            ));
        }
        
        // Set non-blocking mode
        let flags = unsafe { libc::fcntl(self.inner.stdin_fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(ConsoleError::IoError(
                "Failed to get file flags".to_string()
            ));
        }
        
        if unsafe { libc::fcntl(self.inner.stdin_fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
            return Err(ConsoleError::IoError(
                "Failed to set non-blocking mode".to_string()
            ));
        }
        
        Ok(original_termios)
    }
    
    fn restore_terminal(&mut self) -> ConsoleResult<()> {
        if let Some(original) = self.original_termios.take() {
            if unsafe { libc::tcsetattr(self.stdin_fd, libc::TCSANOW, &original) } != 0 {
                return Err(ConsoleError::IoError(
                    "Failed to restore terminal attributes".to_string()
                ));
            }
        }
        Ok(())
    }
    
    fn create_wake_pipe(&self) -> ConsoleResult<()> {
        let mut pipe_fds = [0i32; 2];
        if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
            return Err(ConsoleError::IoError(
                "Failed to create wake pipe".to_string()
            ));
        }
        
        // Set CLOEXEC on both ends
        for &fd in &pipe_fds {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags != -1 {
                unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
            }
        }
        
        // Set non-blocking on read end
        let flags = unsafe { libc::fcntl(pipe_fds[0], libc::F_GETFL) };
        if flags != -1 {
            unsafe { libc::fcntl(pipe_fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }
        
        *self.inner.wake_pipe.lock().unwrap() = Some((pipe_fds[0], pipe_fds[1]));
        Ok(())
    }
    
    fn setup_signal_handlers(&self) -> ConsoleResult<()> {
        // Install SIGWINCH handler for window resize detection
        #[cfg(target_os = "linux")]
        {
            // Linux: Use signalfd for clean signal handling
            use libc::{signalfd, signalfd_siginfo, sigset_t, sigemptyset, sigaddset, sigprocmask, SIG_BLOCK};
            
            let mut mask: sigset_t = unsafe { std::mem::zeroed() };
            unsafe {
                sigemptyset(&mut mask);
                sigaddset(&mut mask, libc::SIGWINCH);
                sigprocmask(SIG_BLOCK, &mask, std::ptr::null_mut());
            }
            
            let signal_fd = unsafe { signalfd(-1, &mask, 0) };
            if signal_fd == -1 {
                return Err(ConsoleError::IoError("Failed to create signalfd".to_string()));
            }
            
            // Store signal_fd for use in event loop
            // (This would need to be added to the struct)
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Other Unix: Use sigaction + self-pipe
            use libc::{sigaction, sighandler_t};
            
            let (_, write_fd) = self.wake_pipe.ok_or_else(|| {
                ConsoleError::IoError("Wake pipe not initialized".to_string())
            })?;
            
            extern "C" fn sigwinch_handler(_: libc::c_int) {
                // Write to self-pipe to wake up poll()
                // Note: This is a simplified example - in real implementation,
                // we'd need to store write_fd in a static or use a different approach
                let wake_byte = [1u8];
                unsafe {
                    libc::write(write_fd, wake_byte.as_ptr() as *const libc::c_void, 1);
                }
            }
            
            let mut sa: sigaction = unsafe { std::mem::zeroed() };
            sa.sa_sigaction = sigwinch_handler as sighandler_t;
            
            if unsafe { sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut()) } != 0 {
                return Err(ConsoleError::IoError("Failed to install SIGWINCH handler".to_string()));
            }
        }
        
        Ok(())
    }
    
    fn event_loop_thread(inner: Arc<UnixConsoleInputInner>) {
        let stdin_fd = inner.stdin_fd;
        let (wake_read_fd, _) = *inner.wake_pipe.lock().unwrap().unwrap();
        let mut buffer = [0u8; 1024];
        
        'main_loop: while inner.event_loop_state.load(Ordering::Relaxed) == EventLoopState::Running as u8 {
            // Set up poll for stdin and wake pipe
            let mut poll_fds = [
                libc::pollfd {
                    fd: stdin_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: wake_read_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            
            // Poll with timeout for periodic size checks
            let poll_result = unsafe {
                libc::poll(poll_fds.as_mut_ptr(), 2, 100) // 100ms timeout
            };
            
            if poll_result < 0 {
                // Error in poll, exit loop
                break;
            }
            
            // Check for stdin input
            if poll_fds[0].revents & libc::POLLIN != 0 {
                loop {
                    let bytes_read = unsafe {
                        libc::read(stdin_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
                    };
                    
                    if bytes_read > 0 {
                        let input = &buffer[..bytes_read as usize];
                        
                        // Parse key events using shared parser instance
                        let key_events = {
                            let mut parser = inner.key_parser.lock().unwrap();
                            parser.feed(input)
                        };
                        
                        // Invoke key callback for each event (avoid holding lock during callback)
                        let callback = {
                            let mut callback_guard = inner.key_callback.lock().unwrap();
                            callback_guard.take()
                        };
                        
                        if let Some(mut callback) = callback {
                            for event in key_events {
                                // Catch panics in user callback
                                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    callback(event);
                                }));
                            }
                            
                            // Restore callback
                            *inner.key_callback.lock().unwrap() = Some(callback);
                        }
                        
                        // Continue reading if there might be more data
                        continue;
                    } else if bytes_read == 0 {
                        // EOF - stdin closed
                        break 'main_loop;
                    } else {
                        // Error occurred
                        let errno = unsafe { *libc::__errno_location() };
                        match errno {
                            libc::EAGAIN | libc::EWOULDBLOCK => {
                                // No more data available, continue with poll
                                break;
                            }
                            libc::EINTR => {
                                // Interrupted by signal, retry read
                                continue;
                            }
                            _ => {
                                // Other error, exit loop
                                break 'main_loop;
                            }
                        }
                    }
                }
            }
            
            // Check for wake signal
            if poll_fds[1].revents & libc::POLLIN != 0 {
                // Drain wake pipe completely
                loop {
                    let bytes_read = unsafe {
                        libc::read(wake_read_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
                    };
                    if bytes_read <= 0 {
                        break;
                    }
                }
                // Wake signal received, check if we should exit
                if inner.event_loop_state.load(Ordering::Relaxed) == EventLoopState::Stopping as u8 {
                    break 'main_loop;
                }
            }
            
            // Periodic window size check (with debouncing)
            if let Ok(current_size) = Self::query_window_size_static(stdin_fd) {
                let size_changed = {
                    let mut last_size_guard = inner.last_window_size.lock().unwrap();
                    let changed = match *last_size_guard {
                        Some(last) => last != current_size,
                        None => true,
                    };
                    if changed {
                        *last_size_guard = Some(current_size);
                    }
                    changed
                };
                
                if size_changed {
                    // Invoke resize callback (avoid holding lock during callback)
                    let callback = {
                        let mut callback_guard = inner.resize_callback.lock().unwrap();
                        callback_guard.take()
                    };
                    
                    if let Some(mut callback) = callback {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            callback(current_size.0, current_size.1);
                        }));
                        
                        // Restore callback
                        *inner.resize_callback.lock().unwrap() = Some(callback);
                    }
                }
            }
        }
    }
    
    fn query_window_size_static(fd: RawFd) -> ConsoleResult<(u16, u16)> {
        let mut winsize = unsafe { std::mem::zeroed::<libc::winsize>() };
        
        if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize) } == 0 {
            // Return visible window size (equivalent to Windows srWindow)
            // ws_col/ws_row represent the visible terminal area, not buffer size
            Ok((winsize.ws_col, winsize.ws_row))
        } else {
            Err(ConsoleError::IoError(
                "Failed to query window size".to_string()
            ))
        }
    }
}

impl AsAny for UnixConsoleInput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ConsoleInput for UnixConsoleInput {
    fn enable_raw_mode(&mut self) -> ConsoleResult<RawModeGuard> {
        let original_termios = self.setup_raw_mode()?;
        
        let stdin_fd = self.inner.stdin_fd;
        
        let restore_fn = move || {
            unsafe {
                libc::tcsetattr(stdin_fd, libc::TCSANOW, &original_termios);
            }
        };
        
        Ok(RawModeGuard::new(
            restore_fn,
            "Unix VT (termios)".to_string(),
        ))
    }
    
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        Self::query_window_size_static(self.stdin_fd)
    }
    
    fn start_event_loop(&self) -> ConsoleResult<()> {
        // Use compare_exchange to prevent race conditions
        match self.inner.event_loop_state.compare_exchange(
            EventLoopState::Stopped as u8,
            EventLoopState::Starting as u8,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // Successfully transitioned to Starting state
            }
            Err(current) => {
                let current_state = match current {
                    x if x == EventLoopState::Starting as u8 => EventLoopState::Starting,
                    x if x == EventLoopState::Running as u8 => EventLoopState::Running,
                    x if x == EventLoopState::Stopping as u8 => EventLoopState::Stopping,
                    _ => EventLoopState::Stopped,
                };
                
                return match current_state {
                    EventLoopState::Starting | EventLoopState::Running => {
                        Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning))
                    }
                    EventLoopState::Stopping => {
                        Err(ConsoleError::EventLoopError(EventLoopError::StartupFailed(
                            "Event loop is currently stopping".to_string()
                        )))
                    }
                    _ => unreachable!(),
                };
            }
        }
        
        // Create wake pipe for clean shutdown
        self.create_wake_pipe()?;
        
        // Set up signal handlers
        self.setup_signal_handlers()?;
        
        // Start event loop thread
        let inner_clone = Arc::clone(&self.inner);
        let handle = thread::spawn(move || {
            // Mark as running
            inner_clone.event_loop_state.store(EventLoopState::Running as u8, Ordering::SeqCst);
            
            Self::event_loop_thread(inner_clone.clone());
            
            // Mark as stopped
            inner_clone.event_loop_state.store(EventLoopState::Stopped as u8, Ordering::SeqCst);
        });
        
        *self.inner.event_thread.lock().unwrap() = Some(handle);
        Ok(())
    }
    
    fn stop_event_loop(&self) -> ConsoleResult<()> {
        // Use compare_exchange to prevent race conditions
        match self.inner.event_loop_state.compare_exchange(
            EventLoopState::Running as u8,
            EventLoopState::Stopping as u8,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // Successfully transitioned to Stopping state
            }
            Err(current) => {
                let current_state = match current {
                    x if x == EventLoopState::Stopped as u8 => EventLoopState::Stopped,
                    x if x == EventLoopState::Starting as u8 => EventLoopState::Starting,
                    x if x == EventLoopState::Stopping as u8 => EventLoopState::Stopping,
                    _ => EventLoopState::Running,
                };
                
                return match current_state {
                    EventLoopState::Stopped => {
                        Err(ConsoleError::EventLoopError(EventLoopError::NotRunning))
                    }
                    EventLoopState::Starting => {
                        Err(ConsoleError::EventLoopError(EventLoopError::StartupFailed(
                            "Event loop is currently starting".to_string()
                        )))
                    }
                    EventLoopState::Stopping => {
                        // Already stopping, this is okay
                        return Ok(());
                    }
                    _ => unreachable!(),
                };
            }
        }
        
        // Wake up the event loop
        if let Some((_, write_fd)) = *self.inner.wake_pipe.lock().unwrap() {
            let wake_byte = [1u8];
            unsafe {
                libc::write(write_fd, wake_byte.as_ptr() as *const libc::c_void, 1);
            }
        }
        
        // Wait for thread to finish
        if let Some(handle) = self.inner.event_thread.lock().unwrap().take() {
            match handle.join() {
                Ok(_) => {}
                Err(_) => {
                    return Err(ConsoleError::EventLoopError(
                        EventLoopError::ThreadPanic("Event loop thread panicked".to_string())
                    ));
                }
            }
        }
        
        // Clean up wake pipe
        if let Some((read_fd, write_fd)) = self.inner.wake_pipe.lock().unwrap().take() {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
        }
        
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        let state = self.inner.event_loop_state.load(Ordering::Relaxed);
        state == EventLoopState::Running as u8 || state == EventLoopState::Starting as u8
    }
    
    fn stop_event_loop(&mut self) -> ConsoleResult<()> {
        if !self.event_loop_running.load(Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(EventLoopError::NotRunning));
        }
        
        // Signal thread to stop
        self.event_loop_running.store(false, Ordering::Relaxed);
        
        // Wake up the event loop
        if let Some((_, write_fd)) = self.wake_pipe {
            let wake_byte = [1u8];
            unsafe {
                libc::write(write_fd, wake_byte.as_ptr() as *const libc::c_void, 1);
            }
        }
        
        // Wait for thread to finish
        if let Some(handle) = self.event_thread.take() {
            match handle.join() {
                Ok(_) => {}
                Err(_) => {
                    return Err(ConsoleError::EventLoopError(
                        EventLoopError::ThreadPanic("Event loop thread panicked".to_string())
                    ));
                }
            }
        }
        
        // Clean up wake pipe
        if let Some((read_fd, write_fd)) = self.wake_pipe.take() {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
        }
        
        Ok(())
    }
    
    fn on_window_resize(&mut self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        *self.resize_callback.lock().unwrap() = Some(callback);
    }
    
    fn on_key_pressed(&mut self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        *self.key_callback.lock().unwrap() = Some(callback);
    }
    
    fn is_running(&self) -> bool {
        self.event_loop_running.load(Ordering::Relaxed)
    }
    
    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: true,
            supports_bracketed_paste: true,
            supports_mouse_events: true,
            supports_unicode: true,
            platform_name: "Unix/Linux".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
}

impl Drop for UnixConsoleInput {
    fn drop(&mut self) {
        let _ = self.stop_event_loop();
        let _ = self.restore_terminal();
    }
}
```

```rust
// Unix Console Output Implementation
pub struct UnixConsoleOutput {
    stdout_fd: RawFd,
    output_buffer: Vec<u8>,
    current_style: TextStyle,
    cursor_visible: bool,
    alternate_screen: bool,
}

impl UnixConsoleOutput {
    pub fn new() -> ConsoleResult<Self> {
        Ok(UnixConsoleOutput {
            stdout_fd: libc::STDOUT_FILENO,
            output_buffer: Vec::new(),
            current_style: TextStyle::default(),
            cursor_visible: true,
            alternate_screen: false,
        })
    }
    
    fn write_ansi_sequence(&mut self, sequence: &[u8]) {
        self.output_buffer.extend_from_slice(sequence);
    }
    
    fn style_to_ansi(&self, style: &TextStyle) -> Vec<u8> {
        let mut ansi = Vec::new();
        ansi.extend_from_slice(b"\x1b[0m"); // Reset
        
        if let Some(fg) = style.foreground {
            ansi.extend_from_slice(&self.color_to_ansi_fg(fg));
        }
        if let Some(bg) = style.background {
            ansi.extend_from_slice(&self.color_to_ansi_bg(bg));
        }
        if style.bold { ansi.extend_from_slice(b"\x1b[1m"); }
        if style.italic { ansi.extend_from_slice(b"\x1b[3m"); }
        if style.underline { ansi.extend_from_slice(b"\x1b[4m"); }
        if style.dim { ansi.extend_from_slice(b"\x1b[2m"); }
        if style.reverse { ansi.extend_from_slice(b"\x1b[7m"); }
        if style.strikethrough { ansi.extend_from_slice(b"\x1b[9m"); }
        
        ansi
    }
    
    fn color_to_ansi_fg(&self, color: Color) -> Vec<u8> {
        match color {
            Color::Black => b"\x1b[30m".to_vec(),
            Color::Red => b"\x1b[31m".to_vec(),
            Color::Green => b"\x1b[32m".to_vec(),
            Color::Yellow => b"\x1b[33m".to_vec(),
            Color::Blue => b"\x1b[34m".to_vec(),
            Color::Magenta => b"\x1b[35m".to_vec(),
            Color::Cyan => b"\x1b[36m".to_vec(),
            Color::White => b"\x1b[37m".to_vec(),
            Color::BrightBlack => b"\x1b[90m".to_vec(),
            Color::BrightRed => b"\x1b[91m".to_vec(),
            Color::BrightGreen => b"\x1b[92m".to_vec(),
            Color::BrightYellow => b"\x1b[93m".to_vec(),
            Color::BrightBlue => b"\x1b[94m".to_vec(),
            Color::BrightMagenta => b"\x1b[95m".to_vec(),
            Color::BrightCyan => b"\x1b[96m".to_vec(),
            Color::BrightWhite => b"\x1b[97m".to_vec(),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b).into_bytes(),
            Color::Ansi256(n) => format!("\x1b[38;5;{}m", n).into_bytes(),
        }
    }
    
    fn color_to_ansi_bg(&self, color: Color) -> Vec<u8> {
        match color {
            Color::Black => b"\x1b[40m".to_vec(),
            Color::Red => b"\x1b[41m".to_vec(),
            Color::Green => b"\x1b[42m".to_vec(),
            Color::Yellow => b"\x1b[43m".to_vec(),
            Color::Blue => b"\x1b[44m".to_vec(),
            Color::Magenta => b"\x1b[45m".to_vec(),
            Color::Cyan => b"\x1b[46m".to_vec(),
            Color::White => b"\x1b[47m".to_vec(),
            Color::BrightBlack => b"\x1b[100m".to_vec(),
            Color::BrightRed => b"\x1b[101m".to_vec(),
            Color::BrightGreen => b"\x1b[102m".to_vec(),
            Color::BrightYellow => b"\x1b[103m".to_vec(),
            Color::BrightBlue => b"\x1b[104m".to_vec(),
            Color::BrightMagenta => b"\x1b[105m".to_vec(),
            Color::BrightCyan => b"\x1b[106m".to_vec(),
            Color::BrightWhite => b"\x1b[107m".to_vec(),
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b).into_bytes(),
            Color::Ansi256(n) => format!("\x1b[48;5;{}m", n).into_bytes(),
        }
    }
}

impl AsAny for UnixConsoleOutput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ConsoleOutput for UnixConsoleOutput {
    fn write_text(&mut self, text: &str) -> ConsoleResult<()> {
        self.output_buffer.extend_from_slice(text.as_bytes());
        Ok(())
    }
    
    fn write_styled_text(&mut self, text: &str, style: &TextStyle) -> ConsoleResult<()> {
        let ansi = self.style_to_ansi(style);
        self.output_buffer.extend_from_slice(&ansi);
        self.output_buffer.extend_from_slice(text.as_bytes());
        self.write_ansi_sequence(b"\x1b[0m"); // Reset after text
        Ok(())
    }
    
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
        // Convert from 0-based API coordinates to 1-based ANSI coordinates
        // API: (0,0) = top-left, ANSI: (1,1) = top-left
        let sequence = format!("\x1b[{};{}H", row + 1, col + 1);
        self.write_ansi_sequence(sequence.as_bytes());
        Ok(())
    }
    
    fn move_cursor_relative(&mut self, col_delta: i16, row_delta: i16) -> ConsoleResult<()> {
        if row_delta > 0 {
            let sequence = format!("\x1b[{}B", row_delta);
            self.write_ansi_sequence(sequence.as_bytes());
        } else if row_delta < 0 {
            let sequence = format!("\x1b[{}A", -row_delta);
            self.write_ansi_sequence(sequence.as_bytes());
        }
        
        if col_delta > 0 {
            let sequence = format!("\x1b[{}C", col_delta);
            self.write_ansi_sequence(sequence.as_bytes());
        } else if col_delta < 0 {
            let sequence = format!("\x1b[{}D", -col_delta);
            self.write_ansi_sequence(sequence.as_bytes());
        }
        
        Ok(())
    }
    
    fn clear(&mut self, clear_type: ClearType) -> ConsoleResult<()> {
        let sequence = match clear_type {
            ClearType::All => b"\x1b[2J",
            ClearType::FromCursor => b"\x1b[0J",
            ClearType::ToCursor => b"\x1b[1J",
            ClearType::CurrentLine => b"\x1b[2K",
            ClearType::FromCursorToEndOfLine => b"\x1b[0K",
            ClearType::FromBeginningOfLineToCursor => b"\x1b[1K",
        };
        self.write_ansi_sequence(sequence);
        Ok(())
    }
    
    fn set_style(&mut self, style: &TextStyle) -> ConsoleResult<()> {
        self.current_style = style.clone();
        let ansi = self.style_to_ansi(style);
        self.write_ansi_sequence(&ansi);
        Ok(())
    }
    
    fn reset_style(&mut self) -> ConsoleResult<()> {
        self.current_style = TextStyle::default();
        self.write_ansi_sequence(b"\x1b[0m");
        Ok(())
    }
    
    fn flush(&mut self) -> ConsoleResult<()> {
        if !self.output_buffer.is_empty() {
            let bytes_written = unsafe {
                libc::write(
                    self.stdout_fd,
                    self.output_buffer.as_ptr() as *const libc::c_void,
                    self.output_buffer.len(),
                )
            };
            
            if bytes_written < 0 {
                return Err(ConsoleError::IoError("Failed to write to stdout".to_string()));
            }
            
            self.output_buffer.clear();
        }
        Ok(())
    }
    
    fn set_alternate_screen(&mut self, enabled: bool) -> ConsoleResult<()> {
        if enabled && !self.alternate_screen {
            self.write_ansi_sequence(b"\x1b[?1049h");
            self.alternate_screen = true;
        } else if !enabled && self.alternate_screen {
            self.write_ansi_sequence(b"\x1b[?1049l");
            self.alternate_screen = false;
        }
        Ok(())
    }
    
    fn set_cursor_visible(&mut self, visible: bool) -> ConsoleResult<()> {
        if visible && !self.cursor_visible {
            self.write_ansi_sequence(b"\x1b[?25h");
            self.cursor_visible = true;
        } else if !visible && self.cursor_visible {
            self.write_ansi_sequence(b"\x1b[?25l");
            self.cursor_visible = false;
        }
        Ok(())
    }
    
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        // Cursor position query requires CPR (Cursor Position Report) via DSR sequence:
        // 1. Send "\x1b[6n" to stdout
        // 2. Read response "\x1b[{row};{col}R" from stdin
        // 
        // This creates a dependency between ConsoleOutput and ConsoleInput, which
        // violates the current API boundary design. Future implementation would need:
        // - Shared communication channel between Input/Output
        // - Timeout handling for unresponsive terminals
        // - Proper sequence parsing in the input stream
        //
        // For now, return UnsupportedFeature to maintain clean API boundaries
        Err(ConsoleError::UnsupportedFeature {
            feature: "cursor position query".to_string(),
            platform: "Unix (requires Input/Output coordination)".to_string(),
        })
    }
    
    fn get_capabilities(&self) -> OutputCapabilities {
        // Detect true color support through environment variables or runtime testing
        let supports_true_color = Self::detect_true_color_support();
        let max_colors = if supports_true_color { 16777216 } else { 256 };
        
        OutputCapabilities {
            supports_colors: true,
            supports_true_color,
            supports_styling: true,
            supports_alternate_screen: true,
            supports_cursor_control: true,
            max_colors,
            platform_name: "Unix/Linux".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
    
    fn detect_true_color_support() -> bool {
        // Check environment variables first
        if let Ok(colorterm) = std::env::var("COLORTERM") {
            if colorterm == "truecolor" || colorterm == "24bit" {
                return true;
            }
        }
        
        // Check TERM variable for known true color terminals
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("256color") || term.contains("truecolor") {
                return true;
            }
        }
        
        // Runtime detection could be added here:
        // 1. Send a true color sequence
        // 2. Query terminal capabilities
        // 3. Check if the sequence was interpreted correctly
        // For now, default to true on modern Unix systems
        true
    }
}
```

## Multi-Language Binding Strategy

This section outlines the strategy for providing idiomatic APIs in different programming languages, leveraging the core Rust implementation.

### Go

The Go binding adopts a hybrid approach to balance performance-critical operations with implementation efficiency:

*   **ConsoleInput**: Implemented **natively in Go** for direct, low-level access to platform APIs (e.g., `termios`, `select`), ensuring maximum input responsiveness.
*   **ConsoleOutput**: Implemented by wrapping the Rust core via **WASM**, reusing the robust, cross-platform rendering logic and ensuring consistent output.

This diagram illustrates the hybrid strategy for Go bindings:

```
                                  ┌──────────────────────────┐
                                  │      Go Application      │
                                  └──────────────────────────┘
                                               │
                       ┌───────────────────────┴───────────────────────┐
                       │                     Go Layer                  │
                       ├───────────────────────────────────────────────┤
                       │                                               │
           ┌───────────┴───────────┐                   ┌───────────────┴───────────────┐
           │  ConsoleInput (Native)│                   │  ConsoleOutput (WASM Wrapper) │
           │ (Go implementation of │                   │ (Thin wrapper in Go)          │
           │  the ConsoleInput     │                   │                               │
           │  interface)           │                   └───────────────┬───────────────┘
           └───────────┬───────────┘                                   │ (WASM Call)
                       │                                               │
                       │ (Direct system calls)                         │
                       ▼                                               ▼
        ┌──────────────────────────┐                   ┌──────────────────────────────────┐
        │    OS/Platform APIs      │                   │ Rust Core (Compiled to WASM)     │
        │ (termios, select, Win32) │                   │   - ANSI sequence generation     │
        └──────────────────────────┘                   │   - Style management             │
                                                       └──────────────────────────────────┘
```

This architecture provides the "best of both worlds": the performance of native code for time-sensitive input handling and the robustness and development speed of reusing the core Rust logic for complex output rendering.

#### WASM Bridge for ConsoleOutput

This section details the design of the WASM bridge used to expose the Rust `ConsoleOutput` implementation to other language bindings, specifically for the Go binding. The goal is to reuse the complex, cross-platform rendering logic of the Rust core, ensuring consistent terminal output while minimizing reimplementation effort in the host language (Go).

The bridge operates via a single, command-based function exported from WASM. The Go wrapper serializes output commands into JSON and passes them to this function.

##### Architecture

```
┌──────────────────────────────┐
│     Go ConsoleOutput Wrapper │
└──────────────┬───────────────┘
               │ 1. Serialize command to JSON
               ▼
┌──────────────────────────────┐
│      WASM Host Runtime       │
└──────────────┬��──────────────┘
               │ 2. Call exported WASM function
               ▼
┌──────────────────────────────────┐
│  WASM Module (from replkit-core)  │
│                                  │
│  ┌───────────────────────────┐   │
│  │ `wasm_output_command`     │   │
│  │  - Deserialize JSON       │   │
│  │  - Call Rust method       │   │
│  └───────────┬───────────────┘   │
│              │ 3. Execute command│
│              ▼                   │
│  ┌───────────────────────────┐   │
│  │ Rust `ConsoleOutput` Impl │   │
│  │  - Buffer ANSI codes      │   │
│  └───────────┬───────────────┘   │
│              │ 4. On flush       │
│              ▼                   │
│  ┌───────────────────��───────┐   │
│  │          stdout           │   │
│  └───────────────────────────┘   │
└──────────────────────────────────┘
```

##### Command-based Interface

A single function is exported from the WASM module to handle all output operations. This simplifies the foreign function interface (FFI) boundary.

```rust
// In replkit-wasm/src/lib.rs

#[no_mangle]
pub extern "C" fn wasm_output_command(command_ptr: *const u8, command_len: usize) -> i32 {
    // 1. Reconstruct the JSON string from the pointer and length
    let command_json = unsafe {
        let slice = std::slice::from_raw_parts(command_ptr, command_len);
        std::str::from_utf8(slice).unwrap_or("")
    };

    // 2. Deserialize the command
    let command: Result<OutputCommand, _> = serde_json::from_str(command_json);

    // 3. Get the global ConsoleOutput instance
    let mut output = CONSOLE_OUTPUT.lock().unwrap();

    // 4. Execute the command
    match command {
        Ok(cmd) => match output.execute(cmd) {
            Ok(_) => 0, // Success
            Err(_) => -1, // Execution error
        },
        Err(_) => -2, // Deserialization error
    }
}
```

##### JSON Command Structure

The Go wrapper creates JSON objects representing the desired output operation.

**Enum for commands:**
```json
{
  "command": "<CommandType>",
  "payload": { ... }
}
```

**Example Payloads:**

*   **WriteText**:
    ```json
    {
      "command": "WriteText",
      "payload": {
        "text": "Hello, World!"
      }
    }
    ```

*   **SetStyle**:
    ```json
    {
      "command": "SetStyle",
      "payload": {
        "foreground": { "type": "Rgb", "r": 100, "g": 200, "b": 50 },
        "background": null,
        "bold": true,
        "italic": false
      }
    }
    ```

*   **MoveCursorTo**:
    ```json
    {
      "command": "MoveCursorTo",
      "payload": {
        "row": 10,
        "col": 5
      }
    }
    ```

*   **Flush**:
    ```json
    {
      "command": "Flush",
      "payload": {}
    }
    ```

This design provides a clean, extensible, and serializable interface between the Go host and the WASM guest, allowing the Go binding to leverage the full power of the Rust `ConsoleOutput` implementation without direct access to low-level Rust objects.

### Python Binding

The Python binding is implemented using **PyO3**. This provides a thin, efficient wrapper around the native Rust implementation, exposing the `ConsoleInput` and `ConsoleOutput` traits directly to Python. This allows Python applications to benefit from the near-native performance of the Rust core.

```python
# crates/replkit-pyo3/src/console.rs
use pyo3::prelude::*;
use pyo3::types::PyFunction;
use replkit_core::console::*;
use std::sync::{Arc, Mutex};

#[pyclass]
pub struct PyConsoleInput {
    inner: Box<dyn ConsoleInput>,
    key_callback: Arc<Mutex<Option<PyObject>>>,
    resize_callback: Arc<Mutex<Option<PyObject>>>,
}

#[pymethods]
impl PyConsoleInput {
    #[new]
    fn new() -> PyResult<Self> {
        let console_input = replkit_terminal::create_console_input()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(PyConsoleInput {
            inner: console_input,
            key_callback: Arc::new(Mutex::new(None)),
            resize_callback: Arc::new(Mutex::new(None)),
        })
    }
    
    fn enable_raw_mode(&mut self) -> PyResult<PyRawModeGuard> {
        let guard = self.inner.enable_raw_mode()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(PyRawModeGuard { inner: Some(guard) })
    }
    
    fn get_window_size(&self) -> PyResult<(u16, u16)> {
        self.inner.get_window_size()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    fn start_event_loop(&mut self) -> PyResult<()> {
        self.inner.start_event_loop()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    fn stop_event_loop(&mut self) -> PyResult<()> {
        self.inner.stop_event_loop()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    fn on_key_pressed(&mut self, callback: PyObject) -> PyResult<()> {
        *self.key_callback.lock().unwrap() = Some(callback.clone());
        
        let callback_ref = Arc::clone(&self.key_callback);
        self.inner.on_key_pressed(Box::new(move |event| {
            Python::with_gil(|py| {
                if let Some(ref callback) = *callback_ref.lock().unwrap() {
                    let py_event = PyKeyEvent::from_rust_event(event);
                    let _ = callback.call1(py, (py_event,));
                }
            });
        }));
        
        Ok(())
    }
    
    fn on_window_resize(&mut self, callback: PyObject) -> PyResult<()> {
        *self.resize_callback.lock().unwrap() = Some(callback.clone());
        
        let callback_ref = Arc::clone(&self.resize_callback);
        self.inner.on_window_resize(Box::new(move |cols, rows| {
            Python::with_gil(|py| {
                if let Some(ref callback) = *callback_ref.lock().unwrap() {
                    let _ = callback.call1(py, (cols, rows));
                }
            });
        }));
        
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}

#[pyclass]
pub struct PyKeyEvent {
    #[pyo3(get)]
    key: u32,
    #[pyo3(get)]
    raw_bytes: Vec<u8>,
    #[pyo3(get)]
    text: Option<String>,
}

impl PyKeyEvent {
    fn from_rust_event(event: KeyEvent) -> Self {
        PyKeyEvent {
            key: crate::wasm::key_to_u32(event.key),
            raw_bytes: event.raw_bytes,
            text: event.text,
        }
    }
}

#[pyclass]
pub struct PyRawModeGuard {
    inner: Option<RawModeGuard>,
}

#[pymethods]
impl PyRawModeGuard {
    fn platform_info(&self) -> Option<String> {
        self.inner.as_ref().map(|g| g.platform_info().to_string())
    }
    
    fn close(&mut self) -> PyResult<()> {
        self.inner.take();
        Ok(())
    }
}

impl Drop for PyRawModeGuard {
    fn drop(&mut self) {
        self.inner.take();
    }
}
```

## Testing Strategy

### Advanced Testing Framework

#### 1. TTY Pseudo-Terminal Testing (Unix)

```rust
#[cfg(unix)]
mod pty_tests {
    use super::*;
    use libc::{openpty, write, close};
    use std::os::unix::io::RawFd;
    
    struct PtyPair {
        master_fd: RawFd,
        slave_fd: RawFd,
    }
    
    impl PtyPair {
        fn new() -> Result<Self, std::io::Error> {
            let mut master_fd = 0;
            let mut slave_fd = 0;
            
            if unsafe { openpty(&mut master_fd, &mut slave_fd, std::ptr::null_mut(), std::ptr::null(), std::ptr::null()) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            
            Ok(PtyPair { master_fd, slave_fd })
        }
        
        fn write_to_slave(&self, data: &[u8]) -> Result<(), std::io::Error> {
            let written = unsafe { write(self.master_fd, data.as_ptr() as *const libc::c_void, data.len()) };
            if written < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
    
    impl Drop for PtyPair {
        fn drop(&mut self) {
            unsafe {
                close(self.master_fd);
                close(self.slave_fd);
            }
        }
    }
    
    #[test]
    fn test_real_terminal_key_parsing() {
        let pty = PtyPair::new().expect("Failed to create PTY");
        
        // Create console input using slave side of PTY
        let mut console_input = UnixConsoleInput::new_with_fd(pty.slave_fd).unwrap();
        
        let received_keys = Arc::new(Mutex::new(Vec::new()));
        let keys_clone = Arc::clone(&received_keys);
        
        console_input.on_key_pressed(Box::new(move |event| {
            keys_clone.lock().unwrap().push(event);
        }));
        
        console_input.start_event_loop().unwrap();
        
        // Send key sequences through master side
        pty.write_to_slave(b"\x1b[A").unwrap(); // Up arrow
        pty.write_to_slave(b"\x1b[B").unwrap(); // Down arrow
        pty.write_to_slave(b"hello").unwrap();  // Text
        
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        console_input.stop_event_loop().unwrap();
        
        let keys = received_keys.lock().unwrap();
        assert_eq!(keys.len(), 7); // Up, Down, h, e, l, l, o
        assert_eq!(keys[0].key, Key::Up);
        assert_eq!(keys[1].key, Key::Down);
    }
}
```

#### 2. ANSI Sequence Golden Testing

```rust
#[cfg(test)]
mod golden_tests {
    use super::*;
    use insta::assert_snapshot;
    
    #[test]
    fn test_ansi_sequence_generation() {
        let mut output = UnixConsoleOutput::new().unwrap();
        
        // Test cursor movement (API uses 0-based coordinates)
        output.move_cursor_to(5, 10).unwrap();  // Should generate \x1b[6;11H (1-based ANSI)
        output.move_cursor_relative(-2, 3).unwrap();
        
        // Test styling
        let style = TextStyle {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
            bold: true,
            italic: true,
            ..Default::default()
        };
        output.set_style(&style).unwrap();
        output.write_text("Hello, World!").unwrap();
        output.reset_style().unwrap();
        
        // Capture generated ANSI sequences
        let ansi_output = output.get_output_buffer();
        let ansi_string = String::from_utf8_lossy(&ansi_output);
        
        // Snapshot test - will create/update golden file
        assert_snapshot!(ansi_string);
    }
    
    #[test]
    fn test_coordinate_system_consistency() {
        let mut output = UnixConsoleOutput::new().unwrap();
        
        // Test 0-based API to 1-based ANSI conversion
        output.move_cursor_to(0, 0).unwrap();  // Top-left corner
        let ansi_output = String::from_utf8_lossy(&output.get_output_buffer());
        assert!(ansi_output.contains("\x1b[1;1H"), "Should convert (0,0) to ANSI (1,1)");
        
        output.clear_output_buffer();
        output.move_cursor_to(10, 20).unwrap();
        let ansi_output = String::from_utf8_lossy(&output.get_output_buffer());
        assert!(ansi_output.contains("\x1b[11;21H"), "Should convert (10,20) to ANSI (11,21)");
    }
}
```

#### 3. Property-Based Control Sequence Filtering

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult, Arbitrary, Gen};
    use rand::Rng;
    
    #[derive(Debug, Clone)]
    struct MixedText {
        content: String,
    }
    
    impl Arbitrary for MixedText {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut content = String::new();
            let len = g.gen_range(0..200);
            
            for _ in 0..len {
                match g.gen_range(0..10) {
                    0..=6 => {
                        // Regular printable character
                        content.push(g.gen_range('a'..='z'));
                    }
                    7 => {
                        // CSI sequence
                        content.push_str(&format!("\x1b[{}m", g.gen_range(0..100)));
                    }
                    8 => {
                        // OSC sequence
                        content.push_str(&format!("\x1b]0;{}\x07", 
                            (0..g.gen_range(1..20)).map(|_| g.gen_range('A'..='Z')).collect::<String>()));
                    }
                    9 => {
                        // Random control character
                        content.push(g.gen_range('\x00'..='\x1f'));
                    }
                    _ => unreachable!(),
                }
            }
            
            MixedText { content }
        }
    }
    
    #[quickcheck]
    fn prop_safe_text_removes_all_control_sequences(mixed: MixedText) -> TestResult {
        if mixed.content.len() > 1000 {
            return TestResult::discard();
        }
        
        let mut filter = SafeTextFilter::new(SanitizationPolicy::RemoveAll);
        let filtered = filter.filter(&mixed.content);
        
        // Property: filtered text should contain no control sequences
        for byte in filtered.bytes() {
            if byte < 0x20 && byte != 0x09 && byte != 0x0a && byte != 0x0d {
                return TestResult::failed();
            }
        }
        
        // Property: no escape sequences should remain
        if filtered.contains('\x1b') {
            return TestResult::failed();
        }
        
        TestResult::passed()
    }
}
```

### Unit Testing Framework

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use replkit_terminal::create_mock_console_input;
    
    #[test]
    fn test_mock_console_basic_functionality() {
        let (mut console, mut output) = create_mock_console_io();
        
        // Test window size
        assert_eq!(console.get_window_size().unwrap(), (80, 24));
        
        // Test raw mode
        let _guard = console.enable_raw_mode().unwrap();
        
        // Test event loop
        assert!(!console.is_running());
        console.start_event_loop().unwrap();
        assert!(console.is_running());
        console.stop_event_loop().unwrap();
        assert!(!console.is_running());
    }
    
    #[test]
    fn test_key_event_callbacks() {
        let (mut console, _) = create_mock_console_io();
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&received_events);
        
        console.on_key_pressed(Box::new(move |event| {
            events_clone.lock().unwrap().push(event);
        }));
        
        // Queue test events
        if let Some(mock) = console.as_any_mut().downcast_mut::<MockConsoleInput>() {
            mock.queue_key_sequence(&[Key::ControlA, Key::Right, Key::Enter]);
            mock.process_queued_events();
        }
        
        let events = received_events.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].key, Key::ControlA);
        assert_eq!(events[1].key, Key::Right);
        assert_eq!(events[2].key, Key::Enter);
    }
    
    #[test]
    fn test_resize_event_callbacks() {
        let (mut console, _) = create_mock_console_io();
        let received_sizes = Arc::new(Mutex::new(Vec::new()));
        let sizes_clone = Arc::clone(&received_sizes);
        
        console.on_window_resize(Box::new(move |cols, rows| {
            sizes_clone.lock().unwrap().push((cols, rows));
        }));
        
        // Queue resize events
        if let Some(mock) = console.as_any_mut().downcast_mut::<MockConsoleInput>() {
            mock.queue_resize_event(120, 30);
            mock.queue_resize_event(100, 25);
            mock.process_queued_events();
        }
        
        let sizes = received_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 2);
        assert_eq!(sizes[0], (120, 30));
        assert_eq!(sizes[1], (100, 25));
    }
    
    #[test]
    fn test_raw_mode_guard_restoration() {
        let (mut console, _) = create_mock_console_io();
        
        {
            let _guard = console.enable_raw_mode().unwrap();
            if let Some(mock) = console.as_any().downcast_ref::<MockConsoleInput>() {
                assert!(mock.is_raw_mode_enabled());
            }
        } // Guard drops here
        
        if let Some(mock) = console.as_any().downcast_ref::<MockConsoleInput>() {
            assert!(!mock.is_raw_mode_enabled());
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_platform_detection() {
        let (console, _) = replkit_io::create_console_io().unwrap();
        let capabilities = console.get_capabilities();
        
        #[cfg(unix)]
        assert_eq!(capabilities.backend_type, BackendType::UnixVt);
        
        #[cfg(windows)]
        assert!(matches!(capabilities.backend_type, BackendType::WindowsVt | BackendType::WindowsLegacy));
        
        #[cfg(target_arch = "wasm32")]
        assert_eq!(capabilities.backend_type, BackendType::WasmBridge);
    }
    
    #[test]
    fn test_error_handling() {
        let (mut console, _) = create_mock_console_io();
        
        // Test double start
        console.start_event_loop().unwrap();
        let result = console.start_event_loop();
        assert!(matches!(result, Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning))));
        
        console.stop_event_loop().unwrap();
        
        // Test double stop
        let result = console.stop_event_loop();
        assert!(matches!(result, Err(ConsoleError::EventLoopError(EventLoopError::NotRunning))));
    }
}
```

## Dependencies and Module Structure

### External Dependencies

```toml
# replkit-core/Cargo.toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# replkit-io/Cargo.toml
[dependencies]
replkit-core = { path = "../replkit-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["wincon", "handleapi", "consoleapi", "processenv"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"

[dev-dependencies]
quickcheck = "1.0"
quickcheck_macros = "1.0"
insta = "1.0"  # For snapshot testing
rand = "0.8"   # For property-based testing

[target.'cfg(unix)'.dev-dependencies]
libc = "0.2"   # For PTY testing

[features]
default = []
wasm = []
```

### Module Structure

```
crates/
├── replkit-core/src/
│   ├── lib.rs                    # Public API exports
│   ├── key.rs                    # Key definitions (existing)
│   ├── buffer.rs                 # Buffer implementation (existing)
│   ├── document.rs               # Document implementation (existing)
│   ├── unicode.rs                # Unicode utilities (existing)
│   ├── error.rs                  # Error types (existing)
│   ├── console.rs                # ConsoleInput trait and types
│   └── wasm.rs                   # WASM bindings (existing, extended)
│
├── replkit-io/src/
│   ├── lib.rs                    # Platform factory and exports
│   ├── unix.rs                   # Unix I/O implementation
│   ├── windows/
│   │   ├── mod.rs               # Windows module
│   │   ├── vt.rs                # Windows VT implementation
│   │   └── legacy.rs            # Windows Legacy implementation
│   ├── wasm.rs                   # WASM bridge implementation
│   └── mock.rs                   # Mock implementation for testing
│
└── replkit-pyo3/src/
    ├── lib.rs                    # Python module exports
    ├── console.rs                # Python console bindings
    └── ... (existing files)
```

## Input/Output Coordination Design Considerations

### The `get_cursor_position` Challenge

The `get_cursor_position` method presents a fundamental design challenge that highlights the tension between clean API boundaries and practical functionality.

#### Problem Statement

Cursor position querying requires bidirectional communication:
1. **Output**: Send DSR (Device Status Report) sequence `\x1b[6n` to stdout
2. **Input**: Read CPR (Cursor Position Report) response `\x1b[{row};{col}R` from stdin
3. **Parsing**: Extract coordinates from the response

This violates the Input/Output separation principle.

#### Design Options

**Option 1: Current Approach - Strict Separation**
```rust
// Pros: Clean API boundaries, testable, platform-flexible
// Cons: get_cursor_position() unavailable on some platforms

impl ConsoleOutput for UnixConsoleOutput {
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        Err(ConsoleError::UnsupportedFeature { ... })
    }
}
```

**Option 2: Shared Communication Channel**
```rust
// Requires coordination between Input/Output instances
pub struct ConsoleCoordinator {
    input: Box<dyn ConsoleInput>,
    output: Box<dyn ConsoleOutput>,
    shared_channel: Arc<Mutex<Channel>>,
}

impl ConsoleCoordinator {
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        // Coordinate between input and output
    }
}
```

**Option 3: Unified Console Interface**
```rust
// Single trait combining Input/Output
pub trait Console: ConsoleInput + ConsoleOutput {
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)>;
}
```

**Option 4: Capability-Based Design**
```rust
// Optional coordination capability
pub trait ConsoleInputOutputCoordination {
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)>;
}

// Only implement for platforms that support it
impl ConsoleInputOutputCoordination for UnixConsole {
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        // Implementation with internal coordination
    }
}
```

#### Recommended Approach

**Current Phase**: Maintain Option 1 (strict separation) because:
- Preserves clean architecture
- Supports diverse platform combinations (PowerShell VT input + legacy output)
- Avoids complex coordination logic
- Most applications can work without cursor position queries

**Future Enhancement**: Consider Option 4 (capability-based) when:
- Real-world usage demonstrates need for cursor position queries
- Platform-specific coordination patterns are well understood
- Testing strategies for bidirectional communication are established

#### Platform-Specific Considerations

- **Unix**: CPR via DSR is standard but requires stdin/stdout coordination
- **Windows VT**: Same as Unix when VT mode is available
- **Windows Legacy**: `GetConsoleScreenBufferInfo` provides direct access without coordination
- **WASM**: Host environment must provide cursor position through different mechanism

This design decision prioritizes architectural clarity over feature completeness in the initial implementation.

## Implementation Phases

This design provides a solid foundation for cross-platform terminal input handling with the following key benefits:

1. **Platform Abstraction**: Unified interface across all supported platforms
2. **Safety**: RAII guards ensure terminal state restoration
3. **Performance**: Non-blocking I/O with efficient kernel primitives
4. **Extensibility**: Easy to add new platforms and features
5. **Multi-language Support**: Clear bridging strategy for Go and Python
6. **WASM Compatibility**: Serialization-based communication for constrained environments
7. **Testability**: Comprehensive mock implementation for testing

The architecture separates concerns cleanly while providing the flexibility needed for different deployment scenarios and platform-specific optimizations.