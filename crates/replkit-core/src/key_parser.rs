//! State machine-based key parser for handling raw terminal input.
//!
//! This module provides a state machine parser that can handle partial byte sequences
//! correctly, similar to prompt_toolkit's VT100 parser. The parser maintains state
//! between calls to handle multi-byte escape sequences and special terminal modes.

use crate::key::{Key, KeyEvent};
use crate::sequence_matcher::{MatchResult, SequenceMatcher};

/// Maximum buffer size to prevent unbounded memory growth
const MAX_BUFFER_SIZE: usize = 1024;

/// Parser state for handling different types of input sequences
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserState {
    /// Normal state for handling plain ASCII input and known single-byte sequences
    Normal,
    /// Handling escape sequences that begin with ESC (0x1B)
    EscapeSequence,
    /// Handling Control Sequence Introducer sequences (ESC[)
    CsiSequence,
    /// Handling mouse event sequences
    MouseEvent,
    /// Handling bracketed paste mode content (between ESC[200~ and ESC[201~)
    BracketedPaste,
}

/// State machine parser for converting raw terminal input bytes to key events
pub struct KeyParser {
    /// Current parser state
    state: ParserState,
    /// Buffer for accumulating partial sequences
    buffer: Vec<u8>,
    /// Sequence matcher for identifying complete key sequences
    sequence_matcher: SequenceMatcher,
    /// Buffer for bracketed paste content
    paste_buffer: Vec<u8>,
}

impl KeyParser {
    /// Create a new KeyParser with default configuration
    pub fn new() -> Self {
        Self {
            state: ParserState::Normal,
            buffer: Vec::new(),
            sequence_matcher: SequenceMatcher::new(),
            paste_buffer: Vec::new(),
        }
    }

    /// Feed raw bytes to the parser and return any complete key events
    ///
    /// This method processes the input bytes according to the current parser state
    /// and returns a vector of complete key events. Partial sequences are buffered
    /// until they can be completed or determined to be invalid.
    pub fn feed(&mut self, data: &[u8]) -> Vec<KeyEvent> {
        let mut events = Vec::new();

        for &byte in data {
            // Prevent buffer overflow
            if self.buffer.len() >= MAX_BUFFER_SIZE {
                // If buffer is too large, flush it and reset to normal state
                if let Some(event) = self.flush_buffer_as_text() {
                    events.push(event);
                }
                self.reset_to_normal();
            }

            match self.state {
                ParserState::Normal => {
                    self.handle_normal_byte(byte, &mut events);
                }
                ParserState::EscapeSequence => {
                    self.handle_escape_byte(byte, &mut events);
                }
                ParserState::CsiSequence => {
                    self.handle_csi_byte(byte, &mut events);
                }
                ParserState::MouseEvent => {
                    self.handle_mouse_byte(byte, &mut events);
                }
                ParserState::BracketedPaste => {
                    self.handle_paste_byte(byte, &mut events);
                }
            }
        }

        events
    }

    /// Flush any incomplete sequences and return them as key events
    ///
    /// This method should be called when input is complete (e.g., on EOF)
    /// to handle any remaining partial sequences in the buffer.
    pub fn flush(&mut self) -> Vec<KeyEvent> {
        let mut events = Vec::new();

        if !self.buffer.is_empty() {
            match self.state {
                ParserState::BracketedPaste => {
                    // In bracketed paste mode, return the accumulated paste content
                    // Include both paste_buffer and current buffer content
                    let mut all_content = self.paste_buffer.clone();
                    all_content.extend_from_slice(&self.buffer);

                    if !all_content.is_empty() {
                        if let Ok(text) = String::from_utf8(all_content.clone()) {
                            events.push(KeyEvent::with_text(
                                Key::BracketedPaste,
                                all_content,
                                text,
                            ));
                        } else {
                            events.push(KeyEvent::simple(Key::BracketedPaste, all_content));
                        }
                    }
                    self.paste_buffer.clear();
                }
                _ => {
                    // For other states, try to find the longest valid sequence
                    if let Some(longest) = self.sequence_matcher.find_longest_match(&self.buffer) {
                        events.push(KeyEvent::simple(
                            longest.key,
                            self.buffer[..longest.consumed_bytes].to_vec(),
                        ));

                        // Handle remaining bytes as individual characters
                        for &byte in &self.buffer[longest.consumed_bytes..] {
                            events.push(self.create_char_event(byte));
                        }
                    } else {
                        // No valid sequences found, treat as individual characters
                        for &byte in &self.buffer {
                            events.push(self.create_char_event(byte));
                        }
                    }
                }
            }
        }

        self.reset();
        events
    }

    /// Reset the parser state and clear all buffers
    pub fn reset(&mut self) {
        self.state = ParserState::Normal;
        self.buffer.clear();
        self.paste_buffer.clear();
    }

    /// Handle a byte in Normal state
    fn handle_normal_byte(&mut self, byte: u8, events: &mut Vec<KeyEvent>) {
        if byte == 0x1b {
            // ESC - start of escape sequence
            self.buffer.push(byte);
            self.state = ParserState::EscapeSequence;
        } else {
            // Check if this is a known single-byte sequence
            match self.sequence_matcher.match_sequence(&[byte]) {
                MatchResult::Exact(key) => {
                    events.push(KeyEvent::simple(key, vec![byte]));
                }
                _ => {
                    // Regular character
                    events.push(self.create_char_event(byte));
                }
            }
        }
    }

    /// Handle a byte in EscapeSequence state
    fn handle_escape_byte(&mut self, byte: u8, events: &mut Vec<KeyEvent>) {
        self.buffer.push(byte);

        if byte == 0x5b {
            // ESC[ - Control Sequence Introducer
            self.state = ParserState::CsiSequence;
        } else {
            // Check if we have a complete escape sequence
            match self.sequence_matcher.match_sequence(&self.buffer) {
                MatchResult::Exact(key) => {
                    events.push(KeyEvent::simple(key, self.buffer.clone()));
                    self.reset_to_normal();
                }
                MatchResult::Prefix => {
                    // Continue accumulating
                }
                MatchResult::NoMatch => {
                    // Invalid escape sequence, emit ESC and handle the byte normally
                    events.push(KeyEvent::simple(Key::Escape, vec![0x1b]));
                    self.reset_to_normal();
                    self.handle_normal_byte(byte, events);
                }
            }
        }
    }

    /// Handle a byte in CsiSequence state
    fn handle_csi_byte(&mut self, byte: u8, events: &mut Vec<KeyEvent>) {
        self.buffer.push(byte);

        // Check for bracketed paste start sequence (ESC[200~)
        if self.buffer == b"\x1b[200~" {
            self.state = ParserState::BracketedPaste;
            self.buffer.clear();
            return;
        }

        // Check for mouse event sequences (ESC[M or ESC[<)
        if self.buffer.len() == 3 && (byte == b'M' || byte == b'<') {
            self.state = ParserState::MouseEvent;
            return;
        }

        // Check for CPR (Cursor Position Report) response pattern: ESC[{row};{col}R
        if byte == b'R' && self.is_cpr_response(&self.buffer) {
            events.push(KeyEvent::simple(Key::CPRResponse, self.buffer.clone()));
            self.reset_to_normal();
            return;
        }

        // Check if we have a complete CSI sequence
        match self.sequence_matcher.match_sequence(&self.buffer) {
            MatchResult::Exact(key) => {
                if key == Key::Ignore {
                    // Ignore this sequence
                } else {
                    events.push(KeyEvent::simple(key, self.buffer.clone()));
                }
                self.reset_to_normal();
            }
            MatchResult::Prefix => {
                // Continue accumulating
            }
            MatchResult::NoMatch => {
                // Check if this might be a parameterized CSI sequence
                if self.is_csi_parameter_byte(byte) {
                    // Continue accumulating parameters
                } else if self.is_csi_final_byte(byte) {
                    // This is a final byte but we don't recognize the sequence
                    // Check if it might be a special sequence we should handle
                    if self.is_special_csi_sequence(&self.buffer) {
                        events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
                    } else {
                        // Emit as unknown sequence
                        events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
                    }
                    self.reset_to_normal();
                } else {
                    // Invalid CSI sequence, emit ESC[ and handle remaining bytes
                    events.push(KeyEvent::simple(Key::Escape, vec![0x1b]));
                    events.push(self.create_char_event(0x5b)); // [

                    // Collect remaining bytes before resetting
                    let remaining_bytes: Vec<u8> = self.buffer[2..].to_vec();
                    self.reset_to_normal();

                    // Handle remaining bytes
                    for b in remaining_bytes {
                        self.handle_normal_byte(b, events);
                    }
                }
            }
        }
    }

    /// Handle a byte in MouseEvent state
    fn handle_mouse_byte(&mut self, byte: u8, events: &mut Vec<KeyEvent>) {
        self.buffer.push(byte);

        // Handle different mouse event formats
        if self.buffer.starts_with(b"\x1b[M") {
            // X10 mouse format: ESC[M + 3 bytes (button, x, y)
            if self.buffer.len() >= 6 {
                events.push(KeyEvent::simple(Key::Vt100MouseEvent, self.buffer.clone()));
                self.reset_to_normal();
            }
        } else if self.buffer.starts_with(b"\x1b[<") {
            // SGR mouse format: ESC[<button;x;y[Mm]
            if byte == b'M' || byte == b'm' {
                // Complete SGR mouse sequence
                if self.is_valid_sgr_mouse_sequence(&self.buffer) {
                    events.push(KeyEvent::simple(Key::Vt100MouseEvent, self.buffer.clone()));
                } else {
                    // Invalid SGR sequence, emit as unknown
                    events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
                }
                self.reset_to_normal();
            } else if !self.is_sgr_mouse_parameter_byte(byte) {
                // Invalid character in SGR sequence
                events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
                self.reset_to_normal();
            }
            // Otherwise continue accumulating
        } else {
            // Unknown mouse format, treat as regular sequence after timeout
            if self.buffer.len() >= 10 {
                events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
                self.reset_to_normal();
            }
        }
    }

    /// Handle a byte in BracketedPaste state
    fn handle_paste_byte(&mut self, byte: u8, events: &mut Vec<KeyEvent>) {
        self.buffer.push(byte);

        // Check for bracketed paste end sequence (ESC[201~)
        if self.buffer.ends_with(b"\x1b[201~") {
            // Remove the end sequence from the buffer and add remaining content to paste_buffer
            let end_seq_len = 6; // Length of "\x1b[201~"
            if self.buffer.len() >= end_seq_len {
                self.paste_buffer
                    .extend_from_slice(&self.buffer[..self.buffer.len() - end_seq_len]);
            }

            // Emit bracketed paste event
            if let Ok(text) = String::from_utf8(self.paste_buffer.clone()) {
                events.push(KeyEvent::with_text(
                    Key::BracketedPaste,
                    self.paste_buffer.clone(),
                    text,
                ));
            } else {
                // Invalid UTF-8, emit as raw bytes
                events.push(KeyEvent::simple(
                    Key::BracketedPaste,
                    self.paste_buffer.clone(),
                ));
            }

            self.paste_buffer.clear();
            self.reset_to_normal();
        } else if self.buffer.len() >= 6 {
            // If buffer is getting long and we haven't found the end sequence,
            // move some content to paste_buffer to avoid keeping too much in buffer
            let keep_len = 5; // Keep last 5 bytes in buffer (enough for partial end sequence)
            let move_len = self.buffer.len() - keep_len;
            self.paste_buffer
                .extend_from_slice(&self.buffer[..move_len]);
            self.buffer.drain(..move_len);
        }
    }

    /// Check if a byte is a CSI parameter byte (digits, semicolon, etc.)
    fn is_csi_parameter_byte(&self, byte: u8) -> bool {
        matches!(byte, b'0'..=b'9' | b';' | b':' | b'<' | b'=' | b'>' | b'?')
    }

    /// Check if a byte is a CSI final byte (letters)
    fn is_csi_final_byte(&self, byte: u8) -> bool {
        matches!(byte, b'@'..=b'~')
    }

    /// Reset to normal state and clear buffer
    fn reset_to_normal(&mut self) {
        self.state = ParserState::Normal;
        self.buffer.clear();
    }

    /// Create a character event for a regular byte
    fn create_char_event(&self, byte: u8) -> KeyEvent {
        if byte.is_ascii() && !byte.is_ascii_control() {
            // Printable ASCII character
            KeyEvent::with_text(
                Key::NotDefined,
                vec![byte],
                String::from_utf8_lossy(&[byte]).to_string(),
            )
        } else {
            // Non-printable or control character
            KeyEvent::simple(Key::NotDefined, vec![byte])
        }
    }

    /// Flush buffer as text (for overflow protection)
    fn flush_buffer_as_text(&mut self) -> Option<KeyEvent> {
        if self.buffer.is_empty() {
            return None;
        }

        let text = String::from_utf8_lossy(&self.buffer).to_string();
        let event = KeyEvent::with_text(Key::NotDefined, self.buffer.clone(), text);
        self.buffer.clear();
        Some(event)
    }

    /// Check if the buffer contains a CPR (Cursor Position Report) response
    /// CPR responses have the format: ESC[{row};{col}R
    fn is_cpr_response(&self, buffer: &[u8]) -> bool {
        if buffer.len() < 4 || !buffer.starts_with(b"\x1b[") || !buffer.ends_with(b"R") {
            return false;
        }

        // Extract the middle part (between ESC[ and R)
        let middle = &buffer[2..buffer.len() - 1];

        // Check if it matches the pattern: digits, semicolon, digits
        let mut found_semicolon = false;
        let mut has_digits_before = false;
        let mut has_digits_after = false;

        for &byte in middle {
            match byte {
                b'0'..=b'9' => {
                    if found_semicolon {
                        has_digits_after = true;
                    } else {
                        has_digits_before = true;
                    }
                }
                b';' => {
                    if found_semicolon || !has_digits_before {
                        return false; // Multiple semicolons or semicolon without preceding digits
                    }
                    found_semicolon = true;
                }
                _ => return false, // Invalid character
            }
        }

        // Must have digits before and after semicolon
        has_digits_before && found_semicolon && has_digits_after
    }

    /// Check if the buffer contains a special CSI sequence that we should handle
    /// This includes sequences that might not be in our standard mapping but are valid
    fn is_special_csi_sequence(&self, buffer: &[u8]) -> bool {
        if buffer.len() < 3 || !buffer.starts_with(b"\x1b[") {
            return false;
        }

        let sequence = &buffer[2..];

        // Check for various special sequence patterns
        // Device Status Report responses, etc.
        if sequence.ends_with(b"n") || sequence.ends_with(b"c") {
            return true;
        }

        // Check for mouse tracking sequences that we might not have mapped
        if sequence.starts_with(b"<") && (sequence.ends_with(b"M") || sequence.ends_with(b"m")) {
            return true;
        }

        false
    }

    /// Check if a byte is valid in SGR mouse parameter sequences
    /// SGR mouse sequences contain digits and semicolons: ESC[<button;x;y[Mm]
    fn is_sgr_mouse_parameter_byte(&self, byte: u8) -> bool {
        matches!(byte, b'0'..=b'9' | b';')
    }

    /// Validate that the buffer contains a properly formatted SGR mouse sequence
    /// SGR format: ESC[<button;x;y[Mm] where button, x, y are decimal numbers
    fn is_valid_sgr_mouse_sequence(&self, buffer: &[u8]) -> bool {
        if buffer.len() < 5 || !buffer.starts_with(b"\x1b[<") {
            return false;
        }

        let last_byte = buffer[buffer.len() - 1];
        if last_byte != b'M' && last_byte != b'm' {
            return false;
        }

        // Extract the parameter part (between ESC[< and final M/m)
        let params = &buffer[3..buffer.len() - 1];

        // Should contain exactly two semicolons separating three numbers
        let semicolon_count = params.iter().filter(|&&b| b == b';').count();
        if semicolon_count != 2 {
            return false;
        }

        // Check that all characters are digits or semicolons
        for &byte in params {
            if !matches!(byte, b'0'..=b'9' | b';') {
                return false;
            }
        }

        // Ensure we don't have empty parameters (consecutive semicolons or leading/trailing semicolons)
        if params.is_empty() || params[0] == b';' || params[params.len() - 1] == b';' {
            return false;
        }

        // Check for consecutive semicolons
        for window in params.windows(2) {
            if window[0] == b';' && window[1] == b';' {
                return false;
            }
        }

        true
    }
}

impl Default for KeyParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = KeyParser::new();
        assert_eq!(parser.state, ParserState::Normal);
        assert!(parser.buffer.is_empty());
    }

    #[test]
    fn test_simple_control_characters() {
        let mut parser = KeyParser::new();

        // Test Ctrl+C
        let events = parser.feed(&[0x03]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::ControlC);
        assert_eq!(events[0].raw_bytes, vec![0x03]);

        // Test Tab
        let events = parser.feed(&[0x09]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Tab);
    }

    #[test]
    fn test_escape_key() {
        let mut parser = KeyParser::new();

        // Single ESC should be handled as Escape key after timeout/flush
        let events = parser.feed(&[0x1b]);
        assert_eq!(events.len(), 0); // No events yet, waiting for more input
        assert_eq!(parser.state, ParserState::EscapeSequence);

        // Flush should emit the Escape key
        let events = parser.flush();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Escape);
    }

    #[test]
    fn test_arrow_keys() {
        let mut parser = KeyParser::new();

        // Test Up arrow (ESC[A)
        let events = parser.feed(&[0x1b, 0x5b, 0x41]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Up);
        assert_eq!(events[0].raw_bytes, vec![0x1b, 0x5b, 0x41]);
        assert_eq!(parser.state, ParserState::Normal);

        // Test Down arrow
        let events = parser.feed(&[0x1b, 0x5b, 0x42]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Down);
    }

    #[test]
    fn test_partial_sequences() {
        let mut parser = KeyParser::new();

        // Feed partial escape sequence
        let events = parser.feed(&[0x1b]);
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::EscapeSequence);

        let events = parser.feed(&[0x5b]);
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::CsiSequence);

        let events = parser.feed(&[0x41]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Up);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let mut parser = KeyParser::new();

        // ESC followed by invalid byte
        let events = parser.feed(&[0x1b, 0xff]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].key, Key::Escape);
        assert_eq!(events[1].key, Key::NotDefined);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_function_keys() {
        let mut parser = KeyParser::new();

        // Test F1 (ESC OP)
        let events = parser.feed(&[0x1b, 0x4f, 0x50]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::F1);

        // Test F5 (ESC[15~)
        let events = parser.feed(&[0x1b, 0x5b, 0x31, 0x35, 0x7e]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::F5);
    }

    #[test]
    fn test_bracketed_paste() {
        let mut parser = KeyParser::new();

        // Start bracketed paste
        let events = parser.feed(b"\x1b[200~");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::BracketedPaste);

        // Add some content
        let events = parser.feed(b"hello world");
        assert_eq!(events.len(), 0); // Still accumulating

        // End bracketed paste
        let events = parser.feed(b"\x1b[201~");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(events[0].text.as_ref().unwrap(), "hello world");
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_mixed_input() {
        let mut parser = KeyParser::new();

        // Mix of control chars, escape sequences, and regular chars
        let input = b"\x03\x1b[A\x61\x1b[B";
        let events = parser.feed(input);

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].key, Key::ControlC);
        assert_eq!(events[1].key, Key::Up);
        assert_eq!(events[2].key, Key::NotDefined); // 'a'
        assert_eq!(events[2].text.as_ref().unwrap(), "a");
        assert_eq!(events[3].key, Key::Down);
    }

    #[test]
    fn test_reset() {
        let mut parser = KeyParser::new();

        // Put parser in a non-normal state
        parser.feed(&[0x1b, 0x5b]);
        assert_eq!(parser.state, ParserState::CsiSequence);
        assert!(!parser.buffer.is_empty());

        // Reset should clear everything
        parser.reset();
        assert_eq!(parser.state, ParserState::Normal);
        assert!(parser.buffer.is_empty());
    }

    #[test]
    fn test_flush_partial_sequences() {
        let mut parser = KeyParser::new();

        // Leave parser with partial sequence
        parser.feed(&[0x1b, 0x5b, 0x31]);
        assert_eq!(parser.state, ParserState::CsiSequence);

        // Flush should handle the partial sequence
        let events = parser.flush();
        assert!(!events.is_empty());
        assert_eq!(parser.state, ParserState::Normal);
        assert!(parser.buffer.is_empty());
    }

    #[test]
    fn test_buffer_overflow_protection() {
        let mut parser = KeyParser::new();

        // Create a very long invalid sequence
        let mut long_input = vec![0x1b, 0x5b]; // Start CSI sequence
        long_input.extend(vec![0x30; MAX_BUFFER_SIZE]); // Add many '0' characters

        let events = parser.feed(&long_input);

        // Should not crash and should produce some events
        assert!(!events.is_empty());
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_printable_characters() {
        let mut parser = KeyParser::new();

        // Test regular ASCII characters
        let events = parser.feed(b"hello");
        assert_eq!(events.len(), 5);

        for (i, &ch) in b"hello".iter().enumerate() {
            assert_eq!(events[i].key, Key::NotDefined);
            assert_eq!(events[i].raw_bytes, vec![ch]);
            assert_eq!(
                events[i].text.as_ref().unwrap(),
                &String::from_utf8_lossy(&[ch])
            );
        }
    }

    #[test]
    fn test_modifier_combinations() {
        let mut parser = KeyParser::new();

        // Test Shift+Up (ESC[1;2A)
        let events = parser.feed(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x41]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::ShiftUp);

        // Test Ctrl+Right (ESC[1;5C)
        let events = parser.feed(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x43]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::ControlRight);
    }

    #[test]
    fn test_ignore_sequences() {
        let mut parser = KeyParser::new();

        // Test sequence that should be ignored
        let events = parser.feed(&[0x1b, 0x5b, 0x45]);
        assert_eq!(events.len(), 0); // Should be ignored
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_mouse_events() {
        let mut parser = KeyParser::new();

        // Test mouse event sequence (ESC[M followed by 3 bytes)
        let events = parser.feed(&[0x1b, 0x5b, 0x4d, 0x20, 0x21, 0x22]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_csi_parameter_detection() {
        let parser = KeyParser::new();

        // Test parameter bytes
        assert!(parser.is_csi_parameter_byte(b'0'));
        assert!(parser.is_csi_parameter_byte(b'9'));
        assert!(parser.is_csi_parameter_byte(b';'));
        assert!(parser.is_csi_parameter_byte(b'?'));

        // Test non-parameter bytes
        assert!(!parser.is_csi_parameter_byte(b'A'));
        assert!(!parser.is_csi_parameter_byte(b'~'));
    }

    #[test]
    fn test_csi_final_byte_detection() {
        let parser = KeyParser::new();

        // Test final bytes
        assert!(parser.is_csi_final_byte(b'A'));
        assert!(parser.is_csi_final_byte(b'~'));
        assert!(parser.is_csi_final_byte(b'H'));

        // Test non-final bytes
        assert!(!parser.is_csi_final_byte(b'0'));
        assert!(!parser.is_csi_final_byte(b';'));
    }

    #[test]
    fn test_unknown_csi_sequence() {
        let mut parser = KeyParser::new();

        // Test unknown CSI sequence (ESC[999z)
        let events = parser.feed(&[0x1b, 0x5b, 0x39, 0x39, 0x39, 0x7a]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::NotDefined);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_incremental_feeding() {
        let mut parser = KeyParser::new();

        // Feed bytes one at a time for arrow key
        let mut events = parser.feed(&[0x1b]);
        assert_eq!(events.len(), 0);

        events = parser.feed(&[0x5b]);
        assert_eq!(events.len(), 0);

        events = parser.feed(&[0x41]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Up);
    }

    #[test]
    fn test_state_transitions() {
        let mut parser = KeyParser::new();

        // Test state transitions
        assert_eq!(parser.state, ParserState::Normal);

        parser.feed(&[0x1b]);
        assert_eq!(parser.state, ParserState::EscapeSequence);

        parser.feed(&[0x5b]);
        assert_eq!(parser.state, ParserState::CsiSequence);

        parser.feed(&[0x41]); // Complete Up arrow
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_bracketed_paste_edge_cases() {
        let mut parser = KeyParser::new();

        // Test bracketed paste with end sequence in content
        parser.feed(b"\x1b[200~");
        assert_eq!(parser.state, ParserState::BracketedPaste);

        // Add content that contains part of end sequence
        parser.feed(b"hello\x1b[201world");

        // Add actual end sequence
        let events = parser.feed(b"\x1b[201~");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        // The content should include the partial end sequence
        assert!(events[0].text.as_ref().unwrap().contains("hello"));
    }

    #[test]
    fn test_flush_bracketed_paste() {
        let mut parser = KeyParser::new();

        // Start bracketed paste but don't end it
        parser.feed(b"\x1b[200~hello world");
        assert_eq!(parser.state, ParserState::BracketedPaste);

        // Flush should emit the paste content
        let events = parser.flush();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_multiple_sequences_in_one_feed() {
        let mut parser = KeyParser::new();

        // Multiple complete sequences in one call
        let events = parser.feed(b"\x03\x1b[A\x1b[B\x04");
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].key, Key::ControlC);
        assert_eq!(events[1].key, Key::Up);
        assert_eq!(events[2].key, Key::Down);
        assert_eq!(events[3].key, Key::ControlD);
    }

    // Tests for special sequence handling (Task 4)

    #[test]
    fn test_cpr_response_detection() {
        let parser = KeyParser::new();

        // Valid CPR responses
        assert!(parser.is_cpr_response(b"\x1b[24;80R"));
        assert!(parser.is_cpr_response(b"\x1b[1;1R"));
        assert!(parser.is_cpr_response(b"\x1b[999;999R"));

        // Invalid CPR responses
        assert!(!parser.is_cpr_response(b"\x1b[24;80")); // Missing R
        assert!(!parser.is_cpr_response(b"\x1b[24R")); // Missing semicolon
        assert!(!parser.is_cpr_response(b"\x1b[;80R")); // Missing first number
        assert!(!parser.is_cpr_response(b"\x1b[24;R")); // Missing second number
        assert!(!parser.is_cpr_response(b"\x1b[24;80;1R")); // Too many semicolons
        assert!(!parser.is_cpr_response(b"\x1b[24aR")); // Invalid character
        assert!(!parser.is_cpr_response(b"[24;80R")); // Missing ESC
    }

    #[test]
    fn test_cpr_response_parsing() {
        let mut parser = KeyParser::new();

        // Test complete CPR response
        let events = parser.feed(b"\x1b[24;80R");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::CPRResponse);
        assert_eq!(events[0].raw_bytes, b"\x1b[24;80R");
        assert_eq!(parser.state, ParserState::Normal);

        // Test incremental CPR response
        let mut events = parser.feed(b"\x1b[");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::CsiSequence);

        events = parser.feed(b"24;80");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::CsiSequence);

        events = parser.feed(b"R");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::CPRResponse);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_sgr_mouse_parameter_detection() {
        let parser = KeyParser::new();

        // Valid SGR mouse parameter bytes
        assert!(parser.is_sgr_mouse_parameter_byte(b'0'));
        assert!(parser.is_sgr_mouse_parameter_byte(b'9'));
        assert!(parser.is_sgr_mouse_parameter_byte(b';'));

        // Invalid SGR mouse parameter bytes
        assert!(!parser.is_sgr_mouse_parameter_byte(b'M'));
        assert!(!parser.is_sgr_mouse_parameter_byte(b'm'));
        assert!(!parser.is_sgr_mouse_parameter_byte(b'A'));
        assert!(!parser.is_sgr_mouse_parameter_byte(b'<'));
    }

    #[test]
    fn test_sgr_mouse_sequence_validation() {
        let parser = KeyParser::new();

        // Valid SGR mouse sequences
        assert!(parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1;1M"));
        assert!(parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1;1m"));
        assert!(parser.is_valid_sgr_mouse_sequence(b"\x1b[<32;100;50M"));
        assert!(parser.is_valid_sgr_mouse_sequence(b"\x1b[<1;999;999m"));

        // Invalid SGR mouse sequences
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1M")); // Missing parameter
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1;1;2M")); // Too many parameters
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<;1;1M")); // Empty first parameter
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;;1M")); // Empty middle parameter
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1;M")); // Empty last parameter
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0;1;1X")); // Invalid final byte
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[<0a1;1M")); // Invalid character
        assert!(!parser.is_valid_sgr_mouse_sequence(b"\x1b[0;1;1M")); // Missing <
    }

    #[test]
    fn test_x10_mouse_events() {
        let mut parser = KeyParser::new();

        // Test X10 mouse event (ESC[M + 3 bytes)
        let events = parser.feed(b"\x1b[M !!");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(events[0].raw_bytes, b"\x1b[M !!");
        assert_eq!(parser.state, ParserState::Normal);

        // Test incremental X10 mouse event
        let mut events = parser.feed(b"\x1b[M");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::MouseEvent);

        events = parser.feed(b" ");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::MouseEvent);

        events = parser.feed(b"!!");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_sgr_mouse_events() {
        let mut parser = KeyParser::new();

        // Test SGR mouse press event
        let events = parser.feed(b"\x1b[<0;10;20M");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(events[0].raw_bytes, b"\x1b[<0;10;20M");
        assert_eq!(parser.state, ParserState::Normal);

        // Test SGR mouse release event
        let events = parser.feed(b"\x1b[<0;10;20m");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(events[0].raw_bytes, b"\x1b[<0;10;20m");
        assert_eq!(parser.state, ParserState::Normal);

        // Test incremental SGR mouse event
        let mut events = parser.feed(b"\x1b[<");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::MouseEvent);

        events = parser.feed(b"32;100;50");
        assert_eq!(events.len(), 0);
        assert_eq!(parser.state, ParserState::MouseEvent);

        events = parser.feed(b"M");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_invalid_sgr_mouse_events() {
        let mut parser = KeyParser::new();

        // Test invalid SGR mouse event (missing parameter)
        let events = parser.feed(b"\x1b[<0;10M");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::NotDefined);
        assert_eq!(parser.state, ParserState::Normal);

        // Test invalid character in SGR sequence
        parser.reset();
        parser.feed(b"\x1b[<");
        assert_eq!(parser.state, ParserState::MouseEvent);

        let events = parser.feed(b"0;10;20X"); // X is invalid
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::NotDefined);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_bracketed_paste_comprehensive() {
        let mut parser = KeyParser::new();

        // Test simple bracketed paste
        parser.feed(b"\x1b[200~");
        assert_eq!(parser.state, ParserState::BracketedPaste);

        parser.feed(b"Hello, World!");
        assert_eq!(parser.state, ParserState::BracketedPaste);

        let events = parser.feed(b"\x1b[201~");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(events[0].text.as_ref().unwrap(), "Hello, World!");
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_bracketed_paste_with_escape_sequences() {
        let mut parser = KeyParser::new();

        // Test bracketed paste containing escape sequences
        parser.feed(b"\x1b[200~");
        parser.feed(b"Text with \x1b[31mcolor\x1b[0m codes");
        let events = parser.feed(b"\x1b[201~");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(
            events[0].text.as_ref().unwrap(),
            "Text with \x1b[31mcolor\x1b[0m codes"
        );
    }

    #[test]
    fn test_bracketed_paste_partial_end_sequence() {
        let mut parser = KeyParser::new();

        // Test bracketed paste with partial end sequence in content
        parser.feed(b"\x1b[200~");
        parser.feed(b"Content with \x1b[201 partial end");
        let events = parser.feed(b"\x1b[201~");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert!(events[0]
            .text
            .as_ref()
            .unwrap()
            .contains("Content with \x1b[201 partial end"));
    }

    #[test]
    fn test_bracketed_paste_buffer_management() {
        let mut parser = KeyParser::new();

        // Test that long content is properly managed between buffers
        parser.feed(b"\x1b[200~");

        // Add content longer than the buffer management threshold
        let long_content = "x".repeat(100);
        parser.feed(long_content.as_bytes());

        // Verify we're still in paste mode
        assert_eq!(parser.state, ParserState::BracketedPaste);

        let events = parser.feed(b"\x1b[201~");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(events[0].text.as_ref().unwrap().len(), 100);
    }

    #[test]
    fn test_bracketed_paste_invalid_utf8() {
        let mut parser = KeyParser::new();

        // Test bracketed paste with invalid UTF-8
        parser.feed(b"\x1b[200~");
        parser.feed(&[0xff, 0xfe, 0xfd]); // Invalid UTF-8 bytes
        let events = parser.feed(b"\x1b[201~");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert!(events[0].text.is_none()); // Should be None for invalid UTF-8
        assert_eq!(events[0].raw_bytes, vec![0xff, 0xfe, 0xfd]);
    }

    #[test]
    fn test_bracketed_paste_flush() {
        let mut parser = KeyParser::new();

        // Test flushing incomplete bracketed paste
        parser.feed(b"\x1b[200~");
        parser.feed(b"Incomplete paste content");

        // Flush without end sequence
        let events = parser.flush();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::BracketedPaste);
        assert_eq!(events[0].text.as_ref().unwrap(), "Incomplete paste content");
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_special_csi_sequence_detection() {
        let parser = KeyParser::new();

        // Test Device Status Report responses
        assert!(parser.is_special_csi_sequence(b"\x1b[0n"));
        assert!(parser.is_special_csi_sequence(b"\x1b[3n"));
        assert!(parser.is_special_csi_sequence(b"\x1b[?1;2c"));

        // Test SGR mouse sequences
        assert!(parser.is_special_csi_sequence(b"\x1b[<0;1;1M"));
        assert!(parser.is_special_csi_sequence(b"\x1b[<32;100;50m"));

        // Test non-special sequences
        assert!(!parser.is_special_csi_sequence(b"\x1b[A"));
        assert!(!parser.is_special_csi_sequence(b"\x1b[1;2A"));
        assert!(!parser.is_special_csi_sequence(b"\x1b[999z"));
    }

    #[test]
    fn test_mouse_event_state_transitions() {
        let mut parser = KeyParser::new();

        // Test transition to mouse event state
        parser.feed(b"\x1b[M");
        assert_eq!(parser.state, ParserState::MouseEvent);

        // Reset and test SGR mouse transition
        parser.reset();
        parser.feed(b"\x1b[<");
        assert_eq!(parser.state, ParserState::MouseEvent);

        // Test return to normal state after complete sequence
        parser.feed(b"0;1;1M");
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_unknown_mouse_format_timeout() {
        let mut parser = KeyParser::new();

        // Start with a valid mouse sequence first
        parser.feed(b"\x1b[M"); // This will transition to MouseEvent state
        assert_eq!(parser.state, ParserState::MouseEvent);

        // Add exactly 3 bytes (normal X10 format)
        let events = parser.feed(b"abc");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::Vt100MouseEvent);
        assert_eq!(parser.state, ParserState::Normal);

        // Test SGR mouse format with invalid character
        parser.feed(b"\x1b[<"); // This will transition to MouseEvent state
        assert_eq!(parser.state, ParserState::MouseEvent);

        // Add valid digits and semicolons first
        let events = parser.feed(b"0;10;20");
        assert_eq!(events.len(), 0); // Should still be accumulating
        assert_eq!(parser.state, ParserState::MouseEvent);

        // Add invalid character (not M or m)
        let events = parser.feed(b"X"); // Invalid final character
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, Key::NotDefined);
        assert_eq!(parser.state, ParserState::Normal);
    }

    #[test]
    fn test_mixed_special_sequences() {
        let mut parser = KeyParser::new();

        // Test mix of CPR, mouse events, and bracketed paste
        let input = b"\x1b[24;80R\x1b[<0;10;20M\x1b[200~hello\x1b[201~";
        let events = parser.feed(input);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].key, Key::CPRResponse);
        assert_eq!(events[1].key, Key::Vt100MouseEvent);
        assert_eq!(events[2].key, Key::BracketedPaste);
        assert_eq!(events[2].text.as_ref().unwrap(), "hello");
    }

    #[test]
    fn test_incremental_special_sequence_parsing() {
        let mut parser = KeyParser::new();

        // Test incremental parsing of CPR response
        let cpr_bytes = b"\x1b[24;80R";
        for &byte in cpr_bytes {
            let events = parser.feed(&[byte]);
            if byte == b'R' {
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].key, Key::CPRResponse);
            } else {
                assert_eq!(events.len(), 0);
            }
        }

        // Test incremental parsing of SGR mouse event
        parser.reset();
        let mouse_bytes = b"\x1b[<32;100;50M";
        for &byte in mouse_bytes {
            let events = parser.feed(&[byte]);
            if byte == b'M' {
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].key, Key::Vt100MouseEvent);
            } else {
                assert_eq!(events.len(), 0);
            }
        }
    }
}
