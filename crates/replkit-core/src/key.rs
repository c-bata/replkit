//! Key definitions and key event structures for terminal input parsing.
//!
//! This module provides comprehensive key type definitions matching go-prompt's structure,
//! along with the KeyEvent struct that represents parsed key input events.

/// Key represents all possible key inputs that can be parsed from terminal input.
/// This enum matches the structure and naming from go-prompt for compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Escape key
    Escape,

    // Control characters (Ctrl+A through Ctrl+Z)
    ControlA,
    ControlB,
    ControlC,
    ControlD,
    ControlE,
    ControlF,
    ControlG,
    ControlH,
    ControlI,
    ControlJ,
    ControlK,
    ControlL,
    ControlM,
    ControlN,
    ControlO,
    ControlP,
    ControlQ,
    ControlR,
    ControlS,
    ControlT,
    ControlU,
    ControlV,
    ControlW,
    ControlX,
    ControlY,
    ControlZ,

    // Additional control combinations
    ControlSpace,
    ControlBackslash,
    ControlSquareClose,
    ControlCircumflex,
    ControlUnderscore,
    ControlLeft,
    ControlRight,
    ControlUp,
    ControlDown,

    // Navigation keys (arrow keys)
    Up,
    Down,
    Right,
    Left,

    // Shift + arrow key combinations
    ShiftLeft,
    ShiftUp,
    ShiftDown,
    ShiftRight,

    // Navigation and editing keys
    Home,
    End,
    Delete,
    ShiftDelete,
    ControlDelete,
    PageUp,
    PageDown,
    BackTab,
    Insert,
    Backspace,

    // Aliases for common keys
    Tab,
    Enter,

    // Function keys F1-F24
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Special matching key
    /// Matches any key (used for key binding patterns)
    Any,

    // Special sequences
    /// Cursor Position Report response
    CPRResponse,
    /// VT100-style mouse event
    Vt100MouseEvent,
    /// Windows-style mouse event
    WindowsMouseEvent,
    /// Bracketed paste mode content
    BracketedPaste,

    // Meta keys
    /// Key which should be ignored (no action should be taken)
    Ignore,
    /// Key is not defined or unknown sequence
    NotDefined,
}

/// KeyEvent represents a parsed key input event with associated metadata.
/// This struct contains the parsed key, the raw bytes that produced it,
/// and any associated text content.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyEvent {
    /// The parsed key type
    pub key: Key,
    /// The raw bytes that were parsed to produce this key event
    pub raw_bytes: Vec<u8>,
    /// Optional text content associated with this key event
    /// (e.g., for printable characters or bracketed paste content)
    pub text: Option<String>,
}

impl KeyEvent {
    /// Create a new KeyEvent with the specified key, raw bytes, and optional text
    pub fn new(key: Key, raw_bytes: Vec<u8>, text: Option<String>) -> Self {
        Self {
            key,
            raw_bytes,
            text,
        }
    }

    /// Create a KeyEvent for a simple key without text content
    pub fn simple(key: Key, raw_bytes: Vec<u8>) -> Self {
        Self::new(key, raw_bytes, None)
    }

    /// Create a KeyEvent with text content (e.g., for printable characters)
    pub fn with_text(key: Key, raw_bytes: Vec<u8>, text: String) -> Self {
        Self::new(key, raw_bytes, Some(text))
    }

    /// Check if this key event has associated text content
    pub fn has_text(&self) -> bool {
        self.text.is_some()
    }

    /// Get the text content, returning an empty string if none exists
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }
}

impl Default for KeyEvent {
    fn default() -> Self {
        Self {
            key: Key::NotDefined,
            raw_bytes: Vec::new(),
            text: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_creation() {
        let event = KeyEvent::new(Key::ControlC, vec![0x03], None);
        assert_eq!(event.key, Key::ControlC);
        assert_eq!(event.raw_bytes, vec![0x03]);
        assert_eq!(event.text, None);
    }

    #[test]
    fn test_key_event_simple() {
        let event = KeyEvent::simple(Key::Enter, vec![0x0D]);
        assert_eq!(event.key, Key::Enter);
        assert_eq!(event.raw_bytes, vec![0x0D]);
        assert!(!event.has_text());
    }

    #[test]
    fn test_key_event_with_text() {
        let event = KeyEvent::with_text(Key::NotDefined, vec![0x61], "a".to_string());
        assert_eq!(event.key, Key::NotDefined);
        assert_eq!(event.raw_bytes, vec![0x61]);
        assert!(event.has_text());
        assert_eq!(event.text_or_empty(), "a");
    }

    #[test]
    fn test_key_event_default() {
        let event = KeyEvent::default();
        assert_eq!(event.key, Key::NotDefined);
        assert!(event.raw_bytes.is_empty());
        assert!(!event.has_text());
        assert_eq!(event.text_or_empty(), "");
    }

    #[test]
    fn test_key_equality() {
        assert_eq!(Key::ControlC, Key::ControlC);
        assert_ne!(Key::ControlC, Key::ControlD);
    }

    #[test]
    fn test_key_debug() {
        let key = Key::F1;
        let debug_str = format!("{:?}", key);
        assert_eq!(debug_str, "F1");
    }
}
