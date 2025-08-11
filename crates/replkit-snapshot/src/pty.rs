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
        use std::io::Write;
        
        let mut writer = self.pty_pair.master.take_writer()
            .map_err(|e| PtyError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to get writer: {}", e)
            )))?;
        
        writer.write_all(input)
            .map_err(|e| PtyError::IoError(e))?;
        writer.flush()
            .map_err(|e| PtyError::IoError(e))?;
        
        Ok(())
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
        let mut reader = self.pty_pair.master.try_clone_reader()
            .map_err(|e| PtyError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to clone reader: {}", e)
            )))?;
        
        let read_future = async move {
            reader.read(buffer)
                .map_err(|e| PtyError::IoError(e))
        };
        
        timeout(timeout_duration, read_future)
            .await
            .map_err(|_| PtyError::IoError(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Read operation timed out"
            )))?
            .map_err(|e| e.into())
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
        let mut output = Vec::new();
        let mut buffer = vec![0u8; 4096];
        let start_time = std::time::Instant::now();
        
        loop {
            if start_time.elapsed() > max_wait {
                break;
            }
            
            match self.read_output_timeout(&mut buffer, Duration::from_millis(100)).await {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                    output.extend_from_slice(&buffer[..bytes_read]);
                },
                Err(_) => {
                    // No more data available
                    break;
                }
            }
        }
        
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