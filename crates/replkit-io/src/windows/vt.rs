//! Windows VT (Virtual Terminal) console input implementation
//!
//! This module provides console input functionality for Windows systems that support
//! Virtual Terminal sequences (Windows 10 version 1607 and later). It enables VT input
//! mode and uses the KeyParser to process VT sequences, similar to Unix systems.

use std::ffi::c_void;
use std::io;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::{
    BackendType, ConsoleCapabilities, ConsoleError, ConsoleInput, ConsoleResult, RawModeGuard,
};
use replkit_core::console::EventLoopError;
use replkit_core::{KeyEvent, KeyParser};

type BOOL = i32;
type HANDLE = isize;
type DWORD = u32;
type WORD = u16;
type WCHAR = u16;
type SHORT = i16;

const STD_INPUT_HANDLE: DWORD = 0xFFFF_FFF6; // (DWORD)-10
const STD_OUTPUT_HANDLE: DWORD = 0xFFFF_FFF5; // (DWORD)-11
const WAIT_OBJECT_0: DWORD = 0x00000000;
const WAIT_FAILED: DWORD = 0xFFFF_FFFF;
const WAIT_TIMEOUT: DWORD = 0x00000102;

// Console mode flags for VT input
const ENABLE_VIRTUAL_TERMINAL_INPUT: DWORD = 0x0200;
const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
const ENABLE_LINE_INPUT: DWORD = 0x0002;
const ENABLE_ECHO_INPUT: DWORD = 0x0004;
const ENABLE_WINDOW_INPUT: DWORD = 0x0008;
const ENABLE_MOUSE_INPUT: DWORD = 0x0010;
const ENABLE_EXTENDED_FLAGS: DWORD = 0x0080;
const ENABLE_QUICK_EDIT_MODE: DWORD = 0x0040;

#[repr(C)]
#[derive(Copy, Clone)]
struct COORD {
    X: SHORT,
    Y: SHORT,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SMALL_RECT {
    Left: SHORT,
    Top: SHORT,
    Right: SHORT,
    Bottom: SHORT,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CONSOLE_SCREEN_BUFFER_INFO {
    dwSize: COORD,
    dwCursorPosition: COORD,
    wAttributes: WORD,
    srWindow: SMALL_RECT,
    dwMaximumWindowSize: COORD,
}

extern "system" {
    fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
    fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
    fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
    fn ReadFile(
        hFile: HANDLE,
        lpBuffer: *mut c_void,
        nNumberOfBytesToRead: DWORD,
        lpNumberOfBytesRead: *mut DWORD,
        lpOverlapped: *mut c_void,
    ) -> BOOL;
    fn GetConsoleScreenBufferInfo(
        hConsoleOutput: HANDLE,
        lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO,
    ) -> BOOL;
    fn CreateEventW(
        lpEventAttributes: *mut c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const WCHAR,
    ) -> HANDLE;
    fn SetEvent(hEvent: HANDLE) -> BOOL;
    fn WaitForMultipleObjects(
        nCount: DWORD,
        lpHandles: *const HANDLE,
        bWaitAll: BOOL,
        dwMilliseconds: DWORD,
    ) -> DWORD;
    fn CloseHandle(hObject: HANDLE) -> BOOL;
    fn GetLastError() -> DWORD;
}

// Event loop state machine for proper synchronization
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum EventLoopState {
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Stopping = 3,
}

/// Windows VT console input implementation
///
/// This implementation enables Virtual Terminal input mode on Windows and uses
/// the KeyParser to process VT escape sequences, providing Unix-like behavior
/// on modern Windows systems.
pub struct WindowsVtConsoleInput {
    h_input: HANDLE,
    h_output: HANDLE,
    original_input_mode: DWORD,
    original_output_mode: DWORD,

    // Event loop management
    event_loop_state: AtomicU8,
    stop_event: HANDLE,
    event_thread: Mutex<Option<JoinHandle<()>>>,

    // Key parsing
    key_parser: Mutex<KeyParser>,

    // Callbacks
    resize_callback: Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>,
    key_callback: Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>,

    // Window size tracking
    last_window_size: Mutex<Option<(u16, u16)>>,
}

impl WindowsVtConsoleInput {
    /// Create a new Windows VT console input instance
    ///
    /// This will fail if VT input mode is not supported on the current system
    pub fn new() -> io::Result<Self> {
        unsafe {
            let h_input = GetStdHandle(STD_INPUT_HANDLE);
            if h_input == 0 || h_input == -1 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get stdin handle",
                ));
            }

            let h_output = GetStdHandle(STD_OUTPUT_HANDLE);
            if h_output == 0 || h_output == -1 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get stdout handle",
                ));
            }

            // Save original console modes
            let mut input_mode: DWORD = 0;
            if GetConsoleMode(h_input, &mut input_mode) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get input console mode",
                ));
            }

            let mut output_mode: DWORD = 0;
            if GetConsoleMode(h_output, &mut output_mode) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get output console mode",
                ));
            }

            // Test if VT input mode is supported by trying to enable it
            let vt_input_mode = input_mode | ENABLE_VIRTUAL_TERMINAL_INPUT;
            if SetConsoleMode(h_input, vt_input_mode) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Virtual Terminal input mode not supported on this system",
                ));
            }

            // Restore original mode for now - we'll enable VT mode in raw mode
            SetConsoleMode(h_input, input_mode);

            // Create stop event for clean shutdown
            let stop_event = CreateEventW(null_mut(), 1, 0, null());
            if stop_event == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to create stop event",
                ));
            }

            Ok(Self {
                h_input,
                h_output,
                original_input_mode: input_mode,
                original_output_mode: output_mode,
                event_loop_state: AtomicU8::new(EventLoopState::Stopped as u8),
                stop_event,
                event_thread: Mutex::new(None),
                key_parser: Mutex::new(KeyParser::new()),
                resize_callback: Mutex::new(None),
                key_callback: Mutex::new(None),
                last_window_size: Mutex::new(None),
            })
        }
    }

    /// Enable VT input mode and configure for raw input
    fn setup_vt_mode(&self) -> io::Result<()> {
        unsafe {
            // Enable VT input mode with window events but disable line buffering and echo
            let vt_mode = self.original_input_mode
                | ENABLE_VIRTUAL_TERMINAL_INPUT
                | ENABLE_WINDOW_INPUT
                | ENABLE_EXTENDED_FLAGS;
            let vt_mode = vt_mode
                & !(ENABLE_LINE_INPUT
                    | ENABLE_ECHO_INPUT
                    | ENABLE_PROCESSED_INPUT
                    | ENABLE_QUICK_EDIT_MODE);

            if SetConsoleMode(self.h_input, vt_mode) == 0 {
                let error = GetLastError();
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to enable VT input mode: error {}", error),
                ));
            }

            Ok(())
        }
    }

    /// Restore original console mode
    fn restore_console_mode(&self) -> io::Result<()> {
        unsafe {
            if SetConsoleMode(self.h_input, self.original_input_mode) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to restore input console mode",
                ));
            }

            if SetConsoleMode(self.h_output, self.original_output_mode) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to restore output console mode",
                ));
            }

            Ok(())
        }
    }

    /// Query current window size from console buffer info
    fn query_window_size(&self) -> io::Result<(u16, u16)> {
        unsafe {
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = zeroed();
            if GetConsoleScreenBufferInfo(self.h_output, &mut info) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to get console screen buffer info",
                ));
            }

            // Return visible window size (srWindow), not buffer size (dwSize)
            let width = (info.srWindow.Right - info.srWindow.Left + 1) as u16;
            let height = (info.srWindow.Bottom - info.srWindow.Top + 1) as u16;

            Ok((width, height))
        }
    }

    /// Check if window size has changed and notify callback if so
    fn check_window_size_change(&self) {
        if let Ok(current_size) = self.query_window_size() {
            let mut last_size_guard = self.last_window_size.lock().unwrap();
            let size_changed = match *last_size_guard {
                Some(last_size) => last_size != current_size,
                None => true, // First time checking
            };

            if size_changed {
                *last_size_guard = Some(current_size);
                drop(last_size_guard); // Release lock before callback

                // Invoke resize callback
                if let Ok(mut callback_guard) = self.resize_callback.lock() {
                    if let Some(callback) = callback_guard.as_mut() {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            callback(current_size.0, current_size.1);
                        }));
                    }
                }
            }
        }
    }

    /// Main event loop thread function
    fn event_loop_thread(
        h_input: HANDLE,
        stop_event: HANDLE,
        event_loop_state: Arc<AtomicU8>,
        key_parser: Arc<Mutex<KeyParser>>,
        key_callback: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
        window_size_checker: Arc<dyn Fn() + Send + Sync>,
    ) {
        let mut buffer = [0u8; 1024];
        let handles = [h_input, stop_event];

        'main_loop: while event_loop_state.load(Ordering::Relaxed) == EventLoopState::Running as u8
        {
            // Wait for input or stop signal with timeout for periodic window size checks
            let wait_result = unsafe {
                WaitForMultipleObjects(2, handles.as_ptr(), 0, 100) // 100ms timeout
            };

            match wait_result {
                WAIT_OBJECT_0 => {
                    // Input available - read it
                    let mut bytes_read: DWORD = 0;
                    let read_result = unsafe {
                        ReadFile(
                            h_input,
                            buffer.as_mut_ptr() as *mut c_void,
                            buffer.len() as DWORD,
                            &mut bytes_read,
                            null_mut(),
                        )
                    };

                    if read_result != 0 && bytes_read > 0 {
                        let input_bytes = &buffer[..bytes_read as usize];

                        // Parse key events using shared parser instance
                        let key_events = {
                            let mut parser = key_parser.lock().unwrap();
                            parser.feed(input_bytes)
                        };

                        // Invoke key callback for each event
                        if let Ok(mut callback_guard) = key_callback.lock() {
                            if let Some(callback) = callback_guard.as_mut() {
                                for event in key_events {
                                    // Catch panics in user callback
                                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                                        || {
                                            callback(event);
                                        },
                                    ));
                                }
                            }
                        }
                    } else if read_result == 0 {
                        // Read failed - exit loop
                        break 'main_loop;
                    }
                }
                val if val == WAIT_OBJECT_0 + 1 => {
                    // Stop event signaled
                    break 'main_loop;
                }
                WAIT_TIMEOUT => {
                    // Timeout - check window size
                    window_size_checker();
                }
                WAIT_FAILED => {
                    // Wait failed - exit loop
                    break 'main_loop;
                }
                _ => {
                    // Unexpected result - continue
                    continue;
                }
            }
        }

        // Mark as stopped
        event_loop_state.store(EventLoopState::Stopped as u8, Ordering::Relaxed);
    }
}

impl Drop for WindowsVtConsoleInput {
    fn drop(&mut self) {
        // Ensure event loop is stopped
        let _ = self.stop_event_loop();

        // Restore console mode
        let _ = self.restore_console_mode();

        // Close stop event handle
        unsafe {
            CloseHandle(self.stop_event);
        }
    }
}

impl ConsoleInput for WindowsVtConsoleInput {
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError> {
        self.setup_vt_mode()
            .map_err(|e| ConsoleError::TerminalError(format!("Failed to setup VT mode: {}", e)))?;

        let h_input = self.h_input;
        let h_output = self.h_output;
        let original_input_mode = self.original_input_mode;
        let original_output_mode = self.original_output_mode;

        let restore_fn = move || unsafe {
            SetConsoleMode(h_input, original_input_mode);
            SetConsoleMode(h_output, original_output_mode);
        };

        Ok(RawModeGuard::new(restore_fn, "Windows VT".to_string()))
    }

    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        self.query_window_size()
            .map_err(|e| ConsoleError::IoError(format!("Failed to get window size: {}", e)))
    }

    fn start_event_loop(&self) -> ConsoleResult<()> {
        // Check if already running
        let current_state = self.event_loop_state.load(Ordering::Relaxed);
        if current_state != EventLoopState::Stopped as u8 {
            return Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning));
        }

        // Set state to starting
        if self
            .event_loop_state
            .compare_exchange(
                EventLoopState::Stopped as u8,
                EventLoopState::Starting as u8,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning));
        }

        // Create shared references for the thread
        let h_input = self.h_input;
        let stop_event = self.stop_event;
        let event_loop_state = Arc::new(AtomicU8::new(EventLoopState::Running as u8));
        let key_parser = Arc::new(Mutex::new(KeyParser::new()));
        let key_callback = Arc::new(Mutex::new(None::<Box<dyn FnMut(KeyEvent) + Send>>));

        // Copy current callback if any
        if let Ok(mut current_callback) = self.key_callback.lock() {
            if let Some(callback) = current_callback.take() {
                *key_callback.lock().unwrap() = Some(callback);
            }
        }

        // Create window size checker closure
        let h_output = self.h_output;
        let resize_callback = Arc::new(Mutex::new(None::<Box<dyn FnMut(u16, u16) + Send>>));
        if let Ok(mut current_resize_callback) = self.resize_callback.lock() {
            if let Some(callback) = current_resize_callback.take() {
                *resize_callback.lock().unwrap() = Some(callback);
            }
        }

        let last_window_size = Arc::new(Mutex::new(None::<(u16, u16)>));
        let window_size_checker = {
            let resize_callback = Arc::clone(&resize_callback);
            let last_window_size = Arc::clone(&last_window_size);

            Arc::new(move || unsafe {
                let mut info: CONSOLE_SCREEN_BUFFER_INFO = zeroed();
                if GetConsoleScreenBufferInfo(h_output, &mut info) != 0 {
                    let width = (info.srWindow.Right - info.srWindow.Left + 1) as u16;
                    let height = (info.srWindow.Bottom - info.srWindow.Top + 1) as u16;
                    let current_size = (width, height);

                    let mut last_size_guard = last_window_size.lock().unwrap();
                    let size_changed = match *last_size_guard {
                        Some(last_size) => last_size != current_size,
                        None => true,
                    };

                    if size_changed {
                        *last_size_guard = Some(current_size);
                        drop(last_size_guard);

                        if let Ok(mut callback_guard) = resize_callback.lock() {
                            if let Some(callback) = callback_guard.as_mut() {
                                let _ =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        callback(current_size.0, current_size.1);
                                    }));
                            }
                        }
                    }
                }
            }) as Arc<dyn Fn() + Send + Sync>
        };

        // Update our state to running
        self.event_loop_state
            .store(EventLoopState::Running as u8, Ordering::Relaxed);

        // Start the event loop thread
        let thread_handle = thread::spawn(move || {
            Self::event_loop_thread(
                h_input,
                stop_event,
                event_loop_state,
                key_parser,
                key_callback,
                window_size_checker,
            );
        });

        // Store thread handle
        *self.event_thread.lock().unwrap() = Some(thread_handle);

        Ok(())
    }

    fn stop_event_loop(&self) -> ConsoleResult<()> {
        let current_state = self.event_loop_state.load(Ordering::Relaxed);
        if current_state == EventLoopState::Stopped as u8 {
            return Err(ConsoleError::EventLoopError(EventLoopError::NotRunning));
        }

        // Set state to stopping
        self.event_loop_state
            .store(EventLoopState::Stopping as u8, Ordering::Relaxed);

        // Signal stop event
        unsafe {
            SetEvent(self.stop_event);
        }

        // Wait for thread to finish
        if let Ok(mut thread_guard) = self.event_thread.lock() {
            if let Some(thread_handle) = thread_guard.take() {
                let _ = thread_handle.join();
            }
        }

        // Mark as stopped
        self.event_loop_state
            .store(EventLoopState::Stopped as u8, Ordering::Relaxed);

        Ok(())
    }

    fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        if let Ok(mut guard) = self.resize_callback.lock() {
            *guard = Some(callback);
        }
    }

    fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        if let Ok(mut guard) = self.key_callback.lock() {
            *guard = Some(callback);
        }
    }

    fn is_running(&self) -> bool {
        self.event_loop_state.load(Ordering::Relaxed) == EventLoopState::Running as u8
    }

    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: true,
            supports_bracketed_paste: true,
            supports_mouse_events: true,
            supports_unicode: true,
            platform_name: "Windows VT".to_string(),
            backend_type: BackendType::WindowsVt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_vt_console_creation() {
        // This test will only pass on systems with VT support
        match WindowsVtConsoleInput::new() {
            Ok(console) => {
                assert!(!console.is_running());
                let caps = console.get_capabilities();
                assert_eq!(caps.backend_type, BackendType::WindowsVt);
                assert!(caps.supports_raw_mode);
                assert!(caps.supports_unicode);
            }
            Err(e) => {
                // On systems without VT support, this is expected
                println!("VT mode not supported: {}", e);
            }
        }
    }

    #[test]
    fn test_window_size_query() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            match console.get_window_size() {
                Ok((width, height)) => {
                    assert!(width > 0);
                    assert!(height > 0);
                    println!("Window size: {}x{}", width, height);
                }
                Err(e) => {
                    println!("Failed to get window size: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_raw_mode_guard() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            match console.enable_raw_mode() {
                Ok(guard) => {
                    assert!(guard.is_active());
                    assert_eq!(guard.platform_info(), "Windows VT");
                    // Guard should restore mode when dropped
                }
                Err(e) => {
                    println!("Failed to enable raw mode: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_event_loop_lifecycle() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            // Should not be running initially
            assert!(!console.is_running());

            // Start should succeed
            if console.start_event_loop().is_ok() {
                assert!(console.is_running());

                // Stop should succeed
                assert!(console.stop_event_loop().is_ok());

                // Give thread time to stop
                std::thread::sleep(Duration::from_millis(50));
                assert!(!console.is_running());
            }
        }
    }

    #[test]
    fn test_callback_registration() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            let (tx, _rx) = mpsc::channel();

            // Register key callback
            console.on_key_pressed(Box::new(move |event| {
                let _ = tx.send(event);
            }));

            let (resize_tx, _resize_rx) = mpsc::channel();

            // Register resize callback
            console.on_window_resize(Box::new(move |w, h| {
                let _ = resize_tx.send((w, h));
            }));

            // Callbacks should be registered (we can't easily test invocation without actual input)
        }
    }
}
