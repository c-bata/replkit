//! Mock console implementations for testing

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use prompt_core::KeyEvent;
use crate::{ConsoleInput, ConsoleOutput, ConsoleResult, ConsoleError, RawModeGuard,
           ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, ClearType, AsAny};

/// Mock console input for testing
pub struct MockConsoleInput {
    input_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
    running: Arc<AtomicBool>,
    resize_callback: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
    key_callback: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
}

impl MockConsoleInput {
    pub fn new() -> Self {
        Self {
            input_queue: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(AtomicBool::new(false)),
            resize_callback: Arc::new(Mutex::new(None)),
            key_callback: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Queue a key event for testing
    pub fn queue_key_event(&self, event: KeyEvent) {
        if let Ok(mut queue) = self.input_queue.lock() {
            queue.push_back(event);
        }
    }
    
    /// Simulate a window resize event
    pub fn simulate_resize(&self, cols: u16, rows: u16) {
        if let Ok(mut callback) = self.resize_callback.lock() {
            if let Some(cb) = callback.as_mut() {
                cb(cols, rows);
            }
        }
    }
    
    /// Process queued events (for testing)
    pub fn process_queued_events(&self) {
        if let Ok(mut queue) = self.input_queue.lock() {
            if let Ok(mut callback) = self.key_callback.lock() {
                if let Some(cb) = callback.as_mut() {
                    while let Some(event) = queue.pop_front() {
                        cb(event);
                    }
                }
            }
        }
    }
}

impl AsAny for MockConsoleInput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ConsoleInput for MockConsoleInput {
    fn enable_raw_mode(&self) -> Result<RawModeGuard, ConsoleError> {
        let restore_fn = || {
            // Mock restore - no-op
        };
        Ok(RawModeGuard::new(restore_fn, "Mock".to_string()))
    }
    
    fn get_window_size(&self) -> ConsoleResult<(u16, u16)> {
        Ok((80, 24)) // Default mock size
    }
    
    fn start_event_loop(&self) -> ConsoleResult<()> {
        if self.running.swap(true, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::AlreadyRunning));
        }
        Ok(())
    }
    
    fn stop_event_loop(&self) -> ConsoleResult<()> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(crate::EventLoopError::NotRunning));
        }
        Ok(())
    }
    
    fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        if let Ok(mut cb) = self.resize_callback.lock() {
            *cb = Some(callback);
        }
    }
    
    fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        if let Ok(mut cb) = self.key_callback.lock() {
            *cb = Some(callback);
        }
    }
    
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    
    fn get_capabilities(&self) -> ConsoleCapabilities {
        ConsoleCapabilities {
            supports_raw_mode: true,
            supports_resize_events: true,
            supports_bracketed_paste: false,
            supports_mouse_events: false,
            supports_unicode: true,
            platform_name: "Mock".to_string(),
            backend_type: BackendType::Mock,
        }
    }
}

/// Mock console output for testing
pub struct MockConsoleOutput {
    output_buffer: Arc<Mutex<Vec<u8>>>,
    cursor_position: Arc<Mutex<(u16, u16)>>,
    current_style: Arc<Mutex<TextStyle>>,
}

impl MockConsoleOutput {
    pub fn new() -> Self {
        Self {
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            cursor_position: Arc::new(Mutex::new((0, 0))),
            current_style: Arc::new(Mutex::new(TextStyle::default())),
        }
    }
    
    /// Get captured output for testing
    pub fn get_output(&self) -> Vec<u8> {
        self.output_buffer.lock().unwrap().clone()
    }
    
    /// Get output as string for testing
    pub fn get_output_string(&self) -> String {
        String::from_utf8_lossy(&self.get_output()).to_string()
    }
    
    /// Clear captured output
    pub fn clear_output(&self) {
        self.output_buffer.lock().unwrap().clear();
    }
    
    /// Get current cursor position
    pub fn get_mock_cursor_position(&self) -> (u16, u16) {
        *self.cursor_position.lock().unwrap()
    }
}

impl AsAny for MockConsoleOutput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ConsoleOutput for MockConsoleOutput {
    fn write_text(&self, text: &str) -> ConsoleResult<()> {
        if let Ok(mut buffer) = self.output_buffer.lock() {
            buffer.extend_from_slice(text.as_bytes());
        }
        Ok(())
    }
    
    fn write_styled_text(&self, text: &str, style: &TextStyle) -> ConsoleResult<()> {
        self.set_style(style)?;
        self.write_text(text)?;
        self.reset_style()
    }
    
    fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
        // For mock, just write text directly
        self.write_text(text)
    }
    
    fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
        if let Ok(mut pos) = self.cursor_position.lock() {
            *pos = (row, col);
        }
        // Also write ANSI sequence to buffer for verification
        let ansi_seq = format!("\x1b[{};{}H", row + 1, col + 1);
        self.write_text(&ansi_seq)
    }
    
    fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
        if let Ok(mut pos) = self.cursor_position.lock() {
            pos.0 = (pos.0 as i16 + row_delta).max(0) as u16;
            pos.1 = (pos.1 as i16 + col_delta).max(0) as u16;
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
        self.write_text(ansi_seq)
    }
    
    fn set_style(&self, style: &TextStyle) -> ConsoleResult<()> {
        if let Ok(mut current) = self.current_style.lock() {
            *current = style.clone();
        }
        // Write style change to buffer for verification
        self.write_text("\x1b[1m") // Simplified - just write bold as example
    }
    
    fn reset_style(&self) -> ConsoleResult<()> {
        if let Ok(mut current) = self.current_style.lock() {
            *current = TextStyle::default();
        }
        self.write_text("\x1b[0m")
    }
    
    fn flush(&self) -> ConsoleResult<()> {
        // Mock flush - no-op
        Ok(())
    }
    
    fn set_alternate_screen(&self, enabled: bool) -> ConsoleResult<()> {
        if enabled {
            self.write_text("\x1b[?1049h")
        } else {
            self.write_text("\x1b[?1049l")
        }
    }
    
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
        if visible {
            self.write_text("\x1b[?25h")
        } else {
            self.write_text("\x1b[?25l")
        }
    }
    
    fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
        Ok(*self.cursor_position.lock().unwrap())
    }
    
    fn get_capabilities(&self) -> OutputCapabilities {
        OutputCapabilities {
            supports_colors: true,
            supports_true_color: true,
            supports_styling: true,
            supports_alternate_screen: true,
            supports_cursor_control: true,
            max_colors: 65535,
            platform_name: "Mock".to_string(),
            backend_type: BackendType::Mock,
        }
    }
}