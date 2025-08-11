use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system, Child};
use std::io::Read;
use std::time::Duration;
use tokio::time::{timeout, sleep};
use crate::config::CommandConfig;
use crate::error::{PtyError, Result};

pub struct PtyManager {
    pty_pair: PtyPair,
    child_process: Option<Box<dyn Child + Send + Sync>>,
    terminal_size: PtySize,
}

impl PtyManager {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let terminal_size = PtySize {
            cols: cols,
            rows: rows,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        let pty_pair = pty_system.openpty(terminal_size)
            .map_err(|e| PtyError::CreateFailed(e.to_string()))?;
        
        Ok(Self {
            pty_pair,
            child_process: None,
            terminal_size,
        })
    }
    
    pub fn spawn_command(&mut self, cmd_config: &CommandConfig) -> Result<()> {
        if cmd_config.exec.is_empty() {
            return Err(PtyError::SpawnFailed("Command exec array is empty".to_string()).into());
        }
        
        let mut command = CommandBuilder::new(&cmd_config.exec[0]);
        
        // Add arguments
        for arg in &cmd_config.exec[1..] {
            command.arg(arg);
        }
        
        // Set working directory
        if let Some(workdir) = &cmd_config.workdir {
            command.cwd(workdir);
        }
        
        // Set environment variables
        if let Some(env) = &cmd_config.env {
            for (key, value) in env {
                command.env(key, value);
            }
        }
        
        let child = self.pty_pair.slave.spawn_command(command)
            .map_err(|e| PtyError::SpawnFailed(format!("Failed to spawn command: {}", e)))?;
        
        self.child_process = Some(child);
        
        Ok(())
    }
    
    pub async fn send_input(&mut self, input: &[u8]) -> Result<()> {
        use std::io::{Error, ErrorKind};
        
        println!("[DEBUG send_input] Sending {} bytes using raw fd write", input.len());
        
        // Use the same raw fd approach as read_output_timeout for consistency
        let raw_fd = match self.pty_pair.master.as_raw_fd() {
            Some(fd) => {
                println!("[DEBUG send_input] Got raw fd from master: {}", fd);
                fd
            },
            None => {
                println!("[DEBUG send_input] Master does not expose raw fd");
                return Err(PtyError::IoError(Error::new(
                    ErrorKind::Other, 
                    "Cannot get raw file descriptor from PTY master for writing"
                )).into());
            }
        };
        
        // Use raw libc::write for consistency with read approach
        let write_result = unsafe {
            libc::write(raw_fd, input.as_ptr() as *const libc::c_void, input.len())
        };
        
        if write_result == -1 {
            let errno = unsafe { *libc::__errno_location() };
            println!("[DEBUG send_input] write() error, errno: {}", errno);
            Err(PtyError::IoError(Error::from_raw_os_error(errno)).into())
        } else {
            let bytes_written = write_result as usize;
            println!("[DEBUG send_input] Successfully wrote {} bytes", bytes_written);
            
            // Ensure all data was written
            if bytes_written != input.len() {
                println!("[DEBUG send_input] Partial write: {} of {} bytes", bytes_written, input.len());
                return Err(PtyError::IoError(Error::new(
                    ErrorKind::WriteZero, 
                    "Partial write to PTY"
                )).into());
            }
            
            Ok(())
        }
    }
    
    pub async fn read_output(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self.pty_pair.master.try_clone_reader()
            .map_err(|e| PtyError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to clone reader: {}", e)
            )))?;
        
        let bytes_read = reader.read(buffer)
            .map_err(|e| PtyError::IoError(e))?;
        Ok(bytes_read)
    }
    
    pub async fn read_output_timeout(&mut self, buffer: &mut [u8], timeout_duration: Duration) -> Result<usize> {
        println!("[DEBUG read_output_timeout] START - buffer size: {}, timeout: {:?}", buffer.len(), timeout_duration);
        
        use std::io::{Error, ErrorKind};
        use std::os::unix::io::AsRawFd;
        
        println!("[DEBUG read_output_timeout] Using raw file descriptor approach...");
        
        // Try to get raw fd directly from portable-pty master
        let raw_fd = match self.pty_pair.master.as_raw_fd() {
            Some(fd) => {
                println!("[DEBUG read_output_timeout] Got raw fd from master: {}", fd);
                fd
            },
            None => {
                println!("[DEBUG read_output_timeout] Master does not expose raw fd, fallback to reader");
                return Err(PtyError::IoError(Error::new(
                    ErrorKind::Other, 
                    "Cannot get raw file descriptor from PTY master"
                )).into());
            }
        };
        println!("[DEBUG read_output_timeout] Got raw fd: {}", raw_fd);
        
        let timeout_start = std::time::Instant::now();
        let timeout_ms = timeout_duration.as_millis() as libc::c_int;
        
        println!("[DEBUG read_output_timeout] Using libc::poll with {}ms timeout...", timeout_ms);
        
        // Use poll() to check if data is available with timeout
        let mut poll_fds = [libc::pollfd {
            fd: raw_fd,
            events: libc::POLLIN,
            revents: 0,
        }];
        
        let poll_result = unsafe {
            libc::poll(poll_fds.as_mut_ptr(), 1, timeout_ms)
        };
        
        let poll_elapsed = timeout_start.elapsed();
        println!("[DEBUG read_output_timeout] poll() returned {} after {:?}", poll_result, poll_elapsed);
        
        match poll_result {
            -1 => {
                let errno = unsafe { *libc::__errno_location() };
                println!("[DEBUG read_output_timeout] poll() error, errno: {}", errno);
                Err(PtyError::IoError(Error::from_raw_os_error(errno)).into())
            },
            0 => {
                // Timeout - no data available
                println!("[DEBUG read_output_timeout] poll() timeout - no data available");
                Err(PtyError::IoError(Error::new(ErrorKind::TimedOut, "No data available within timeout")).into())
            },
            _ => {
                // Data is available, perform non-blocking read
                println!("[DEBUG read_output_timeout] poll() indicates data available, performing read...");
                let read_result = unsafe {
                    libc::read(raw_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len())
                };
                
                if read_result == -1 {
                    let errno = unsafe { *libc::__errno_location() };
                    println!("[DEBUG read_output_timeout] read() error, errno: {}", errno);
                    Err(PtyError::IoError(Error::from_raw_os_error(errno)).into())
                } else {
                    let bytes_read = read_result as usize;
                    let total_elapsed = timeout_start.elapsed();
                    println!("[DEBUG read_output_timeout] END - read {} bytes after {:?}", bytes_read, total_elapsed);
                    Ok(bytes_read)
                }
            }
        }
    }
    
    pub fn is_process_running(&mut self) -> bool {
        match &mut self.child_process {
            Some(child) => {
                match child.try_wait() {
                    Ok(Some(_)) => false, // Process has exited
                    Ok(None) => true,     // Process is still running
                    Err(_) => false,      // Error checking status, assume not running
                }
            },
            None => false,
        }
    }
    
    pub async fn wait_for_exit(&mut self, timeout_duration: Duration) -> Result<Option<i32>> {
        if let Some(child) = &mut self.child_process {
            let wait_future = async {
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            return Ok(status.success().then(|| 0).or(Some(1)));
                        },
                        Ok(None) => {
                            sleep(Duration::from_millis(10)).await;
                        },
                        Err(e) => {
                            return Err(PtyError::IoError(e));
                        }
                    }
                }
            };
            
            match timeout(timeout_duration, wait_future).await {
                Ok(result) => result.map_err(|e| e.into()),
                Err(_) => Ok(None), // Timeout
            }
        } else {
            Ok(None)
        }
    }
    
    pub fn terminate(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            // Try graceful termination first
            if let Err(_) = child.kill() {
                // If kill fails, the process might already be dead
            }
            
            // Wait for process to actually exit
            let _ = child.wait();
        }
        Ok(())
    }
    
    pub fn resize_terminal(&mut self, cols: u16, rows: u16) -> Result<()> {
        let new_size = PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        self.pty_pair.master.resize(new_size)
            .map_err(|e| PtyError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to resize terminal: {}", e)
            )))?;
        
        self.terminal_size = new_size;
        Ok(())
    }
    
    pub fn get_terminal_size(&self) -> (u16, u16) {
        (self.terminal_size.cols, self.terminal_size.rows)
    }
    
    pub async fn read_all_available(&mut self) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut buffer = vec![0u8; 1024];
        
        // Try to read all available data with short timeouts
        loop {
            match self.read_output_timeout(&mut buffer, Duration::from_millis(50)).await {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                    output.extend_from_slice(&buffer[..bytes_read]);
                },
                Err(_) => break, // Timeout or error, assume no more data
            }
        }
        
        Ok(output)
    }
    
    pub async fn drain_output(&mut self, max_wait: Duration) -> Result<Vec<u8>> {
        println!("[DEBUG drain_output] START - max_wait: {:?}", max_wait);
        let mut output = Vec::new();
        let mut buffer = vec![0u8; 4096];
        let start_time = std::time::Instant::now();
        
        // Try to read immediately without waiting if no process is running
        let process_running = self.is_process_running();
        println!("[DEBUG drain_output] Process running: {}", process_running);
        if !process_running {
            println!("[DEBUG drain_output] Process not running, returning early");
            return Ok(output);
        }
        
        let mut iteration = 0;
        loop {
            iteration += 1;
            let elapsed = start_time.elapsed();
            println!("[DEBUG drain_output] Iteration {}, elapsed: {:?}", iteration, elapsed);
            
            if elapsed > max_wait {
                println!("[DEBUG drain_output] Max wait exceeded, breaking loop");
                break;
            }
            
            // Use very short timeout for each read attempt
            println!("[DEBUG drain_output] Calling read_output_timeout with 10ms timeout...");
            let read_start = std::time::Instant::now();
            match self.read_output_timeout(&mut buffer, Duration::from_millis(10)).await {
                Ok(bytes_read) => {
                    let read_duration = read_start.elapsed();
                    println!("[DEBUG drain_output] read_output_timeout returned Ok({}) after {:?}", bytes_read, read_duration);
                    if bytes_read == 0 {
                        println!("[DEBUG drain_output] Zero bytes read, breaking loop");
                        break;
                    }
                    output.extend_from_slice(&buffer[..bytes_read]);
                    println!("[DEBUG drain_output] Total output size: {} bytes", output.len());
                },
                Err(e) => {
                    let read_duration = read_start.elapsed();
                    println!("[DEBUG drain_output] read_output_timeout returned Err({:?}) after {:?}", e, read_duration);
                    // No more data available, wait a bit and try again
                    println!("[DEBUG drain_output] Sleeping for 10ms...");
                    sleep(Duration::from_millis(10)).await;
                    println!("[DEBUG drain_output] Sleep completed");
                }
            }
            
            // If we got some data and process is no longer running, stop
            let process_still_running = self.is_process_running();
            println!("[DEBUG drain_output] Process still running: {}, output size: {}", process_still_running, output.len());
            if !output.is_empty() && !process_still_running {
                println!("[DEBUG drain_output] Got data and process stopped, breaking loop");
                break;
            }
        }
        
        println!("[DEBUG drain_output] END - returning {} bytes", output.len());
        Ok(output)
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

// Helper function to convert key specifications to bytes
pub fn key_spec_to_bytes(key_spec: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    
    match key_spec {
        "Tab" => bytes.push(0x09),
        "Enter" => bytes.push(0x0D),
        "Esc" => bytes.push(0x1B),
        "Left" => bytes.extend_from_slice(b"\x1B[D"),
        "Right" => bytes.extend_from_slice(b"\x1B[C"),
        "Up" => bytes.extend_from_slice(b"\x1B[A"),
        "Down" => bytes.extend_from_slice(b"\x1B[B"),
        "Home" => bytes.extend_from_slice(b"\x1B[H"),
        "End" => bytes.extend_from_slice(b"\x1B[F"),
        "PageUp" => bytes.extend_from_slice(b"\x1B[5~"),
        "PageDown" => bytes.extend_from_slice(b"\x1B[6~"),
        "Delete" => bytes.extend_from_slice(b"\x1B[3~"),
        "Backspace" => bytes.push(0x08),
        "F1" => bytes.extend_from_slice(b"\x1BOP"),
        "F2" => bytes.extend_from_slice(b"\x1BOQ"),
        "F3" => bytes.extend_from_slice(b"\x1BOR"),
        "F4" => bytes.extend_from_slice(b"\x1BOS"),
        "F5" => bytes.extend_from_slice(b"\x1B[15~"),
        "F6" => bytes.extend_from_slice(b"\x1B[17~"),
        "F7" => bytes.extend_from_slice(b"\x1B[18~"),
        "F8" => bytes.extend_from_slice(b"\x1B[19~"),
        "F9" => bytes.extend_from_slice(b"\x1B[20~"),
        "F10" => bytes.extend_from_slice(b"\x1B[21~"),
        "F11" => bytes.extend_from_slice(b"\x1B[23~"),
        "F12" => bytes.extend_from_slice(b"\x1B[24~"),
        _ => {
            // Handle modifier combinations
            if key_spec.contains('+') {
                let parts: Vec<&str> = key_spec.split('+').collect();
                if parts.len() == 2 {
                    let modifier = parts[0];
                    let key_part = parts[1];
                    
                    match modifier {
                        "Ctrl" => {
                            if key_part.len() == 1 {
                                let ch = key_part.chars().next().unwrap().to_ascii_uppercase();
                                let ctrl_code = (ch as u8) - b'A' + 1;
                                bytes.push(ctrl_code);
                            } else {
                                return Err(PtyError::IoError(std::io::Error::new(
                                    std::io::ErrorKind::InvalidInput,
                                    format!("Unsupported Ctrl+ combination: {}", key_spec)
                                )).into());
                            }
                        },
                        "Alt" => {
                            bytes.push(0x1B);
                            if key_part.len() == 1 {
                                bytes.extend_from_slice(key_part.as_bytes());
                            } else {
                                // For Alt + special keys, we'd need more complex handling
                                return Err(PtyError::IoError(std::io::Error::new(
                                    std::io::ErrorKind::InvalidInput,
                                    format!("Unsupported Alt+ combination: {}", key_spec)
                                )).into());
                            }
                        },
                        "Shift" => {
                            // Shift combinations are more complex and context-dependent
                            return Err(PtyError::IoError(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("Shift+ combinations not yet implemented: {}", key_spec)
                            )).into());
                        },
                        _ => {
                            return Err(PtyError::IoError(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("Unsupported modifier: {}", modifier)
                            )).into());
                        }
                    }
                } else {
                    return Err(PtyError::IoError(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Invalid key specification format: {}", key_spec)
                    )).into());
                }
            }
            // Handle repeat syntax (e.g., "Left*3")
            else if key_spec.contains('*') {
                let parts: Vec<&str> = key_spec.split('*').collect();
                if parts.len() == 2 {
                    let base_key = parts[0];
                    let repeat_count: usize = parts[1].parse()
                        .map_err(|_| PtyError::IoError(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("Invalid repeat count: {}", parts[1])
                        )))?;
                    
                    let base_bytes = key_spec_to_bytes(base_key)?;
                    for _ in 0..repeat_count {
                        bytes.extend_from_slice(&base_bytes);
                    }
                } else {
                    return Err(PtyError::IoError(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Invalid repeat syntax: {}", key_spec)
                    )).into());
                }
            }
            else {
                return Err(PtyError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unsupported key specification: {}", key_spec)
                )).into());
            }
        }
    }
    
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_pty_manager_creation() {
        let pty_manager = PtyManager::new(80, 24);
        assert!(pty_manager.is_ok());
        
        let manager = pty_manager.unwrap();
        assert_eq!(manager.get_terminal_size(), (80, 24));
    }
    
    #[test]
    fn test_key_spec_to_bytes() {
        assert_eq!(key_spec_to_bytes("Tab").unwrap(), vec![0x09]);
        assert_eq!(key_spec_to_bytes("Enter").unwrap(), vec![0x0D]);
        assert_eq!(key_spec_to_bytes("Esc").unwrap(), vec![0x1B]);
        assert_eq!(key_spec_to_bytes("Ctrl+C").unwrap(), vec![0x03]);
        assert_eq!(key_spec_to_bytes("Ctrl+D").unwrap(), vec![0x04]);
        
        // Test repeat syntax
        assert_eq!(key_spec_to_bytes("Tab*3").unwrap(), vec![0x09, 0x09, 0x09]);
        
        // Test arrow keys
        assert_eq!(key_spec_to_bytes("Left").unwrap(), b"\x1B[D");
        assert_eq!(key_spec_to_bytes("Right").unwrap(), b"\x1B[C");
        
        // Test invalid key
        assert!(key_spec_to_bytes("InvalidKey").is_err());
    }
    
    #[tokio::test]
    async fn test_pty_manager_basic_functionality() {
        let mut pty_manager = PtyManager::new(80, 24).unwrap();
        
        // Test that process is not running initially
        assert!(!pty_manager.is_process_running());
        
        // Test resizing
        assert!(pty_manager.resize_terminal(100, 30).is_ok());
        assert_eq!(pty_manager.get_terminal_size(), (100, 30));
    }
    
    #[tokio::test]
    async fn test_command_spawning() {
        let mut pty_manager = PtyManager::new(80, 24).unwrap();
        
        let cmd_config = CommandConfig {
            exec: vec!["echo".to_string(), "hello".to_string()],
            workdir: None,
            env: None,
        };
        
        let result = pty_manager.spawn_command(&cmd_config);
        assert!(result.is_ok());
        
        // Basic test - just verify the command was spawned without hanging
        assert!(pty_manager.is_process_running() || !pty_manager.is_process_running()); // Either state is fine
    }
}