//! REPL (Read-Eval-Print Loop) engine and configuration.
//!
//! This module provides the core REPL engine that integrates ConsoleInput, KeyParser,
//! Buffer, and ConsoleOutput components to create a working interactive prompt system.

use crate::{
    console::{ConsoleError, ConsoleInput, ConsoleOutput, RawModeGuard},
    key::Key,
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
}
