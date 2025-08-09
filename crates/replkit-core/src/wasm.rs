//! WASM-compatible interface functions for the key parser
//! These functions provide a C-style API that can be easily called from WASM

#[cfg(feature = "wasm")]
pub use self::wasm_impl::*;

#[cfg(feature = "wasm")]
mod wasm_impl {
    use crate::{Buffer, Document, Key, KeyEvent, KeyParser};
    use serde::{Deserialize, Serialize};

    /// Serializable version of KeyEvent for WASM interop
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WasmKeyEvent {
        pub key: u32,
        pub raw_bytes: Vec<u8>,
        pub text: Option<String>,
    }

    impl From<KeyEvent> for WasmKeyEvent {
        fn from(event: KeyEvent) -> Self {
            WasmKeyEvent {
                key: key_to_u32(event.key),
                raw_bytes: event.raw_bytes,
                text: event.text,
            }
        }
    }

    impl From<WasmKeyEvent> for KeyEvent {
        fn from(wasm_event: WasmKeyEvent) -> Self {
            KeyEvent {
                key: u32_to_key(wasm_event.key),
                raw_bytes: wasm_event.raw_bytes,
                text: wasm_event.text,
            }
        }
    }

    /// Serializable version of Buffer state for WASM interop
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WasmBufferState {
        pub working_lines: Vec<String>,
        pub working_index: usize,
        pub cursor_position: usize,
        pub preferred_column: Option<usize>,
        pub last_key_stroke: Option<u32>,
    }

    /// Serializable version of Document state for WASM interop
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WasmDocumentState {
        pub text: String,
        pub cursor_position: usize,
        pub last_key: Option<u32>,
    }

    /// Convert Key enum to u32 for WASM serialization
    pub fn key_to_u32(key: Key) -> u32 {
        match key {
            Key::Escape => 0,
            Key::ControlA => 1,
            Key::ControlB => 2,
            Key::ControlC => 3,
            Key::ControlD => 4,
            Key::ControlE => 5,
            Key::ControlF => 6,
            Key::ControlG => 7,
            Key::ControlH => 8,
            Key::ControlI => 9,
            Key::ControlJ => 10,
            Key::ControlK => 11,
            Key::ControlL => 12,
            Key::ControlM => 13,
            Key::ControlN => 14,
            Key::ControlO => 15,
            Key::ControlP => 16,
            Key::ControlQ => 17,
            Key::ControlR => 18,
            Key::ControlS => 19,
            Key::ControlT => 20,
            Key::ControlU => 21,
            Key::ControlV => 22,
            Key::ControlW => 23,
            Key::ControlX => 24,
            Key::ControlY => 25,
            Key::ControlZ => 26,
            Key::ControlSpace => 27,
            Key::ControlBackslash => 28,
            Key::ControlSquareClose => 29,
            Key::ControlCircumflex => 30,
            Key::ControlUnderscore => 31,
            Key::ControlLeft => 32,
            Key::ControlRight => 33,
            Key::ControlUp => 34,
            Key::ControlDown => 35,
            Key::Up => 36,
            Key::Down => 37,
            Key::Right => 38,
            Key::Left => 39,
            Key::ShiftLeft => 40,
            Key::ShiftUp => 41,
            Key::ShiftDown => 42,
            Key::ShiftRight => 43,
            Key::Home => 44,
            Key::End => 45,
            Key::Delete => 46,
            Key::ShiftDelete => 47,
            Key::ControlDelete => 48,
            Key::PageUp => 49,
            Key::PageDown => 50,
            Key::BackTab => 51,
            Key::Insert => 52,
            Key::Backspace => 53,
            Key::Tab => 54,
            Key::Enter => 55,
            Key::F1 => 56,
            Key::F2 => 57,
            Key::F3 => 58,
            Key::F4 => 59,
            Key::F5 => 60,
            Key::F6 => 61,
            Key::F7 => 62,
            Key::F8 => 63,
            Key::F9 => 64,
            Key::F10 => 65,
            Key::F11 => 66,
            Key::F12 => 67,
            Key::F13 => 68,
            Key::F14 => 69,
            Key::F15 => 70,
            Key::F16 => 71,
            Key::F17 => 72,
            Key::F18 => 73,
            Key::F19 => 74,
            Key::F20 => 75,
            Key::F21 => 76,
            Key::F22 => 77,
            Key::F23 => 78,
            Key::F24 => 79,
            Key::Any => 80,
            Key::CPRResponse => 81,
            Key::Vt100MouseEvent => 82,
            Key::WindowsMouseEvent => 83,
            Key::BracketedPaste => 84,
            Key::Ignore => 85,
            Key::NotDefined => 86,
        }
    }

    /// Convert u32 back to Key enum for WASM deserialization
    pub fn u32_to_key(value: u32) -> Key {
        match value {
            0 => Key::Escape,
            1 => Key::ControlA,
            2 => Key::ControlB,
            3 => Key::ControlC,
            4 => Key::ControlD,
            5 => Key::ControlE,
            6 => Key::ControlF,
            7 => Key::ControlG,
            8 => Key::ControlH,
            9 => Key::ControlI,
            10 => Key::ControlJ,
            11 => Key::ControlK,
            12 => Key::ControlL,
            13 => Key::ControlM,
            14 => Key::ControlN,
            15 => Key::ControlO,
            16 => Key::ControlP,
            17 => Key::ControlQ,
            18 => Key::ControlR,
            19 => Key::ControlS,
            20 => Key::ControlT,
            21 => Key::ControlU,
            22 => Key::ControlV,
            23 => Key::ControlW,
            24 => Key::ControlX,
            25 => Key::ControlY,
            26 => Key::ControlZ,
            27 => Key::ControlSpace,
            28 => Key::ControlBackslash,
            29 => Key::ControlSquareClose,
            30 => Key::ControlCircumflex,
            31 => Key::ControlUnderscore,
            32 => Key::ControlLeft,
            33 => Key::ControlRight,
            34 => Key::ControlUp,
            35 => Key::ControlDown,
            36 => Key::Up,
            37 => Key::Down,
            38 => Key::Right,
            39 => Key::Left,
            40 => Key::ShiftLeft,
            41 => Key::ShiftUp,
            42 => Key::ShiftDown,
            43 => Key::ShiftRight,
            44 => Key::Home,
            45 => Key::End,
            46 => Key::Delete,
            47 => Key::ShiftDelete,
            48 => Key::ControlDelete,
            49 => Key::PageUp,
            50 => Key::PageDown,
            51 => Key::BackTab,
            52 => Key::Insert,
            53 => Key::Backspace,
            54 => Key::Tab,
            55 => Key::Enter,
            56 => Key::F1,
            57 => Key::F2,
            58 => Key::F3,
            59 => Key::F4,
            60 => Key::F5,
            61 => Key::F6,
            62 => Key::F7,
            63 => Key::F8,
            64 => Key::F9,
            65 => Key::F10,
            66 => Key::F11,
            67 => Key::F12,
            68 => Key::F13,
            69 => Key::F14,
            70 => Key::F15,
            71 => Key::F16,
            72 => Key::F17,
            73 => Key::F18,
            74 => Key::F19,
            75 => Key::F20,
            76 => Key::F21,
            77 => Key::F22,
            78 => Key::F23,
            79 => Key::F24,
            80 => Key::Any,
            81 => Key::CPRResponse,
            82 => Key::Vt100MouseEvent,
            83 => Key::WindowsMouseEvent,
            84 => Key::BracketedPaste,
            85 => Key::Ignore,
            86 => Key::NotDefined,
            _ => Key::NotDefined,
        }
    }

    /// WASM-compatible wrapper for KeyParser
    pub struct WasmKeyParser {
        parser: KeyParser,
    }

    impl WasmKeyParser {
        /// Create a new WASM-compatible key parser
        pub fn new() -> Self {
            WasmKeyParser {
                parser: KeyParser::new(),
            }
        }

        /// Feed input bytes to the parser and return serializable events
        pub fn feed(&mut self, data: &[u8]) -> Vec<WasmKeyEvent> {
            let events = self.parser.feed(data);
            events.into_iter().map(WasmKeyEvent::from).collect()
        }

        /// Flush any remaining buffered input and return serializable events
        pub fn flush(&mut self) -> Vec<WasmKeyEvent> {
            let events = self.parser.flush();
            events.into_iter().map(WasmKeyEvent::from).collect()
        }

        /// Reset the parser state
        pub fn reset(&mut self) {
            self.parser.reset();
        }
    }

    impl Default for WasmKeyParser {
        fn default() -> Self {
            Self::new()
        }
    }

    /// WASM serialization methods for Buffer
    impl Buffer {
        /// Convert Buffer to WASM-serializable state
        pub fn to_wasm_state(&self) -> WasmBufferState {
            WasmBufferState {
                working_lines: self.working_lines().clone(),
                working_index: self.working_index(),
                cursor_position: self.cursor_position(),
                preferred_column: self.preferred_column(),
                last_key_stroke: self.last_key_stroke().map(key_to_u32),
            }
        }

        /// Create Buffer from WASM-serializable state
        pub fn from_wasm_state(state: WasmBufferState) -> Self {
            let mut buffer = Buffer::new();
            buffer.set_working_lines(state.working_lines);
            let _ = buffer.set_working_index(state.working_index); // Ignore error, will be clamped
            buffer.set_cursor_position(state.cursor_position);
            buffer.set_preferred_column(state.preferred_column);
            buffer.set_last_key_stroke_optional(state.last_key_stroke.map(u32_to_key));
            buffer
        }
    }

    /// WASM serialization methods for Document
    impl Document {
        /// Convert Document to WASM-serializable state
        pub fn to_wasm_state(&self) -> WasmDocumentState {
            WasmDocumentState {
                text: self.text().to_string(),
                cursor_position: self.cursor_position(),
                last_key: self.last_key_stroke().map(key_to_u32),
            }
        }

        /// Create Document from WASM-serializable state
        pub fn from_wasm_state(state: WasmDocumentState) -> Self {
            let last_key = state.last_key.map(u32_to_key);
            Document::with_text_and_key(state.text, state.cursor_position, last_key)
        }
    }
}

// Provide empty implementations when wasm feature is not enabled
#[cfg(not(feature = "wasm"))]
pub struct WasmKeyEvent;

#[cfg(not(feature = "wasm"))]
pub struct WasmKeyParser;

#[cfg(not(feature = "wasm"))]
pub struct WasmBufferState;

#[cfg(not(feature = "wasm"))]
pub struct WasmDocumentState;

#[cfg(not(feature = "wasm"))]
pub fn key_to_u32(_key: crate::Key) -> u32 {
    0
}

#[cfg(not(feature = "wasm"))]
pub fn u32_to_key(_value: u32) -> crate::Key {
    crate::Key::NotDefined
}
