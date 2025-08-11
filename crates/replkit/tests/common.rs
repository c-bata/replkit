// Common test utilities and types for automated rendering tests
// This module provides shared functionality across all integration tests

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Represents a captured console operation for testing
#[derive(Debug, Clone, PartialEq)]
pub enum ConsoleOperation {
    MoveCursor { row: u16, col: u16 },
    WriteText { text: String },
    SetStyle { style: String }, // Simplified for now
    ResetStyle,
    Clear { clear_type: String }, // Simplified for now
    Flush,
    HideCursor,
    ShowCursor,
    GetCursorPosition,
}

/// Capture mode for test console output
#[derive(Debug, Clone, Copy)]
pub enum CaptureMode {
    Operations,      // Capture operation sequence only
    TerminalState,   // Use termwiz emulation only
    Both,           // Capture both for comparison
}

/// Error types for test operations
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Terminal emulation error: {0}")]
    TerminalError(String),
    
    #[error("Snapshot comparison failed: {0}")]
    SnapshotError(String),
    
    #[error("Compatibility validation failed: {0}")]
    CompatibilityError(String),
    
    #[error("Test setup error: {0}")]
    SetupError(String),
    
    #[error("Operation capture error: {0}")]
    CaptureError(String),
    
    #[error("Test timeout after {timeout_ms}ms")]
    TimeoutError { timeout_ms: u64 },
    
    #[error("Invalid test configuration: {0}")]
    ConfigurationError(String),
    
    #[error("Test assertion failed: {0}")]
    AssertionError(String),
}

/// Result type for test operations
pub type TestResult<T> = Result<T, TestError>;

/// Terminal emulator wrapper for testing
pub struct TerminalEmulator {
    screen_size: (u16, u16),
    cursor_position: (u16, u16),
    screen_buffer: Vec<String>,
}

impl TerminalEmulator {
    /// Create a new terminal emulator with specified dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        let screen_buffer = vec![" ".repeat(cols as usize); rows as usize];
        Self {
            screen_size: (cols, rows),
            cursor_position: (0, 0),
            screen_buffer,
        }
    }

    /// Write text to the terminal at current cursor position
    pub fn write_text(&mut self, text: &str) -> Result<(), TestError> {
        // Simple implementation for now - will be enhanced with termwiz integration
        let (row, col) = self.cursor_position;
        if row < self.screen_size.1 && col < self.screen_size.0 {
            let line = &mut self.screen_buffer[row as usize];
            let start = col as usize;
            
            if start < line.len() {
                let mut chars: Vec<char> = line.chars().collect();
                for (i, ch) in text.chars().enumerate() {
                    if start + i < chars.len() {
                        chars[start + i] = ch;
                    }
                }
                *line = chars.into_iter().collect();
            }
            
            // Move cursor forward
            self.cursor_position.1 = std::cmp::min(
                self.cursor_position.1 + text.len() as u16,
                self.screen_size.0 - 1
            );
        }
        Ok(())
    }

    /// Move cursor to specified position
    pub fn move_cursor_to(&mut self, row: u16, col: u16) -> Result<(), TestError> {
        if row < self.screen_size.1 && col < self.screen_size.0 {
            self.cursor_position = (row, col);
            Ok(())
        } else {
            Err(TestError::TerminalError(format!(
                "Invalid cursor position ({}, {}) for terminal size ({}, {})",
                row, col, self.screen_size.0, self.screen_size.1
            )))
        }
    }

    /// Get current cursor position
    pub fn get_cursor_position(&self) -> (u16, u16) {
        self.cursor_position
    }

    /// Get screen contents as a single string
    pub fn get_screen_contents(&self) -> String {
        self.screen_buffer.join("\n")
    }

    /// Get screen lines as vector
    pub fn get_screen_lines(&self) -> Vec<String> {
        self.screen_buffer.clone()
    }

    /// Clear the screen
    pub fn clear_screen(&mut self) -> Result<(), TestError> {
        self.screen_buffer = vec![" ".repeat(self.screen_size.0 as usize); self.screen_size.1 as usize];
        self.cursor_position = (0, 0);
        Ok(())
    }

    /// Get terminal dimensions
    pub fn get_size(&self) -> (u16, u16) {
        self.screen_size
    }
}

/// Test console output implementation that captures operations
pub struct TestConsoleOutput {
    operations: Arc<Mutex<Vec<ConsoleOperation>>>,
    terminal_emulator: TerminalEmulator,
    capture_mode: CaptureMode,
}

impl TestConsoleOutput {
    /// Create a new test console with specified dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            operations: Arc::new(Mutex::new(Vec::new())),
            terminal_emulator: TerminalEmulator::new(cols, rows),
            capture_mode: CaptureMode::Both,
        }
    }

    /// Create a new test console with capture mode
    pub fn with_capture_mode(cols: u16, rows: u16, mode: CaptureMode) -> Self {
        Self {
            operations: Arc::new(Mutex::new(Vec::new())),
            terminal_emulator: TerminalEmulator::new(cols, rows),
            capture_mode: mode,
        }
    }

    /// Get captured operations
    pub fn get_operations(&self) -> Vec<ConsoleOperation> {
        self.operations.lock().unwrap().clone()
    }

    /// Clear captured operations
    pub fn clear_operations(&self) {
        self.operations.lock().unwrap().clear();
    }

    /// Get terminal emulator reference
    pub fn terminal_emulator(&self) -> &TerminalEmulator {
        &self.terminal_emulator
    }

    /// Record an operation if capture mode includes operations
    fn record_operation(&self, operation: ConsoleOperation) {
        match self.capture_mode {
            CaptureMode::Operations | CaptureMode::Both => {
                self.operations.lock().unwrap().push(operation);
            }
            CaptureMode::TerminalState => {
                // Only use terminal emulator, don't record operations
            }
        }
    }
}

/// Manager for snapshot testing operations
pub struct SnapshotManager {
    test_name: String,
}

impl SnapshotManager {
    /// Create a new snapshot manager for a test
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
        }
    }

    /// Assert terminal output matches snapshot
    pub fn assert_terminal_snapshot(&self, console: &TestConsoleOutput) {
        let screen_contents = console.terminal_emulator().get_screen_contents();
        
        // Note: insta integration will be added when insta dependency is available
        // For now, this is a placeholder implementation
        println!("Snapshot test '{}' - Terminal output:", self.test_name);
        println!("{}", screen_contents);
    }

    /// Assert operation sequence matches snapshot
    pub fn assert_operations_snapshot(&self, console: &TestConsoleOutput) {
        let operations = console.get_operations();
        let formatted = self.format_operations(&operations);
        
        // Note: insta integration will be added when insta dependency is available
        // For now, this is a placeholder implementation
        println!("Snapshot test '{}' - Operations:", self.test_name);
        println!("{}", formatted);
    }

    /// Format operations for readable output
    fn format_operations(&self, operations: &[ConsoleOperation]) -> String {
        operations
            .iter()
            .enumerate()
            .map(|(i, op)| format!("{:2}: {:?}", i, op))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Create a combined snapshot with both terminal and operations
    pub fn assert_combined_snapshot(&self, console: &TestConsoleOutput) {
        let screen_contents = console.terminal_emulator().get_screen_contents();
        let operations = console.get_operations();
        let formatted_ops = self.format_operations(&operations);
        
        let combined = format!(
            "=== Terminal Output ===\n{}\n\n=== Operations ===\n{}",
            screen_contents, formatted_ops
        );
        
        // Note: insta integration will be added when insta dependency is available
        println!("Combined snapshot test '{}':", self.test_name);
        println!("{}", combined);
    }
}

/// Reference output data from go-prompt for comparison
#[derive(Debug, Clone)]
pub struct ReferenceOutput {
    pub scenario: String,
    pub terminal_output: String,
    pub cursor_position: (u16, u16),
    pub operations: Vec<String>, // Simplified operation descriptions
}

/// Result of compatibility validation
#[derive(Debug)]
pub struct CompatibilityResult {
    pub scenario: String,
    pub matches: bool,
    pub differences: Vec<Difference>,
}

/// Types of differences found during compatibility validation
#[derive(Debug)]
pub enum Difference {
    TerminalOutput { expected: String, actual: String },
    CursorPosition { expected: (u16, u16), actual: (u16, u16) },
    OperationSequence { expected: Vec<String>, actual: Vec<String> },
}

/// Validator for go-prompt compatibility
pub struct CompatibilityValidator {
    reference_data: HashMap<String, ReferenceOutput>,
}

impl CompatibilityValidator {
    /// Create a new compatibility validator
    pub fn new() -> Self {
        Self {
            reference_data: HashMap::new(),
        }
    }

    /// Load reference data from go-prompt
    pub fn load_references() -> Self {
        let mut validator = Self::new();
        
        // TODO: Load actual reference data from files or embedded data
        // For now, create some placeholder reference data
        validator.add_reference(ReferenceOutput {
            scenario: "basic_prompt".to_string(),
            terminal_output: "$ hello world".to_string(),
            cursor_position: (0, 13),
            operations: vec![
                "MoveCursor(0, 0)".to_string(),
                "WriteText('$ ')".to_string(),
                "WriteText('hello world')".to_string(),
            ],
        });
        
        validator
    }

    /// Add a reference output for a scenario
    pub fn add_reference(&mut self, reference: ReferenceOutput) {
        self.reference_data.insert(reference.scenario.clone(), reference);
    }

    /// Validate a scenario against reference data
    pub fn validate_scenario(&self, scenario: &str, console: &TestConsoleOutput) -> CompatibilityResult {
        let reference = match self.reference_data.get(scenario) {
            Some(ref_data) => ref_data,
            None => {
                return CompatibilityResult {
                    scenario: scenario.to_string(),
                    matches: false,
                    differences: vec![Difference::TerminalOutput {
                        expected: format!("No reference data for scenario: {}", scenario),
                        actual: "N/A".to_string(),
                    }],
                };
            }
        };

        let actual_output = console.terminal_emulator().get_screen_contents();
        let actual_cursor = console.terminal_emulator().get_cursor_position();
        
        let differences = self.compute_differences(reference, &actual_output, actual_cursor);
        let matches = differences.is_empty();

        CompatibilityResult {
            scenario: scenario.to_string(),
            matches,
            differences,
        }
    }

    /// Compute differences between reference and actual output
    fn compute_differences(
        &self,
        reference: &ReferenceOutput,
        actual_output: &str,
        actual_cursor: (u16, u16),
    ) -> Vec<Difference> {
        let mut differences = Vec::new();

        if actual_output != reference.terminal_output {
            differences.push(Difference::TerminalOutput {
                expected: reference.terminal_output.clone(),
                actual: actual_output.to_string(),
            });
        }

        if actual_cursor != reference.cursor_position {
            differences.push(Difference::CursorPosition {
                expected: reference.cursor_position,
                actual: actual_cursor,
            });
        }

        differences
    }

    /// Get all available reference scenarios
    pub fn get_scenarios(&self) -> Vec<String> {
        self.reference_data.keys().cloned().collect()
    }
}

impl Default for CompatibilityValidator {
    fn default() -> Self {
        Self::load_references()
    }
}

/// Configuration for test execution
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub terminal_size: (u16, u16),
    pub capture_mode: CaptureMode,
    pub enable_snapshots: bool,
    pub enable_compatibility_checks: bool,
    pub test_timeout_ms: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            terminal_size: (80, 24),
            capture_mode: CaptureMode::Both,
            enable_snapshots: true,
            enable_compatibility_checks: true,
            test_timeout_ms: 5000,
        }
    }
}

impl TestConfig {
    /// Create a new test configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set terminal size
    pub fn with_terminal_size(mut self, cols: u16, rows: u16) -> Self {
        self.terminal_size = (cols, rows);
        self
    }

    /// Set capture mode
    pub fn with_capture_mode(mut self, mode: CaptureMode) -> Self {
        self.capture_mode = mode;
        self
    }

    /// Enable or disable snapshot testing
    pub fn with_snapshots(mut self, enabled: bool) -> Self {
        self.enable_snapshots = enabled;
        self
    }

    /// Enable or disable compatibility checks
    pub fn with_compatibility_checks(mut self, enabled: bool) -> Self {
        self.enable_compatibility_checks = enabled;
        self
    }

    /// Set test timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.test_timeout_ms = timeout_ms;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), TestError> {
        if self.terminal_size.0 == 0 || self.terminal_size.1 == 0 {
            return Err(TestError::SetupError(
                "Terminal size must be greater than 0".to_string()
            ));
        }

        if self.test_timeout_ms == 0 {
            return Err(TestError::SetupError(
                "Test timeout must be greater than 0".to_string()
            ));
        }

        Ok(())
    }
}

/// Test scenario definition for structured testing
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub name: String,
    pub description: String,
    pub config: TestConfig,
}

impl TestScenario {
    /// Create a new test scenario
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            config: TestConfig::default(),
        }
    }

    /// Set configuration for this scenario
    pub fn with_config(mut self, config: TestConfig) -> Self {
        self.config = config;
        self
    }
}

/// Common test scenarios for rendering tests
pub struct CommonScenarios;

impl CommonScenarios {
    /// Basic prompt rendering scenario
    pub fn basic_prompt() -> TestScenario {
        TestScenario::new(
            "basic_prompt",
            "Test basic prompt rendering with prefix and text input"
        )
    }

    /// Completion menu rendering scenario
    pub fn completion_menu() -> TestScenario {
        TestScenario::new(
            "completion_menu",
            "Test completion menu display and selection highlighting"
        )
    }

    /// Multi-line text scenario
    pub fn multi_line() -> TestScenario {
        TestScenario::new(
            "multi_line",
            "Test multi-line text handling and line wrapping"
        )
    }

    /// Terminal resize scenario
    pub fn terminal_resize() -> TestScenario {
        TestScenario::new(
            "terminal_resize",
            "Test rendering adaptation to terminal size changes"
        )
    }

    /// Get all common scenarios
    pub fn all() -> Vec<TestScenario> {
        vec![
            Self::basic_prompt(),
            Self::completion_menu(),
            Self::multi_line(),
            Self::terminal_resize(),
        ]
    }
}