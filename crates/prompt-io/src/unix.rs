use std::io::{self};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use prompt_core::{KeyEvent, KeyParser};
use crate::{ConsoleError, ConsoleInput, ConsoleOutput, ConsoleResult, RawModeGuard, 
           ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, Color, ClearType, AsAny};

struct UnixRawModeGuard {
    stdin_fd: i32,
    original_termios: libc::termios,
    original_flags: i32,
}

impl Drop for UnixRawModeGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::tcsetattr(self.stdin_fd, libc::TCSANOW, &self.original_termios);
            let _ = libc::fcntl(self.stdin_fd, libc::F_SETFL, self.original_flags);
        }
    }
}

pub struct UnixConsoleInput {
    stdin_fd: i32,
    raw_guard: Option<UnixRawModeGuard>,
    running: Arc<AtomicBool>,
    wake_fds: (i32, i32), // (read, write)
    resize_cb: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
    key_cb: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
    thread: Option<JoinHandle<()>>,
}

impl UnixConsoleInput {
    pub fn new() -> io::Result<Self> {
        // Create self-pipe for waking up poll on stop/resize checks
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Set read end non-blocking
        let flags = unsafe { libc::fcntl(fds[0], libc::F_GETFL) };
        if flags != -1 {
            unsafe { libc::fcntl(fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        Ok(Self {
            stdin_fd: io::stdin().as_raw_fd(),
            raw_guard: None,
            running: Arc::new(AtomicBool::new(false)),
            wake_fds: (fds[0], fds[1]),
            resize_cb: Arc::new(Mutex::new(None)),
            key_cb: Arc::new(Mutex::new(None)),
            thread: None,
        })
    }

    fn enter_raw_mode(fd: i32) -> io::Result<UnixRawModeGuard> {
        let mut original_termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut original_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut raw = original_termios;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ECHOE | libc::ECHOK | libc::ECHONL | libc::ISIG | libc::IEXTEN);
        raw.c_iflag &= !(libc::IXON | libc::IXOFF | libc::ICRNL | libc::INLCR | libc::IGNCR | libc::BRKINT | libc::PARMRK | libc::ISTRIP);
        raw.c_oflag &= !libc::OPOST;
        raw.c_cflag &= !libc::CSIZE;
        raw.c_cflag |= libc::CS8;
        raw.c_cc[libc::VMIN] = 0; // non-blocking
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags == -1 { return Err(io::Error::last_os_error()); }
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(UnixRawModeGuard { stdin_fd: fd, original_termios, original_flags: flags })
    }

    fn poll_loop(
        stdin_fd: i32,
        wake_read: i32,
        running: Arc<AtomicBool>,
        resize_cb: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
        key_cb: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
    ) {
        let mut parser = KeyParser::new();

        // Initial window size to detect changes
        let mut last_size = Self::query_window_size().ok();
        if let Some((cols, rows)) = last_size {
            if let Ok(mut g) = resize_cb.lock() {
                if let Some(cb) = g.as_mut() {
                    (cb)(cols, rows);
                }
            }
        }

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            // Prepare poll fds: stdin and wake pipe
            let mut fds = [
                libc::pollfd { fd: stdin_fd, events: libc::POLLIN, revents: 0 },
                libc::pollfd { fd: wake_read, events: libc::POLLIN, revents: 0 },
            ];
            let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 50) }; // 50ms timeout to check resize
            if rc < 0 {
                // Interrupted; continue
                continue;
            }

            // Drain wake pipe if signaled
            if fds[1].revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 64];
                unsafe { libc::read(wake_read, buf.as_mut_ptr() as *mut _, buf.len()) };
            }

            // Read stdin if ready
            if fds[0].revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 1024];
                loop {
                    let n = unsafe { libc::read(stdin_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                    if n <= 0 { break; }
                    let bytes = &buf[..n as usize];
                    let events = parser.feed(bytes);
                    if !events.is_empty() {
                        if let Ok(mut g) = key_cb.lock() {
                            if let Some(cb) = g.as_mut() {
                                for ev in events { (cb)(ev); }
                            }
                        }
                    }
                }
            }

            // Resize check
            if let Ok((cols, rows)) = Self::query_window_size() {
                match last_size {
                    Some((c, r)) if c == cols && r == rows => {}
                    _ => {
                        last_size = Some((cols, rows));
                        if let Ok(mut g) = resize_cb.lock() {
                            if let Some(cb) = g.as_mut() { (cb)(cols, rows); }
                        }
                    }
                }
            }
        }
    }

    fn query_window_size() -> io::Result<(u16, u16)> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok((ws.ws_col as u16, ws.ws_row as u16))
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
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError> {
        let unix_guard = Self::enter_raw_mode(self.stdin_fd).map_err(crate::io_error_to_console_error)?;
        let stdin_fd = self.stdin_fd;
        let original_termios = unix_guard.original_termios;
        let original_flags = unix_guard.original_flags;
        
        // Prevent the unix_guard from running its Drop
        std::mem::forget(unix_guard);
        
        let restore_fn = move || {
            unsafe {
                let _ = libc::tcsetattr(stdin_fd, libc::TCSANOW, &original_termios);
                let _ = libc::fcntl(stdin_fd, libc::F_SETFL, original_flags);
            }
        };
        
        Ok(RawModeGuard::new(restore_fn, "Unix VT".to_string()))
    }

    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        Self::query_window_size().map_err(crate::io_error_to_console_error)
    }

    fn start_event_loop(&self) -> ConsoleResult<()> {
        if self.running.swap(true, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::AlreadyRunning));
        }
        let stdin_fd = self.stdin_fd;
        let wake_read = self.wake_fds.0;
        let running = self.running.clone();
        let resize_cb = self.resize_cb.clone();
        let key_cb = self.key_cb.clone();
        
        // Store thread handle - this is a simplified approach
        // In a real implementation, we'd need better thread management
        thread::spawn(move || {
            Self::poll_loop(stdin_fd, wake_read, running, resize_cb, key_cb);
        });
        Ok(())
    }

    fn stop_event_loop(&self) -> ConsoleResult<()> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::NotRunning));
        }
        // Wake the poll by writing a byte
        let _ = unsafe { libc::write(self.wake_fds.1, &1u8 as *const _ as *const _, 1) };
        Ok(())
    }

    fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        if let Ok(mut g) = self.resize_cb.lock() {
            *g = Some(callback);
        }
    }

    fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        if let Ok(mut g) = self.key_cb.lock() {
            *g = Some(callback);
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: true,
            supports_bracketed_paste: true,
            supports_mouse_events: true,
            supports_unicode: true,
            platform_name: "Unix VT".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
}

/// Unix console output implementation using ANSI escape sequences
pub struct UnixConsoleOutput {
    stdout_fd: i32,
}

impl UnixConsoleOutput {
    pub fn new() -> ConsoleResult<Self> {
        Ok(Self {
            stdout_fd: libc::STDOUT_FILENO,
        })
    }
    
    fn write_ansi(&self, sequence: &str) -> ConsoleResult<()> {
        let bytes = sequence.as_bytes();
        let result = unsafe {
            libc::write(self.stdout_fd, bytes.as_ptr() as *const libc::c_void, bytes.len())
        };
        if result == -1 {
            Err(ConsoleError::IoError("Failed to write to stdout".to_string()))
        } else {
            Ok(())
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
    fn write_text(&self, text: &str) -> ConsoleResult<()> {
        let bytes = text.as_bytes();
        let result = unsafe {
            libc::write(self.stdout_fd, bytes.as_ptr() as *const libc::c_void, bytes.len())
        };
        if result == -1 {
            Err(ConsoleError::IoError("Failed to write text".to_string()))
        } else {
            Ok(())
        }
    }
    
    fn write_styled_text(&self, text: &str, style: &TextStyle) -> ConsoleResult<()> {
        // Apply style first
        self.set_style(style)?;
        // Write text
        self.write_text(text)?;
        // Reset style
        self.reset_style()
    }
    
    fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
        // For now, just write text directly - in a full implementation,
        // we'd use SafeTextFilter here
        self.write_text(text)
    }
    
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
        // Convert 0-based to 1-based for ANSI
        let ansi_seq = format!("\x1b[{};{}H", row + 1, col + 1);
        self.write_ansi(&ansi_seq)
    }
    
    fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
        if row_delta != 0 {
            if row_delta > 0 {
                self.write_ansi(&format!("\x1b[{}B", row_delta))?;
            } else {
                self.write_ansi(&format!("\x1b[{}A", -row_delta))?;
            }
        }
        if col_delta != 0 {
            if col_delta > 0 {
                self.write_ansi(&format!("\x1b[{}C", col_delta))?;
            } else {
                self.write_ansi(&format!("\x1b[{}D", -col_delta))?;
            }
        }
        Ok(())
    }
    
    fn clear(&self, clear_type: ClearType) -> ConsoleResult<()> {
        let ansi_seq = match clear_type {
            ClearType::All => "\x1b[2J",
            ClearType::FromCursor => "\x1b[0J",
            ClearType::ToCursor => "\x1b[1J",
            ClearType::CurrentLine => "\x1b[2K",
            ClearType::FromCursorToEndOfLine => "\x1b[0K",
            ClearType::FromBeginningOfLineToCursor => "\x1b[1K",
        };
        self.write_ansi(ansi_seq)
    }
    
    fn set_style(&self, style: &TextStyle) -> ConsoleResult<()> {
        let mut ansi_codes = Vec::new();
        
        // Foreground color
        if let Some(fg) = &style.foreground {
            match fg {
                Color::Black => ansi_codes.push("30"),
                Color::Red => ansi_codes.push("31"),
                Color::Green => ansi_codes.push("32"),
                Color::Yellow => ansi_codes.push("33"),
                Color::Blue => ansi_codes.push("34"),
                Color::Magenta => ansi_codes.push("35"),
                Color::Cyan => ansi_codes.push("36"),
                Color::White => ansi_codes.push("37"),
                Color::BrightBlack => ansi_codes.push("90"),
                Color::BrightRed => ansi_codes.push("91"),
                Color::BrightGreen => ansi_codes.push("92"),
                Color::BrightYellow => ansi_codes.push("93"),
                Color::BrightBlue => ansi_codes.push("94"),
                Color::BrightMagenta => ansi_codes.push("95"),
                Color::BrightCyan => ansi_codes.push("96"),
                Color::BrightWhite => ansi_codes.push("97"),
                Color::Rgb(r, g, b) => {
                    let rgb_code = format!("38;2;{};{};{}", r, g, b);
                    return self.write_ansi(&format!("\x1b[{}m", rgb_code));
                }
                Color::Ansi256(n) => {
                    let ansi_code = format!("38;5;{}", n);
                    return self.write_ansi(&format!("\x1b[{}m", ansi_code));
                }
            }
        }
        
        // Background color
        if let Some(bg) = &style.background {
            match bg {
                Color::Black => ansi_codes.push("40"),
                Color::Red => ansi_codes.push("41"),
                Color::Green => ansi_codes.push("42"),
                Color::Yellow => ansi_codes.push("43"),
                Color::Blue => ansi_codes.push("44"),
                Color::Magenta => ansi_codes.push("45"),
                Color::Cyan => ansi_codes.push("46"),
                Color::White => ansi_codes.push("47"),
                Color::BrightBlack => ansi_codes.push("100"),
                Color::BrightRed => ansi_codes.push("101"),
                Color::BrightGreen => ansi_codes.push("102"),
                Color::BrightYellow => ansi_codes.push("103"),
                Color::BrightBlue => ansi_codes.push("104"),
                Color::BrightMagenta => ansi_codes.push("105"),
                Color::BrightCyan => ansi_codes.push("106"),
                Color::BrightWhite => ansi_codes.push("107"),
                Color::Rgb(r, g, b) => {
                    let rgb_code = format!("48;2;{};{};{}", r, g, b);
                    return self.write_ansi(&format!("\x1b[{}m", rgb_code));
                }
                Color::Ansi256(n) => {
                    let ansi_code = format!("48;5;{}", n);
                    return self.write_ansi(&format!("\x1b[{}m", ansi_code));
                }
            }
        }
        
        // Text attributes
        if style.bold { ansi_codes.push("1"); }
        if style.italic { ansi_codes.push("3"); }
        if style.underline { ansi_codes.push("4"); }
        if style.strikethrough { ansi_codes.push("9"); }
        if style.dim { ansi_codes.push("2"); }
        if style.reverse { ansi_codes.push("7"); }
        
        if !ansi_codes.is_empty() {
            let ansi_seq = format!("\x1b[{}m", ansi_codes.join(";"));
            self.write_ansi(&ansi_seq)
        } else {
            Ok(())
        }
    }
    
    fn reset_style(&self) -> ConsoleResult<()> {
        self.write_ansi("\x1b[0m")
    }
    
    fn flush(&self) -> ConsoleResult<()> {
        // Unix doesn't need explicit flush for write() calls
        Ok(())
    }
    
    fn set_alternate_screen(&self, enabled: bool) -> ConsoleResult<()> {
        if enabled {
            self.write_ansi("\x1b[?1049h")
        } else {
            self.write_ansi("\x1b[?1049l")
        }
    }
    
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
        if visible {
            self.write_ansi("\x1b[?25h")
        } else {
            self.write_ansi("\x1b[?25l")
        }
    }
    
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        // This is a simplified implementation - in practice, we'd need to:
        // 1. Send cursor position query: "\x1b[6n"
        // 2. Read response from stdin: "\x1b[{row};{col}R"
        // 3. Parse the response
        // For now, return a placeholder
        Err(ConsoleError::UnsupportedFeature {
            feature: "cursor position query".to_string(),
            platform: "Unix".to_string(),
        })
    }
    
    fn get_capabilities(&self) -> OutputCapabilities {
        OutputCapabilities {
            supports_colors: true,
            supports_true_color: true,
            supports_styling: true,
            supports_alternate_screen: true,
            supports_cursor_control: true,
            max_colors: 65535, // Limited by u16, but supports true color
            platform_name: "Unix VT".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
}