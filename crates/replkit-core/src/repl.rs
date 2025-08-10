//! REPL (Read-Eval-Print Loop) engine and configuration.
//!
//! This module provides the core REPL engine that integrates ConsoleInput, KeyParser,
//! Buffer, and ConsoleOutput components to create a working interactive prompt system.

use crate::{
    console::{ConsoleError, ConsoleInput, ConsoleOutput, RawModeGuard},
    event_loop::{EventLoop, ReplEvent},
    key::{Key, KeyEvent},
    key_handler::{KeyHandler, KeyResult},
    renderer::Renderer,
    Buffer, KeyParser,
};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// Main REPL engine that orchestrates all components.
pub struct ReplEngine {
    config: ReplConfig,
    state: ReplState,
    console_input: Option<Box<dyn ConsoleInput>>,
    console_output: Option<Box<dyn ConsoleOutput>>,
    buffer: Buffer,
    key_parser: KeyParser,
    key_handler: Option<KeyHandler>,
    renderer: Option<Renderer>,
    event_loop: Option<EventLoop>,
}

/// Configuration for the REPL engine.
pub struct ReplConfig {
    /// Prompt prefix to display before user input
    pub prompt: String,
    /// Callback function to execute when user presses Enter
    pub executor: Box<dyn Fn(&str) -> Result<(), Box<dyn Error + Send + Sync>> + Send + Sync>,
    /// Optional function to check if input should cause REPL to exit
    pub exit_checker: Option<Box<dyn Fn(&str, bool) -> bool + Send + Sync>>,
    /// Custom key bindings for the REPL
    pub key_bindings: Vec<KeyBinding>,
    /// Whether to enable history support
    pub enable_history: bool,
    /// Maximum number of history entries to keep
    pub max_history_size: usize,
    /// Whether to enable multi-line input support
    pub enable_multiline: bool,
}

/// Internal state management for the REPL engine.
struct ReplState {
    /// Whether the REPL is currently running
    running: bool,
    /// Whether the REPL should exit
    should_exit: bool,
    /// Raw mode guard for terminal restoration
    raw_mode_guard: Option<RawModeGuard>,
    /// Current terminal window size (columns, rows)
    window_size: (u16, u16),
    /// Hash of last rendered content to optimize rendering
    last_render_hash: u64,
}

/// Custom key binding configuration.
pub struct KeyBinding {
    /// The key that triggers this binding
    pub key: Key,
    /// The action to perform when the key is pressed
    pub action: KeyAction,
}

/// Actions that can be performed when a key is pressed.
pub enum KeyAction {
    /// Move cursor left by specified number of characters
    MoveCursorLeft(usize),
    /// Move cursor right by specified number of characters
    MoveCursorRight(usize),
    /// Delete specified number of characters backward from cursor
    DeleteBackward(usize),
    /// Delete specified number of characters forward from cursor
    DeleteForward(usize),
    /// Move cursor to beginning of line
    MoveToBeginning,
    /// Move cursor to end of line
    MoveToEnd,
    /// Clear the current line
    ClearLine,
    /// Execute the current input
    Execute,
    /// Exit the REPL
    Exit,
    /// Custom action with user-defined function
    Custom(Box<dyn Fn(&mut Buffer) -> Result<(), ReplError> + Send + Sync>),
}

impl std::fmt::Debug for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAction::MoveCursorLeft(n) => write!(f, "MoveCursorLeft({n})"),
            KeyAction::MoveCursorRight(n) => write!(f, "MoveCursorRight({n})"),
            KeyAction::DeleteBackward(n) => write!(f, "DeleteBackward({n})"),
            KeyAction::DeleteForward(n) => write!(f, "DeleteForward({n})"),
            KeyAction::MoveToBeginning => write!(f, "MoveToBeginning"),
            KeyAction::MoveToEnd => write!(f, "MoveToEnd"),
            KeyAction::ClearLine => write!(f, "ClearLine"),
            KeyAction::Execute => write!(f, "Execute"),
            KeyAction::Exit => write!(f, "Exit"),
            KeyAction::Custom(_) => write!(f, "Custom(<function>)"),
        }
    }
}

impl Clone for KeyAction {
    fn clone(&self) -> Self {
        match self {
            KeyAction::MoveCursorLeft(n) => KeyAction::MoveCursorLeft(*n),
            KeyAction::MoveCursorRight(n) => KeyAction::MoveCursorRight(*n),
            KeyAction::DeleteBackward(n) => KeyAction::DeleteBackward(*n),
            KeyAction::DeleteForward(n) => KeyAction::DeleteForward(*n),
            KeyAction::MoveToBeginning => KeyAction::MoveToBeginning,
            KeyAction::MoveToEnd => KeyAction::MoveToEnd,
            KeyAction::ClearLine => KeyAction::ClearLine,
            KeyAction::Execute => KeyAction::Execute,
            KeyAction::Exit => KeyAction::Exit,
            KeyAction::Custom(_) => {
                // Cannot clone function pointers, so we create a no-op custom action
                KeyAction::Custom(Box::new(|_| Ok(())))
            }
        }
    }
}

impl std::fmt::Debug for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyBinding")
            .field("key", &self.key)
            .field("action", &self.action)
            .finish()
    }
}

impl Clone for KeyBinding {
    fn clone(&self) -> Self {
        KeyBinding {
            key: self.key,
            action: self.action.clone(),
        }
    }
}

/// Errors that can occur during REPL operations.
#[derive(Debug)]
pub enum ReplError {
    /// Console I/O error
    ConsoleError(ConsoleError),
    /// Configuration validation error
    ConfigurationError(String),
    /// Event loop error
    EventLoopError(String),
    /// Rendering error
    RenderError(String),
    /// Callback execution error
    CallbackError(String),
    /// Buffer operation error
    BufferError(crate::error::BufferError),
    /// Key parsing error
    KeyParsingError(String),
    /// Terminal state error
    TerminalStateError(String),
}

impl fmt::Display for ReplError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplError::ConsoleError(e) => write!(f, "Console error: {e}"),
            ReplError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
            ReplError::EventLoopError(msg) => write!(f, "Event loop error: {msg}"),
            ReplError::RenderError(msg) => write!(f, "Render error: {msg}"),
            ReplError::CallbackError(msg) => write!(f, "Callback error: {msg}"),
            ReplError::BufferError(e) => write!(f, "Buffer error: {e}"),
            ReplError::KeyParsingError(msg) => write!(f, "Key parsing error: {msg}"),
            ReplError::TerminalStateError(msg) => write!(f, "Terminal state error: {msg}"),
        }
    }
}

impl Error for ReplError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReplError::ConsoleError(e) => Some(e),
            ReplError::BufferError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ConsoleError> for ReplError {
    fn from(err: ConsoleError) -> Self {
        ReplError::ConsoleError(err)
    }
}

impl From<crate::error::BufferError> for ReplError {
    fn from(err: crate::error::BufferError) -> Self {
        ReplError::BufferError(err)
    }
}

impl From<crate::console::EventLoopError> for ReplError {
    fn from(err: crate::console::EventLoopError) -> Self {
        ReplError::EventLoopError(format!("{:?}", err))
    }
}

impl Default for ReplConfig {
    fn default() -> Self {
        ReplConfig {
            prompt: ">>> ".to_string(),
            executor: Box::new(|_input| Ok(())),
            exit_checker: None,
            key_bindings: Vec::new(),
            enable_history: false,
            max_history_size: 1000,
            enable_multiline: false,
        }
    }
}

impl ReplState {
    fn new() -> Self {
        ReplState {
            running: false,
            should_exit: false,
            raw_mode_guard: None,
            window_size: (80, 24), // Default terminal size
            last_render_hash: 0,
        }
    }
}

impl ReplEngine {
    /// Create a new REPL engine with the given configuration.
    pub fn new(config: ReplConfig) -> Result<Self, ReplError> {
        // Validate configuration
        Self::validate_config(&config)?;

        let state = ReplState::new();
        let buffer = Buffer::new();
        let key_parser = KeyParser::new();

        Ok(ReplEngine {
            config,
            state,
            console_input: None,
            console_output: None,
            buffer,
            key_parser,
            key_handler: None,
            renderer: None,
            event_loop: None,
        })
    }

    /// Validate the REPL configuration.
    fn validate_config(config: &ReplConfig) -> Result<(), ReplError> {
        // Validate prompt is not empty
        if config.prompt.is_empty() {
            return Err(ReplError::ConfigurationError(
                "Prompt cannot be empty".to_string(),
            ));
        }

        // Validate prompt doesn't contain control characters that could interfere with rendering
        if config.prompt.chars().any(|c| c.is_control() && c != '\t') {
            return Err(ReplError::ConfigurationError(
                "Prompt cannot contain control characters (except tab)".to_string(),
            ));
        }

        // Validate history size is reasonable
        if config.enable_history && config.max_history_size == 0 {
            return Err(ReplError::ConfigurationError(
                "History size must be greater than 0 when history is enabled".to_string(),
            ));
        }

        if config.max_history_size > 100_000 {
            return Err(ReplError::ConfigurationError(
                "History size cannot exceed 100,000 entries".to_string(),
            ));
        }

        // Validate key bindings don't have duplicates
        let mut seen_keys = std::collections::HashSet::new();
        for binding in &config.key_bindings {
            if !seen_keys.insert(binding.key) {
                return Err(ReplError::ConfigurationError(format!(
                    "Duplicate key binding for key: {:?}",
                    binding.key
                )));
            }
        }

        Ok(())
    }

    /// Set the console input implementation.
    pub fn set_console_input(&mut self, input: Box<dyn ConsoleInput>) {
        self.console_input = Some(input);
    }

    /// Set the console output implementation.
    pub fn set_console_output(&mut self, output: Box<dyn ConsoleOutput>) {
        self.console_output = Some(output);
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &ReplConfig {
        &self.config
    }

    /// Get a reference to the current buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get a mutable reference to the current buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// Check if the REPL is currently running.
    pub fn is_running(&self) -> bool {
        self.state.running
    }

    /// Check if the REPL should exit.
    pub fn should_exit(&self) -> bool {
        self.state.should_exit
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u16, u16) {
        self.state.window_size
    }

    /// Update the window size.
    pub fn set_window_size(&mut self, width: u16, height: u16) {
        self.state.window_size = (width, height);
    }

    /// Create default key bindings for the REPL.
    pub fn create_default_key_bindings() -> Vec<KeyBinding> {
        vec![
            KeyBinding {
                key: Key::ControlA,
                action: KeyAction::MoveToBeginning,
            },
            KeyBinding {
                key: Key::ControlE,
                action: KeyAction::MoveToEnd,
            },
            KeyBinding {
                key: Key::ControlC,
                action: KeyAction::ClearLine,
            },
            KeyBinding {
                key: Key::ControlD,
                action: KeyAction::Exit,
            },
            KeyBinding {
                key: Key::Home,
                action: KeyAction::MoveToBeginning,
            },
            KeyBinding {
                key: Key::End,
                action: KeyAction::MoveToEnd,
            },
            KeyBinding {
                key: Key::Left,
                action: KeyAction::MoveCursorLeft(1),
            },
            KeyBinding {
                key: Key::Right,
                action: KeyAction::MoveCursorRight(1),
            },
            KeyBinding {
                key: Key::Backspace,
                action: KeyAction::DeleteBackward(1),
            },
            KeyBinding {
                key: Key::Delete,
                action: KeyAction::DeleteForward(1),
            },
            KeyBinding {
                key: Key::Enter,
                action: KeyAction::Execute,
            },
        ]
    }

    /// Build a key binding map from the configuration.
    pub fn build_key_binding_map(&self) -> HashMap<Key, KeyAction> {
        let mut map = HashMap::new();

        // Add default bindings first
        for binding in Self::create_default_key_bindings() {
            map.insert(binding.key, binding.action);
        }

        // Override with custom bindings
        for binding in &self.config.key_bindings {
            map.insert(binding.key, binding.action.clone());
        }

        map
    }

    /// Initialize the REPL components.
    /// 
    /// This method sets up the KeyHandler, Renderer, and EventLoop with the
    /// configured ConsoleInput and ConsoleOutput implementations.
    fn initialize_components(&mut self) -> Result<(), ReplError> {
        // Ensure we have console input and output
        let console_input = self.console_input.take()
            .ok_or_else(|| ReplError::ConfigurationError("ConsoleInput not set".to_string()))?;
        
        let console_output = self.console_output.take()
            .ok_or_else(|| ReplError::ConfigurationError("ConsoleOutput not set".to_string()))?;

        // Create key handler with custom bindings
        let key_bindings = self.config.key_bindings.clone();
        self.key_handler = Some(KeyHandler::new(key_bindings));

        // Create renderer with prompt
        self.renderer = Some(Renderer::new(console_output, self.config.prompt.clone()));

        // Create event loop with console input
        self.event_loop = Some(EventLoop::new(console_input));

        Ok(())
    }

    /// Run the REPL with the main event processing loop.
    /// 
    /// This method starts the REPL and runs until the user exits or an
    /// unrecoverable error occurs. It handles all key events, rendering,
    /// and callback execution.
    /// 
    /// # Returns
    /// 
    /// `Ok(())` when the REPL exits normally, or a `ReplError` if an
    /// unrecoverable error occurs.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use replkit_core::repl::{ReplEngine, ReplConfig};
    /// 
    /// let config = ReplConfig::default();
    /// let mut engine = ReplEngine::new(config).unwrap();
    /// 
    /// // Set console input/output implementations here
    /// // engine.set_console_input(...);
    /// // engine.set_console_output(...);
    /// 
    /// // engine.run().unwrap();
    /// ```
    pub fn run(&mut self) -> Result<(), ReplError> {
        // Initialize components if not already done
        if self.key_handler.is_none() || self.renderer.is_none() || self.event_loop.is_none() {
            self.initialize_components()?;
        }

        // Enable raw mode
        let console_input = self.console_input.as_ref()
            .ok_or_else(|| ReplError::TerminalStateError("ConsoleInput not available".to_string()))?;
        
        let raw_mode_guard = console_input.enable_raw_mode()
            .map_err(|e| ReplError::TerminalStateError(format!("Failed to enable raw mode: {e}")))?;
        
        self.state.raw_mode_guard = Some(raw_mode_guard);

        // Get initial window size
        if let Ok((width, height)) = console_input.get_window_size() {
            self.state.window_size = (width, height);
            if let Some(renderer) = &mut self.renderer {
                renderer.update_window_size(width, height);
            }
        }

        // Start event loop
        if let Some(event_loop) = &mut self.event_loop {
            event_loop.start()
                .map_err(|e| ReplError::EventLoopError(format!("Failed to start event loop: {:?}", e)))?;
        }

        self.state.running = true;

        // Initial render
        if let Some(renderer) = &mut self.renderer {
            renderer.render(&self.buffer)?;
        }

        // Main event processing loop
        while self.state.running && !self.state.should_exit {
            match self.run_once() {
                Ok(Some(input)) => {
                    // User pressed Enter - execute callback
                    if let Err(e) = self.execute_callback(&input) {
                        // Handle callback errors gracefully
                        self.handle_callback_error(e)?;
                    }
                    
                    // Clear buffer and render new prompt
                    self.buffer.set_text(String::new());
                    self.buffer.set_cursor_position(0);
                    
                    if let Some(renderer) = &mut self.renderer {
                        renderer.break_line()?;
                        renderer.render(&self.buffer)?;
                    }
                }
                Ok(None) => {
                    // Continue processing
                }
                Err(e) => {
                    // Try to recover from errors
                    if let Err(recovery_error) = self.handle_error(e) {
                        // Unrecoverable error
                        return Err(recovery_error);
                    }
                }
            }
        }

        // Clean shutdown
        self.shutdown()?;
        Ok(())
    }

    /// Run a single iteration of the REPL event loop.
    /// 
    /// This method processes one event from the event loop and returns
    /// the result. It's useful for integrating the REPL into other event
    /// loops or for testing.
    /// 
    /// # Returns
    /// 
    /// - `Ok(Some(String))` if the user pressed Enter and input should be executed
    /// - `Ok(None)` if processing should continue
    /// - `Err(ReplError)` if an error occurred
    pub fn run_once(&mut self) -> Result<Option<String>, ReplError> {
        if !self.state.running {
            return Err(ReplError::EventLoopError("REPL not running".to_string()));
        }

        let event_loop = self.event_loop.as_mut()
            .ok_or_else(|| ReplError::EventLoopError("EventLoop not initialized".to_string()))?;

        // Get next event (non-blocking)
        match event_loop.next_event()? {
            Some(event) => self.process_event(event),
            None => Ok(None), // No events available
        }
    }

    /// Process a single REPL event.
    fn process_event(&mut self, event: ReplEvent) -> Result<Option<String>, ReplError> {
        match event {
            ReplEvent::KeyPressed(key_event) => {
                self.process_key_event(key_event)
            }
            ReplEvent::WindowResized(width, height) => {
                self.handle_window_resize(width, height)?;
                Ok(None)
            }
            ReplEvent::Shutdown => {
                self.state.should_exit = true;
                Ok(None)
            }
        }
    }

    /// Process a key event and update the buffer state.
    fn process_key_event(&mut self, key_event: KeyEvent) -> Result<Option<String>, ReplError> {
        let key_handler = self.key_handler.as_ref()
            .ok_or_else(|| ReplError::EventLoopError("KeyHandler not initialized".to_string()))?;

        // Process the key event
        let key_result = key_handler.handle_key(key_event, &mut self.buffer)?;

        // Handle the result
        match key_result {
            KeyResult::Continue => {
                // Update display
                if let Some(renderer) = &mut self.renderer {
                    renderer.render(&self.buffer)?;
                }
                Ok(None)
            }
            KeyResult::Execute(input) => {
                // Return input for execution
                Ok(Some(input))
            }
            KeyResult::Exit => {
                self.state.should_exit = true;
                Ok(None)
            }
            KeyResult::ClearLine => {
                // Clear display and render new prompt
                if let Some(renderer) = &mut self.renderer {
                    renderer.clear_line()?;
                    renderer.render(&self.buffer)?;
                }
                Ok(None)
            }
            KeyResult::Ignore => {
                // Do nothing
                Ok(None)
            }
        }
    }

    /// Handle window resize events.
    fn handle_window_resize(&mut self, width: u16, height: u16) -> Result<(), ReplError> {
        self.state.window_size = (width, height);
        
        if let Some(renderer) = &mut self.renderer {
            renderer.update_window_size(width, height);
            // Force re-render with new dimensions
            renderer.force_render(&self.buffer)?;
        }
        
        Ok(())
    }

    /// Execute the user-provided callback function.
    fn execute_callback(&self, input: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        (self.config.executor)(input)
    }

    /// Handle callback execution errors.
    fn handle_callback_error(&mut self, error: Box<dyn Error + Send + Sync>) -> Result<(), ReplError> {
        // For now, we'll just log the error and continue
        // In a real implementation, this might be configurable
        eprintln!("Callback error: {}", error);
        Ok(())
    }

    /// Handle recoverable errors during REPL operation.
    fn handle_error(&mut self, error: ReplError) -> Result<(), ReplError> {
        match error {
            ReplError::ConsoleError(_) => {
                // Try to recover from console errors by re-initializing
                self.attempt_console_recovery()?;
                Ok(())
            }
            ReplError::RenderError(_) => {
                // Try to recover by forcing a re-render
                if let Some(renderer) = &mut self.renderer {
                    renderer.force_render(&self.buffer)?;
                }
                Ok(())
            }
            ReplError::CallbackError(_) => {
                // Callback errors are already handled elsewhere
                Ok(())
            }
            _ => {
                // Other errors are unrecoverable
                Err(error)
            }
        }
    }

    /// Attempt to recover from console I/O errors.
    fn attempt_console_recovery(&mut self) -> Result<(), ReplError> {
        // For now, this is a placeholder
        // A real implementation might try to reinitialize console components
        Err(ReplError::TerminalStateError("Console recovery not implemented".to_string()))
    }

    /// Shutdown the REPL and clean up resources.
    /// 
    /// This method stops the event loop, restores the terminal state,
    /// and cleans up all resources. It should be called when the REPL
    /// is no longer needed.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use replkit_core::repl::{ReplEngine, ReplConfig};
    /// 
    /// let config = ReplConfig::default();
    /// let mut engine = ReplEngine::new(config).unwrap();
    /// 
    /// // Use the engine...
    /// 
    /// engine.shutdown().unwrap();
    /// ```
    pub fn shutdown(&mut self) -> Result<(), ReplError> {
        self.state.running = false;

        // Stop event loop
        if let Some(event_loop) = &mut self.event_loop {
            event_loop.stop()
                .map_err(|e| ReplError::EventLoopError(format!("Failed to stop event loop: {:?}", e)))?;
        }

        // Clear the current line
        if let Some(renderer) = &mut self.renderer {
            let _ = renderer.clear_line(); // Ignore errors during shutdown
        }

        // Restore terminal state by dropping raw mode guard
        self.state.raw_mode_guard = None;

        // Reset cursor visibility
        if let Some(renderer) = &mut self.renderer {
            let _ = renderer.set_cursor_visible(true); // Ignore errors during shutdown
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert_eq!(config.prompt, ">>> ");
        assert!(!config.enable_history);
        assert_eq!(config.max_history_size, 1000);
        assert!(!config.enable_multiline);
        assert!(config.key_bindings.is_empty());
    }

    #[test]
    fn test_repl_engine_new_with_valid_config() {
        let config = ReplConfig::default();
        let result = ReplEngine::new(config);
        assert!(result.is_ok());

        let engine = result.unwrap();
        assert_eq!(engine.config().prompt, ">>> ");
        assert!(!engine.is_running());
        assert!(!engine.should_exit());
        assert_eq!(engine.window_size(), (80, 24));
    }

    #[test]
    fn test_validate_config_empty_prompt() {
        let mut config = ReplConfig::default();
        config.prompt = String::new();

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_err());

        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("Prompt cannot be empty"));
        } else {
            panic!("Expected ConfigurationError for empty prompt");
        }
    }

    #[test]
    fn test_validate_config_prompt_with_control_characters() {
        let mut config = ReplConfig::default();
        config.prompt = ">>> \x1b[31m".to_string(); // Contains escape sequence

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_err());

        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("control characters"));
        } else {
            panic!("Expected ConfigurationError for prompt with control characters");
        }
    }

    #[test]
    fn test_validate_config_prompt_with_tab_allowed() {
        let mut config = ReplConfig::default();
        config.prompt = ">>>\t".to_string(); // Tab should be allowed

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_history_size_zero() {
        let mut config = ReplConfig::default();
        config.enable_history = true;
        config.max_history_size = 0;

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_err());

        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("History size must be greater than 0"));
        } else {
            panic!("Expected ConfigurationError for zero history size");
        }
    }

    #[test]
    fn test_validate_config_history_size_too_large() {
        let mut config = ReplConfig::default();
        config.max_history_size = 200_000;

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_err());

        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("cannot exceed 100,000"));
        } else {
            panic!("Expected ConfigurationError for excessive history size");
        }
    }

    #[test]
    fn test_validate_config_duplicate_key_bindings() {
        let mut config = ReplConfig::default();
        config.key_bindings = vec![
            KeyBinding {
                key: Key::ControlA,
                action: KeyAction::MoveToBeginning,
            },
            KeyBinding {
                key: Key::ControlA, // Duplicate key
                action: KeyAction::MoveToEnd,
            },
        ];

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_err());

        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("Duplicate key binding"));
            assert!(msg.contains("ControlA"));
        } else {
            panic!("Expected ConfigurationError for duplicate key bindings");
        }
    }

    #[test]
    fn test_validate_config_valid_configuration() {
        let mut config = ReplConfig::default();
        config.prompt = "my-prompt> ".to_string();
        config.enable_history = true;
        config.max_history_size = 500;
        config.enable_multiline = true;
        config.key_bindings = vec![
            KeyBinding {
                key: Key::ControlX,
                action: KeyAction::ClearLine,
            },
            KeyBinding {
                key: Key::ControlY,
                action: KeyAction::Exit,
            },
        ];

        let result = ReplEngine::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_default_key_bindings() {
        let bindings = ReplEngine::create_default_key_bindings();

        // Check that we have the expected default bindings
        assert!(!bindings.is_empty());

        // Verify some key bindings exist
        let binding_map: HashMap<Key, &KeyAction> =
            bindings.iter().map(|b| (b.key, &b.action)).collect();

        assert!(binding_map.contains_key(&Key::ControlA));
        assert!(binding_map.contains_key(&Key::ControlE));
        assert!(binding_map.contains_key(&Key::ControlC));
        assert!(binding_map.contains_key(&Key::ControlD));
        assert!(binding_map.contains_key(&Key::Enter));
        assert!(binding_map.contains_key(&Key::Backspace));
        assert!(binding_map.contains_key(&Key::Left));
        assert!(binding_map.contains_key(&Key::Right));

        // Verify specific actions
        if let KeyAction::MoveToBeginning = binding_map[&Key::ControlA] {
            // Expected
        } else {
            panic!("Expected MoveToBeginning for Ctrl+A");
        }

        if let KeyAction::Execute = binding_map[&Key::Enter] {
            // Expected
        } else {
            panic!("Expected Execute for Enter");
        }
    }

    #[test]
    fn test_build_key_binding_map() {
        let mut config = ReplConfig::default();
        config.key_bindings = vec![
            KeyBinding {
                key: Key::ControlA, // Override default
                action: KeyAction::ClearLine,
            },
            KeyBinding {
                key: Key::ControlX, // New binding
                action: KeyAction::Exit,
            },
        ];

        let engine = ReplEngine::new(config).unwrap();
        let binding_map = engine.build_key_binding_map();

        // Check that custom binding overrides default
        if let KeyAction::ClearLine = &binding_map[&Key::ControlA] {
            // Expected - should override default MoveToBeginning
        } else {
            panic!("Expected custom binding to override default");
        }

        // Check that new binding is present
        assert!(binding_map.contains_key(&Key::ControlX));
        if let KeyAction::Exit = &binding_map[&Key::ControlX] {
            // Expected
        } else {
            panic!("Expected custom Exit binding for Ctrl+X");
        }

        // Check that other defaults are still present
        assert!(binding_map.contains_key(&Key::ControlE));
        assert!(binding_map.contains_key(&Key::Enter));
    }

    #[test]
    fn test_repl_error_display() {
        let console_error = ConsoleError::IoError("test error".to_string());
        let repl_error = ReplError::from(console_error);
        assert!(repl_error.to_string().contains("Console error"));
        assert!(repl_error.to_string().contains("test error"));

        let config_error = ReplError::ConfigurationError("invalid config".to_string());
        assert!(config_error.to_string().contains("Configuration error"));
        assert!(config_error.to_string().contains("invalid config"));
    }

    #[test]
    fn test_repl_state_new() {
        let state = ReplState::new();
        assert!(!state.running);
        assert!(!state.should_exit);
        assert!(state.raw_mode_guard.is_none());
        assert_eq!(state.window_size, (80, 24));
        assert_eq!(state.last_render_hash, 0);
    }

    #[test]
    fn test_key_action_clone() {
        let action1 = KeyAction::MoveCursorLeft(5);
        let action2 = action1.clone();

        if let (KeyAction::MoveCursorLeft(n1), KeyAction::MoveCursorLeft(n2)) = (action1, action2) {
            assert_eq!(n1, n2);
        } else {
            panic!("KeyAction clone failed");
        }
    }

    #[test]
    fn test_key_binding_clone() {
        let binding1 = KeyBinding {
            key: Key::ControlA,
            action: KeyAction::MoveToBeginning,
        };
        let binding2 = binding1.clone();

        assert_eq!(binding1.key, binding2.key);
        // KeyAction clone works but we can't directly compare the actions
        // since they don't implement PartialEq
    }

    #[test]
    fn test_key_action_debug() {
        let action = KeyAction::MoveCursorLeft(3);
        let debug_str = format!("{:?}", action);
        assert_eq!(debug_str, "MoveCursorLeft(3)");

        let custom_action = KeyAction::Custom(Box::new(|_| Ok(())));
        let debug_str = format!("{:?}", custom_action);
        assert_eq!(debug_str, "Custom(<function>)");
    }

    #[test]
    fn test_key_binding_debug() {
        let binding = KeyBinding {
            key: Key::ControlA,
            action: KeyAction::MoveToBeginning,
        };
        let debug_str = format!("{:?}", binding);
        assert!(debug_str.contains("ControlA"));
        assert!(debug_str.contains("MoveToBeginning"));
    }

    #[test]
    fn test_engine_accessors() {
        let config = ReplConfig::default();
        let mut engine = ReplEngine::new(config).unwrap();

        // Test config accessor
        assert_eq!(engine.config().prompt, ">>> ");

        // Test buffer accessors
        let buffer_ref = engine.buffer();
        assert_eq!(buffer_ref.text(), "");

        let buffer_mut = engine.buffer_mut();
        buffer_mut.insert_text("test", false, true);
        assert_eq!(engine.buffer().text(), "test");

        // Test window size
        engine.set_window_size(120, 30);
        assert_eq!(engine.window_size(), (120, 30));
    }

    // Integration tests for complete REPL workflow
    mod integration_tests {
        use super::*;
        use crate::console::{AsAny, BackendType, ConsoleCapabilities, ConsoleResult, OutputCapabilities, TextStyle, ClearType};
        use crate::key::{Key, KeyEvent};
        use std::sync::{Arc, Mutex};

        // Mock ConsoleInput for integration testing
        struct MockConsoleInput {
            running: Arc<Mutex<bool>>,
            key_callback: Arc<Mutex<Option<Box<dyn FnMut(KeyEvent) + Send>>>>,
            resize_callback: Arc<Mutex<Option<Box<dyn FnMut(u16, u16) + Send>>>>,
            capabilities: ConsoleCapabilities,
            raw_mode_enabled: Arc<Mutex<bool>>,
        }

        impl MockConsoleInput {
            fn new() -> Self {
                MockConsoleInput {
                    running: Arc::new(Mutex::new(false)),
                    key_callback: Arc::new(Mutex::new(None)),
                    resize_callback: Arc::new(Mutex::new(None)),
                    capabilities: ConsoleCapabilities {
                        supports_raw_mode: true,
                        supports_resize_events: true,
                        supports_bracketed_paste: false,
                        supports_mouse_events: false,
                        supports_unicode: true,
                        platform_name: "Mock".to_string(),
                        backend_type: BackendType::Mock,
                    },
                    raw_mode_enabled: Arc::new(Mutex::new(false)),
                }
            }

            fn simulate_key_press(&self, key: Key) {
                let event = KeyEvent::simple(key, vec![]);
                if let Ok(mut callback_opt) = self.key_callback.lock() {
                    if let Some(callback) = callback_opt.as_mut() {
                        callback(event);
                    }
                }
            }

            fn simulate_text_input(&self, text: &str) {
                let event = KeyEvent::with_text(Key::NotDefined, vec![], text.to_string());
                if let Ok(mut callback_opt) = self.key_callback.lock() {
                    if let Some(callback) = callback_opt.as_mut() {
                        callback(event);
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
                *self.raw_mode_enabled.lock().unwrap() = true;
                Ok(RawModeGuard::new(
                    || {},
                    "Mock".to_string(),
                ))
            }

            fn get_window_size(&self) -> Result<(u16, u16), ConsoleError> {
                Ok((80, 24))
            }

            fn start_event_loop(&self) -> Result<(), ConsoleError> {
                *self.running.lock().unwrap() = true;
                Ok(())
            }

            fn stop_event_loop(&self) -> Result<(), ConsoleError> {
                *self.running.lock().unwrap() = false;
                Ok(())
            }

            fn on_window_resize(&self, callback: Box<dyn FnMut(u16, u16) + Send>) {
                if let Ok(mut callback_opt) = self.resize_callback.lock() {
                    *callback_opt = Some(callback);
                }
            }

            fn on_key_pressed(&self, callback: Box<dyn FnMut(KeyEvent) + Send>) {
                if let Ok(mut callback_opt) = self.key_callback.lock() {
                    *callback_opt = Some(callback);
                }
            }

            fn is_running(&self) -> bool {
                *self.running.lock().unwrap()
            }

            fn get_capabilities(&self) -> ConsoleCapabilities {
                self.capabilities.clone()
            }
        }

        // Mock ConsoleOutput for integration testing
        struct MockConsoleOutput {
            operations: Arc<Mutex<Vec<String>>>,
            cursor_pos: Arc<Mutex<(u16, u16)>>,
            cursor_visible: Arc<Mutex<bool>>,
        }

        impl MockConsoleOutput {
            fn new() -> Self {
                MockConsoleOutput {
                    operations: Arc::new(Mutex::new(Vec::new())),
                    cursor_pos: Arc::new(Mutex::new((0, 0))),
                    cursor_visible: Arc::new(Mutex::new(true)),
                }
            }

            fn get_operations(&self) -> Vec<String> {
                self.operations.lock().unwrap().clone()
            }

            fn clear_operations(&self) {
                self.operations.lock().unwrap().clear();
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
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("write_text: {}", text));
                Ok(())
            }

            fn write_styled_text(&self, text: &str, _style: &TextStyle) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("write_styled_text: {}", text));
                Ok(())
            }

            fn write_safe_text(&self, text: &str) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("write_safe_text: {}", text));
                Ok(())
            }

            fn move_cursor_to(&self, row: u16, col: u16) -> ConsoleResult<()> {
                *self.cursor_pos.lock().unwrap() = (row, col);
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("move_cursor_to: ({}, {})", row, col));
                Ok(())
            }

            fn move_cursor_relative(&self, row_delta: i16, col_delta: i16) -> ConsoleResult<()> {
                self.operations.lock().unwrap().push(format!(
                    "move_cursor_relative: ({}, {})",
                    row_delta, col_delta
                ));
                Ok(())
            }

            fn clear(&self, clear_type: ClearType) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("clear: {:?}", clear_type));
                Ok(())
            }

            fn set_style(&self, _style: &TextStyle) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push("set_style".to_string());
                Ok(())
            }

            fn reset_style(&self) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push("reset_style".to_string());
                Ok(())
            }

            fn flush(&self) -> ConsoleResult<()> {
                self.operations.lock().unwrap().push("flush".to_string());
                Ok(())
            }

            fn set_alternate_screen(&self, _enabled: bool) -> ConsoleResult<()> {
                self.operations
                    .lock()
                    .unwrap()
                    .push("set_alternate_screen".to_string());
                Ok(())
            }

            fn set_cursor_visible(&self, visible: bool) -> ConsoleResult<()> {
                *self.cursor_visible.lock().unwrap() = visible;
                self.operations
                    .lock()
                    .unwrap()
                    .push(format!("set_cursor_visible: {}", visible));
                Ok(())
            }

            fn get_cursor_position(&self) -> ConsoleResult<(u16, u16)> {
                Ok(*self.cursor_pos.lock().unwrap())
            }

            fn get_capabilities(&self) -> OutputCapabilities {
                OutputCapabilities {
                    supports_colors: true,
                    supports_true_color: true,
                    supports_styling: true,
                    supports_alternate_screen: true,
                    supports_cursor_control: true,
                    max_colors: 256,
                    platform_name: "mock".to_string(),
                    backend_type: BackendType::Mock,
                }
            }
        }

        #[test]
        fn test_repl_engine_initialization() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);

            // Test that initialization works
            let result = engine.initialize_components();
            assert!(result.is_ok());
        }

        #[test]
        fn test_repl_engine_initialization_missing_input() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let output = Box::new(MockConsoleOutput::new());
            engine.set_console_output(output);

            // Should fail without console input
            let result = engine.initialize_components();
            assert!(result.is_err());
            
            if let Err(ReplError::ConfigurationError(msg)) = result {
                assert!(msg.contains("ConsoleInput not set"));
            } else {
                panic!("Expected ConfigurationError for missing ConsoleInput");
            }
        }

        #[test]
        fn test_repl_engine_initialization_missing_output() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            engine.set_console_input(input);

            // Should fail without console output
            let result = engine.initialize_components();
            assert!(result.is_err());
            
            if let Err(ReplError::ConfigurationError(msg)) = result {
                assert!(msg.contains("ConsoleOutput not set"));
            } else {
                panic!("Expected ConfigurationError for missing ConsoleOutput");
            }
        }

        #[test]
        fn test_repl_engine_run_once_not_running() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            // Should fail when not running
            let result = engine.run_once();
            assert!(result.is_err());
            
            if let Err(ReplError::EventLoopError(msg)) = result {
                assert!(msg.contains("REPL not running"));
            } else {
                panic!("Expected EventLoopError for not running");
            }
        }

        #[test]
        fn test_repl_engine_shutdown() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);

            // Initialize components
            engine.initialize_components().unwrap();
            
            // Set running state and start event loop
            engine.state.running = true;
            if let Some(event_loop) = &mut engine.event_loop {
                let _ = event_loop.start(); // Ignore errors in test
            }

            // Test shutdown
            let result = engine.shutdown();
            assert!(result.is_ok());
            assert!(!engine.state.running);
        }

        #[test]
        fn test_repl_engine_handle_window_resize() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Test window resize handling
            let result = engine.handle_window_resize(120, 30);
            assert!(result.is_ok());
            assert_eq!(engine.window_size(), (120, 30));
        }

        #[test]
        fn test_repl_engine_execute_callback() {
            let executed_input = Arc::new(Mutex::new(String::new()));
            let executed_input_clone = Arc::clone(&executed_input);

            let config = ReplConfig {
                prompt: "test> ".to_string(),
                executor: Box::new(move |input| {
                    *executed_input_clone.lock().unwrap() = input.to_string();
                    Ok(())
                }),
                ..Default::default()
            };

            let engine = ReplEngine::new(config).unwrap();

            // Test callback execution
            let result = engine.execute_callback("hello world");
            assert!(result.is_ok());

            let executed = executed_input.lock().unwrap();
            assert_eq!(*executed, "hello world");
        }

        #[test]
        fn test_repl_engine_execute_callback_error() {
            let config = ReplConfig {
                prompt: "test> ".to_string(),
                executor: Box::new(|_input| {
                    Err("Test error".into())
                }),
                ..Default::default()
            };

            let engine = ReplEngine::new(config).unwrap();

            // Test callback error handling
            let result = engine.execute_callback("test");
            assert!(result.is_err());
        }

        #[test]
        fn test_repl_engine_handle_callback_error() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let error: Box<dyn Error + Send + Sync> = "Test error".into();
            
            // Should handle callback errors gracefully
            let result = engine.handle_callback_error(error);
            assert!(result.is_ok());
        }

        #[test]
        fn test_repl_engine_process_key_event_continue() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Test processing a regular character
            let key_event = KeyEvent::with_text(Key::NotDefined, vec![b'a'], "a".to_string());
            let result = engine.process_key_event(key_event);
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None); // Should continue
            assert_eq!(engine.buffer().text(), "a");
        }

        #[test]
        fn test_repl_engine_process_key_event_execute() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Add some text to buffer
            engine.buffer_mut().insert_text("hello", false, true);

            // Test processing Enter key
            let key_event = KeyEvent::simple(Key::Enter, vec![0x0d]);
            let result = engine.process_key_event(key_event);
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("hello".to_string()));
        }

        #[test]
        fn test_repl_engine_process_key_event_exit() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Test processing Ctrl+D (exit)
            let key_event = KeyEvent::simple(Key::ControlD, vec![0x04]);
            let result = engine.process_key_event(key_event);
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
            assert!(engine.should_exit());
        }

        #[test]
        fn test_repl_engine_process_key_event_clear_line() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Add some text to buffer
            engine.buffer_mut().insert_text("hello", false, true);

            // Test processing Ctrl+C (clear line)
            let key_event = KeyEvent::simple(Key::ControlC, vec![0x03]);
            let result = engine.process_key_event(key_event);
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
            assert_eq!(engine.buffer().text(), "");
        }

        #[test]
        fn test_repl_engine_process_event_shutdown() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Test processing shutdown event
            let result = engine.process_event(ReplEvent::Shutdown);
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
            assert!(engine.should_exit());
        }

        #[test]
        fn test_repl_engine_process_event_window_resize() {
            let config = ReplConfig::default();
            let mut engine = ReplEngine::new(config).unwrap();

            let input = Box::new(MockConsoleInput::new());
            let output = Box::new(MockConsoleOutput::new());

            engine.set_console_input(input);
            engine.set_console_output(output);
            engine.initialize_components().unwrap();

            // Test processing window resize event
            let result = engine.process_event(ReplEvent::WindowResized(120, 30));
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
            assert_eq!(engine.window_size(), (120, 30));
        }
    }
}
