//! VT100 Debug Example - Raw terminal input parsing with SIGIO-based input
//!
//! This example demonstrates how to use the key parser with raw terminal input.
//! It sets up raw terminal mode, configures SIGIO signal handling for non-blocking
//! input detection, and displays parsed key events with their raw byte information.
//!
//! Usage: cargo run --example vt100_debug
//! Press Ctrl+C to exit gracefully.

use prompt_core::{KeyParser, Key};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};

// Global flag to indicate when input is ready
static INPUT_READY: AtomicBool = AtomicBool::new(false);

// Global flag for graceful shutdown
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Raw terminal mode configuration using termios
struct RawTerminal {
    original_termios: libc::termios,
    stdin_fd: i32,
}

impl RawTerminal {
    /// Enter raw terminal mode and configure non-blocking stdin
    fn new() -> io::Result<Self> {
        let stdin_fd = io::stdin().as_raw_fd();
        
        // Get current terminal attributes
        let mut original_termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(stdin_fd, &mut original_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }
        
        // Configure raw mode
        let mut raw_termios = original_termios;
        
        // Disable canonical mode, echo, and signal processing
        raw_termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ECHOE | libc::ECHOK | 
                                 libc::ECHONL | libc::ISIG | libc::IEXTEN);
        
        // Disable input processing
        raw_termios.c_iflag &= !(libc::IXON | libc::IXOFF | libc::ICRNL | libc::INLCR | 
                                 libc::IGNCR | libc::BRKINT | libc::PARMRK | libc::ISTRIP);
        
        // Disable output processing
        raw_termios.c_oflag &= !libc::OPOST;
        
        // Set character size to 8 bits
        raw_termios.c_cflag &= !libc::CSIZE;
        raw_termios.c_cflag |= libc::CS8;
        
        // Set minimum characters to read and timeout
        raw_termios.c_cc[libc::VMIN] = 0;  // Non-blocking read
        raw_termios.c_cc[libc::VTIME] = 0; // No timeout
        
        // Apply the new terminal settings
        if unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw_termios) } != 0 {
            return Err(io::Error::last_os_error());
        }
        
        // Set stdin to non-blocking mode
        let flags = unsafe { libc::fcntl(stdin_fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(io::Error::last_os_error());
        }
        
        if unsafe { libc::fcntl(stdin_fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
            return Err(io::Error::last_os_error());
        }
        
        Ok(RawTerminal {
            original_termios,
            stdin_fd,
        })
    }
    
    /// Read available bytes from stdin without blocking
    fn read_available(&self) -> io::Result<Vec<u8>> {
        let mut buffer = [0u8; 1024];
        let mut result = Vec::new();
        
        loop {
            match unsafe { libc::read(self.stdin_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) } {
                -1 => {
                    let error = io::Error::last_os_error();
                    if error.kind() == io::ErrorKind::WouldBlock {
                        break; // No more data available
                    }
                    return Err(error);
                }
                0 => break, // EOF
                n => {
                    result.extend_from_slice(&buffer[..n as usize]);
                }
            }
        }
        
        Ok(result)
    }
}

impl Drop for RawTerminal {
    /// Restore original terminal settings when dropped
    fn drop(&mut self) {
        // Restore original terminal settings
        unsafe {
            libc::tcsetattr(self.stdin_fd, libc::TCSANOW, &self.original_termios);
        }
        
        // Remove non-blocking flag
        let flags = unsafe { libc::fcntl(self.stdin_fd, libc::F_GETFL) };
        if flags != -1 {
            unsafe {
                libc::fcntl(self.stdin_fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
            }
        }
    }
}

/// SIGIO signal handler
extern "C" fn sigio_handler(_sig: libc::c_int) {
    INPUT_READY.store(true, Ordering::Relaxed);
}

/// SIGINT signal handler for graceful shutdown
extern "C" fn sigint_handler(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

/// Setup SIGIO signal handling for stdin
fn setup_sigio(stdin_fd: i32) -> io::Result<()> {
    // Set the process to receive SIGIO signals for this file descriptor
    if unsafe { libc::fcntl(stdin_fd, libc::F_SETOWN, libc::getpid()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    
    // Enable SIGIO signal generation
    let flags = unsafe { libc::fcntl(stdin_fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    
    if unsafe { libc::fcntl(stdin_fd, libc::F_SETFL, flags | libc::O_ASYNC) } == -1 {
        return Err(io::Error::last_os_error());
    }
    
    // Install SIGIO signal handler
    let mut sigio_action: libc::sigaction = unsafe { std::mem::zeroed() };
    sigio_action.sa_sigaction = sigio_handler as usize;
    sigio_action.sa_flags = libc::SA_RESTART;
    
    if unsafe { libc::sigaction(libc::SIGIO, &sigio_action, std::ptr::null_mut()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    
    // Install SIGINT signal handler for graceful shutdown
    let mut sigint_action: libc::sigaction = unsafe { std::mem::zeroed() };
    sigint_action.sa_sigaction = sigint_handler as usize;
    sigint_action.sa_flags = libc::SA_RESTART;
    
    if unsafe { libc::sigaction(libc::SIGINT, &sigint_action, std::ptr::null_mut()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    
    Ok(())
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

fn main() -> io::Result<()> {
    println!("VT100 Debug - Raw Terminal Input Parser");
    println!("=======================================");
    println!("Press keys to see parsed events. Press Ctrl+C to exit.");
    println!();
    
    // Enter raw terminal mode
    let terminal = RawTerminal::new()?;
    
    // Setup SIGIO signal handling
    setup_sigio(terminal.stdin_fd)?;
    
    // Create key parser
    let mut parser = KeyParser::new();
    
    print!("Ready for input...\r\n");
    io::stdout().flush()?;
    
    // Main event loop
    loop {
        // Check for shutdown signal
        if SHUTDOWN.load(Ordering::Relaxed) {
            println!("\nReceived Ctrl+C, shutting down gracefully...");
            break;
        }
        
        // Check if input is ready
        if INPUT_READY.load(Ordering::Relaxed) {
            INPUT_READY.store(false, Ordering::Relaxed);
            
            // Read available input
            match terminal.read_available() {
                Ok(bytes) if !bytes.is_empty() => {
                    // Parse the input bytes
                    let events = parser.feed(&bytes);
                    
                    // Display each parsed event
                    for event in events {
                        display_key_event(&event);
                        
                        // Check for Ctrl+C in parsed events as backup
                        if event.key == Key::ControlC {
                            println!("Detected Ctrl+C in parsed events, exiting...");
                            return Ok(());
                        }
                    }
                }
                Ok(_) => {
                    // No bytes available, continue
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    break;
                }
            }
        }
        
        // Small sleep to prevent busy waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    // Flush any remaining partial sequences
    let remaining_events = parser.flush();
    if !remaining_events.is_empty() {
        println!("\nFlushing remaining partial sequences:");
        for event in remaining_events {
            display_key_event(&event);
        }
    }
    
    println!("\nTerminal restored. Goodbye!");
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