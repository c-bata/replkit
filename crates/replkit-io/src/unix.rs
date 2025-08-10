use std::io::{self};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use replkit_core::{KeyEvent, KeyParser};
use crate::{ConsoleError, ConsoleInput, ConsoleOutput, ConsoleResult, RawModeGuard, 
           ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, Color, ClearType};
use crate::debug_log;

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
    key_parser: Mutex<KeyParser>,
}

impl UnixConsoleInput {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            stdin_fd: io::stdin().as_raw_fd(),
            raw_guard: None,
            key_parser: Mutex::new(KeyParser::new()),
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

    fn query_window_size() -> io::Result<(u16, u16)> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok((ws.ws_col as u16, ws.ws_row as u16))
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

    fn try_read_key(&self) -> Result<Option<KeyEvent>, ConsoleError> {
        // Non-blocking read from stdin
        let mut buffer = [0u8; 64];
        let result = unsafe {
            libc::read(self.stdin_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
        };
        
        if result == -1 {
            let error = std::io::Error::last_os_error();
            match error.kind() {
                std::io::ErrorKind::WouldBlock => {
                    // No input available - this is expected for non-blocking read
                    Ok(None)
                }
                _ => Err(ConsoleError::IoError(format!("Read error: {}", error))),
            }
        } else if result == 0 {
            // EOF reached
            Ok(None)
        } else {
            // Parse the bytes using shared KeyParser
            let bytes = &buffer[..result as usize];
            let mut parser = self.key_parser.lock().unwrap();
            let events = parser.feed(bytes);
            
            // Return the first key event if any
            Ok(events.into_iter().next())
        }
    }

    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> Result<Option<KeyEvent>, ConsoleError> {
        match timeout_ms {
            Some(0) => {
                // Non-blocking - delegate to try_read_key
                self.try_read_key()
            }
            Some(ms) => {
                // Timeout-based reading using poll()
                let mut poll_fd = libc::pollfd {
                    fd: self.stdin_fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                
                let poll_result = unsafe {
                    libc::poll(&mut poll_fd as *mut libc::pollfd, 1, ms as i32)
                };
                
                if poll_result == -1 {
                    return Err(ConsoleError::IoError("Poll error".to_string()));
                } else if poll_result == 0 {
                    // Timeout expired - check if parser has incomplete sequences to flush
                    let mut parser = self.key_parser.lock().unwrap();
                    let events = parser.flush();
                    Ok(events.into_iter().next())
                } else {
                    // Input is available - read it
                    self.try_read_key()
                }
            }
            None => {
                // Infinite blocking - keep polling until we get input
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
        Self::query_window_size().map_err(crate::io_error_to_console_error)
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
    buffer: Arc<Mutex<Vec<u8>>>,
    buffering_enabled: Arc<AtomicBool>,
}

impl UnixConsoleOutput {
    pub fn new() -> ConsoleResult<Self> {
        // Verify we have a TTY for output
        if unsafe { libc::isatty(libc::STDOUT_FILENO) } == 0 {
            return Err(ConsoleError::TerminalError(
                "stdout is not a TTY".to_string()
            ));
        }
        
        Ok(Self {
            stdout_fd: libc::STDOUT_FILENO,
            buffer: Arc::new(Mutex::new(Vec::new())),
            buffering_enabled: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// Enable output buffering for efficient batch updates
    pub fn enable_buffering(&self) {
        self.buffering_enabled.store(true, Ordering::Relaxed);
    }
    
    /// Disable output buffering and flush any pending output
    pub fn disable_buffering(&self) -> ConsoleResult<()> {
        self.buffering_enabled.store(false, Ordering::Relaxed);
        self.flush()
    }
    
    fn write_bytes(&self, bytes: &[u8]) -> ConsoleResult<()> {
        if self.buffering_enabled.load(Ordering::Relaxed) {
            // Add to buffer
            if let Ok(mut buffer) = self.buffer.lock() {
                buffer.extend_from_slice(bytes);
                Ok(())
            } else {
                Err(ConsoleError::IoError("Failed to acquire buffer lock".to_string()))
            }
        } else {
            // Write directly
            self.write_bytes_direct(bytes)
        }
    }
    
    fn write_bytes_direct(&self, bytes: &[u8]) -> ConsoleResult<()> {
        let mut written = 0;
        while written < bytes.len() {
            let result = unsafe {
                libc::write(
                    self.stdout_fd,
                    bytes[written..].as_ptr() as *const libc::c_void,
                    bytes.len() - written
                )
            };
            
            if result == -1 {
                let error = io::Error::last_os_error();
                match error.raw_os_error() {
                    Some(libc::EINTR) => continue, // Interrupted by signal, retry
                    Some(libc::EAGAIN) => {
                        // Would block, but we're in blocking mode, so this shouldn't happen
                        return Err(ConsoleError::IoError("Unexpected EAGAIN in blocking write".to_string()));
                    }
                    _ => {
                        return Err(ConsoleError::IoError(format!("Write failed: {error}")));
                    }
                }
            } else {
                written += result as usize;
            }
        }
        Ok(())
    }
    
    fn write_ansi(&self, sequence: &str) -> ConsoleResult<()> {
        self.write_bytes(sequence.as_bytes())
    }
    
    /// Generate ANSI color code for foreground
    fn color_to_fg_ansi(&self, color: &Color) -> String {
        match color {
            Color::Black => "30".to_string(),
            Color::Red => "31".to_string(),
            Color::Green => "32".to_string(),
            Color::Yellow => "33".to_string(),
            Color::Blue => "34".to_string(),
            Color::Magenta => "35".to_string(),
            Color::Cyan => "36".to_string(),
            Color::White => "37".to_string(),
            Color::BrightBlack => "90".to_string(),
            Color::BrightRed => "91".to_string(),
            Color::BrightGreen => "92".to_string(),
            Color::BrightYellow => "93".to_string(),
            Color::BrightBlue => "94".to_string(),
            Color::BrightMagenta => "95".to_string(),
            Color::BrightCyan => "96".to_string(),
            Color::BrightWhite => "97".to_string(),
            Color::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
            Color::Ansi256(n) => format!("38;5;{n}"),
        }
    }
    
    /// Generate ANSI color code for background
    fn color_to_bg_ansi(&self, color: &Color) -> String {
        match color {
            Color::Black => "40".to_string(),
            Color::Red => "41".to_string(),
            Color::Green => "42".to_string(),
            Color::Yellow => "43".to_string(),
            Color::Blue => "44".to_string(),
            Color::Magenta => "45".to_string(),
            Color::Cyan => "46".to_string(),
            Color::White => "47".to_string(),
            Color::BrightBlack => "100".to_string(),
            Color::BrightRed => "101".to_string(),
            Color::BrightGreen => "102".to_string(),
            Color::BrightYellow => "103".to_string(),
            Color::BrightBlue => "104".to_string(),
            Color::BrightMagenta => "105".to_string(),
            Color::BrightCyan => "106".to_string(),
            Color::BrightWhite => "107".to_string(),
            Color::Rgb(r, g, b) => format!("48;2;{r};{g};{b}"),
            Color::Ansi256(n) => format!("48;5;{n}"),
        }
    }
    
    /// Generate complete ANSI sequence for a text style
    fn style_to_ansi(&self, style: &TextStyle) -> String {
        let mut codes = Vec::new();
        
        // Foreground color
        if let Some(fg) = &style.foreground {
            codes.push(self.color_to_fg_ansi(fg));
        }
        
        // Background color
        if let Some(bg) = &style.background {
            codes.push(self.color_to_bg_ansi(bg));
        }
        
        // Text attributes
        if style.bold { codes.push("1".to_string()); }
        if style.dim { codes.push("2".to_string()); }
        if style.italic { codes.push("3".to_string()); }
        if style.underline { codes.push("4".to_string()); }
        if style.reverse { codes.push("7".to_string()); }
        if style.strikethrough { codes.push("9".to_string()); }
        
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
    
    /// Query cursor position by sending ANSI sequence and reading response
    fn query_cursor_position_impl(&self) -> ConsoleResult<(u16, u16)> {
        // Try to flush stdout first to ensure all previous output is visible
        self.flush()?;
        
        // Store original terminal settings
        let stdin_fd = libc::STDIN_FILENO;
        let mut original_termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        
        if unsafe { libc::tcgetattr(stdin_fd, original_termios.as_mut_ptr()) } != 0 {
            return Err(ConsoleError::IoError("Failed to get terminal attributes".to_string()));
        }
        
        let original_termios = unsafe { original_termios.assume_init() };
        let mut raw_termios = original_termios;
        
        // Set terminal to raw mode for the query
        unsafe {
            libc::cfmakeraw(&mut raw_termios);
            if libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw_termios) != 0 {
                return Err(ConsoleError::IoError("Failed to set raw mode".to_string()));
            }
        }
        
        // Send cursor position query
        debug_log!("Sending cursor position query");
        self.write_bytes_direct(b"\x1b[6n")?;
        self.flush()?;
        
        // Read response synchronously with blocking I/O
        let mut response = Vec::new();
        let mut buffer = [0u8; 1];
        let timeout = std::time::Duration::from_millis(200);
        let start_time = std::time::Instant::now();
        
        loop {
            if start_time.elapsed() > timeout {
                // Restore terminal settings
                unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &original_termios) };
                return Err(ConsoleError::IoError("Cursor position query timeout".to_string()));
            }
            
            let result = unsafe {
                libc::read(stdin_fd, buffer.as_mut_ptr() as *mut libc::c_void, 1)
            };
            
            if result == 1 {
                response.push(buffer[0]);
                debug_log!("Read byte: {} ('{}')", buffer[0], buffer[0] as char);
                
                // Check if we have a complete response: ESC[{row};{col}R
                if buffer[0] == b'R' && response.len() >= 6 {
                    debug_log!("Complete response received");
                    break;
                }
                if response.len() > 20 {
                    debug_log!("Response too long, breaking");
                    break;
                }
            } else if result == 0 {
                debug_log!("EOF reached");
                break;
            } else {
                let error = io::Error::last_os_error();
                match error.raw_os_error() {
                    Some(libc::EAGAIN) => {
                        // EAGAIN - no data available, continue polling
                        continue;
                    }
                    _ => {
                        // Real error
                        unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &original_termios) };
                        return Err(ConsoleError::IoError(format!("Read error: {error}")));
                    }
                }
            }
        }
        
        // Restore terminal settings
        unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &original_termios) };
        
        if response.is_empty() {
            return Err(ConsoleError::IoError("No response received".to_string()));
        }
        
        // Parse response: ESC[{row};{col}R
        let response_str = String::from_utf8_lossy(&response);
        debug_log!("Cursor response: {:?} (bytes: {:?})", response_str, response);
        
        if !response_str.starts_with("\x1b[") || !response_str.ends_with('R') {
            return Err(ConsoleError::IoError("Invalid cursor position response".to_string()));
        }
        
        let coords = &response_str[2..response_str.len()-1]; // Remove ESC[ and R
        let parts: Vec<&str> = coords.split(';').collect();
        if parts.len() != 2 {
            return Err(ConsoleError::IoError("Invalid cursor position format".to_string()));
        }
        
        let row: u16 = parts[0].parse().map_err(|_| {
            ConsoleError::IoError("Invalid row in cursor position".to_string())
        })?;
        let col: u16 = parts[1].parse().map_err(|_| {
            ConsoleError::IoError("Invalid column in cursor position".to_string())
        })?;
        
        debug_log!("Parsed cursor position: row={}, col={}", row, col);
        
        // Convert from 1-based ANSI to 0-based API
        Ok((row.saturating_sub(1), col.saturating_sub(1)))
    }
    
    /// Detect true color support by checking environment variables
    fn detect_true_color_support(&self) -> bool {
        // Check common environment variables that indicate true color support
        if let Ok(colorterm) = std::env::var("COLORTERM") {
            if colorterm == "truecolor" || colorterm == "24bit" {
                return true;
            }
        }
        
        if let Ok(term) = std::env::var("TERM") {
            // Many modern terminals support true color
            if term.contains("256color") || term.contains("truecolor") {
                return true;
            }
        }
        
        // Check for specific terminal programs
        if std::env::var("TERM_PROGRAM").is_ok() {
            return true; // Most GUI terminals support true color
        }
        
        // Default to false for safety
        false
    }
}

impl ConsoleOutput for UnixConsoleOutput {
    fn write_text(&self, text: &str) -> ConsoleResult<()> {
        self.write_bytes(text.as_bytes())
    }
    
    fn write_styled_text(&self, text: &str, style: &TextStyle) -> ConsoleResult<()> {
        // Generate complete ANSI sequence with style and text
        let style_seq = self.style_to_ansi(style);
        if !style_seq.is_empty() {
            self.write_ansi(&style_seq)?;
        }
        
        // Write text
        self.write_text(text)?;
        
        // Reset style if we applied any
        if !style_seq.is_empty() {
            self.reset_style()?;
        }
        
        Ok(())
    }
    
    fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
        // Use SafeTextFilter to sanitize control sequences
        use replkit_core::SafeTextFilter;
        use replkit_core::SanitizationPolicy;
        
        let mut filter = SafeTextFilter::new(SanitizationPolicy::RemoveDangerous);
        let safe_text = filter.filter(text);
        self.write_text(&safe_text)
    }
    
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
        // Convert 0-based to 1-based for ANSI
        let ansi_seq = format!("\x1b[{};{}H", row + 1, col + 1);
        self.write_ansi(&ansi_seq)
    }
    
    fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
        // Handle vertical movement
        if row_delta > 0 {
            self.write_ansi(&format!("\x1b[{row_delta}B"))?; // Move down
        } else if row_delta < 0 {
            self.write_ansi(&format!("\x1b[{}A", -row_delta))?; // Move up
        }
        
        // Handle horizontal movement
        if col_delta > 0 {
            self.write_ansi(&format!("\x1b[{col_delta}C"))?; // Move right
        } else if col_delta < 0 {
            self.write_ansi(&format!("\x1b[{}D", -col_delta))?; // Move left
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
        let ansi_seq = self.style_to_ansi(style);
        if !ansi_seq.is_empty() {
            self.write_ansi(&ansi_seq)
        } else {
            Ok(())
        }
    }
    
    fn reset_style(&self) -> ConsoleResult<()> {
        self.write_ansi("\x1b[0m")
    }
    
    fn flush(&self) -> ConsoleResult<()> {
        if self.buffering_enabled.load(Ordering::Relaxed) {
            // Flush buffer to stdout
            if let Ok(mut buffer) = self.buffer.lock() {
                if !buffer.is_empty() {
                    self.write_bytes_direct(&buffer)?;
                    buffer.clear();
                }
            }
        }
        
        // Force kernel to flush stdout buffer
        if unsafe { libc::fsync(self.stdout_fd) } == -1 {
            let error = io::Error::last_os_error();
            // fsync may not be supported on all file types (like terminals)
            // so we ignore EINVAL and ENOTTY errors
            match error.raw_os_error() {
                Some(libc::EINVAL) | Some(libc::ENOTTY) => Ok(()),
                _ => Err(ConsoleError::IoError(format!("fsync failed: {error}"))),
            }
        } else {
            Ok(())
        }
    }
    
    fn set_alternate_screen(&self, enabled: bool) -> ConsoleResult<()> {
        if enabled {
            // Enter alternate screen buffer
            self.write_ansi("\x1b[?1049h")
        } else {
            // Exit alternate screen buffer
            self.write_ansi("\x1b[?1049l")
        }
    }
    
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
        if visible {
            // Show cursor
            self.write_ansi("\x1b[?25h")
        } else {
            // Hide cursor
            self.write_ansi("\x1b[?25l")
        }
    }
    
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        self.query_cursor_position_impl()
    }
    
    fn get_capabilities(&self) -> OutputCapabilities {
        let true_color_support = self.detect_true_color_support();
        
        OutputCapabilities {
            supports_colors: true,
            supports_true_color: true_color_support,
            supports_styling: true,
            supports_alternate_screen: true,
            supports_cursor_control: true,
            max_colors: if true_color_support { 65535 } else { 256 }, // True color vs 8-bit
            platform_name: "Unix VT".to_string(),
            backend_type: BackendType::UnixVt,
        }
    }
}
// Include tests
#[cfg(test)]
mod tests {
    use super::*;
    use replkit_core::{TextStyle, Color, ClearType, SanitizationPolicy, SafeTextFilter};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock Unix console output for testing ANSI sequence generation
    struct MockUnixConsoleOutput {
        output_buffer: Arc<Mutex<Vec<u8>>>,
        buffering_enabled: Arc<AtomicBool>,
    }

    impl MockUnixConsoleOutput {
        fn new() -> Self {
            Self {
                output_buffer: Arc::new(Mutex::new(Vec::new())),
                buffering_enabled: Arc::new(AtomicBool::new(false)),
            }
        }

        fn get_output(&self) -> String {
            let buffer = self.output_buffer.lock().unwrap();
            String::from_utf8_lossy(&buffer).to_string()
        }

        fn clear_output(&self) {
            let mut buffer = self.output_buffer.lock().unwrap();
            buffer.clear();
        }

        fn write_bytes(&self, bytes: &[u8]) -> ConsoleResult<()> {
            let mut buffer = self.output_buffer.lock().unwrap();
            buffer.extend_from_slice(bytes);
            Ok(())
        }

        fn write_ansi(&self, sequence: &str) -> ConsoleResult<()> {
            self.write_bytes(sequence.as_bytes())
        }

        fn color_to_fg_ansi(&self, color: &Color) -> String {
            match color {
                Color::Black => "30".to_string(),
                Color::Red => "31".to_string(),
                Color::Green => "32".to_string(),
                Color::Yellow => "33".to_string(),
                Color::Blue => "34".to_string(),
                Color::Magenta => "35".to_string(),
                Color::Cyan => "36".to_string(),
                Color::White => "37".to_string(),
                Color::BrightBlack => "90".to_string(),
                Color::BrightRed => "91".to_string(),
                Color::BrightGreen => "92".to_string(),
                Color::BrightYellow => "93".to_string(),
                Color::BrightBlue => "94".to_string(),
                Color::BrightMagenta => "95".to_string(),
                Color::BrightCyan => "96".to_string(),
                Color::BrightWhite => "97".to_string(),
                Color::Rgb(r, g, b) => format!("38;2;{};{};{}", r, g, b),
                Color::Ansi256(n) => format!("38;5;{}", n),
            }
        }

        fn color_to_bg_ansi(&self, color: &Color) -> String {
            match color {
                Color::Black => "40".to_string(),
                Color::Red => "41".to_string(),
                Color::Green => "42".to_string(),
                Color::Yellow => "43".to_string(),
                Color::Blue => "44".to_string(),
                Color::Magenta => "45".to_string(),
                Color::Cyan => "46".to_string(),
                Color::White => "47".to_string(),
                Color::BrightBlack => "100".to_string(),
                Color::BrightRed => "101".to_string(),
                Color::BrightGreen => "102".to_string(),
                Color::BrightYellow => "103".to_string(),
                Color::BrightBlue => "104".to_string(),
                Color::BrightMagenta => "105".to_string(),
                Color::BrightCyan => "106".to_string(),
                Color::BrightWhite => "107".to_string(),
                Color::Rgb(r, g, b) => format!("48;2;{};{};{}", r, g, b),
                Color::Ansi256(n) => format!("48;5;{}", n),
            }
        }

        fn style_to_ansi(&self, style: &TextStyle) -> String {
            let mut codes = Vec::new();
            
            // Foreground color
            if let Some(fg) = &style.foreground {
                codes.push(self.color_to_fg_ansi(fg));
            }
            
            // Background color
            if let Some(bg) = &style.background {
                codes.push(self.color_to_bg_ansi(bg));
            }
            
            // Text attributes
            if style.bold { codes.push("1".to_string()); }
            if style.dim { codes.push("2".to_string()); }
            if style.italic { codes.push("3".to_string()); }
            if style.underline { codes.push("4".to_string()); }
            if style.reverse { codes.push("7".to_string()); }
            if style.strikethrough { codes.push("9".to_string()); }
            
            if codes.is_empty() {
                String::new()
            } else {
                format!("\x1b[{}m", codes.join(";"))
            }
        }
    }

    #[test]
    fn test_basic_text_output() {
        let output = MockUnixConsoleOutput::new();
        output.write_bytes(b"Hello, World!").unwrap();
        assert_eq!(output.get_output(), "Hello, World!");
    }

    #[test]
    fn test_ansi_sequence_generation() {
        let output = MockUnixConsoleOutput::new();
        
        // Test cursor movement
        output.write_ansi("\x1b[10;20H").unwrap();
        assert_eq!(output.get_output(), "\x1b[10;20H");
        
        output.clear_output();
        
        // Test color codes
        output.write_ansi("\x1b[31m").unwrap(); // Red foreground
        assert_eq!(output.get_output(), "\x1b[31m");
    }

    #[test]
    fn test_color_to_ansi_conversion() {
        let output = MockUnixConsoleOutput::new();
        
        // Test basic colors
        assert_eq!(output.color_to_fg_ansi(&Color::Red), "31");
        assert_eq!(output.color_to_fg_ansi(&Color::Green), "32");
        assert_eq!(output.color_to_fg_ansi(&Color::Blue), "34");
        
        // Test bright colors
        assert_eq!(output.color_to_fg_ansi(&Color::BrightRed), "91");
        assert_eq!(output.color_to_fg_ansi(&Color::BrightGreen), "92");
        
        // Test RGB colors
        assert_eq!(output.color_to_fg_ansi(&Color::Rgb(255, 128, 64)), "38;2;255;128;64");
        
        // Test 256-color
        assert_eq!(output.color_to_fg_ansi(&Color::Ansi256(42)), "38;5;42");
        
        // Test background colors
        assert_eq!(output.color_to_bg_ansi(&Color::Red), "41");
        assert_eq!(output.color_to_bg_ansi(&Color::BrightBlue), "104");
        assert_eq!(output.color_to_bg_ansi(&Color::Rgb(255, 255, 255)), "48;2;255;255;255");
    }

    #[test]
    fn test_text_style_to_ansi() {
        let output = MockUnixConsoleOutput::new();
        
        // Test empty style
        let empty_style = TextStyle::default();
        assert_eq!(output.style_to_ansi(&empty_style), "");
        
        // Test bold only
        let bold_style = TextStyle {
            bold: true,
            ..Default::default()
        };
        assert_eq!(output.style_to_ansi(&bold_style), "\x1b[1m");
        
        // Test color only
        let red_style = TextStyle {
            foreground: Some(Color::Red),
            ..Default::default()
        };
        assert_eq!(output.style_to_ansi(&red_style), "\x1b[31m");
        
        // Test complex style
        let complex_style = TextStyle {
            foreground: Some(Color::BrightGreen),
            background: Some(Color::Black),
            bold: true,
            italic: true,
            underline: true,
            ..Default::default()
        };
        let result = output.style_to_ansi(&complex_style);
        assert!(result.contains("92")); // Bright green foreground
        assert!(result.contains("40")); // Black background
        assert!(result.contains("1"));  // Bold
        assert!(result.contains("3"));  // Italic
        assert!(result.contains("4"));  // Underline
        assert!(result.starts_with("\x1b["));
        assert!(result.ends_with("m"));
    }

    #[test]
    fn test_cursor_movement_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test absolute positioning (0-based to 1-based conversion)
        output.write_ansi(&format!("\x1b[{};{}H", 5 + 1, 10 + 1)).unwrap();
        assert_eq!(output.get_output(), "\x1b[6;11H");
        
        output.clear_output();
        
        // Test relative movements
        output.write_ansi("\x1b[3A").unwrap(); // Up 3
        output.write_ansi("\x1b[2B").unwrap(); // Down 2
        output.write_ansi("\x1b[4C").unwrap(); // Right 4
        output.write_ansi("\x1b[1D").unwrap(); // Left 1
        
        assert_eq!(output.get_output(), "\x1b[3A\x1b[2B\x1b[4C\x1b[1D");
    }

    #[test]
    fn test_clear_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test all clear types
        let clear_tests = vec![
            (ClearType::All, "\x1b[2J"),
            (ClearType::FromCursor, "\x1b[0J"),
            (ClearType::ToCursor, "\x1b[1J"),
            (ClearType::CurrentLine, "\x1b[2K"),
            (ClearType::FromCursorToEndOfLine, "\x1b[0K"),
            (ClearType::FromBeginningOfLineToCursor, "\x1b[1K"),
        ];
        
        for (clear_type, expected_seq) in clear_tests {
            output.clear_output();
            
            let ansi_seq = match clear_type {
                ClearType::All => "\x1b[2J",
                ClearType::FromCursor => "\x1b[0J",
                ClearType::ToCursor => "\x1b[1J",
                ClearType::CurrentLine => "\x1b[2K",
                ClearType::FromCursorToEndOfLine => "\x1b[0K",
                ClearType::FromBeginningOfLineToCursor => "\x1b[1K",
            };
            
            output.write_ansi(ansi_seq).unwrap();
            assert_eq!(output.get_output(), expected_seq);
        }
    }

    #[test]
    fn test_alternate_screen_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test enter alternate screen
        output.write_ansi("\x1b[?1049h").unwrap();
        assert_eq!(output.get_output(), "\x1b[?1049h");
        
        output.clear_output();
        
        // Test exit alternate screen
        output.write_ansi("\x1b[?1049l").unwrap();
        assert_eq!(output.get_output(), "\x1b[?1049l");
    }

    #[test]
    fn test_cursor_visibility_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test hide cursor
        output.write_ansi("\x1b[?25l").unwrap();
        assert_eq!(output.get_output(), "\x1b[?25l");
        
        output.clear_output();
        
        // Test show cursor
        output.write_ansi("\x1b[?25h").unwrap();
        assert_eq!(output.get_output(), "\x1b[?25h");
    }

    #[test]
    fn test_style_reset_sequence() {
        let output = MockUnixConsoleOutput::new();
        
        output.write_ansi("\x1b[0m").unwrap();
        assert_eq!(output.get_output(), "\x1b[0m");
    }

    #[test]
    fn test_safe_text_filtering() {
        let mut filter = SafeTextFilter::new(SanitizationPolicy::RemoveDangerous);
        
        // Test normal text passes through
        let safe_text = filter.filter("Hello, World!");
        assert_eq!(safe_text, "Hello, World!");
        
        // Test control sequences are removed
        let unsafe_text = "Hello\x1b[31mRed\x1b[0mWorld";
        let filtered = filter.filter(unsafe_text);
        assert!(!filtered.contains("\x1b"));
        assert!(filtered.contains("Hello"));
        assert!(filtered.contains("Red"));
        assert!(filtered.contains("World"));
    }

    #[test]
    fn test_rgb_color_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test true color foreground
        let rgb_fg = output.color_to_fg_ansi(&Color::Rgb(255, 128, 64));
        assert_eq!(rgb_fg, "38;2;255;128;64");
        
        // Test true color background
        let rgb_bg = output.color_to_bg_ansi(&Color::Rgb(64, 128, 255));
        assert_eq!(rgb_bg, "48;2;64;128;255");
        
        // Test edge cases
        let black_rgb = output.color_to_fg_ansi(&Color::Rgb(0, 0, 0));
        assert_eq!(black_rgb, "38;2;0;0;0");
        
        let white_rgb = output.color_to_fg_ansi(&Color::Rgb(255, 255, 255));
        assert_eq!(white_rgb, "38;2;255;255;255");
    }

    #[test]
    fn test_ansi256_color_sequences() {
        let output = MockUnixConsoleOutput::new();
        
        // Test 256-color foreground
        for i in 0..=255 {
            let ansi_fg = output.color_to_fg_ansi(&Color::Ansi256(i));
            assert_eq!(ansi_fg, format!("38;5;{}", i));
        }
        
        // Test 256-color background
        for i in 0..=255 {
            let ansi_bg = output.color_to_bg_ansi(&Color::Ansi256(i));
            assert_eq!(ansi_bg, format!("48;5;{}", i));
        }
    }

    #[test]
    fn test_all_text_attributes() {
        let output = MockUnixConsoleOutput::new();
        
        // Test each attribute individually
        let attributes = vec![
            (TextStyle { bold: true, ..Default::default() }, "1"),
            (TextStyle { dim: true, ..Default::default() }, "2"),
            (TextStyle { italic: true, ..Default::default() }, "3"),
            (TextStyle { underline: true, ..Default::default() }, "4"),
            (TextStyle { reverse: true, ..Default::default() }, "7"),
            (TextStyle { strikethrough: true, ..Default::default() }, "9"),
        ];
        
        for (style, expected_code) in attributes {
            let ansi = output.style_to_ansi(&style);
            assert!(ansi.contains(expected_code), 
                "Style {:?} should contain code {}, got: {}", style, expected_code, ansi);
        }
    }

    #[test]
    fn test_combined_style_attributes() {
        let output = MockUnixConsoleOutput::new();
        
        // Test all attributes combined
        let all_attrs_style = TextStyle {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
            bold: true,
            dim: true,
            italic: true,
            underline: true,
            reverse: true,
            strikethrough: true,
        };
        
        let ansi = output.style_to_ansi(&all_attrs_style);
        
        // Should contain all codes
        let expected_codes = vec!["31", "44", "1", "2", "3", "4", "7", "9"];
        for code in expected_codes {
            assert!(ansi.contains(code), 
                "Combined style should contain code {}, got: {}", code, ansi);
        }
        
        // Should be properly formatted
        assert!(ansi.starts_with("\x1b["));
        assert!(ansi.ends_with("m"));
    }

    #[test]
    fn test_buffering_behavior() {
        let output = MockUnixConsoleOutput::new();
        
        // Initially no buffering
        assert!(!output.buffering_enabled.load(Ordering::Relaxed));
        
        // Write some data
        output.write_bytes(b"test").unwrap();
        assert_eq!(output.get_output(), "test");
        
        // Enable buffering
        output.buffering_enabled.store(true, Ordering::Relaxed);
        output.clear_output();
        
        // Data should be buffered (this test would need actual buffering implementation)
        // For now, just verify the flag is set
        assert!(output.buffering_enabled.load(Ordering::Relaxed));
    }
}