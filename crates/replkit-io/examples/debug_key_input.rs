//! Key Input Debug Example - Console Input Debug Tool
//!
//! Usage: cargo run --example debug_key_input
//! Press Ctrl+C to exit.

use replkit_core::{Key, KeyEvent};
use replkit_io::{ConsoleError, ConsoleInput};
use std::io::{self, Write};

/// Wrapper to ensure proper cleanup of console input
struct ConsoleInputGuard {
    input: Box<dyn ConsoleInput>,
    _raw_guard: Option<replkit_io::RawModeGuard>,
}

impl ConsoleInputGuard {
    fn new(input: Box<dyn ConsoleInput>) -> io::Result<Self> {
        let raw_guard = input
            .enable_raw_mode()
            .map_err(|e| io::Error::other(format!("raw mode error: {}", e)))?;
        Ok(Self {
            input,
            _raw_guard: Some(raw_guard),
        })
    }

    #[allow(dead_code)]
    fn try_read_key(&self) -> Result<Option<KeyEvent>, ConsoleError> {
        self.input.try_read_key()
    }

    fn read_key_timeout(&self, timeout_ms: Option<u32>) -> Result<Option<KeyEvent>, ConsoleError> {
        self.input.read_key_timeout(timeout_ms)
    }

    fn get_window_size(&self) -> Result<(u16, u16), ConsoleError> {
        self.input.get_window_size()
    }
}

/// Format raw bytes for display
fn format_bytes(bytes: &[u8]) -> String {
    let hex: String = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");

    let ascii: String = bytes
        .iter()
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
fn display_key_event(event: &KeyEvent) {
    let key_name = format!("{:?}", event.key);
    let raw_bytes = format_bytes(&event.raw_bytes);

    // Use \r\n for proper line endings in raw mode
    print!(
        "Key: {} | Raw: {} | Text: {:?}\r\n",
        key_name, raw_bytes, event.text
    );
    let _ = io::stdout().flush();
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

#[cfg(target_arch = "wasm32")]
fn make_input() -> io::Result<Box<dyn ConsoleInput>> {
    Ok(Box::new(replkit_io::WasmBridgeConsoleInput::new()?))
}

#[cfg(not(any(unix, windows, target_arch = "wasm32")))]
fn make_input() -> io::Result<Box<dyn ConsoleInput>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Unsupported platform",
    ))
}

fn main() -> io::Result<()> {
    println!("Key Input Debug Tool");
    println!("Press keys to see their events. Press Ctrl+C to exit.");
    println!();

    let input_impl = make_input()
        .map_err(|e| io::Error::other(format!("init error: {}", e)))?;
    let input = ConsoleInputGuard::new(input_impl)?;

    // Display current window size
    match input.get_window_size() {
        Ok((cols, rows)) => print!("[window size] cols={}, rows={}\r\n", cols, rows),
        Err(e) => print!("[window size] error: {}\r\n", e),
    }

    print!("Ready for input...\r\n");
    io::stdout().flush()?;

    // Main input loop using new API
    loop {
        match input.read_key_timeout(Some(50)) {
            // 50ms timeout for better Escape key detection
            Ok(Some(key_event)) => {
                display_key_event(&key_event);

                // Exit on Ctrl+C
                if key_event.key == Key::ControlC {
                    print!("Received Ctrl+C, shutting down...\r\n");
                    let _ = io::stdout().flush();
                    break;
                }
            }
            Ok(None) => {
                // Timeout - continue loop
                continue;
            }
            Err(e) => {
                print!("Input error: {}\r\n", e);
                let _ = io::stdout().flush();
                break;
            }
        }
    }

    print!("Done. Goodbye!\r\n");
    let _ = io::stdout().flush();
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
}
