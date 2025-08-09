use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;
use replkit_core::{KeyParser as CoreKeyParser, Key as CoreKey, KeyEvent as CoreKeyEvent};
use std::panic;

/// Python representation of a Key enum with proper string representations
#[pyclass(name = "Key")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyKey {
    // Control characters
    Escape,
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
    ControlSpace,
    ControlBackslash,
    ControlSquareClose,
    ControlCircumflex,
    ControlUnderscore,
    ControlLeft,
    ControlRight,
    ControlUp,
    ControlDown,

    // Navigation keys
    Up,
    Down,
    Right,
    Left,
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
    Tab,
    Enter,

    // Function keys
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

    // Special keys
    Any,
    CPRResponse,
    Vt100MouseEvent,
    WindowsMouseEvent,
    BracketedPaste,
    Ignore,
    NotDefined,
}

impl From<CoreKey> for PyKey {
    fn from(core_key: CoreKey) -> Self {
        match core_key {
            CoreKey::Escape => Self::Escape,
            CoreKey::ControlA => Self::ControlA,
            CoreKey::ControlB => Self::ControlB,
            CoreKey::ControlC => Self::ControlC,
            CoreKey::ControlD => Self::ControlD,
            CoreKey::ControlE => Self::ControlE,
            CoreKey::ControlF => Self::ControlF,
            CoreKey::ControlG => Self::ControlG,
            CoreKey::ControlH => Self::ControlH,
            CoreKey::ControlI => Self::ControlI,
            CoreKey::ControlJ => Self::ControlJ,
            CoreKey::ControlK => Self::ControlK,
            CoreKey::ControlL => Self::ControlL,
            CoreKey::ControlM => Self::ControlM,
            CoreKey::ControlN => Self::ControlN,
            CoreKey::ControlO => Self::ControlO,
            CoreKey::ControlP => Self::ControlP,
            CoreKey::ControlQ => Self::ControlQ,
            CoreKey::ControlR => Self::ControlR,
            CoreKey::ControlS => Self::ControlS,
            CoreKey::ControlT => Self::ControlT,
            CoreKey::ControlU => Self::ControlU,
            CoreKey::ControlV => Self::ControlV,
            CoreKey::ControlW => Self::ControlW,
            CoreKey::ControlX => Self::ControlX,
            CoreKey::ControlY => Self::ControlY,
            CoreKey::ControlZ => Self::ControlZ,
            CoreKey::ControlSpace => Self::ControlSpace,
            CoreKey::ControlBackslash => Self::ControlBackslash,
            CoreKey::ControlSquareClose => Self::ControlSquareClose,
            CoreKey::ControlCircumflex => Self::ControlCircumflex,
            CoreKey::ControlUnderscore => Self::ControlUnderscore,
            CoreKey::ControlLeft => Self::ControlLeft,
            CoreKey::ControlRight => Self::ControlRight,
            CoreKey::ControlUp => Self::ControlUp,
            CoreKey::ControlDown => Self::ControlDown,
            CoreKey::Up => Self::Up,
            CoreKey::Down => Self::Down,
            CoreKey::Right => Self::Right,
            CoreKey::Left => Self::Left,
            CoreKey::ShiftLeft => Self::ShiftLeft,
            CoreKey::ShiftUp => Self::ShiftUp,
            CoreKey::ShiftDown => Self::ShiftDown,
            CoreKey::ShiftRight => Self::ShiftRight,
            CoreKey::Home => Self::Home,
            CoreKey::End => Self::End,
            CoreKey::Delete => Self::Delete,
            CoreKey::ShiftDelete => Self::ShiftDelete,
            CoreKey::ControlDelete => Self::ControlDelete,
            CoreKey::PageUp => Self::PageUp,
            CoreKey::PageDown => Self::PageDown,
            CoreKey::BackTab => Self::BackTab,
            CoreKey::Insert => Self::Insert,
            CoreKey::Backspace => Self::Backspace,
            CoreKey::Tab => Self::Tab,
            CoreKey::Enter => Self::Enter,
            CoreKey::F1 => Self::F1,
            CoreKey::F2 => Self::F2,
            CoreKey::F3 => Self::F3,
            CoreKey::F4 => Self::F4,
            CoreKey::F5 => Self::F5,
            CoreKey::F6 => Self::F6,
            CoreKey::F7 => Self::F7,
            CoreKey::F8 => Self::F8,
            CoreKey::F9 => Self::F9,
            CoreKey::F10 => Self::F10,
            CoreKey::F11 => Self::F11,
            CoreKey::F12 => Self::F12,
            CoreKey::F13 => Self::F13,
            CoreKey::F14 => Self::F14,
            CoreKey::F15 => Self::F15,
            CoreKey::F16 => Self::F16,
            CoreKey::F17 => Self::F17,
            CoreKey::F18 => Self::F18,
            CoreKey::F19 => Self::F19,
            CoreKey::F20 => Self::F20,
            CoreKey::F21 => Self::F21,
            CoreKey::F22 => Self::F22,
            CoreKey::F23 => Self::F23,
            CoreKey::F24 => Self::F24,
            CoreKey::Any => Self::Any,
            CoreKey::CPRResponse => Self::CPRResponse,
            CoreKey::Vt100MouseEvent => Self::Vt100MouseEvent,
            CoreKey::WindowsMouseEvent => Self::WindowsMouseEvent,
            CoreKey::BracketedPaste => Self::BracketedPaste,
            CoreKey::Ignore => Self::Ignore,
            CoreKey::NotDefined => Self::NotDefined,
        }
    }
}

#[pymethods]
impl PyKey {
    /// Return a string representation of the key
    fn __str__(&self) -> &'static str {
        match self {
            Self::Escape => "Escape",
            Self::ControlA => "Ctrl+A",
            Self::ControlB => "Ctrl+B",
            Self::ControlC => "Ctrl+C",
            Self::ControlD => "Ctrl+D",
            Self::ControlE => "Ctrl+E",
            Self::ControlF => "Ctrl+F",
            Self::ControlG => "Ctrl+G",
            Self::ControlH => "Ctrl+H",
            Self::ControlI => "Ctrl+I",
            Self::ControlJ => "Ctrl+J",
            Self::ControlK => "Ctrl+K",
            Self::ControlL => "Ctrl+L",
            Self::ControlM => "Ctrl+M",
            Self::ControlN => "Ctrl+N",
            Self::ControlO => "Ctrl+O",
            Self::ControlP => "Ctrl+P",
            Self::ControlQ => "Ctrl+Q",
            Self::ControlR => "Ctrl+R",
            Self::ControlS => "Ctrl+S",
            Self::ControlT => "Ctrl+T",
            Self::ControlU => "Ctrl+U",
            Self::ControlV => "Ctrl+V",
            Self::ControlW => "Ctrl+W",
            Self::ControlX => "Ctrl+X",
            Self::ControlY => "Ctrl+Y",
            Self::ControlZ => "Ctrl+Z",
            Self::ControlSpace => "Ctrl+Space",
            Self::ControlBackslash => "Ctrl+\\",
            Self::ControlSquareClose => "Ctrl+]",
            Self::ControlCircumflex => "Ctrl+^",
            Self::ControlUnderscore => "Ctrl+_",
            Self::ControlLeft => "Ctrl+Left",
            Self::ControlRight => "Ctrl+Right",
            Self::ControlUp => "Ctrl+Up",
            Self::ControlDown => "Ctrl+Down",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Right => "Right",
            Self::Left => "Left",
            Self::ShiftLeft => "Shift+Left",
            Self::ShiftUp => "Shift+Up",
            Self::ShiftDown => "Shift+Down",
            Self::ShiftRight => "Shift+Right",
            Self::Home => "Home",
            Self::End => "End",
            Self::Delete => "Delete",
            Self::ShiftDelete => "Shift+Delete",
            Self::ControlDelete => "Ctrl+Delete",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::BackTab => "BackTab",
            Self::Insert => "Insert",
            Self::Backspace => "Backspace",
            Self::Tab => "Tab",
            Self::Enter => "Enter",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
            Self::F20 => "F20",
            Self::F21 => "F21",
            Self::F22 => "F22",
            Self::F23 => "F23",
            Self::F24 => "F24",
            Self::Any => "Any",
            Self::CPRResponse => "CPRResponse",
            Self::Vt100MouseEvent => "Vt100MouseEvent",
            Self::WindowsMouseEvent => "WindowsMouseEvent",
            Self::BracketedPaste => "BracketedPaste",
            Self::Ignore => "Ignore",
            Self::NotDefined => "NotDefined",
        }
    }

    /// Return a debug representation of the key
    fn __repr__(&self) -> String {
        format!("Key.{}", self.__str__())
    }
}

/// Python representation of a KeyEvent with proper Python integration
#[pyclass]
#[derive(Clone, Debug)]
pub struct KeyEvent {
    /// The parsed key type
    #[pyo3(get)]
    pub key: PyKey,
    /// The raw bytes that were parsed to produce this key event
    #[pyo3(get)]
    pub raw_bytes: Py<PyBytes>,
    /// Optional text content associated with this key event
    #[pyo3(get)]
    pub text: Option<String>,
}

impl KeyEvent {
    /// Create a new KeyEvent from a core KeyEvent
    fn from_core(core_event: CoreKeyEvent, py: Python) -> Self {
        let raw_bytes = PyBytes::new(py, &core_event.raw_bytes).into();
        Self {
            key: core_event.key.into(),
            raw_bytes,
            text: core_event.text,
        }
    }
}

#[pymethods]
impl KeyEvent {
    /// Create a new KeyEvent
    #[new]
    fn new(key: PyKey, raw_bytes: &PyBytes, text: Option<String>) -> Self {
        Self {
            key,
            raw_bytes: raw_bytes.into(),
            text,
        }
    }

    /// Check if this key event has associated text content
    fn has_text(&self) -> bool {
        self.text.is_some()
    }

    /// Get the text content, returning an empty string if none exists
    fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    /// String representation of the KeyEvent
    fn __str__(&self) -> String {
        match &self.text {
            Some(text) => format!("KeyEvent(key={}, text='{}')", self.key.__str__(), text),
            None => format!("KeyEvent(key={})", self.key.__str__()),
        }
    }

    /// Debug representation of the KeyEvent
    fn __repr__(&self, py: Python) -> String {
        let raw_bytes = self.raw_bytes.as_ref(py).as_bytes();
        format!("KeyEvent(key={}, raw_bytes={:?}, text={:?})", 
                self.key.__repr__(), 
                raw_bytes,
                self.text)
    }
}

/// Python wrapper for the Rust KeyParser with proper error handling
#[pyclass]
pub struct KeyParser {
    inner: CoreKeyParser,
}

#[pymethods]
impl KeyParser {
    /// Create a new KeyParser instance
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreKeyParser::new(),
        }
    }

    /// Feed raw bytes to the parser and return any complete key events
    /// 
    /// Args:
    ///     data: Raw bytes from terminal input
    /// 
    /// Returns:
    ///     List of KeyEvent objects representing parsed key events
    /// 
    /// Raises:
    ///     RuntimeError: If parsing fails due to internal error
    ///     ValueError: If input data is invalid
    fn feed(&mut self, py: Python, data: &PyBytes) -> PyResult<Vec<KeyEvent>> {
        let bytes = data.as_bytes();
        
        // Validate input
        if bytes.is_empty() {
            return Ok(Vec::new());
        }

        // Set up panic hook to convert panics to Python exceptions
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.inner.feed(bytes)
        }));

        match result {
            Ok(core_events) => {
                let py_events = core_events
                    .into_iter()
                    .map(|event| KeyEvent::from_core(event, py))
                    .collect();
                Ok(py_events)
            }
            Err(_) => Err(PyRuntimeError::new_err(
                "Internal parser error occurred during feed operation"
            )),
        }
    }

    /// Flush any incomplete sequences and return them as key events
    /// 
    /// This method should be called when input is complete (e.g., on EOF)
    /// to handle any remaining partial sequences in the buffer.
    /// 
    /// Returns:
    ///     List of KeyEvent objects representing any remaining parsed events
    /// 
    /// Raises:
    ///     RuntimeError: If flushing fails due to internal error
    fn flush(&mut self, py: Python) -> PyResult<Vec<KeyEvent>> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.inner.flush()
        }));

        match result {
            Ok(core_events) => {
                let py_events = core_events
                    .into_iter()
                    .map(|event| KeyEvent::from_core(event, py))
                    .collect();
                Ok(py_events)
            }
            Err(_) => Err(PyRuntimeError::new_err(
                "Internal parser error occurred during flush operation"
            )),
        }
    }

    /// Reset the parser state and clear all buffers
    /// 
    /// This method clears any accumulated state and returns the parser
    /// to its initial state. It's useful for handling connection resets
    /// or when starting fresh parsing.
    /// 
    /// Raises:
    ///     RuntimeError: If reset fails due to internal error
    fn reset(&mut self) -> PyResult<()> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.inner.reset()
        }));

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(PyRuntimeError::new_err(
                "Internal parser error occurred during reset operation"
            )),
        }
    }

    /// Get the current parser state as a string (for debugging)
    fn get_state(&self) -> &'static str {
        // This is a simplified representation since we can't expose the internal state directly
        "Normal" // The actual state would require more complex introspection
    }

    /// String representation of the KeyParser
    fn __str__(&self) -> &'static str {
        "KeyParser"
    }

    /// Debug representation of the KeyParser
    fn __repr__(&self) -> &'static str {
        "KeyParser()"
    }
}

/// Python bindings for REPLKIT.
/// 
/// This module provides Python access to the Rust-based key input parser
/// that can handle raw terminal input and convert byte sequences to 
/// structured key events.
/// 
/// Classes:
///     KeyParser: Main parser class for processing terminal input
///     KeyEvent: Represents a parsed key event
///     Key: Enumeration of all possible key types
/// 
/// Example:
///     >>> import replkit
///     >>> parser = replkit.KeyParser()
///     >>> events = parser.feed(b'\x1b[A')  # Up arrow
///     >>> print(events[0].key)
///     Up
#[pymodule]
fn replkit(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "Python bindings for REPLKIT")?;
    
    // Add classes
    m.add_class::<KeyParser>()?;
    m.add_class::<KeyEvent>()?;
    m.add_class::<PyKey>()?;
    
    // Add module-level constants for convenience
    m.add("ESCAPE", PyKey::Escape)?;
    m.add("CTRL_C", PyKey::ControlC)?;
    m.add("ENTER", PyKey::Enter)?;
    m.add("TAB", PyKey::Tab)?;
    m.add("UP", PyKey::Up)?;
    m.add("DOWN", PyKey::Down)?;
    m.add("LEFT", PyKey::Left)?;
    m.add("RIGHT", PyKey::Right)?;
    
    Ok(())
}