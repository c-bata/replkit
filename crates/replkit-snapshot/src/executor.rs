use std::time::Duration;
use tokio::time::sleep;
use crate::config::{Step, InputSpec, SnapshotConfig};
use crate::error::{ExecutionError, Result};
use crate::pty::{PtyManager, key_spec_to_bytes};

pub struct StepExecutor {
    pty_manager: PtyManager,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub step_index: usize,
    pub step_name: String,
    pub success: bool,
    pub output: Option<Vec<u8>>,
    pub duration: Duration,
    pub error: Option<String>,
}

impl StepExecutor {
    pub fn new(pty_manager: PtyManager) -> Self {
        Self { pty_manager }
    }
    
    pub async fn execute_steps(&mut self, steps: &[Step]) -> Result<Vec<ExecutionResult>> {
        let mut results = Vec::new();
        
        for (index, step) in steps.iter().enumerate() {
            let start_time = std::time::Instant::now();
            let step_name = self.get_step_name(step);
            
            println!("Executing step {}: {}", index + 1, step_name);
            
            let result = match self.execute_single_step(step).await {
                Ok(output) => ExecutionResult {
                    step_index: index,
                    step_name: step_name.clone(),
                    success: true,
                    output,
                    duration: start_time.elapsed(),
                    error: None,
                },
                Err(e) => {
                    eprintln!("Step {} failed: {}", index + 1, e);
                    ExecutionResult {
                        step_index: index,
                        step_name: step_name.clone(),
                        success: false,
                        output: None,
                        duration: start_time.elapsed(),
                        error: Some(e.to_string()),
                    }
                }
            };
            
            results.push(result);
            
            // Stop execution if a step fails
            if !results.last().unwrap().success {
                println!("Stopping execution due to step failure");
                break;
            }
        }
        
        Ok(results)
    }
    
    async fn execute_single_step(&mut self, step: &Step) -> Result<Option<Vec<u8>>> {
        match step {
            Step::Send { send } => {
                self.execute_send(send).await?;
                Ok(None)
            },
            Step::WaitIdle { wait_idle } => {
                self.execute_wait_idle(wait_idle).await?;
                Ok(None)
            },
            Step::WaitRegex { wait_for_regex } => {
                self.execute_wait_regex(wait_for_regex).await?;
                Ok(None)
            },
            Step::WaitExit { wait_exit } => {
                self.execute_wait_exit(wait_exit).await?;
                Ok(None)
            },
            Step::Snapshot { snapshot } => {
                let output = self.capture_snapshot(snapshot).await?;
                Ok(Some(output))
            },
            Step::Sleep { sleep } => {
                self.execute_sleep(sleep).await?;
                Ok(None)
            },
        }
    }
    
    async fn execute_send(&mut self, input_spec: &InputSpec) -> Result<()> {
        match input_spec {
            InputSpec::Text(text) => {
                println!("  Sending text: \"{}\"", text);
                self.pty_manager.send_input(text.as_bytes()).await?;
            },
            InputSpec::Keys(keys) => {
                println!("  Sending keys: {:?}", keys);
                let mut bytes = Vec::new();
                for key in keys {
                    let key_bytes = key_spec_to_bytes(key)?;
                    bytes.extend_from_slice(&key_bytes);
                }
                self.pty_manager.send_input(&bytes).await?;
            },
        }
        Ok(())
    }
    
    async fn execute_wait_idle(&mut self, wait_idle: &str) -> Result<()> {
        let duration = parse_duration(wait_idle)?;
        println!("  Waiting idle for {:?}", duration);
        sleep(duration).await;
        Ok(())
    }
    
    async fn execute_wait_regex(&mut self, wait_for_regex: &str) -> Result<()> {
        println!("  Waiting for regex: \"{}\"", wait_for_regex);
        let regex = regex::Regex::new(wait_for_regex)
            .map_err(|e| ExecutionError::WaitConditionFailed(
                format!("Invalid regex: {}", e)
            ))?;
        
        // Wait for output matching the regex (with timeout)
        let timeout = Duration::from_secs(30); // Default timeout
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout {
            let output = self.pty_manager.drain_output(Duration::from_millis(100)).await?;
            if !output.is_empty() {
                let output_str = String::from_utf8_lossy(&output);
                if regex.is_match(&output_str) {
                    println!("  Regex matched in output");
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
        
        Err(ExecutionError::WaitConditionFailed(
            format!("Regex '{}' did not match within timeout", wait_for_regex)
        ).into())
    }
    
    async fn execute_wait_exit(&mut self, wait_exit: &str) -> Result<()> {
        let timeout = parse_duration(wait_exit)?;
        println!("  Waiting for process exit (timeout: {:?})", timeout);
        
        match self.pty_manager.wait_for_exit(timeout).await? {
            Some(exit_code) => {
                println!("  Process exited with code: {}", exit_code);
                Ok(())
            },
            None => {
                Err(ExecutionError::WaitConditionFailed(
                    "Process did not exit within timeout".to_string()
                ).into())
            }
        }
    }
    
    async fn capture_snapshot(&mut self, snapshot_config: &SnapshotConfig) -> Result<Vec<u8>> {
        println!("  Capturing snapshot: \"{}\"", snapshot_config.name);
        
        // Drain all available output with short timeout
        let output = self.pty_manager.drain_output(Duration::from_millis(100)).await?;
        
        if output.is_empty() {
            println!("  No output captured for snapshot (this is normal for completed processes)");
        } else {
            println!("  Captured {} bytes of output", output.len());
        }
        
        Ok(output)
    }
    
    async fn execute_sleep(&mut self, sleep_duration: &str) -> Result<()> {
        let duration = parse_duration(sleep_duration)?;
        println!("  Sleeping for {:?}", duration);
        sleep(duration).await;
        Ok(())
    }
    
    fn get_step_name(&self, step: &Step) -> String {
        match step {
            Step::Send { send } => {
                match send {
                    InputSpec::Text(text) => {
                        let preview = if text.len() > 20 {
                            format!("{}...", &text[..20])
                        } else {
                            text.clone()
                        };
                        format!("Send text: \"{}\"", preview)
                    },
                    InputSpec::Keys(keys) => format!("Send keys: {:?}", keys),
                }
            },
            Step::WaitIdle { wait_idle } => format!("Wait idle: {}", wait_idle),
            Step::WaitRegex { wait_for_regex } => format!("Wait for regex: \"{}\"", wait_for_regex),
            Step::WaitExit { wait_exit } => format!("Wait for exit: {}", wait_exit),
            Step::Snapshot { snapshot } => format!("Take snapshot: \"{}\"", snapshot.name),
            Step::Sleep { sleep } => format!("Sleep: {}", sleep),
        }
    }
    
    pub fn get_pty_manager(&self) -> &PtyManager {
        &self.pty_manager
    }
    
    pub fn get_pty_manager_mut(&mut self) -> &mut PtyManager {
        &mut self.pty_manager
    }
}

fn parse_duration(duration_str: &str) -> Result<Duration> {
    if duration_str.ends_with("ms") {
        let millis = duration_str.trim_end_matches("ms").parse::<u64>()
            .map_err(|_| ExecutionError::WaitConditionFailed(
                format!("Invalid duration format: {}", duration_str)
            ))?;
        Ok(Duration::from_millis(millis))
    } else if duration_str.ends_with('s') {
        let seconds = duration_str.trim_end_matches('s').parse::<u64>()
            .map_err(|_| ExecutionError::WaitConditionFailed(
                format!("Invalid duration format: {}", duration_str)
            ))?;
        Ok(Duration::from_secs(seconds))
    } else {
        Err(ExecutionError::WaitConditionFailed(
            format!("Invalid duration format: {}", duration_str)
        ).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CommandConfig;
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert!(parse_duration("invalid").is_err());
    }
    
    #[test]
    fn test_get_step_name() {
        let pty_manager = PtyManager::new(80, 24).unwrap();
        let executor = StepExecutor::new(pty_manager);
        
        let step = Step::Send { 
            send: InputSpec::Text("hello world".to_string()) 
        };
        assert_eq!(executor.get_step_name(&step), "Send text: \"hello world\"");
        
        let step = Step::Send { 
            send: InputSpec::Keys(vec!["Tab".to_string(), "Enter".to_string()]) 
        };
        assert_eq!(executor.get_step_name(&step), "Send keys: [\"Tab\", \"Enter\"]");
        
        let step = Step::WaitIdle { wait_idle: "100ms".to_string() };
        assert_eq!(executor.get_step_name(&step), "Wait idle: 100ms");
    }
    
    #[tokio::test]
    async fn test_execute_sleep() {
        let pty_manager = PtyManager::new(80, 24).unwrap();
        let mut executor = StepExecutor::new(pty_manager);
        
        let start_time = std::time::Instant::now();
        executor.execute_sleep("100ms").await.unwrap();
        let elapsed = start_time.elapsed();
        
        // Should sleep for approximately 100ms (with some tolerance)
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed <= Duration::from_millis(200));
    }
    
    #[tokio::test]
    async fn test_execute_send_text_basic() {
        let pty_manager = PtyManager::new(80, 24).unwrap();
        let mut executor = StepExecutor::new(pty_manager);
        
        let input_spec = InputSpec::Text("hello".to_string());
        let result = executor.execute_send(&input_spec).await;
        // Just test that the method completes without error - don't try to read output
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_execute_steps_basic() {
        let pty_manager = PtyManager::new(80, 24).unwrap();
        let mut executor = StepExecutor::new(pty_manager);
        
        let steps = vec![
            Step::Sleep { sleep: "10ms".to_string() }, // Very short sleep
            Step::Snapshot { 
                snapshot: SnapshotConfig { 
                    name: "test".to_string(), 
                    strip_ansi: true,
                    mask: None,
                }
            },
        ];
        
        let results = executor.execute_steps(&steps).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
        assert_eq!(results[0].step_name, "Sleep: 10ms");
        assert_eq!(results[1].step_name, "Take snapshot: \"test\"");
    }
}