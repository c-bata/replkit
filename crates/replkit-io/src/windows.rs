#[cfg(windows)]
pub mod vt;

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::io;
    use std::mem::zeroed;
    use std::ptr::{null, null_mut};

    use replkit_core::{Key, KeyEvent};
    use crate::{ConsoleError, ConsoleInput, ConsoleOutput, ConsoleResult, RawModeGuard,
               ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, ClearType};

    type BOOL = i32;
    type HANDLE = isize;
    type DWORD = u32;
    type WORD = u16;
    type WCHAR = u16;
    type SHORT = i16;

    const STD_INPUT_HANDLE: DWORD = 0xFFFF_FFF6; // (DWORD)-10
    const WAIT_OBJECT_0: DWORD = 0x00000000;
    const WAIT_FAILED: DWORD = 0xFFFF_FFFF;

    const KEY_EVENT: WORD = 0x0001;
    const WINDOW_BUFFER_SIZE_EVENT: WORD = 0x0004;

    // Console mode flags
    const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
    const ENABLE_LINE_INPUT: DWORD = 0x0002;
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;
    const ENABLE_WINDOW_INPUT: DWORD = 0x0008;
    const ENABLE_MOUSE_INPUT: DWORD = 0x0010;
    const ENABLE_EXTENDED_FLAGS: DWORD = 0x0080;
    const ENABLE_QUICK_EDIT_MODE: DWORD = 0x0040;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct COORD { X: SHORT, Y: SHORT }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct SMALL_RECT { Left: SHORT, Top: SHORT, Right: SHORT, Bottom: SHORT }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CONSOLE_SCREEN_BUFFER_INFO {
        dwSize: COORD,
        dwCursorPosition: COORD,
        wAttributes: WORD,
        srWindow: SMALL_RECT,
        dwMaximumWindowSize: COORD,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct KEY_EVENT_RECORD {
        bKeyDown: BOOL,
        wRepeatCount: WORD,
        wVirtualKeyCode: WORD,
        wVirtualScanCode: WORD,
        UnicodeChar: WCHAR, // simplifying union
        dwControlKeyState: DWORD,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct WINDOW_BUFFER_SIZE_RECORD { dwSize: COORD }

    #[repr(C)]
    #[derive(Copy, Clone)]
    union INPUT_EVENT {
        KeyEvent: KEY_EVENT_RECORD,
        WindowBufferSizeEvent: WINDOW_BUFFER_SIZE_RECORD,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct INPUT_RECORD { EventType: WORD, Event: INPUT_EVENT }

    extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
        fn ReadConsoleInputW(hConsoleInput: HANDLE, lpBuffer: *mut INPUT_RECORD, nLength: DWORD, lpNumberOfEventsRead: *mut DWORD) -> BOOL;
        fn GetNumberOfConsoleInputEvents(hConsoleInput: HANDLE, lpNumberOfEvents: *mut DWORD) -> BOOL;
        fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
        fn GetConsoleScreenBufferInfo(hConsoleOutput: HANDLE, lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO) -> BOOL;
        fn CreateEventW(lpEventAttributes:*mut c_void, bManualReset: BOOL, bInitialState: BOOL, lpName: *const WCHAR) -> HANDLE;
        fn SetEvent(hEvent: HANDLE) -> BOOL;
        fn WaitForMultipleObjects(nCount: DWORD, lpHandles: *const HANDLE, bWaitAll: BOOL, dwMilliseconds: DWORD) -> DWORD;
    }

    pub use super::vt::WindowsVtConsoleInput;
    pub struct WindowsVtConsoleOutput; // TODO

    pub struct WindowsLegacyConsoleInput {
        h_input: HANDLE,
        original_mode: DWORD,
    }

    pub struct WindowsLegacyConsoleOutput {
        h_output: HANDLE,
    }


    
    impl WindowsVtConsoleOutput {
        pub fn new() -> io::Result<Self> { Ok(Self) }
    }
    
    impl WindowsLegacyConsoleOutput {
        pub fn new() -> io::Result<Self> {
            unsafe {
                let h_output = GetStdHandle(0xFFFFFFF5); // STD_OUTPUT_HANDLE
                if h_output == 0 || h_output == -1 {
                    return Err(io::Error::new(io::ErrorKind::Other, "GetStdHandle failed"));
                }
                Ok(Self { h_output })
            }
        }
    }
    impl WindowsLegacyConsoleInput {
        pub fn new() -> io::Result<Self> {
            unsafe {
                let h_input = GetStdHandle(STD_INPUT_HANDLE);
                if h_input == 0 || h_input == -1 { 
                    return Err(io::Error::new(io::ErrorKind::Other, "GetStdHandle failed")); 
                }

                // Save current console mode
                let mut mode: DWORD = 0;
                if GetConsoleMode(h_input, &mut mode as *mut DWORD) == 0 { 
                    return Err(io::Error::new(io::ErrorKind::Other, "GetConsoleMode failed")); 
                }

                Ok(Self {
                    h_input,
                    original_mode: mode,
                })
            }
        }

        unsafe fn set_console_mode(&self, mode: DWORD) -> io::Result<()> {
            if SetConsoleMode(self.h_input, mode) == 0 {
                return Err(io::Error::new(io::ErrorKind::Other, "SetConsoleMode failed"));
            }
            Ok(())
        }

        fn query_window_size(&self) -> io::Result<(u16, u16)> {
            unsafe {
                let mut info: CONSOLE_SCREEN_BUFFER_INFO = zeroed();
                if GetConsoleScreenBufferInfo(self.h_input, &mut info as *mut _) == 0 {
                    return Err(io::Error::new(io::ErrorKind::Other, "GetConsoleScreenBufferInfo failed"));
                }
                Ok((info.dwSize.X as u16, info.dwSize.Y as u16))
            }
        }

        fn translate_key(ev: &KEY_EVENT_RECORD) -> Option<KeyEvent> {
            if ev.bKeyDown == 0 { return None; }
            
            let ch = ev.UnicodeChar as u32;
            let vk = ev.wVirtualKeyCode;
            
            // Handle control characters first (Ctrl+A..Z)
            if ch >= 1 && ch <= 26 {
                let idx = (ch - 1) as u8;
                let key = match idx {
                    0 => Key::ControlA, 1 => Key::ControlB, 2 => Key::ControlC, 3 => Key::ControlD,
                    4 => Key::ControlE, 5 => Key::ControlF, 6 => Key::ControlG, 7 => Key::ControlH,
                    8 => Key::ControlI, 9 => Key::ControlJ, 10 => Key::ControlK, 11 => Key::ControlL,
                    12 => Key::ControlM,13 => Key::ControlN,14 => Key::ControlO,15 => Key::ControlP,
                    16 => Key::ControlQ,17 => Key::ControlR,18 => Key::ControlS,19 => Key::ControlT,
                    20 => Key::ControlU,21 => Key::ControlV,22 => Key::ControlW,23 => Key::ControlX,
                    24 => Key::ControlY,25 => Key::ControlZ, _ => Key::NotDefined
                };
                return Some(KeyEvent::simple(key, vec![ch as u8]));
            }
            
            // Handle special keys by virtual key code
            let key = match vk {
                0x08 => Key::Backspace, // VK_BACK
                0x0D => Key::Enter,     // VK_RETURN
                0x25 => Key::Left,      // VK_LEFT
                0x26 => Key::Up,        // VK_UP
                0x27 => Key::Right,     // VK_RIGHT
                0x28 => Key::Down,      // VK_DOWN
                0x21 => Key::PageUp,    // VK_PRIOR
                0x22 => Key::PageDown,  // VK_NEXT
                0x23 => Key::End,       // VK_END
                0x24 => Key::Home,      // VK_HOME
                0x2E => Key::Delete,    // VK_DELETE
                0x09 => Key::Tab,       // VK_TAB
                0x1B => Key::Escape,    // VK_ESCAPE
                _ => {
                    // Handle printable characters
                    if ch != 0 && ch >= 32 { // Printable ASCII and above
                        if let Some(c) = char::from_u32(ch) {
                            // Create raw bytes from the character
                            let mut raw_bytes = Vec::new();
                            let mut buf = [0u8; 4];
                            let encoded = c.encode_utf8(&mut buf);
                            raw_bytes.extend_from_slice(encoded.as_bytes());
                            return Some(KeyEvent::with_text(Key::NotDefined, raw_bytes, c.to_string()));
                        }
                    }
                    Key::NotDefined
                }
            };
            
            // For special keys, create appropriate raw bytes
            let raw_bytes = if key != Key::NotDefined {
                vec![vk as u8] // Simple representation
            } else {
                vec![]
            };
            
            Some(KeyEvent::simple(key, raw_bytes))
        }
    }

    impl Drop for WindowsLegacyConsoleInput {
        fn drop(&mut self) {
            unsafe {
                let _ = SetConsoleMode(self.h_input, self.original_mode);
            }
        }
    }



    impl ConsoleInput for WindowsLegacyConsoleInput {
        fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError> {
            unsafe {
                // Enable window + mouse input, disable quick-edit, line and echo to reduce buffering
                // Also disable ENABLE_PROCESSED_INPUT to prevent Windows from handling Ctrl+C
                let mut mode: DWORD = 0;
                if GetConsoleMode(self.h_input, &mut mode as *mut DWORD) == 0 {
                    return Err(ConsoleError::IoError("GetConsoleMode failed".to_string()));
                }
                let original_mode = mode;
                let mut new_mode = mode | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
                new_mode &= !ENABLE_QUICK_EDIT_MODE; // disable quick edit to avoid input freezing
                new_mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT); // disable processed input to handle Ctrl+C ourselves
                if SetConsoleMode(self.h_input, new_mode) == 0 {
                    return Err(ConsoleError::IoError("SetConsoleMode failed".to_string()));
                }
                
                let h_input = self.h_input;
                let restore_fn = move || {
                    unsafe {
                        let _ = SetConsoleMode(h_input, original_mode);
                    }
                };
                
                Ok(RawModeGuard::new(restore_fn, "Windows Legacy".to_string()))
            }
        }

        fn try_read_key(&self) -> Result<Option<KeyEvent>, ConsoleError> {
            unsafe {
                // Check if input is available without blocking
                let mut available: DWORD = 0;
                if GetNumberOfConsoleInputEvents(self.h_input, &mut available) == 0 {
                    return Err(ConsoleError::IoError("GetNumberOfConsoleInputEvents failed".to_string()));
                }
                
                if available == 0 {
                    return Ok(None);
                }
                
                // Read one input event
                let mut buffer: INPUT_RECORD = zeroed();
                let mut events_read: DWORD = 0;
                if ReadConsoleInputW(self.h_input, &mut buffer, 1, &mut events_read) == 0 {
                    return Err(ConsoleError::IoError("ReadConsoleInputW failed".to_string()));
                }
                
                if events_read == 0 {
                    return Ok(None);
                }
                
                // Process only key events
                if buffer.EventType == KEY_EVENT {
                    let key_event = buffer.Event.KeyEvent;
                    if key_event.bKeyDown != 0 { // Only process key down events
                        return Ok(Some(self.convert_key_event(&key_event)));
                    }
                }
                
                // For non-key events or key-up events, return None
                Ok(None)
            }
        }

        fn read_key_timeout(&self, timeout_ms: Option<u32>) -> Result<Option<KeyEvent>, ConsoleError> {
            match timeout_ms {
                Some(0) => {
                    // Non-blocking read
                    self.try_read_key()
                }
                Some(ms) => {
                    // Timeout-based reading using WaitForSingleObject
                    unsafe {
                        let wait_result = WaitForSingleObject(self.h_input, ms);
                        if wait_result == WAIT_OBJECT_0 {
                            // Input is available
                            self.try_read_key()
                        } else {
                            // Timeout or error
                            Ok(None)
                        }
                    }
                }
                None => {
                    // Infinite blocking - keep trying until we get input
                    loop {
                        match self.read_key_timeout(Some(100)) {
                            Ok(Some(key)) => return Ok(Some(key)),
                            Ok(None) => continue, // Timeout, try again
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
            self.query_window_size().map_err(crate::io_error_to_console_error)
        }

        fn get_capabilities(&self) -> ConsoleCapabilities {
            ConsoleCapabilities {
                supports_raw_mode: true,
                supports_resize_events: true,
                supports_bracketed_paste: false,
                supports_mouse_events: true,
                supports_unicode: true,
                platform_name: "Windows Legacy".to_string(),
                backend_type: BackendType::WindowsLegacy,
            }
        }
    }
                    if wait == WAIT_FAILED { break; }
                    if wait == WAIT_OBJECT_0 { // input ready
                        // Drain available records (bounded)
                        let mut buf: [INPUT_RECORD; 32] = [zeroed(); 32];
                        if ReadConsoleInputW(h_input, buf.as_mut_ptr(), buf.len() as DWORD, &mut nread as *mut DWORD) == 0 {
                            continue;
                        }
                        let count = nread as usize;
                        for i in 0..count {
                            let ir = &buf[i];
                            match ir.EventType {
                                KEY_EVENT => {
                                    let kev = ir.Event.KeyEvent;
                                    if let Some(ev) = super::imp::WindowsLegacyConsoleInput::translate_key(&kev) {
                                        if let Ok(mut g) = key_cb.lock() { if let Some(cb) = g.as_mut() { (cb)(ev); } }
                                    }
                                }
                                WINDOW_BUFFER_SIZE_EVENT => {
                                    let sz = ir.Event.WindowBufferSizeEvent.dwSize;
                                    if let Ok(mut g) = resize_cb.lock() { if let Some(cb) = g.as_mut() { (cb)(sz.X as u16, sz.Y as u16); } }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }));
            Ok(())
        
        // Helper method to convert Windows key event to our KeyEvent
        fn convert_key_event(&self, key_event: &KEY_EVENT_RECORD) -> KeyEvent {
            // Use existing translate_key function
            Self::translate_key(key_event).unwrap_or_else(|| {
                // Fallback for undefined keys
                KeyEvent::new(Key::NotDefined, Vec::new(), String::new())
            })
        }
    }
    impl ConsoleOutput for WindowsVtConsoleOutput {
        fn write_text(&self, _text: &str) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn write_styled_text(&self, _text: &str, _style: &TextStyle) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn write_safe_text(&self, _text: &str) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn move_cursor_to(&self, _row: u16, _col: u16) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn move_cursor_relative(&self, _row_delta: i16, _col_delta: i16) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn clear(&self, _clear_type: ClearType) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn reset_style(&self) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn flush(&self) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn set_cursor_visible(&self, _visible: bool) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "Windows VT output".to_string(), 
                platform: "Windows".to_string() 
            })
        }
        fn get_capabilities(&self) -> OutputCapabilities {
            OutputCapabilities {
                supports_colors: false,
                supports_true_color: false,
                supports_styling: false,
                supports_alternate_screen: false,
                supports_cursor_control: false,
                max_colors: 0,
                platform_name: "Windows VT (not implemented)".to_string(),
                backend_type: BackendType::WindowsVt,
            }
        }
    }

    impl ConsoleOutput for WindowsLegacyConsoleOutput {
        fn write_text(&self, text: &str) -> ConsoleResult<()> {
            // Basic implementation - write UTF-8 text to console
            // In a full implementation, we'd use WriteConsoleW for proper Unicode support
            let bytes = text.as_bytes();
            unsafe {
                let mut written: DWORD = 0;
                // This is a simplified approach - real implementation would use WriteConsoleW
                if libc::write(1, bytes.as_ptr() as *const libc::c_void, bytes.len()) == -1 {
                    return Err(ConsoleError::IoError("Failed to write to console".to_string()));
                }
            }
            Ok(())
        }
        
        fn write_styled_text(&self, text: &str, _style: &TextStyle) -> ConsoleResult<()> {
            // For now, just write text without styling
            // Full implementation would use SetConsoleTextAttribute
            self.write_text(text)
        }
        
        fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
            self.write_text(text)
        }
        
        fn move_cursor_to(&self, _row: u16, _col: u16) -> ConsoleResult<()> {
            // Would use SetConsoleCursorPosition in full implementation
            Err(ConsoleError::UnsupportedFeature { 
                feature: "cursor positioning".to_string(), 
                platform: "Windows Legacy".to_string() 
            })
        }
        
        fn move_cursor_relative(&self, _row_delta: i16, _col_delta: i16) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "relative cursor movement".to_string(), 
                platform: "Windows Legacy".to_string() 
            })
        }
        
        fn clear(&self, _clear_type: ClearType) -> ConsoleResult<()> {
            // Would use FillConsoleOutputCharacter in full implementation
            Err(ConsoleError::UnsupportedFeature { 
                feature: "screen clearing".to_string(), 
                platform: "Windows Legacy".to_string() 
            })
        }
        
        fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> {
            // Would use SetConsoleTextAttribute in full implementation
            Ok(())
        }
        
        fn reset_style(&self) -> ConsoleResult<()> {
            Ok(())
        }
        
        fn flush(&self) -> ConsoleResult<()> {
            // Windows console doesn't need explicit flushing
            Ok(())
        }
        
        fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> {
            Err(ConsoleError::UnsupportedFeature { 
                feature: "alternate screen".to_string(), 
                platform: "Windows Legacy".to_string() 
            })
        }
        
        fn set_cursor_visible(&self, _visible: bool) -> ConsoleResult<()> {
            // Would use SetConsoleCursorInfo in full implementation
            Ok(())
        }
        
        fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
            // Would use GetConsoleScreenBufferInfo in full implementation
            Ok((0, 0))
        }
        
        fn get_capabilities(&self) -> OutputCapabilities {
            OutputCapabilities {
                supports_colors: true,
                supports_true_color: false,
                supports_styling: true,
                supports_alternate_screen: false,
                supports_cursor_control: true,
                max_colors: 16,
                platform_name: "Windows Legacy".to_string(),
                backend_type: BackendType::WindowsLegacy,
            }
        }
    }}


#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
pub struct WindowsVtConsoleInput;
#[cfg(not(windows))]
pub struct WindowsVtConsoleOutput;
#[cfg(not(windows))]
pub struct WindowsLegacyConsoleInput;
#[cfg(not(windows))]
pub struct WindowsLegacyConsoleOutput;

#[cfg(not(windows))]
impl WindowsVtConsoleInput {
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Windows not supported"))
    }
}

#[cfg(not(windows))]
impl WindowsVtConsoleOutput {
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Windows not supported"))
    }
}

#[cfg(not(windows))]
impl WindowsLegacyConsoleInput {
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Windows not supported"))
    }
}

#[cfg(not(windows))]
impl crate::ConsoleInput for WindowsLegacyConsoleInput {
    fn enable_raw_mode(&self) -> Result<crate::RawModeGuard, crate::ConsoleError> {
        Err(crate::ConsoleError::UnsupportedFeature { 
            feature: "Windows console input".to_string(), 
            platform: "Non-Windows".to_string() 
        })
    }
    
    fn try_read_key(&self) -> Result<Option<replkit_core::KeyEvent>, crate::ConsoleError> {
        Err(crate::ConsoleError::UnsupportedFeature { 
            feature: "Windows console input".to_string(), 
            platform: "Non-Windows".to_string() 
        })
    }
    
    fn read_key_timeout(&self, _timeout_ms: Option<u32>) -> Result<Option<replkit_core::KeyEvent>, crate::ConsoleError> {
        Err(crate::ConsoleError::UnsupportedFeature { 
            feature: "Windows console input".to_string(), 
            platform: "Non-Windows".to_string() 
        })
    }
    
    fn get_window_size(&self) -> crate::ConsoleResult<(u16, u16)> {
        Err(crate::ConsoleError::UnsupportedFeature { 
            feature: "Windows console input".to_string(), 
            platform: "Non-Windows".to_string() 
        })
    }
    
    fn get_capabilities(&self) -> crate::ConsoleCapabilities {
        crate::ConsoleCapabilities {
            supports_raw_mode: false,
            supports_resize_events: false,
            supports_bracketed_paste: false,
            supports_mouse_events: false,
            supports_unicode: false,
            platform_name: "Windows (not available)".to_string(),
            backend_type: crate::BackendType::WindowsLegacy,
        }
    }
}

#[cfg(not(windows))]
impl WindowsLegacyConsoleOutput {
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Windows not supported"))
    }
}