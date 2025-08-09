//! VT Debug Example - portable ConsoleInput-based input printing
//!
//! Usage: cargo run --example vt100_debug
//! Press Ctrl+C (or Enter + Ctrl+C on some shells) to exit.

use replkit_core::{Key, KeyEvent, ConsoleInput};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Wrapper to ensure proper cleanup of console input
struct ConsoleInputGuard {
    input: Box<dyn ConsoleInput>,
    _raw_guard: Option<replkit_core::RawModeGuard>,
    running: bool,
}

impl ConsoleInputGuard {
    fn new(input: Box<dyn ConsoleInput>) -> io::Result<Self> {
        let raw_guard = input.enable_raw_mode().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("raw mode error: {}", e)))?;
        Ok(Self { input, _raw_guard: Some(raw_guard), running: false })
    }

    fn start_event_loop(&mut self) -> io::Result<()> {
        self.input.start_event_loop().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("start error: {}", e)))?;
        self.running = true;
        Ok(())
    }

    fn stop_event_loop(&mut self) -> io::Result<()> {
        if self.running {
            if let Err(e) = self.input.stop_event_loop() {
                eprintln!("Warning: Failed to stop event loop: {}", e);
            }
            self.running = false;
        }
        Ok(())
    }

    fn set_key_event_callback(&mut self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
        self.input.on_key_pressed(callback);
    }

    fn set_resize_callback(&mut self, callback: Box<dyn FnMut(u16, u16) + Send>) {
        self.input.on_window_resize(callback);
    }
}

impl Drop for ConsoleInputGuard {
    fn drop(&mut self) {
        let _ = self.stop_event_loop();
        // Give a moment for cleanup to complete
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Format raw bytes for display
fn format_bytes(bytes: &[u8]) -> String {
    let hex: String = bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");
    
    let ascii: String = bytes.iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    
    format!("[{}] \"{}\"", hex, ascii)
}

/// Display key event information with improved formatting
fn display_key_event(event: &replkit_core::KeyEvent) {
    let key_name = format!("{:?}", event.key);
    let raw_bytes = format_bytes(&event.raw_bytes);
    
    // Use different formatting for better readability
    print!("KeyPress(key={:<20}, raw={:<25}", 
           format!("'{}'", key_name), 
           raw_bytes);
    
    if let Some(text) = &event.text {
        print!(", data='{}'", text);
    }
    
    // Use explicit \r\n for proper line breaks in raw terminal mode
    print!(")\r\n");
    
    // Flush output immediately
    io::stdout().flush().unwrap();
}

#[cfg(unix)]
fn make_input() -> io::Result<Box<dyn ConsoleInput>> {
    Ok(Box::new(replkit_io::UnixConsoleInput::new()?))
}

#[cfg(windows)]
fn make_input() -> io::Result<Box<dyn ConsoleInput>> {
    // Prefer legacy console for cmd.exe compatibility
    Ok(Box::new(replkit_io::WindowsLegacyConsoleInput::new()?))
}

fn main() -> io::Result<()> {
    println!("VT Debug - ConsoleInput");
    println!("========================");
    println!("Press keys to see parsed events. Press Ctrl+C to exit.");
    println!();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_flag = shutdown.clone();

    let input_impl = make_input().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("init error: {}", e)))?;
    let mut input = ConsoleInputGuard::new(input_impl)?;

    input.set_key_event_callback(Box::new(move |ev: KeyEvent| {
        display_key_event(&ev);
        if ev.key == Key::ControlC { 
            println!("\r\nReceived Ctrl+C, shutting down...\r\n");
            io::stdout().flush().unwrap();
            shutdown_flag.store(true, Ordering::Relaxed); 
        }
    }));

    input.set_resize_callback(Box::new(|cols, rows| {
        println!("[resize] cols={}, rows={}\r", cols, rows);
        let _ = io::stdout().flush();
    }));

    input.start_event_loop()?;

    print!("Ready for input...\r\n");
    io::stdout().flush()?;

    // Main loop - check for shutdown signal
    while !shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Explicit cleanup - the Drop impl will also handle this as a fallback
    println!("Stopping event loop...");
    input.stop_event_loop()?;
    
    println!("Done. Goodbye!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        // Test printable ASCII
        assert_eq!(format_bytes(b"hello"), "[68 65 6c 6c 6f] \"hello\"");
        
        // Test control characters
        assert_eq!(format_bytes(&[0x1b, 0x5b, 0x41]), "[1b 5b 41] \".[A\"");
        
        // Test mixed content
        assert_eq!(format_bytes(&[0x03, 0x61, 0x0a]), "[03 61 0a] \".a.\"");
        
        // Test empty
        assert_eq!(format_bytes(&[]), "[] \"\"");
    }
    
    #[test]
    fn test_key_parser_integration() {
        let mut parser = KeyParser::new();
        
        // Test basic control character
        let events = parser.feed(&[0x03]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::ControlC);
        
        // Test arrow key sequence
        let events = parser.feed(&[0x1b, 0x5b, 0x41]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Up);
        
        // Test partial sequence handling
        let events = parser.feed(&[0x1b]);
        assert_eq!(events.len(), 0); // Should buffer partial sequence
        
        let events = parser.feed(&[0x5b, 0x42]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Down);
    }
}
