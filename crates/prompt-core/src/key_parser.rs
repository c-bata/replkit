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
                        events.push(KeyEvent::simple(longest.key, self.buffer[..longest.consumed_bytes].to_vec()));
                        
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
                    // Emit as unknown sequence
                    events.push(KeyEvent::simple(Key::NotDefined, self.buffer.clone()));
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
        
        // Simple mouse event handling - for now, just accumulate until we have enough bytes
        // Real mouse parsing would be more sophisticated
        if self.buffer.len() >= 6 {
            // Emit mouse event and reset
            events.push(KeyEvent::simple(Key::Vt100MouseEvent, self.buffer.clone()));
            self.reset_to_normal();
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
                self.paste_buffer.extend_from_slice(&self.buffer[..self.buffer.len() - end_seq_len]);
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
                events.push(KeyEvent::simple(Key::BracketedPaste, self.paste_buffer.clone()));
            }
            
            self.paste_buffer.clear();
            self.reset_to_normal();
        } else if self.buffer.len() >= 6 {
            // If buffer is getting long and we haven't found the end sequence,
            // move some content to paste_buffer to avoid keeping too much in buffer
            let keep_len = 5; // Keep last 5 bytes in buffer (enough for partial end sequence)
            let move_len = self.buffer.len() - keep_len;
            self.paste_buffer.extend_from_slice(&self.buffer[..move_len]);
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
            KeyEvent::with_text(Key::NotDefined, vec![byte], String::from_utf8_lossy(&[byte]).to_string())
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
            assert_eq!(events[i].text.as_ref().unwrap(), &String::from_utf8_lossy(&[ch]));
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
}