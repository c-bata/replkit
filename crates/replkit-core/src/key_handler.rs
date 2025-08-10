//! Key event handling and processing for REPL operations.
//!
//! This module provides the KeyHandler struct that processes key events and translates
//! them into buffer operations or system actions. It supports both default key bindings
//! for common editing operations and custom key binding registration.

use crate::{
    buffer::Buffer,
    key::{Key, KeyEvent},
    repl::{KeyAction, KeyBinding, ReplError},
};
use std::collections::HashMap;

/// Result of processing a key event.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyResult {
    /// Continue normal REPL operation
    Continue,
    /// Execute the current input (user pressed Enter)
    Execute(String),
    /// Exit the REPL (user pressed Ctrl+D or exit key)
    Exit,
    /// Clear the current line (user pressed Ctrl+C)
    ClearLine,
    /// Ignore this key event (no action needed)
    Ignore,
}

/// Handles key events and translates them into buffer operations or system actions.
pub struct KeyHandler {
    /// Custom key bindings that override defaults
    custom_bindings: HashMap<Key, KeyAction>,
    /// Default key bindings for common operations
    default_bindings: HashMap<Key, KeyAction>,
}

impl KeyHandler {
    /// Create a new KeyHandler with the specified custom key bindings.
    ///
    /// # Arguments
    ///
    /// * `custom_bindings` - Vector of custom key bindings that will override defaults
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::key_handler::KeyHandler;
    /// use replkit_core::repl::{KeyBinding, KeyAction};
    /// use replkit_core::key::Key;
    ///
    /// let custom_bindings = vec![
    ///     KeyBinding {
    ///         key: Key::ControlX,
    ///         action: KeyAction::Exit,
    ///     }
    /// ];
    ///
    /// let handler = KeyHandler::new(custom_bindings);
    /// ```
    pub fn new(custom_bindings: Vec<KeyBinding>) -> Self {
        let mut custom_map = HashMap::new();
        for binding in custom_bindings {
            custom_map.insert(binding.key, binding.action);
        }

        let default_map = Self::create_default_bindings();

        KeyHandler {
            custom_bindings: custom_map,
            default_bindings: default_map,
        }
    }

    /// Create default key bindings for basic editing operations.
    fn create_default_bindings() -> HashMap<Key, KeyAction> {
        let mut bindings = HashMap::new();

        // Navigation keys
        bindings.insert(Key::Left, KeyAction::MoveCursorLeft(1));
        bindings.insert(Key::Right, KeyAction::MoveCursorRight(1));
        bindings.insert(Key::Home, KeyAction::MoveToBeginning);
        bindings.insert(Key::End, KeyAction::MoveToEnd);

        // Editing keys
        bindings.insert(Key::Backspace, KeyAction::DeleteBackward(1));
        bindings.insert(Key::Delete, KeyAction::DeleteForward(1));

        // Control key combinations
        bindings.insert(Key::ControlA, KeyAction::MoveToBeginning);
        bindings.insert(Key::ControlE, KeyAction::MoveToEnd);
        bindings.insert(Key::ControlC, KeyAction::ClearLine);
        bindings.insert(Key::ControlD, KeyAction::Exit);

        // Action keys
        bindings.insert(Key::Enter, KeyAction::Execute);

        bindings
    }

    /// Process a key event and return the appropriate result.
    ///
    /// This method first checks for custom key bindings, then falls back to default
    /// bindings. If no binding is found, it handles printable characters by inserting
    /// them into the buffer.
    ///
    /// # Arguments
    ///
    /// * `key_event` - The key event to process
    /// * `buffer` - Mutable reference to the text buffer
    ///
    /// # Returns
    ///
    /// A `KeyResult` indicating what action should be taken
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::key_handler::{KeyHandler, KeyResult};
    /// use replkit_core::key::{Key, KeyEvent};
    /// use replkit_core::buffer::Buffer;
    ///
    /// let handler = KeyHandler::new(vec![]);
    /// let mut buffer = Buffer::new();
    /// let key_event = KeyEvent::simple(Key::ControlA, vec![0x01]);
    ///
    /// let result = handler.handle_key(key_event, &mut buffer).unwrap();
    /// assert_eq!(result, KeyResult::Continue);
    /// assert_eq!(buffer.cursor_position(), 0); // Moved to beginning
    /// ```
    pub fn handle_key(
        &self,
        key_event: KeyEvent,
        buffer: &mut Buffer,
    ) -> Result<KeyResult, ReplError> {
        // Update buffer with the last key stroke for context-aware operations
        buffer.set_last_key_stroke(key_event.key);

        // Check for custom bindings first
        if let Some(action) = self.custom_bindings.get(&key_event.key) {
            return self.execute_action(action, buffer, &key_event);
        }

        // Check for default bindings
        if let Some(action) = self.default_bindings.get(&key_event.key) {
            return self.execute_action(action, buffer, &key_event);
        }

        // Handle printable characters and special cases
        match key_event.key {
            // Handle special sequences that should be ignored
            Key::CPRResponse | Key::Vt100MouseEvent | Key::WindowsMouseEvent => {
                Ok(KeyResult::Ignore)
            }

            // Handle bracketed paste content
            Key::BracketedPaste => {
                if let Some(text) = &key_event.text {
                    buffer.insert_text(text, false, true);
                }
                Ok(KeyResult::Continue)
            }

            // Handle keys that should be ignored only if they don't have text content
            Key::Ignore => Ok(KeyResult::Ignore),

            // Handle any key (including NotDefined) with text content as printable character
            _ => {
                if let Some(text) = &key_event.text {
                    if !text.is_empty() {
                        buffer.insert_text(text, false, true);
                        return Ok(KeyResult::Continue);
                    }
                }

                // If no text content and no binding, ignore the key
                Ok(KeyResult::Ignore)
            }
        }
    }

    /// Execute a key action on the buffer.
    fn execute_action(
        &self,
        action: &KeyAction,
        buffer: &mut Buffer,
        _key_event: &KeyEvent,
    ) -> Result<KeyResult, ReplError> {
        match action {
            KeyAction::MoveCursorLeft(count) => {
                buffer.cursor_left(*count);
                Ok(KeyResult::Continue)
            }
            KeyAction::MoveCursorRight(count) => {
                buffer.cursor_right(*count);
                Ok(KeyResult::Continue)
            }
            KeyAction::DeleteBackward(count) => {
                buffer.delete_before_cursor(*count);
                Ok(KeyResult::Continue)
            }
            KeyAction::DeleteForward(count) => {
                buffer.delete(*count);
                Ok(KeyResult::Continue)
            }
            KeyAction::MoveToBeginning => {
                buffer.set_cursor_position(0);
                Ok(KeyResult::Continue)
            }
            KeyAction::MoveToEnd => {
                let text_len = crate::unicode::rune_count(buffer.text());
                buffer.set_cursor_position(text_len);
                Ok(KeyResult::Continue)
            }
            KeyAction::ClearLine => {
                buffer.set_text(String::new());
                buffer.set_cursor_position(0);
                Ok(KeyResult::ClearLine)
            }
            KeyAction::Execute => {
                let text = buffer.text().to_string();
                Ok(KeyResult::Execute(text))
            }
            KeyAction::Exit => Ok(KeyResult::Exit),
            KeyAction::Custom(func) => {
                func(buffer)?;
                Ok(KeyResult::Continue)
            }
        }
    }

    /// Register a custom key binding.
    ///
    /// This will override any existing binding for the specified key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to bind
    /// * `action` - The action to perform when the key is pressed
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::key_handler::KeyHandler;
    /// use replkit_core::repl::KeyAction;
    /// use replkit_core::key::Key;
    ///
    /// let mut handler = KeyHandler::new(vec![]);
    /// handler.register_binding(Key::ControlX, KeyAction::Exit);
    /// ```
    pub fn register_binding(&mut self, key: Key, action: KeyAction) {
        self.custom_bindings.insert(key, action);
    }

    /// Remove a custom key binding.
    ///
    /// This will remove the custom binding for the specified key, allowing the
    /// default binding (if any) to take effect.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to unbind
    ///
    /// # Returns
    ///
    /// `true` if a binding was removed, `false` if no binding existed
    pub fn remove_binding(&mut self, key: Key) -> bool {
        self.custom_bindings.remove(&key).is_some()
    }

    /// Get the action for a specific key.
    ///
    /// This checks custom bindings first, then default bindings.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// The action for the key, or `None` if no binding exists
    pub fn get_action(&self, key: Key) -> Option<&KeyAction> {
        self.custom_bindings
            .get(&key)
            .or_else(|| self.default_bindings.get(&key))
    }

    /// Check if a key has a binding (custom or default).
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// `true` if the key has a binding, `false` otherwise
    pub fn has_binding(&self, key: Key) -> bool {
        self.custom_bindings.contains_key(&key) || self.default_bindings.contains_key(&key)
    }

    /// Get all custom key bindings.
    ///
    /// # Returns
    ///
    /// A reference to the custom bindings map
    pub fn custom_bindings(&self) -> &HashMap<Key, KeyAction> {
        &self.custom_bindings
    }

    /// Get all default key bindings.
    ///
    /// # Returns
    ///
    /// A reference to the default bindings map
    pub fn default_bindings(&self) -> &HashMap<Key, KeyAction> {
        &self.default_bindings
    }
}

impl Default for KeyHandler {
    /// Create a KeyHandler with no custom bindings (only defaults).
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::key::{Key, KeyEvent};
    use crate::repl::{KeyAction, KeyBinding};

    #[test]
    fn test_key_handler_new() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlX,
            action: KeyAction::Exit,
        }];

        let handler = KeyHandler::new(custom_bindings);
        assert!(handler.has_binding(Key::ControlX));
        assert!(handler.has_binding(Key::ControlA)); // Default binding
    }

    #[test]
    fn test_key_handler_default() {
        let handler = KeyHandler::default();

        // Should have default bindings
        assert!(handler.has_binding(Key::ControlA));
        assert!(handler.has_binding(Key::ControlE));
        assert!(handler.has_binding(Key::Left));
        assert!(handler.has_binding(Key::Right));
        assert!(handler.has_binding(Key::Backspace));
        assert!(handler.has_binding(Key::Delete));
        assert!(handler.has_binding(Key::Enter));

        // Should not have custom bindings
        assert!(handler.custom_bindings().is_empty());
    }

    #[test]
    fn test_handle_key_move_cursor_left() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(3);

        let key_event = KeyEvent::simple(Key::Left, vec![0x1b, 0x5b, 0x44]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_handle_key_move_cursor_right() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(2);

        let key_event = KeyEvent::simple(Key::Right, vec![0x1b, 0x5b, 0x43]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 3);
    }

    #[test]
    fn test_handle_key_move_to_beginning() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::simple(Key::ControlA, vec![0x01]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_handle_key_move_to_end() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::simple(Key::ControlE, vec![0x05]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 11);
    }

    #[test]
    fn test_handle_key_home_end() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        // Test Home key
        let key_event = KeyEvent::simple(Key::Home, vec![0x1b, 0x5b, 0x48]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();
        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 0);

        // Test End key
        let key_event = KeyEvent::simple(Key::End, vec![0x1b, 0x5b, 0x46]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();
        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 11);
    }

    #[test]
    fn test_handle_key_backspace() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(3);

        let key_event = KeyEvent::simple(Key::Backspace, vec![0x08]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "helo");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_handle_key_delete() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(2);

        let key_event = KeyEvent::simple(Key::Delete, vec![0x1b, 0x5b, 0x33, 0x7e]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "helo");
        assert_eq!(buffer.cursor_position(), 2);
    }

    #[test]
    fn test_handle_key_clear_line() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::simple(Key::ControlC, vec![0x03]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::ClearLine);
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_handle_key_execute() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());

        let key_event = KeyEvent::simple(Key::Enter, vec![0x0d]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Execute("hello world".to_string()));
    }

    #[test]
    fn test_handle_key_exit() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();

        let key_event = KeyEvent::simple(Key::ControlD, vec![0x04]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Exit);
    }

    #[test]
    fn test_handle_key_printable_character() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::with_text(Key::NotDefined, vec![0x20], " ".to_string());
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "hello ");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_handle_key_bracketed_paste() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(5);

        let key_event =
            KeyEvent::with_text(Key::BracketedPaste, vec![], " pasted text".to_string());
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "hello pasted text");
        assert_eq!(buffer.cursor_position(), 17);
    }

    #[test]
    fn test_handle_key_ignore_special_keys() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();

        // Test various keys that should be ignored
        let ignore_keys = vec![
            Key::Ignore,
            Key::CPRResponse,
            Key::Vt100MouseEvent,
            Key::WindowsMouseEvent,
        ];

        for key in ignore_keys {
            let key_event = KeyEvent::simple(key, vec![]);
            let result = handler.handle_key(key_event, &mut buffer).unwrap();
            assert_eq!(result, KeyResult::Ignore);
        }
    }

    #[test]
    fn test_handle_key_not_defined_without_text() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();

        // NotDefined without text should be ignored
        let key_event = KeyEvent::simple(Key::NotDefined, vec![]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();
        assert_eq!(result, KeyResult::Ignore);
    }

    #[test]
    fn test_custom_bindings_override_defaults() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlA, // Override default MoveToBeginning
            action: KeyAction::ClearLine,
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::simple(Key::ControlA, vec![0x01]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        // Should execute custom action (ClearLine) instead of default (MoveToBeginning)
        assert_eq!(result, KeyResult::ClearLine);
        assert_eq!(buffer.text(), "");
    }

    #[test]
    fn test_register_binding() {
        let mut handler = KeyHandler::default();

        // Initially should not have binding for ControlX
        assert!(!handler.has_binding(Key::ControlX));

        // Register custom binding
        handler.register_binding(Key::ControlX, KeyAction::Exit);

        // Now should have the binding
        assert!(handler.has_binding(Key::ControlX));

        // Test the binding works
        let mut buffer = Buffer::new();
        let key_event = KeyEvent::simple(Key::ControlX, vec![0x18]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();
        assert_eq!(result, KeyResult::Exit);
    }

    #[test]
    fn test_remove_binding() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlX,
            action: KeyAction::Exit,
        }];

        let mut handler = KeyHandler::new(custom_bindings);

        // Should have the custom binding
        assert!(handler.has_binding(Key::ControlX));

        // Remove the binding
        let removed = handler.remove_binding(Key::ControlX);
        assert!(removed);

        // Should no longer have the binding
        assert!(!handler.has_binding(Key::ControlX));

        // Removing non-existent binding should return false
        let removed = handler.remove_binding(Key::ControlY);
        assert!(!removed);
    }

    #[test]
    fn test_get_action() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlX,
            action: KeyAction::Exit,
        }];

        let handler = KeyHandler::new(custom_bindings);

        // Test custom binding
        let action = handler.get_action(Key::ControlX);
        assert!(action.is_some());
        if let Some(KeyAction::Exit) = action {
            // Expected
        } else {
            panic!("Expected Exit action for ControlX");
        }

        // Test default binding
        let action = handler.get_action(Key::ControlA);
        assert!(action.is_some());
        if let Some(KeyAction::MoveToBeginning) = action {
            // Expected
        } else {
            panic!("Expected MoveToBeginning action for ControlA");
        }

        // Test non-existent binding
        let action = handler.get_action(Key::ControlY);
        assert!(action.is_none());
    }

    #[test]
    fn test_custom_action_function() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlX,
            action: KeyAction::Custom(Box::new(|buffer| {
                buffer.insert_text("CUSTOM", false, true);
                Ok(())
            })),
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();
        buffer.set_text("hello".to_string());
        buffer.set_cursor_position(5);

        let key_event = KeyEvent::simple(Key::ControlX, vec![0x18]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "helloCUSTOM");
        assert_eq!(buffer.cursor_position(), 11);
    }

    #[test]
    fn test_custom_action_function_error() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlX,
            action: KeyAction::Custom(Box::new(|_buffer| {
                Err(ReplError::CallbackError("Custom error".to_string()))
            })),
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();

        let key_event = KeyEvent::simple(Key::ControlX, vec![0x18]);
        let result = handler.handle_key(key_event, &mut buffer);

        assert!(result.is_err());
        if let Err(ReplError::CallbackError(msg)) = result {
            assert_eq!(msg, "Custom error");
        } else {
            panic!("Expected CallbackError");
        }
    }

    #[test]
    fn test_key_result_debug() {
        assert_eq!(format!("{:?}", KeyResult::Continue), "Continue");
        assert_eq!(format!("{:?}", KeyResult::Exit), "Exit");
        assert_eq!(format!("{:?}", KeyResult::ClearLine), "ClearLine");
        assert_eq!(format!("{:?}", KeyResult::Ignore), "Ignore");
        assert_eq!(
            format!("{:?}", KeyResult::Execute("test".to_string())),
            "Execute(\"test\")"
        );
    }

    #[test]
    fn test_key_result_clone() {
        let result1 = KeyResult::Execute("test".to_string());
        let result2 = result1.clone();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_buffer_last_key_stroke_updated() {
        let handler = KeyHandler::default();
        let mut buffer = Buffer::new();

        let key_event = KeyEvent::simple(Key::ControlA, vec![0x01]);
        let _result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(buffer.last_key_stroke(), Some(Key::ControlA));
    }

    #[test]
    fn test_multiple_character_deletion() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlW,
            action: KeyAction::DeleteBackward(5), // Delete 5 characters
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(11);

        let key_event = KeyEvent::simple(Key::ControlW, vec![0x17]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), "hello ");
        assert_eq!(buffer.cursor_position(), 6);
    }

    #[test]
    fn test_multiple_character_forward_deletion() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlK,
            action: KeyAction::DeleteForward(5), // Delete 5 characters forward
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(0);

        let key_event = KeyEvent::simple(Key::ControlK, vec![0x0b]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.text(), " world");
        assert_eq!(buffer.cursor_position(), 0);
    }

    #[test]
    fn test_multiple_cursor_movement() {
        let custom_bindings = vec![KeyBinding {
            key: Key::ControlF,
            action: KeyAction::MoveCursorRight(3), // Move 3 characters right
        }];

        let handler = KeyHandler::new(custom_bindings);
        let mut buffer = Buffer::new();
        buffer.set_text("hello world".to_string());
        buffer.set_cursor_position(2);

        let key_event = KeyEvent::simple(Key::ControlF, vec![0x06]);
        let result = handler.handle_key(key_event, &mut buffer).unwrap();

        assert_eq!(result, KeyResult::Continue);
        assert_eq!(buffer.cursor_position(), 5);
    }
}
