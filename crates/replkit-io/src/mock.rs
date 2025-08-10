//! Mock console implementations for testing

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use replkit_core::{KeyEvent, Key};
use crate::{ConsoleInput, ConsoleOutput, ConsoleResult, ConsoleError, RawModeGuard,
           ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, ClearType, EventLoopError};

/// Mock console input for testing
pub struct MockConsoleInput {
    input_queue: Arc<Mutex<VecDeque<KeyEvent>>>,
    running: Arc<AtomicBool>,
    resize_callback: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
    key_callback: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
}

impl Default for MockConsoleInput {
    fn default() -> Self {
        Self::new()
    }
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
    
    /// Queue text input as a sequence of character key events
    pub fn queue_text_input(&self, text: &str) {
        if let Ok(mut queue) = self.input_queue.lock() {
            for ch in text.chars() {
                let event = KeyEvent::with_text(
                    Key::NotDefined, // Use NotDefined for regular characters
                    vec![ch as u8],
                    ch.to_string(),
                );
                queue.push_back(event);
            }
        }
    }
    
    /// Queue multiple key events at once
    pub fn queue_key_events(&self, events: &[KeyEvent]) {
        if let Ok(mut queue) = self.input_queue.lock() {
            for event in events {
                queue.push_back(event.clone());
            }
        }
    }
    
    /// Get the number of queued events
    pub fn queued_event_count(&self) -> usize {
        self.input_queue.lock().map(|q| q.len()).unwrap_or(0)
    }
    
    /// Clear all queued events
    pub fn clear_queue(&self) {
        if let Ok(mut queue) = self.input_queue.lock() {
            queue.clear();
        }
    }
    
    /// Simulate a window resize event
    pub fn simulate_resize(&self, cols: u16, rows: u16) {
        if let Ok(mut callback) = self.resize_callback.lock() {
            if let Some(cb) = callback.as_mut() {
                // Use catch_unwind to prevent panics in callbacks from crashing tests
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    cb(cols, rows);
                }));
            }
        }
    }
    
    /// Process queued events (for testing)
    pub fn process_queued_events(&self) {
        if let Ok(mut queue) = self.input_queue.lock() {
            if let Ok(mut callback) = self.key_callback.lock() {
                if let Some(cb) = callback.as_mut() {
                    while let Some(event) = queue.pop_front() {
                        // Use catch_unwind to prevent panics in callbacks from crashing tests
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            cb(event);
                        }));
                    }
                }
            }
        }
    }
    
    /// Process a single queued event (for step-by-step testing)
    pub fn process_single_event(&self) -> bool {
        if let Ok(mut queue) = self.input_queue.lock() {
            if let Some(event) = queue.pop_front() {
                if let Ok(mut callback) = self.key_callback.lock() {
                    if let Some(cb) = callback.as_mut() {
                        // Use catch_unwind to prevent panics in callbacks from crashing tests
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            cb(event);
                        }));
                        return true;
                    }
                }
            }
        }
        false
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
            return Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning));
        }
        Ok(())
    }
    
    fn stop_event_loop(&self) -> ConsoleResult<()> {
        if !self.running.swap(false, Ordering::Relaxed) {
            return Err(ConsoleError::EventLoopError(EventLoopError::NotRunning));
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

/// Styled output event for testing
#[derive(Debug, Clone, PartialEq)]
pub struct StyledOutputEvent {
    pub text: String,
    pub style: TextStyle,
    pub cursor_position: (u16, u16),
}

/// Mock console output for testing
pub struct MockConsoleOutput {
    output_buffer: Arc<Mutex<Vec<u8>>>,
    styled_output_events: Arc<Mutex<Vec<StyledOutputEvent>>>,
    cursor_position: Arc<Mutex<(u16, u16)>>,
    current_style: Arc<Mutex<TextStyle>>,
    alternate_screen_enabled: Arc<Mutex<bool>>,
    cursor_visible: Arc<Mutex<bool>>,
}

impl Default for MockConsoleOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl MockConsoleOutput {
    pub fn new() -> Self {
        Self {
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            styled_output_events: Arc::new(Mutex::new(Vec::new())),
            cursor_position: Arc::new(Mutex::new((0, 0))),
            current_style: Arc::new(Mutex::new(TextStyle::default())),
            alternate_screen_enabled: Arc::new(Mutex::new(false)),
            cursor_visible: Arc::new(Mutex::new(true)),
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
    
    /// Get styled output events for testing
    pub fn get_styled_output(&self) -> Vec<StyledOutputEvent> {
        self.styled_output_events.lock().unwrap().clone()
    }
    
    /// Clear captured output and styled events
    pub fn clear_output(&self) {
        self.output_buffer.lock().unwrap().clear();
        self.styled_output_events.lock().unwrap().clear();
    }
    
    /// Get current cursor position
    pub fn get_mock_cursor_position(&self) -> (u16, u16) {
        *self.cursor_position.lock().unwrap()
    }
    
    /// Get current style for testing
    pub fn get_current_style(&self) -> TextStyle {
        self.current_style.lock().unwrap().clone()
    }
    
    /// Check if alternate screen is enabled
    pub fn is_alternate_screen_enabled(&self) -> bool {
        *self.alternate_screen_enabled.lock().unwrap()
    }
    
    /// Check if cursor is visible
    pub fn is_cursor_visible(&self) -> bool {
        *self.cursor_visible.lock().unwrap()
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
        // Record styled output event
        let cursor_pos = *self.cursor_position.lock().unwrap();
        let event = StyledOutputEvent {
            text: text.to_string(),
            style: style.clone(),
            cursor_position: cursor_pos,
        };
        self.styled_output_events.lock().unwrap().push(event);
        
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
        *self.alternate_screen_enabled.lock().unwrap() = enabled;
        if enabled {
            self.write_text("\x1b[?1049h")
        } else {
            self.write_text("\x1b[?1049l")
        }
    }
    
    fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
        *self.cursor_visible.lock().unwrap() = visible;
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

#[cfg(test)]
mod tests {
    use super::*;
    use replkit_core::{Key, Color};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_mock_console_input_creation() {
        let input = MockConsoleInput::new();
        assert!(!input.is_running());
        assert_eq!(input.queued_event_count(), 0);
        
        let caps = input.get_capabilities();
        assert_eq!(caps.platform_name, "Mock");
        assert_eq!(caps.backend_type, BackendType::Mock);
        assert!(caps.supports_raw_mode);
        assert!(caps.supports_resize_events);
        assert!(caps.supports_unicode);
    }

    #[test]
    fn test_raw_mode_guard() {
        let input = MockConsoleInput::new();
        let guard = input.enable_raw_mode().unwrap();
        
        assert_eq!(guard.platform_info(), "Mock");
        assert!(guard.is_active());
        
        // Test manual restore
        guard.restore().unwrap();
    }

    #[test]
    fn test_event_loop_state_management() {
        let input = MockConsoleInput::new();
        
        // Initially stopped
        assert!(!input.is_running());
        
        // Start event loop
        input.start_event_loop().unwrap();
        assert!(input.is_running());
        
        // Cannot start again
        let result = input.start_event_loop();
        assert!(matches!(result, Err(ConsoleError::EventLoopError(EventLoopError::AlreadyRunning))));
        
        // Stop event loop
        input.stop_event_loop().unwrap();
        assert!(!input.is_running());
        
        // Cannot stop again
        let result = input.stop_event_loop();
        assert!(matches!(result, Err(ConsoleError::EventLoopError(EventLoopError::NotRunning))));
    }

    #[test]
    fn test_window_size() {
        let input = MockConsoleInput::new();
        let (cols, rows) = input.get_window_size().unwrap();
        assert_eq!(cols, 80);
        assert_eq!(rows, 24);
    }

    #[test]
    fn test_queue_key_event() {
        let input = MockConsoleInput::new();
        
        let event = KeyEvent::with_text(Key::NotDefined, vec![b'a'], "a".to_string());
        
        input.queue_key_event(event);
        assert_eq!(input.queued_event_count(), 1);
        
        input.clear_queue();
        assert_eq!(input.queued_event_count(), 0);
    }

    #[test]
    fn test_queue_text_input() {
        let input = MockConsoleInput::new();
        
        input.queue_text_input("hello");
        assert_eq!(input.queued_event_count(), 5);
        
        input.clear_queue();
        assert_eq!(input.queued_event_count(), 0);
    }

    #[test]
    fn test_queue_multiple_events() {
        let input = MockConsoleInput::new();
        
        let events = vec![
            KeyEvent::with_text(Key::NotDefined, vec![b'a'], "a".to_string()),
            KeyEvent::simple(Key::ControlB, vec![0x02]),
            KeyEvent::simple(Key::Enter, vec![0x0D]),
        ];
        
        input.queue_key_events(&events);
        assert_eq!(input.queued_event_count(), 3);
    }

    #[test]
    fn test_key_callback_invocation() {
        let input = MockConsoleInput::new();
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let received_events_clone = Arc::clone(&received_events);
        
        // Register callback
        input.on_key_pressed(Box::new(move |event| {
            received_events_clone.lock().unwrap().push(event);
        }));
        
        // Queue some events
        let events = vec![
            KeyEvent::with_text(Key::NotDefined, vec![b'x'], "x".to_string()),
            KeyEvent::simple(Key::ControlY, vec![0x19]),
        ];
        input.queue_key_events(&events);
        
        // Process events
        input.process_queued_events();
        
        // Verify callback was called
        let received = received_events.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0].key, Key::NotDefined);
        assert_eq!(received[0].text_or_empty(), "x");
        assert_eq!(received[1].key, Key::ControlY);
    }

    #[test]
    fn test_single_event_processing() {
        let input = MockConsoleInput::new();
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        input.on_key_pressed(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));
        
        input.queue_text_input("abc");
        assert_eq!(input.queued_event_count(), 3);
        
        // Process one event at a time
        assert!(input.process_single_event());
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
        assert_eq!(input.queued_event_count(), 2);
        
        assert!(input.process_single_event());
        assert_eq!(call_count.load(Ordering::Relaxed), 2);
        assert_eq!(input.queued_event_count(), 1);
        
        assert!(input.process_single_event());
        assert_eq!(call_count.load(Ordering::Relaxed), 3);
        assert_eq!(input.queued_event_count(), 0);
        
        // No more events to process
        assert!(!input.process_single_event());
        assert_eq!(call_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_resize_callback() {
        let input = MockConsoleInput::new();
        let resize_events = Arc::new(Mutex::new(Vec::new()));
        let resize_events_clone = Arc::clone(&resize_events);
        
        input.on_window_resize(Box::new(move |cols, rows| {
            resize_events_clone.lock().unwrap().push((cols, rows));
        }));
        
        input.simulate_resize(120, 30);
        input.simulate_resize(100, 25);
        
        let events = resize_events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], (120, 30));
        assert_eq!(events[1], (100, 25));
    }

    #[test]
    fn test_callback_panic_handling() {
        let input = MockConsoleInput::new();
        
        // Register a callback that panics
        input.on_key_pressed(Box::new(|_| {
            panic!("Test panic in callback");
        }));
        
        input.queue_key_event(KeyEvent::with_text(Key::NotDefined, vec![b'a'], "a".to_string()));
        
        // Processing should not panic even if callback does
        input.process_queued_events();
        
        // Queue should be empty after processing
        assert_eq!(input.queued_event_count(), 0);
    }

    #[test]
    fn test_resize_callback_panic_handling() {
        let input = MockConsoleInput::new();
        
        // Register a resize callback that panics
        input.on_window_resize(Box::new(|_, _| {
            panic!("Test panic in resize callback");
        }));
        
        // Simulating resize should not panic even if callback does
        input.simulate_resize(80, 24);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;
        
        let input = Arc::new(MockConsoleInput::new());
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);
        
        input.on_key_pressed(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));
        
        let input_clone = Arc::clone(&input);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let ch = (b'a' + i as u8) as char;
                input_clone.queue_key_event(KeyEvent::with_text(
                    Key::NotDefined, 
                    vec![ch as u8], 
                    ch.to_string()
                ));
            }
        });
        
        handle.join().unwrap();
        
        assert_eq!(input.queued_event_count(), 10);
        input.process_queued_events();
        assert_eq!(call_count.load(Ordering::Relaxed), 10);
    }

    // Tests for MockConsoleOutput
    #[test]
    fn test_mock_console_output_creation() {
        let output = MockConsoleOutput::new();
        let caps = output.get_capabilities();
        
        assert_eq!(caps.platform_name, "Mock");
        assert_eq!(caps.backend_type, BackendType::Mock);
        assert!(caps.supports_colors);
        assert!(caps.supports_true_color);
        assert!(caps.supports_styling);
        assert!(caps.supports_alternate_screen);
        assert!(caps.supports_cursor_control);
        assert_eq!(caps.max_colors, 65535);
    }

    #[test]
    fn test_output_text_capture() {
        let output = MockConsoleOutput::new();
        
        output.write_text("Hello").unwrap();
        output.write_text(" World").unwrap();
        
        assert_eq!(output.get_output_string(), "Hello World");
        
        output.clear_output();
        assert_eq!(output.get_output_string(), "");
    }

    #[test]
    fn test_cursor_positioning() {
        let output = MockConsoleOutput::new();
        
        output.move_cursor_to(5, 10).unwrap();
        assert_eq!(output.get_mock_cursor_position(), (5, 10));
        
        // Check that ANSI sequence was written
        let output_str = output.get_output_string();
        assert!(output_str.contains("\x1b[6;11H")); // 1-based in ANSI
        
        output.move_cursor_relative(-2, 3).unwrap();
        assert_eq!(output.get_mock_cursor_position(), (3, 13));
    }

    #[test]
    fn test_cursor_relative_movement_bounds() {
        let output = MockConsoleOutput::new();
        
        // Start at (0, 0)
        output.move_cursor_relative(-5, -5).unwrap();
        assert_eq!(output.get_mock_cursor_position(), (0, 0)); // Should not go negative
    }

    #[test]
    fn test_screen_clearing() {
        let output = MockConsoleOutput::new();
        
        output.clear(ClearType::All).unwrap();
        assert!(output.get_output_string().contains("\x1b[2J"));
        
        output.clear_output();
        output.clear(ClearType::CurrentLine).unwrap();
        assert!(output.get_output_string().contains("\x1b[2K"));
    }

    #[test]
    fn test_styling() {
        let output = MockConsoleOutput::new();
        
        let style = TextStyle {
            foreground: Some(Color::Red),
            bold: true,
            ..Default::default()
        };
        
        output.write_styled_text("Styled text", &style).unwrap();
        let output_str = output.get_output_string();
        
        // Should contain style sequences and reset
        assert!(output_str.contains("\x1b[1m")); // Bold (simplified)
        assert!(output_str.contains("Styled text"));
        assert!(output_str.contains("\x1b[0m")); // Reset
    }

    #[test]
    fn test_alternate_screen() {
        let output = MockConsoleOutput::new();
        
        output.set_alternate_screen(true).unwrap();
        assert!(output.get_output_string().contains("\x1b[?1049h"));
        
        output.clear_output();
        output.set_alternate_screen(false).unwrap();
        assert!(output.get_output_string().contains("\x1b[?1049l"));
    }

    #[test]
    fn test_cursor_visibility() {
        let output = MockConsoleOutput::new();
        
        output.set_cursor_visible(false).unwrap();
        assert!(output.get_output_string().contains("\x1b[?25l"));
        
        output.clear_output();
        output.set_cursor_visible(true).unwrap();
        assert!(output.get_output_string().contains("\x1b[?25h"));
    }

    #[test]
    fn test_safe_text_writing() {
        let output = MockConsoleOutput::new();
        
        // For mock, safe text is just written directly
        output.write_safe_text("Safe text with \x1b[31m ANSI").unwrap();
        assert_eq!(output.get_output_string(), "Safe text with \x1b[31m ANSI");
    }

    #[test]
    fn test_styled_output_tracking() {
        let output = MockConsoleOutput::new();
        
        let red_style = TextStyle {
            foreground: Some(Color::Red),
            bold: true,
            ..Default::default()
        };
        
        let blue_style = TextStyle {
            foreground: Some(Color::Blue),
            italic: true,
            ..Default::default()
        };
        
        output.move_cursor_to(2, 5).unwrap();
        output.write_styled_text("Red text", &red_style).unwrap();
        
        output.move_cursor_to(3, 10).unwrap();
        output.write_styled_text("Blue text", &blue_style).unwrap();
        
        let styled_events = output.get_styled_output();
        assert_eq!(styled_events.len(), 2);
        
        // Check first styled event
        assert_eq!(styled_events[0].text, "Red text");
        assert_eq!(styled_events[0].style.foreground, Some(Color::Red));
        assert!(styled_events[0].style.bold);
        assert_eq!(styled_events[0].cursor_position, (2, 5));
        
        // Check second styled event
        assert_eq!(styled_events[1].text, "Blue text");
        assert_eq!(styled_events[1].style.foreground, Some(Color::Blue));
        assert!(styled_events[1].style.italic);
        assert_eq!(styled_events[1].cursor_position, (3, 10));
    }

    #[test]
    fn test_terminal_state_tracking() {
        let output = MockConsoleOutput::new();
        
        // Initial state
        assert!(!output.is_alternate_screen_enabled());
        assert!(output.is_cursor_visible());
        
        // Enable alternate screen
        output.set_alternate_screen(true).unwrap();
        assert!(output.is_alternate_screen_enabled());
        
        // Hide cursor
        output.set_cursor_visible(false).unwrap();
        assert!(!output.is_cursor_visible());
        
        // Disable alternate screen
        output.set_alternate_screen(false).unwrap();
        assert!(!output.is_alternate_screen_enabled());
        
        // Show cursor
        output.set_cursor_visible(true).unwrap();
        assert!(output.is_cursor_visible());
    }

    #[test]
    fn test_current_style_tracking() {
        let output = MockConsoleOutput::new();
        
        // Initial style should be default
        let initial_style = output.get_current_style();
        assert_eq!(initial_style, TextStyle::default());
        
        // Set a new style
        let new_style = TextStyle {
            foreground: Some(Color::Green),
            bold: true,
            underline: true,
            ..Default::default()
        };
        
        output.set_style(&new_style).unwrap();
        let current_style = output.get_current_style();
        assert_eq!(current_style, new_style);
        
        // Reset style
        output.reset_style().unwrap();
        let reset_style = output.get_current_style();
        assert_eq!(reset_style, TextStyle::default());
    }

    #[test]
    fn test_clear_output_clears_styled_events() {
        let output = MockConsoleOutput::new();
        
        let style = TextStyle {
            foreground: Some(Color::Yellow),
            ..Default::default()
        };
        
        output.write_text("Regular text").unwrap();
        output.write_styled_text("Styled text", &style).unwrap();
        
        assert!(!output.get_output_string().is_empty());
        assert_eq!(output.get_styled_output().len(), 1);
        
        output.clear_output();
        
        assert!(output.get_output_string().is_empty());
        assert!(output.get_styled_output().is_empty());
    }

    #[test]
    fn test_complex_output_sequence() {
        let output = MockConsoleOutput::new();
        
        // Complex sequence: move cursor, write styled text, clear line, move again
        output.move_cursor_to(1, 0).unwrap();
        
        let header_style = TextStyle {
            foreground: Some(Color::White),
            background: Some(Color::Blue),
            bold: true,
            ..Default::default()
        };
        
        output.write_styled_text("Header", &header_style).unwrap();
        output.clear(ClearType::FromCursorToEndOfLine).unwrap();
        output.move_cursor_to(2, 4).unwrap();
        output.write_text("Body text").unwrap();
        
        // Verify cursor position
        assert_eq!(output.get_mock_cursor_position(), (2, 4));
        
        // Verify styled output was captured
        let styled_events = output.get_styled_output();
        assert_eq!(styled_events.len(), 1);
        assert_eq!(styled_events[0].text, "Header");
        assert_eq!(styled_events[0].cursor_position, (1, 0));
        
        // Verify output buffer contains all sequences
        let output_str = output.get_output_string();
        assert!(output_str.contains("\x1b[2;1H")); // Move to (1,0) - 1-based ANSI
        assert!(output_str.contains("Header"));
        assert!(output_str.contains("\x1b[0K")); // Clear to end of line
        assert!(output_str.contains("\x1b[3;5H")); // Move to (2,4) - 1-based ANSI
        assert!(output_str.contains("Body text"));
    }

    #[test]
    fn test_styled_output_event_equality() {
        let event1 = StyledOutputEvent {
            text: "Test".to_string(),
            style: TextStyle {
                foreground: Some(Color::Red),
                bold: true,
                ..Default::default()
            },
            cursor_position: (1, 2),
        };
        
        let event2 = StyledOutputEvent {
            text: "Test".to_string(),
            style: TextStyle {
                foreground: Some(Color::Red),
                bold: true,
                ..Default::default()
            },
            cursor_position: (1, 2),
        };
        
        let event3 = StyledOutputEvent {
            text: "Different".to_string(),
            style: TextStyle {
                foreground: Some(Color::Red),
                bold: true,
                ..Default::default()
            },
            cursor_position: (1, 2),
        };
        
        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    #[test]
    fn test_thread_safety_output() {
        use std::thread;
        
        let output = Arc::new(MockConsoleOutput::new());
        let output_clone = Arc::clone(&output);
        
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let text = format!("Text {}", i);
                output_clone.write_text(&text).unwrap();
            }
        });
        
        handle.join().unwrap();
        
        let output_str = output.get_output_string();
        for i in 0..10 {
            assert!(output_str.contains(&format!("Text {}", i)));
        }
    }
}