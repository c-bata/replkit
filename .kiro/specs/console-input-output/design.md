# Design Document

## Overview

The console-input-output system provides a robust, cross-platform abstraction for terminal input/output operations. The design has been revised to prioritize **synchronous, non-blocking I/O patterns** over asynchronous event loops, making it simpler to implement and more compatible with constrained environments like WASM. The architecture separates platform-specific implementations from the common interface while ensuring consistent behavior and optimal performance on each target platform.

The system includes both ConsoleInput for handling keyboard input and ConsoleOutput for efficient terminal rendering and cursor control, providing a complete terminal I/O solution.

## Design Philosophy

### Core Principles

1. **Cross-Platform Compatibility**: Support Unix/Linux, Windows, WASM, and Go bindings with a unified API
2. **Non-Blocking First**: Prioritize non-blocking operations due to WASM constraints
3. **Clear Intent**: Separate APIs for different use cases to improve code readability
4. **Performance Optimization**: Platform-specific optimizations while maintaining API consistency
5. **Synchronous Simplicity**: Avoid complex async patterns in favor of clear, synchronous interfaces

### API Design Strategy

#### Synchronous Reading Methods

The design provides two complementary methods for key input:

1. **`try_read_key()`**: Pure non-blocking, immediate return
   - Clear intent: "check if input is available"
   - Optimized implementation path
   - Go-friendly (channel select patterns)
   - WASM-compatible

2. **`read_key_timeout(timeout_ms: Option<u32>)`**: Flexible timeout control
   - `Some(0)`: Non-blocking (equivalent to `try_read_key()`)
   - `Some(ms)`: Timeout-based waiting
   - `None`: Infinite blocking (not available on WASM)

This approach **eliminates the need for event loops** while providing flexibility for different use cases.

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│                (prompt.rs, REPL logic)                       │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                  Language Bindings                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Rust Native │  │ Go Bindings │  │ Python (PyO3)       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                 ConsoleInput Trait                          │
│         (synchronous read methods, no event loops)          │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│              Platform Implementations                       │
│                    (replkit-io)                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Unix I/O    │  │ Windows     │  │ WASM Bridge         │ │
│  │ (poll/select)│  │ (Console API)│  │ (Event Queue)       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Platform Compatibility Matrix

| API | Unix/Linux | Windows | WASM | Go Bindings |
|-----|------------|---------|------|-------------|
| `try_read_key()` | ✅ poll() | ✅ PeekConsoleInput | ✅ event queue | ✅ Recommended |
| `read_key_timeout(0)` | ✅ Same as above | ✅ Same as above | ✅ Same as above | ✅ Recommended |
| `read_key_timeout(ms)` | ✅ select() | ✅ WaitForSingleObject | ❌ Unsupported | ⚠️ Short timeouts only |
| `read_key_timeout(None)` | ✅ blocking read | ✅ ReadConsoleInput | ❌ Unsupported | ❌ Not recommended |

## Components and Interfaces

### Core Trait Definitions

```rust
// In replkit-core/src/console.rs
use crate::{KeyEvent};

pub trait ConsoleInput: Send + Sync {
    /// Enable raw terminal mode with automatic restoration
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError>;
    
    /// Try to read a key event without blocking
    /// Returns None if no input is available
    fn try_read_key(&self) -> Result<Option<KeyEvent>, ConsoleError>;
    
    /// Read a key event with optional timeout
    /// - Some(0): Non-blocking (equivalent to try_read_key)
    /// - Some(ms): Wait up to ms milliseconds
    /// - None: Block indefinitely (not supported on WASM)
    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> Result<Option<KeyEvent>, ConsoleError>;
    
    /// Get current terminal window size (columns, rows)
    fn get_window_size(&self) -> Result<(u16, u16), ConsoleError>;
    
    /// Get platform-specific capabilities
    fn get_capabilities(&self) -> ConsoleCapabilities;
}

/// RAII guard for terminal raw mode
pub struct RawModeGuard {
    restore_fn: Option<Box<dyn FnOnce() + Send>>,
    platform_info: String,
}

impl RawModeGuard {
    pub fn new<F>(restore_fn: F, platform_info: String) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            restore_fn: Some(Box::new(restore_fn)),
            platform_info,
        }
    }
    
    pub fn platform_info(&self) -> &str {
        &self.platform_info
    }
    
    /// Manually restore terminal mode
    pub fn restore(mut self) -> Result<(), ConsoleError> {
        if let Some(restore_fn) = self.restore_fn.take() {
            restore_fn();
            Ok(())
        } else {
            Err(ConsoleError::TerminalError("Already restored".to_string()))
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if let Some(restore_fn) = self.restore_fn.take() {
            restore_fn();
        }
    }
}
```

The ConsoleOutput trait remains largely unchanged from the previous design.

### Error Handling

```rust
#[derive(Debug, Clone)]
pub enum ConsoleError {
    /// Platform-specific I/O error
    IoError(String),
    /// Feature not supported on this platform
    UnsupportedFeature { feature: String, platform: String },
    /// Timeout expired while waiting for input
    TimeoutExpired,
    /// Terminal setup/teardown error
    TerminalError(String),
    /// Platform-specific error
    PlatformSpecific(String),
}
```

## Platform-Specific Implementations

### Unix Implementation

The Unix implementation uses `poll()` for non-blocking operations and `select()` for timeout-based operations:

```rust
// In replkit-io/src/unix.rs
use replkit_core::console::*;
use std::os::unix::io::RawFd;

pub struct UnixConsoleInput {
    stdin_fd: RawFd,
    key_parser: KeyParser,
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
        
        Ok(UnixConsoleInput {
            stdin_fd,
            key_parser: KeyParser::new(),
        })
    }
    
    fn setup_raw_mode(&self) -> ConsoleResult<libc::termios> {
        let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
        
        // Get current terminal attributes
        if unsafe { libc::tcgetattr(self.stdin_fd, &mut termios) } != 0 {
            return Err(ConsoleError::IoError(
                "Failed to get terminal attributes".to_string()
            ));
        }
        
        // Save original settings
        let original_termios = termios;
        
        // Configure raw mode
        termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        termios.c_iflag &= !(libc::IXON | libc::ICRNL);
        termios.c_cc[libc::VMIN] = 0;
        termios.c_cc[libc::VTIME] = 0;
        
        // Apply new settings
        if unsafe { libc::tcsetattr(self.stdin_fd, libc::TCSANOW, &termios) } != 0 {
            return Err(ConsoleError::IoError(
                "Failed to set terminal attributes".to_string()
            ));
        }
        
        Ok(original_termios)
    }
}

impl ConsoleInput for UnixConsoleInput {
    fn enable_raw_mode(&self) -> ConsoleResult<RawModeGuard> {
        let original_termios = self.setup_raw_mode()?;
        let stdin_fd = self.stdin_fd;
        
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
    
    fn try_read_key(&self) -> ConsoleResult<Option<KeyEvent>> {
        // Use poll() to check for available input without blocking
        let mut poll_fd = libc::pollfd {
            fd: self.stdin_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        
        let poll_result = unsafe {
            libc::poll(&mut poll_fd, 1, 0) // 0 timeout = non-blocking
        };
        
        if poll_result > 0 && poll_fd.revents & libc::POLLIN != 0 {
            // Input available, read it
            let mut buffer = [0u8; 256];
            let bytes_read = unsafe {
                libc::read(
                    self.stdin_fd,
                    buffer.as_mut_ptr() as *mut libc::c_void,
                    buffer.len()
                )
            };
            
            if bytes_read > 0 {
                let input = &buffer[..bytes_read as usize];
                let events = self.key_parser.feed(input);
                Ok(events.into_iter().next())
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> ConsoleResult<Option<KeyEvent>> {
        match timeout_ms {
            Some(0) => self.try_read_key(),
            Some(ms) => {
                // Use select() with timeout
                let mut read_fds = unsafe { std::mem::zeroed::<libc::fd_set>() };
                unsafe {
                    libc::FD_ZERO(&mut read_fds);
                    libc::FD_SET(self.stdin_fd, &mut read_fds);
                }
                
                let mut timeout = libc::timeval {
                    tv_sec: (ms / 1000) as libc::time_t,
                    tv_usec: ((ms % 1000) * 1000) as libc::suseconds_t,
                };
                
                let select_result = unsafe {
                    libc::select(
                        self.stdin_fd + 1,
                        &mut read_fds,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut timeout
                    )
                };
                
                if select_result > 0 {
                    // Input available
                    self.try_read_key()
                } else if select_result == 0 {
                    // Timeout
                    Ok(None)
                } else {
                    Err(ConsoleError::IoError("Select failed".to_string()))
                }
            }
            None => {
                // Blocking read
                let mut buffer = [0u8; 256];
                let bytes_read = unsafe {
                    libc::read(
                        self.stdin_fd,
                        buffer.as_mut_ptr() as *mut libc::c_void,
                        buffer.len()
                    )
                };
                
                if bytes_read > 0 {
                    let input = &buffer[..bytes_read as usize];
                    let events = self.key_parser.feed(input);
                    Ok(events.into_iter().next())
                } else {
                    Ok(None)
                }
            }
        }
    }
    
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        let mut winsize = unsafe { std::mem::zeroed::<libc::winsize>() };
        
        if unsafe { libc::ioctl(self.stdin_fd, libc::TIOCGWINSZ, &mut winsize) } == 0 {
            Ok((winsize.ws_col, winsize.ws_row))
        } else {
            Err(ConsoleError::IoError(
                "Failed to query window size".to_string()
            ))
        }
    }
    
    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: false, // No event loop for resize
            supports_bracketed_paste: true,
            supports_mouse_events: true,
            supports_unicode: true,
            platform_name: "Unix/Linux".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
}
```

### Windows Implementation

The Windows implementation uses `PeekConsoleInput` for non-blocking and `WaitForSingleObject` for timeouts:

```rust
// In replkit-io/src/windows.rs
use winapi::um::consoleapi::*;
use winapi::um::handleapi::*;
use winapi::um::synchapi::*;

pub struct WindowsConsoleInput {
    stdin_handle: HANDLE,
    key_parser: KeyParser,
}

impl WindowsConsoleInput {
    pub fn new() -> ConsoleResult<Self> {
        let stdin_handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        
        if stdin_handle == INVALID_HANDLE_VALUE {
            return Err(ConsoleError::TerminalError(
                "Failed to get stdin handle".to_string()
            ));
        }
        
        Ok(WindowsConsoleInput {
            stdin_handle,
            key_parser: KeyParser::new(),
        })
    }
}

impl ConsoleInput for WindowsConsoleInput {
    fn try_read_key(&self) -> ConsoleResult<Option<KeyEvent>> {
        let mut events_available: DWORD = 0;
        
        // Check if input is available without blocking
        if unsafe { 
            GetNumberOfConsoleInputEvents(self.stdin_handle, &mut events_available) 
        } == 0 {
            return Err(ConsoleError::IoError("Failed to check input events".to_string()));
        }
        
        if events_available == 0 {
            return Ok(None);
        }
        
        // Peek at events without removing them
        let mut buffer = [INPUT_RECORD::default(); 32];
        let mut events_read: DWORD = 0;
        
        if unsafe {
            PeekConsoleInputW(
                self.stdin_handle,
                buffer.as_mut_ptr(),
                buffer.len() as DWORD,
                &mut events_read
            )
        } == 0 {
            return Ok(None);
        }
        
        // Process key events
        for i in 0..events_read as usize {
            if buffer[i].EventType == KEY_EVENT {
                let key_event = unsafe { buffer[i].Event.KeyEvent() };
                if key_event.bKeyDown == TRUE {
                    // Read and remove this event
                    if unsafe {
                        ReadConsoleInputW(
                            self.stdin_handle,
                            buffer.as_mut_ptr(),
                            1,
                            &mut events_read
                        )
                    } != 0 {
                        // Convert Windows key event to our KeyEvent
                        return Ok(Some(self.convert_key_event(key_event)));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> ConsoleResult<Option<KeyEvent>> {
        match timeout_ms {
            Some(0) => self.try_read_key(),
            Some(ms) => {
                // Wait for input with timeout
                let wait_result = unsafe {
                    WaitForSingleObject(self.stdin_handle, ms)
                };
                
                match wait_result {
                    WAIT_OBJECT_0 => self.try_read_key(),
                    WAIT_TIMEOUT => Ok(None),
                    _ => Err(ConsoleError::IoError("Wait failed".to_string()))
                }
            }
            None => {
                // Blocking read
                let mut buffer = [INPUT_RECORD::default(); 1];
                let mut events_read: DWORD = 0;
                
                if unsafe {
                    ReadConsoleInputW(
                        self.stdin_handle,
                        buffer.as_mut_ptr(),
                        1,
                        &mut events_read
                    )
                } != 0 && events_read > 0 {
                    if buffer[0].EventType == KEY_EVENT {
                        let key_event = unsafe { buffer[0].Event.KeyEvent() };
                        if key_event.bKeyDown == TRUE {
                            return Ok(Some(self.convert_key_event(key_event)));
                        }
                    }
                }
                
                Ok(None)
            }
        }
    }
}
```

- Distinguish between legacy and VT console modes

### WASM Implementation

The WASM implementation uses an event queue populated by the host environment:

```rust
// In replkit-io/src/wasm.rs
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct WasmConsoleInput {
    event_queue: Mutex<VecDeque<KeyEvent>>,
    window_size: Mutex<(u16, u16)>,
}

impl WasmConsoleInput {
    pub fn new() -> ConsoleResult<Self> {
        Ok(WasmConsoleInput {
            event_queue: Mutex::new(VecDeque::new()),
            window_size: Mutex::new((80, 24)),
        })
    }
    
    /// Called by the host to queue key events
    pub fn queue_key_event(&self, event: KeyEvent) {
        self.event_queue.lock().unwrap().push_back(event);
    }
    
    /// Called by the host to update window size
    pub fn set_window_size(&self, cols: u16, rows: u16) {
        *self.window_size.lock().unwrap() = (cols, rows);
    }
}

impl ConsoleInput for WasmConsoleInput {
    fn try_read_key(&self) -> ConsoleResult<Option<KeyEvent>> {
        Ok(self.event_queue.lock().unwrap().pop_front())
    }
    
    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> ConsoleResult<Option<KeyEvent>> {
        match timeout_ms {
            Some(0) => self.try_read_key(),
            Some(_) | None => {
                // WASM doesn't support blocking or timed waits
                Err(ConsoleError::UnsupportedFeature {
                    feature: "blocking or timed input".to_string(),
                    platform: "WASM".to_string(),
                })
            }
        }
    }
    
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        Ok(*self.window_size.lock().unwrap())
    }
    
    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: false, // Handled by host
            supports_resize_events: false,
            supports_bracketed_paste: false,
            supports_mouse_events: false,
            supports_unicode: true,
            platform_name: "WASM".to_string(),
            backend_type: BackendType::WasmBridge,
        }
    }
}
```

## Multi-Language Binding Strategy

### Go Bindings

The Go binding adopts a channel-based approach that fits Go's concurrency model:

- **Channel-Based High-Level API**: Provide idiomatic Go channels for key events
- **Avoid Infinite Blocking**: Discourage `read_key_timeout(None)` usage
- **Goroutine-Friendly**: Design around Go's concurrency patterns
- **Graceful Shutdown**: Support context cancellation and timeouts

```go
// Recommended Go API patterns
func (c *ConsoleInput) KeyEventChannel() <-chan KeyEvent
func (c *ConsoleInput) TryReadKey() (*KeyEvent, error)
func (c *ConsoleInput) ReadKeyWithTimeout(timeout time.Duration) (*KeyEvent, error)
```

```go
// bindings/go/console_input.go
package replkit

type ConsoleInput struct {
    wasmModule *WasmModule
    keyEvents  chan KeyEvent
}

// TryReadKey attempts to read a key without blocking
func (c *ConsoleInput) TryReadKey() (*KeyEvent, error) {
    select {
    case event := <-c.keyEvents:
        return &event, nil
    default:
        return nil, nil
    }
}

// ReadKeyWithTimeout reads a key with optional timeout
func (c *ConsoleInput) ReadKeyWithTimeout(timeout time.Duration) (*KeyEvent, error) {
    if timeout == 0 {
        return c.TryReadKey()
    }
    
    select {
    case event := <-c.keyEvents:
        return &event, nil
    case <-time.After(timeout):
        return nil, nil
    }
}

// KeyEventChannel returns a channel for receiving key events
func (c *ConsoleInput) KeyEventChannel() <-chan KeyEvent {
    return c.keyEvents
}
```

### Python Bindings

Python bindings use PyO3 to expose the synchronous API:

```python
# Example Python usage
import replkit

console = replkit.ConsoleInput()
with console.enable_raw_mode():
    # Non-blocking read
    key = console.try_read_key()
    if key:
        print(f"Key pressed: {key}")
    
    # Read with timeout
    key = console.read_key_timeout(1000)  # 1 second
    if key:
        print(f"Key pressed: {key}")
```

## Window Size Management


For Go bindings, window size monitoring can be handled separately:

- **SIGWINCH Signal Monitoring**: Use Go's `os/signal` to detect terminal resize
- **Channel-Based Communication**: Provide `MonitorWindowSize(ctx) <-chan WindowSize` for reactive patterns
- **Immediate Notification**: Forward size changes to Rust renderer via WASM calls

```
Go Application Layer          ← High-level logic, prompt management
     ↓ (size changes)
Go Console Wrapper           ← Platform-specific size detection (SIGWINCH)
     ↓ (WASM calls)  
Rust Rendering Engine        ← Cross-platform rendering logic, layout calculation
     ↓ (platform output)
Platform Console Output      ← Raw terminal I/O
```

```go
// Go handles SIGWINCH directly
func MonitorWindowSize(ctx context.Context) <-chan WindowSize {
    sizeChan := make(chan WindowSize)
    
    go func() {
        sigwinch := make(chan os.Signal, 1)
        signal.Notify(sigwinch, syscall.SIGWINCH)
        
        for {
            select {
            case <-ctx.Done():
                return
            case <-sigwinch:
                cols, rows, _ := terminal.GetSize(int(os.Stdin.Fd()))
                sizeChan <- WindowSize{Columns: cols, Rows: rows}
            }
        }
    }()
    
    return sizeChan
}
```

## Testing Strategy

### Mock Implementation

The mock implementation is simplified without event loops:

```rust
// In replkit-io/src/mock.rs
pub struct MockConsoleInput {
    input_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
}

impl MockConsoleInput {
    pub fn new() -> Self {
        Self {
            input_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    pub fn queue_key_event(&self, event: KeyEvent) {
        self.input_queue.lock().unwrap().push_back(event);
    }
    
    pub fn queue_text_input(&self, text: &str) {
        for ch in text.chars() {
            self.queue_key_event(KeyEvent::from_char(ch));
        }
    }
}

impl ConsoleInput for MockConsoleInput {
    fn try_read_key(&self) -> ConsoleResult<Option<KeyEvent>> {
        Ok(self.input_queue.lock().unwrap().pop_front())
    }
    
    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> ConsoleResult<Option<KeyEvent>> {
        // Mock doesn't actually wait, just returns immediately
        self.try_read_key()
    }
}
```

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_non_blocking_read() {
        let console = MockConsoleInput::new();
        
        // No input available
        assert!(console.try_read_key().unwrap().is_none());
        
        // Queue input
        console.queue_text_input("hello");
        
        // Read characters
        for expected in "hello".chars() {
            let key = console.try_read_key().unwrap().unwrap();
            assert_eq!(key.to_char(), Some(expected));
        }
        
        // Queue exhausted
        assert!(console.try_read_key().unwrap().is_none());
    }
    
    #[test]
    fn test_timeout_read() {
        let console = MockConsoleInput::new();
        
        // Timeout with no input (mock returns immediately)
        assert!(console.read_key_timeout(Some(100)).unwrap().is_none());
        
        // Queue input
        console.queue_key_event(KeyEvent::from_key(Key::Enter));
        
        // Read with timeout should return immediately when input available
        let key = console.read_key_timeout(Some(100)).unwrap().unwrap();
        assert_eq!(key.key, Key::Enter);
    }
}
```

## Implementation Timeline

### Completed Phases

- ✅ **Phase 1: Core API Implementation**
  - Unix `try_read_key()` and `read_key_timeout()`
  - Windows `try_read_key()` and `read_key_timeout()`
  - Updated prompt.rs to use new API
  - Fixed Escape key detection with KeyParser
  - Fixed raw mode output formatting

- ✅ **Phase 2: WASM Integration**
  - WASM `try_read_key()` implementation
  - WASM `read_key_timeout()` with limitations
  - Go WASM runtime integration

- ✅ **Phase 3: Go Bindings**
  - Channel-based ConsoleInput interface
  - Updated examples

### Future Enhancements

- [ ] **Mouse Event Support**
  - Extend API for mouse input
  - Handle click, scroll, and movement events
  - Cross-platform coordinate mapping
- [ ] **Bracketed Paste Support**
  - Detect and handle bracketed paste mode
  - Distinguish between typed and pasted content
  - Security considerations for paste content
- [ ] **Dynamic Terminal Feature Detection**
  - Runtime detection of terminal capabilities
  - Adaptive rendering based on available features
  - Fallback modes for limited terminals

## Notes

- The dual API approach (`try_read_key()` vs `read_key_timeout()`) provides clear intent while allowing implementation optimization
- WASM constraints drive the overall design toward non-blocking patterns
- Go bindings will emphasize channel-based patterns over direct API mapping
- Platform-specific optimizations are hidden behind the unified API
- Error handling distinguishes between platform limitations and actual errors
