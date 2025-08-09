//! Trie-based sequence matcher for efficient key sequence parsing.
//!
//! This module provides a Trie data structure for mapping byte sequences to keys
//! and determining if partial sequences could be prefixes of longer valid sequences.
//! This is crucial for the state machine to know whether to wait for more bytes
//! or process what it has.

use crate::key::Key;
use std::collections::BTreeMap;

/// A node in the Trie structure for sequence matching.
#[derive(Debug, Clone)]
struct TrieNode {
    /// The key associated with this node if it represents a complete sequence
    key: Option<Key>,
    /// Child nodes indexed by the next byte in the sequence
    children: BTreeMap<u8, TrieNode>,
}

impl TrieNode {
    /// Create a new empty TrieNode
    fn new() -> Self {
        Self {
            key: None,
            children: BTreeMap::new(),
        }
    }
}

/// Result of matching a byte sequence against the Trie
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    /// Found an exact match for the sequence
    Exact(Key),
    /// The sequence is a prefix of one or more longer sequences
    Prefix,
    /// No match possible - the sequence doesn't match any known pattern
    NoMatch,
}

/// Result of finding the longest valid sequence from the start of input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongestMatchResult {
    /// The key that was matched
    pub key: Key,
    /// Number of bytes consumed from the input
    pub consumed_bytes: usize,
}

/// Trie-based sequence matcher for efficient key sequence parsing
pub struct SequenceMatcher {
    /// Root node of the Trie
    root: TrieNode,
}

impl SequenceMatcher {
    /// Create a new SequenceMatcher with all standard key sequences
    pub fn new() -> Self {
        let mut matcher = Self {
            root: TrieNode::new(),
        };
        matcher.build_standard_sequences();
        matcher
    }

    /// Primary API: efficient single-pass matching
    /// Returns whether the given bytes represent an exact match, prefix, or no match
    pub fn match_sequence(&self, bytes: &[u8]) -> MatchResult {
        if bytes.is_empty() {
            return MatchResult::NoMatch;
        }
        
        if let Some(node) = self.find_node(bytes) {
            if node.key.is_some() {
                MatchResult::Exact(node.key.unwrap())
            } else {
                MatchResult::Prefix
            }
        } else {
            MatchResult::NoMatch
        }
    }

    /// Find the longest valid sequence from the start of bytes
    /// This is used for fallback processing when we need to consume partial matches
    pub fn find_longest_match(&self, bytes: &[u8]) -> Option<LongestMatchResult> {
        let mut longest_match = None;
        let mut current_node = &self.root;

        for (i, &byte) in bytes.iter().enumerate() {
            if let Some(child) = current_node.children.get(&byte) {
                current_node = child;
                if let Some(key) = current_node.key {
                    longest_match = Some(LongestMatchResult {
                        key,
                        consumed_bytes: i + 1,
                    });
                }
            } else {
                break;
            }
        }

        longest_match
    }

    /// Internal helper to traverse the Trie and find the node for a given sequence
    fn find_node(&self, bytes: &[u8]) -> Option<&TrieNode> {
        let mut current = &self.root;
        for &byte in bytes {
            current = current.children.get(&byte)?;
        }
        Some(current)
    }

    /// Register a custom sequence mapping
    /// This allows dynamically adding new key sequences
    pub fn insert(&mut self, bytes: &[u8], key: Key) {
        let mut current = &mut self.root;
        for &byte in bytes {
            current = current.children.entry(byte).or_insert_with(TrieNode::new);
        }
        current.key = Some(key);
    }

    /// Build the standard key sequence mappings based on go-prompt's ASCIISequences
    fn build_standard_sequences(&mut self) {
        // Control characters (single byte)
        self.insert(&[0x1b], Key::Escape);
        self.insert(&[0x00], Key::ControlSpace);
        self.insert(&[0x01], Key::ControlA);
        self.insert(&[0x02], Key::ControlB);
        self.insert(&[0x03], Key::ControlC);
        self.insert(&[0x04], Key::ControlD);
        self.insert(&[0x05], Key::ControlE);
        self.insert(&[0x06], Key::ControlF);
        self.insert(&[0x07], Key::ControlG);
        self.insert(&[0x08], Key::ControlH);
        self.insert(&[0x09], Key::Tab);
        self.insert(&[0x0a], Key::Enter);
        self.insert(&[0x0b], Key::ControlK);
        self.insert(&[0x0c], Key::ControlL);
        self.insert(&[0x0d], Key::ControlM);
        self.insert(&[0x0e], Key::ControlN);
        self.insert(&[0x0f], Key::ControlO);
        self.insert(&[0x10], Key::ControlP);
        self.insert(&[0x11], Key::ControlQ);
        self.insert(&[0x12], Key::ControlR);
        self.insert(&[0x13], Key::ControlS);
        self.insert(&[0x14], Key::ControlT);
        self.insert(&[0x15], Key::ControlU);
        self.insert(&[0x16], Key::ControlV);
        self.insert(&[0x17], Key::ControlW);
        self.insert(&[0x18], Key::ControlX);
        self.insert(&[0x19], Key::ControlY);
        self.insert(&[0x1a], Key::ControlZ);
        self.insert(&[0x1c], Key::ControlBackslash);
        self.insert(&[0x1d], Key::ControlSquareClose);
        self.insert(&[0x1e], Key::ControlCircumflex);
        self.insert(&[0x1f], Key::ControlUnderscore);
        self.insert(&[0x7f], Key::Backspace);

        // Arrow keys (standard VT100)
        self.insert(&[0x1b, 0x5b, 0x41], Key::Up);
        self.insert(&[0x1b, 0x5b, 0x42], Key::Down);
        self.insert(&[0x1b, 0x5b, 0x43], Key::Right);
        self.insert(&[0x1b, 0x5b, 0x44], Key::Left);

        // Arrow keys (alternative sequences for some terminals)
        self.insert(&[0x1b, 0x4f, 0x41], Key::Up);
        self.insert(&[0x1b, 0x4f, 0x42], Key::Down);
        self.insert(&[0x1b, 0x4f, 0x43], Key::Right);
        self.insert(&[0x1b, 0x4f, 0x44], Key::Left);

        // Home and End keys (multiple variants)
        self.insert(&[0x1b, 0x5b, 0x48], Key::Home);
        self.insert(&[0x1b, 0x30, 0x48], Key::Home);
        self.insert(&[0x1b, 0x5b, 0x46], Key::End);
        self.insert(&[0x1b, 0x30, 0x46], Key::End);
        self.insert(&[0x1b, 0x5b, 0x31, 0x7e], Key::Home);
        self.insert(&[0x1b, 0x5b, 0x34, 0x7e], Key::End);
        self.insert(&[0x1b, 0x5b, 0x37, 0x7e], Key::Home);
        self.insert(&[0x1b, 0x5b, 0x38, 0x7e], Key::End);

        // Delete keys
        self.insert(&[0x1b, 0x5b, 0x33, 0x7e], Key::Delete);
        self.insert(&[0x1b, 0x5b, 0x33, 0x3b, 0x32, 0x7e], Key::ShiftDelete);
        self.insert(&[0x1b, 0x5b, 0x33, 0x3b, 0x35, 0x7e], Key::ControlDelete);

        // Page Up/Down
        self.insert(&[0x1b, 0x5b, 0x35, 0x7e], Key::PageUp);
        self.insert(&[0x1b, 0x5b, 0x36, 0x7e], Key::PageDown);

        // Insert and BackTab
        self.insert(&[0x1b, 0x5b, 0x32, 0x7e], Key::Insert);
        self.insert(&[0x1b, 0x5b, 0x5a], Key::BackTab);

        // Function keys F1-F4 (standard VT100)
        self.insert(&[0x1b, 0x4f, 0x50], Key::F1);
        self.insert(&[0x1b, 0x4f, 0x51], Key::F2);
        self.insert(&[0x1b, 0x4f, 0x52], Key::F3);
        self.insert(&[0x1b, 0x4f, 0x53], Key::F4);

        // Function keys F1-F5 (Linux console variants)
        self.insert(&[0x1b, 0x4f, 0x50, 0x41], Key::F1);
        self.insert(&[0x1b, 0x5b, 0x5b, 0x42], Key::F2);
        self.insert(&[0x1b, 0x5b, 0x5b, 0x43], Key::F3);
        self.insert(&[0x1b, 0x5b, 0x5b, 0x44], Key::F4);
        self.insert(&[0x1b, 0x5b, 0x5b, 0x45], Key::F5);

        // Function keys F1-F4 (rxvt-unicode variants)
        self.insert(&[0x1b, 0x5b, 0x31, 0x31, 0x7e], Key::F1);
        self.insert(&[0x1b, 0x5b, 0x31, 0x32, 0x7e], Key::F2);
        self.insert(&[0x1b, 0x5b, 0x31, 0x33, 0x7e], Key::F3);
        self.insert(&[0x1b, 0x5b, 0x31, 0x34, 0x7e], Key::F4);

        // Function keys F5-F12
        self.insert(&[0x1b, 0x5b, 0x31, 0x35, 0x7e], Key::F5);
        self.insert(&[0x1b, 0x5b, 0x31, 0x37, 0x7e], Key::F6);
        self.insert(&[0x1b, 0x5b, 0x31, 0x38, 0x7e], Key::F7);
        self.insert(&[0x1b, 0x5b, 0x31, 0x39, 0x7e], Key::F8);
        self.insert(&[0x1b, 0x5b, 0x32, 0x30, 0x7e], Key::F9);
        self.insert(&[0x1b, 0x5b, 0x32, 0x31, 0x7e], Key::F10);
        self.insert(&[0x1b, 0x5b, 0x32, 0x33, 0x7e], Key::F11);
        self.insert(&[0x1b, 0x5b, 0x32, 0x34, 0x7e, 0x8], Key::F12);

        // Function keys F13-F20 (basic sequences)
        self.insert(&[0x1b, 0x5b, 0x32, 0x35, 0x7e], Key::F13);
        self.insert(&[0x1b, 0x5b, 0x32, 0x36, 0x7e], Key::F14);
        self.insert(&[0x1b, 0x5b, 0x32, 0x38, 0x7e], Key::F15);
        self.insert(&[0x1b, 0x5b, 0x32, 0x39, 0x7e], Key::F16);
        self.insert(&[0x1b, 0x5b, 0x33, 0x31, 0x7e], Key::F17);
        self.insert(&[0x1b, 0x5b, 0x33, 0x32, 0x7e], Key::F18);
        self.insert(&[0x1b, 0x5b, 0x33, 0x33, 0x7e], Key::F19);
        self.insert(&[0x1b, 0x5b, 0x33, 0x34, 0x7e], Key::F20);

        // Function keys F13-F24 (Xterm variants)
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x50], Key::F13);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x51], Key::F14);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x52], Key::F16);
        self.insert(&[0x1b, 0x5b, 0x31, 0x35, 0x3b, 0x32, 0x7e], Key::F17);
        self.insert(&[0x1b, 0x5b, 0x31, 0x37, 0x3b, 0x32, 0x7e], Key::F18);
        self.insert(&[0x1b, 0x5b, 0x31, 0x38, 0x3b, 0x32, 0x7e], Key::F19);
        self.insert(&[0x1b, 0x5b, 0x31, 0x39, 0x3b, 0x32, 0x7e], Key::F20);
        self.insert(&[0x1b, 0x5b, 0x32, 0x30, 0x3b, 0x32, 0x7e], Key::F21);
        self.insert(&[0x1b, 0x5b, 0x32, 0x31, 0x3b, 0x32, 0x7e], Key::F22);
        self.insert(&[0x1b, 0x5b, 0x32, 0x33, 0x3b, 0x32, 0x7e], Key::F23);
        self.insert(&[0x1b, 0x5b, 0x32, 0x34, 0x3b, 0x32, 0x7e], Key::F24);

        // Control + Arrow keys
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x41], Key::ControlUp);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x42], Key::ControlDown);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x43], Key::ControlRight);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x44], Key::ControlLeft);

        // Alternative Control + Arrow keys
        self.insert(&[0x1b, 0x5b, 0x35, 0x41], Key::ControlUp);
        self.insert(&[0x1b, 0x5b, 0x35, 0x42], Key::ControlDown);
        self.insert(&[0x1b, 0x5b, 0x35, 0x43], Key::ControlRight);
        self.insert(&[0x1b, 0x5b, 0x35, 0x44], Key::ControlLeft);

        // rxvt Control + Arrow keys
        self.insert(&[0x1b, 0x5b, 0x4f, 0x63], Key::ControlRight);
        self.insert(&[0x1b, 0x5b, 0x4f, 0x64], Key::ControlLeft);

        // Shift + Arrow keys
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x41], Key::ShiftUp);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x42], Key::ShiftDown);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x43], Key::ShiftRight);
        self.insert(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x44], Key::ShiftLeft);

        // Ignore sequences (terminal-specific sequences that should be ignored)
        self.insert(&[0x1b, 0x5b, 0x45], Key::Ignore); // Xterm
        self.insert(&[0x1b, 0x5b, 0x46], Key::Ignore); // Linux console
    }
}

impl Default for SequenceMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let matcher = SequenceMatcher::new();
        
        // Test control characters
        assert_eq!(matcher.match_sequence(&[0x03]), MatchResult::Exact(Key::ControlC));
        assert_eq!(matcher.match_sequence(&[0x1b]), MatchResult::Exact(Key::Escape));
        
        // Test arrow keys
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x41]), MatchResult::Exact(Key::Up));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x42]), MatchResult::Exact(Key::Down));
        
        // Test function keys
        assert_eq!(matcher.match_sequence(&[0x1b, 0x4f, 0x50]), MatchResult::Exact(Key::F1));
    }

    #[test]
    fn test_prefix_match() {
        let matcher = SequenceMatcher::new();
        
        // ESC alone is exact, but ESC[ is a prefix
        assert_eq!(matcher.match_sequence(&[0x1b]), MatchResult::Exact(Key::Escape));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b]), MatchResult::Prefix);
        
        // Partial arrow key sequences
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b]), MatchResult::Prefix);
        assert_eq!(matcher.match_sequence(&[0x1b, 0x4f]), MatchResult::Prefix);
    }

    #[test]
    fn test_no_match() {
        let matcher = SequenceMatcher::new();
        
        // Invalid sequences
        assert_eq!(matcher.match_sequence(&[0xff]), MatchResult::NoMatch);
        assert_eq!(matcher.match_sequence(&[0x1b, 0xff]), MatchResult::NoMatch);
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0xff]), MatchResult::NoMatch);
    }

    #[test]
    fn test_longest_match() {
        let matcher = SequenceMatcher::new();
        
        // Test finding longest match in a sequence
        let result = matcher.find_longest_match(&[0x1b, 0x5b, 0x41, 0x42]);
        assert_eq!(result, Some(LongestMatchResult {
            key: Key::Up,
            consumed_bytes: 3,
        }));
        
        // Test with control character at start
        let result = matcher.find_longest_match(&[0x03, 0x1b, 0x5b]);
        assert_eq!(result, Some(LongestMatchResult {
            key: Key::ControlC,
            consumed_bytes: 1,
        }));
        
        // Test with no match
        let result = matcher.find_longest_match(&[0xff, 0xfe]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_custom_sequence() {
        let mut matcher = SequenceMatcher::new();
        
        // Insert a custom sequence
        matcher.insert(b"gg", Key::F24); // Using F24 as a test key
        
        assert_eq!(matcher.match_sequence(b"g"), MatchResult::Prefix);
        assert_eq!(matcher.match_sequence(b"gg"), MatchResult::Exact(Key::F24));
        assert_eq!(matcher.match_sequence(b"ggg"), MatchResult::NoMatch);
    }

    #[test]
    fn test_overlapping_sequences() {
        let matcher = SequenceMatcher::new();
        
        // ESC is both a complete sequence and a prefix
        assert_eq!(matcher.match_sequence(&[0x1b]), MatchResult::Exact(Key::Escape));
        
        // But ESC[ is only a prefix (no key assigned to it)
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b]), MatchResult::Prefix);
        
        // And ESC[A is a complete sequence
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x41]), MatchResult::Exact(Key::Up));
    }

    #[test]
    fn test_multiple_variants() {
        let matcher = SequenceMatcher::new();
        
        // Test that multiple variants of the same key work
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x48]), MatchResult::Exact(Key::Home));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x31, 0x7e]), MatchResult::Exact(Key::Home));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x37, 0x7e]), MatchResult::Exact(Key::Home));
    }

    #[test]
    fn test_function_key_variants() {
        let matcher = SequenceMatcher::new();
        
        // Test different F1 variants
        assert_eq!(matcher.match_sequence(&[0x1b, 0x4f, 0x50]), MatchResult::Exact(Key::F1));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x4f, 0x50, 0x41]), MatchResult::Exact(Key::F1));
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x31, 0x31, 0x7e]), MatchResult::Exact(Key::F1));
    }

    #[test]
    fn test_modifier_combinations() {
        let matcher = SequenceMatcher::new();
        
        // Test Shift + Arrow keys
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x41]), MatchResult::Exact(Key::ShiftUp));
        
        // Test Control + Arrow keys
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x31, 0x3b, 0x35, 0x41]), MatchResult::Exact(Key::ControlUp));
    }

    #[test]
    fn test_ignore_sequences() {
        let matcher = SequenceMatcher::new();
        
        // Test sequences that should be ignored
        assert_eq!(matcher.match_sequence(&[0x1b, 0x5b, 0x45]), MatchResult::Exact(Key::Ignore));
    }

    #[test]
    fn test_empty_sequence() {
        let matcher = SequenceMatcher::new();
        
        // Empty sequence should not match anything
        assert_eq!(matcher.match_sequence(&[]), MatchResult::NoMatch);
        assert_eq!(matcher.find_longest_match(&[]), None);
    }

    #[test]
    fn test_complex_longest_match_scenarios() {
        let matcher = SequenceMatcher::new();
        
        // Test sequence with multiple potential matches
        let input = &[0x1b, 0x5b, 0x31, 0x3b, 0x32, 0x41, 0x03]; // Shift+Up followed by Ctrl+C
        let result = matcher.find_longest_match(input);
        assert_eq!(result, Some(LongestMatchResult {
            key: Key::ShiftUp,
            consumed_bytes: 6,
        }));
        
        // Test with partial sequence at end
        let input = &[0x03, 0x1b, 0x5b]; // Ctrl+C followed by incomplete escape sequence
        let result = matcher.find_longest_match(input);
        assert_eq!(result, Some(LongestMatchResult {
            key: Key::ControlC,
            consumed_bytes: 1,
        }));
    }

    #[test]
    fn test_all_control_characters() {
        let matcher = SequenceMatcher::new();
        
        // Test all basic control characters
        let control_tests = [
            (0x00, Key::ControlSpace),
            (0x01, Key::ControlA),
            (0x02, Key::ControlB),
            (0x03, Key::ControlC),
            (0x04, Key::ControlD),
            (0x05, Key::ControlE),
            (0x06, Key::ControlF),
            (0x07, Key::ControlG),
            (0x08, Key::ControlH),
            (0x09, Key::Tab),
            (0x0a, Key::Enter),
            (0x0b, Key::ControlK),
            (0x0c, Key::ControlL),
            (0x0d, Key::ControlM),
            (0x0e, Key::ControlN),
            (0x0f, Key::ControlO),
            (0x10, Key::ControlP),
            (0x11, Key::ControlQ),
            (0x12, Key::ControlR),
            (0x13, Key::ControlS),
            (0x14, Key::ControlT),
            (0x15, Key::ControlU),
            (0x16, Key::ControlV),
            (0x17, Key::ControlW),
            (0x18, Key::ControlX),
            (0x19, Key::ControlY),
            (0x1a, Key::ControlZ),
            (0x1c, Key::ControlBackslash),
            (0x1d, Key::ControlSquareClose),
            (0x1e, Key::ControlCircumflex),
            (0x1f, Key::ControlUnderscore),
            (0x7f, Key::Backspace),
        ];
        
        for (byte, expected_key) in control_tests {
            assert_eq!(matcher.match_sequence(&[byte]), MatchResult::Exact(expected_key));
        }
    }

    #[test]
    fn test_prefix_detection_comprehensive() {
        let matcher = SequenceMatcher::new();
        
        // Test various prefix scenarios
        let prefix_tests = [
            (&[0x1b][..], MatchResult::Exact(Key::Escape)), // ESC is both exact and prefix
            (&[0x1b, 0x5b][..], MatchResult::Prefix),       // ESC[ is prefix only
            (&[0x1b, 0x4f][..], MatchResult::Prefix),       // ESC O is prefix only
            (&[0x1b, 0x5b, 0x31][..], MatchResult::Prefix), // ESC[1 is prefix
            (&[0x1b, 0x5b, 0x31, 0x3b][..], MatchResult::Prefix), // ESC[1; is prefix
        ];
        
        for (bytes, expected) in prefix_tests {
            assert_eq!(matcher.match_sequence(bytes), expected);
        }
    }

    #[test]
    fn test_insert_overwrites_existing() {
        let mut matcher = SequenceMatcher::new();
        
        // Insert a custom mapping that overwrites an existing one
        matcher.insert(&[0x03], Key::F1); // Override Ctrl+C with F1
        
        assert_eq!(matcher.match_sequence(&[0x03]), MatchResult::Exact(Key::F1));
    }

    #[test]
    fn test_single_byte_sequences() {
        let matcher = SequenceMatcher::new();
        
        // Test that single-byte sequences work correctly
        assert_eq!(matcher.match_sequence(&[0x1b]), MatchResult::Exact(Key::Escape));
        assert_eq!(matcher.match_sequence(&[0x09]), MatchResult::Exact(Key::Tab));
        assert_eq!(matcher.match_sequence(&[0x0a]), MatchResult::Exact(Key::Enter));
    }
}