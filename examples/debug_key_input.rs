//! VT Debug Example - portable ConsoleInput-based input printing
//!
//! Usage: cargo run --example vt100_debug
//! Press Ctrl+C (or Enter + Ctrl+C on some shells) to exit.

use prompt_core::{Key, KeyEvent};
use prompt_io::ConsoleInput;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
fn display_key_event(event: &prompt_core::KeyEvent) {
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
    Ok(Box::new(prompt_io::UnixVtConsoleInput::new()?))
}

#[cfg(windows)]
fn make_input() -> io::Result<Box<dyn ConsoleInput>> {
    // Prefer legacy console for cmd.exe compatibility
    Ok(Box::new(prompt_io::WindowsLegacyConsoleInput::new()?))
}

fn main() -> io::Result<()> {
    println!("VT Debug - ConsoleInput");
    println!("========================");
    println!("Press keys to see parsed events. Press Ctrl+C to exit.");
    println!();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_flag = shutdown.clone();

    let mut input = make_input().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("init error: {}", e)))?;
    input.enable_raw_mode().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("raw mode error: {}", e)))?;

    input.set_key_event_callback(Box::new(move |ev: KeyEvent| {
        display_key_event(&ev);
        if ev.key == Key::ControlC { shutdown_flag.store(true, Ordering::Relaxed); }
    }));

    input.set_resize_callback(Box::new(|cols, rows| {
        println!("[resize] cols={}, rows={}", cols, rows);
        let _ = io::stdout().flush();
    }));

    input.start_event_loop().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("start error: {}", e)))?;

    print!("Ready for input...\r\n");
    io::stdout().flush()?;

    while !shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("\nReceived Ctrl+C, shutting down...");
    let _ = input.stop_event_loop();
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
