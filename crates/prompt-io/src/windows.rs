#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};

    use crate::{ConsoleError, ConsoleInput, ConsoleResult, KeyEvent};
    use prompt_core::Key;

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
    union INPUT_EVENT {
        KeyEvent: KEY_EVENT_RECORD,
        WindowBufferSizeEvent: WINDOW_BUFFER_SIZE_RECORD,
    }

    #[repr(C)]
    struct INPUT_RECORD { EventType: WORD, Event: INPUT_EVENT }

    extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
        fn ReadConsoleInputW(hConsoleInput: HANDLE, lpBuffer: *mut INPUT_RECORD, nLength: DWORD, lpNumberOfEventsRead: *mut DWORD) -> BOOL;
        fn GetConsoleScreenBufferInfo(hConsoleOutput: HANDLE, lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO) -> BOOL;
        fn CreateEventW(lpEventAttributes:*mut c_void, bManualReset: BOOL, bInitialState: BOOL, lpName: *const WCHAR) -> HANDLE;
        fn SetEvent(hEvent: HANDLE) -> BOOL;
        fn WaitForMultipleObjects(nCount: DWORD, lpHandles: *const HANDLE, bWaitAll: BOOL, dwMilliseconds: DWORD) -> DWORD;
    }

    pub struct WindowsVtConsoleInput; // TODO

    pub struct WindowsLegacyConsoleInput {
        h_input: HANDLE,
        original_mode: DWORD,
        running: Arc<AtomicBool>,
        stop_event: HANDLE,
        resize_cb: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
        key_cb: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
        thread: Option<JoinHandle<()>>,
    }

    impl WindowsVtConsoleInput {
        pub fn new() -> io::Result<Self> { Ok(Self) }
    }
    impl WindowsLegacyConsoleInput {
        pub fn new() -> io::Result<Self> {
            unsafe {
                let h_input = GetStdHandle(STD_INPUT_HANDLE);
                if h_input == 0 || h_input == -1 { return Err(io::Error::new(io::ErrorKind::Other, "GetStdHandle failed")); }

                // Save current console mode
                let mut mode: DWORD = 0;
                if GetConsoleMode(h_input, &mut mode as *mut DWORD) == 0 { return Err(io::Error::new(io::ErrorKind::Other, "GetConsoleMode failed")); }

                // Create stop event
                let stop_event = CreateEventW(null_mut(), 1, 0, null());
                if stop_event == 0 { return Err(io::Error::new(io::ErrorKind::Other, "CreateEventW failed")); }

                Ok(Self {
                    h_input,
                    original_mode: mode,
                    running: Arc::new(AtomicBool::new(false)),
                    stop_event,
                    resize_cb: Arc::new(Mutex::new(None)),
                    key_cb: Arc::new(Mutex::new(None)),
                    thread: None,
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
            if ch != 0 {
                // Printable path
                if let Some(c) = char::from_u32(ch) {
                    return Some(KeyEvent::with_text(Key::NotDefined, vec![], c.to_string()));
                }
            }
            // Fallback on virtual key code mappings
            let vk = ev.wVirtualKeyCode;
            let key = match vk {
                0x08 => Key::Backspace, // VK_BACK
                0x0D => Key::Enter,     // VK_RETURN
                0x25 => Key::Left,      // VK_LEFT
                0x26 => Key::Up,        // VK_UP
                0x27 => Key::Right,     // VK_RIGHT
                0x28 => Key::Down,      // VK_DOWN
                _ => {
                    // Handle Ctrl+A..Z when UnicodeChar is control code 1..26
                    match ev.UnicodeChar {
                        1..=26 => {
                            let idx = (ev.UnicodeChar - 1) as u8;
                            let key = match idx {
                                0 => Key::ControlA, 1 => Key::ControlB, 2 => Key::ControlC, 3 => Key::ControlD,
                                4 => Key::ControlE, 5 => Key::ControlF, 6 => Key::ControlG, 7 => Key::ControlH,
                                8 => Key::ControlI, 9 => Key::ControlJ, 10 => Key::ControlK, 11 => Key::ControlL,
                                12 => Key::ControlM,13 => Key::ControlN,14 => Key::ControlO,15 => Key::ControlP,
                                16 => Key::ControlQ,17 => Key::ControlR,18 => Key::ControlS,19 => Key::ControlT,
                                20 => Key::ControlU,21 => Key::ControlV,22 => Key::ControlW,23 => Key::ControlX,
                                24 => Key::ControlY,25 => Key::ControlZ, _ => Key::NotDefined
                            };
                            key
                        }
                        _ => Key::NotDefined
                    }
                }
            };
            Some(KeyEvent::simple(key, vec![]))
        }
    }

    impl Drop for WindowsLegacyConsoleInput {
        fn drop(&mut self) {
            unsafe {
                let _ = SetConsoleMode(self.h_input, self.original_mode);
            }
        }
    }

    impl ConsoleInput for WindowsVtConsoleInput {
        fn enable_raw_mode(&mut self) -> ConsoleResult<()> { Err(ConsoleError::UnsupportedFeature("Windows VT not yet implemented")) }
        fn get_window_size(&self) -> ConsoleResult<(u16, u16)> { Err(ConsoleError::UnsupportedFeature("Windows VT not yet implemented")) }
        fn set_resize_callback(&mut self, _callback: Box<dyn FnMut(u16, u16) + Send>) {}
        fn set_key_event_callback(&mut self, _callback: Box<dyn FnMut(KeyEvent) + Send>) {}
        fn start_event_loop(&mut self) -> ConsoleResult<()> { Err(ConsoleError::UnsupportedFeature("Windows VT not yet implemented")) }
        fn stop_event_loop(&mut self) -> ConsoleResult<()> { Err(ConsoleError::UnsupportedFeature("Windows VT not yet implemented")) }
    }

    impl ConsoleInput for WindowsLegacyConsoleInput {
        fn enable_raw_mode(&mut self) -> ConsoleResult<()> {
            unsafe {
                // Enable window + mouse input, disable quick-edit, line and echo to reduce buffering
                let mut mode: DWORD = 0;
                if GetConsoleMode(self.h_input, &mut mode as *mut DWORD) == 0 {
                    return Err(ConsoleError::Io(io::Error::new(io::ErrorKind::Other, "GetConsoleMode failed")));
                }
                let mut new_mode = mode | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
                new_mode &= !ENABLE_QUICK_EDIT_MODE; // disable quick edit to avoid input freezing
                new_mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
                if SetConsoleMode(self.h_input, new_mode) == 0 {
                    return Err(ConsoleError::Io(io::Error::new(io::ErrorKind::Other, "SetConsoleMode failed")));
                }
                Ok(())
            }
        }

        fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
            Ok(self.query_window_size()?)
        }

        fn set_resize_callback(&mut self, callback: Box<dyn FnMut(u16, u16) + Send>) {
            if let Ok(mut g) = self.resize_cb.lock() { *g = Some(callback); }
        }

        fn set_key_event_callback(&mut self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
            if let Ok(mut g) = self.key_cb.lock() { *g = Some(callback); }
        }

        fn start_event_loop(&mut self) -> ConsoleResult<()> {
            if self.running.swap(true, Ordering::Relaxed) { return Err(ConsoleError::AlreadyRunning); }
            let h_input = self.h_input;
            let h_stop = self.stop_event;
            let running = self.running.clone();
            let resize_cb = self.resize_cb.clone();
            let key_cb = self.key_cb.clone();
            self.thread = Some(thread::spawn(move || unsafe {
                // Emit initial size
                // We can't call self methods here; minimal omission.
                let mut rec: INPUT_RECORD = zeroed();
                let mut nread: DWORD = 0;
                let handles = [h_input, h_stop];
                loop {
                    if !running.load(Ordering::Relaxed) { break; }
                    let wait = WaitForMultipleObjects(2, handles.as_ptr(), 0, 100); // 100ms timeout for periodic checks
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
        }

        fn stop_event_loop(&mut self) -> ConsoleResult<()> {
            if !self.running.swap(false, Ordering::Relaxed) { return Err(ConsoleError::NotRunning); }
            unsafe { let _ = SetEvent(self.stop_event); }
            if let Some(h) = self.thread.take() { let _ = h.join(); }
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use imp::*;
