#!/usr/bin/env python3
"""
Test suite for the Python key parser bindings.

This module tests the PyO3-based Python bindings for the replkit-core
key input parser, verifying that all functionality works correctly
from Python.
"""

import pytest
import replkit


class TestKey:
    """Test the Key enum functionality."""
    
    def test_key_string_representation(self):
        """Test that keys have proper string representations."""
        assert str(replkit.Key.ControlC) == "Ctrl+C"
        assert str(replkit.Key.Up) == "Up"
        assert str(replkit.Key.F1) == "F1"
        assert str(replkit.Key.Enter) == "Enter"
        
    def test_key_repr(self):
        """Test that keys have proper debug representations."""
        assert repr(replkit.Key.ControlC) == "Key.Ctrl+C"
        assert repr(replkit.Key.Up) == "Key.Up"
        
    def test_key_equality(self):
        """Test that key equality works correctly."""
        assert replkit.Key.ControlC == replkit.Key.ControlC
        assert replkit.Key.ControlC != replkit.Key.ControlD
        
    def test_module_constants(self):
        """Test that module-level key constants are available."""
        assert replkit.CTRL_C == replkit.Key.ControlC
        assert replkit.UP == replkit.Key.Up
        assert replkit.ENTER == replkit.Key.Enter


class TestKeyEvent:
    """Test the KeyEvent class functionality."""
    
    def test_key_event_creation(self):
        """Test creating KeyEvent instances."""
        raw_bytes = b'\x03'
        event = replkit.KeyEvent(replkit.Key.ControlC, raw_bytes, None)
        
        assert event.key == replkit.Key.ControlC
        assert event.raw_bytes == raw_bytes
        assert event.text is None
        assert not event.has_text()
        assert event.text_or_empty() == ""
        
    def test_key_event_with_text(self):
        """Test KeyEvent with text content."""
        raw_bytes = b'a'
        text = "a"
        event = replkit.KeyEvent(replkit.Key.NotDefined, raw_bytes, text)
        
        assert event.key == replkit.Key.NotDefined
        assert event.raw_bytes == raw_bytes
        assert event.text == text
        assert event.has_text()
        assert event.text_or_empty() == text
        
    def test_key_event_string_representation(self):
        """Test KeyEvent string representations."""
        # Event without text
        event1 = replkit.KeyEvent(replkit.Key.ControlC, b'\x03', None)
        assert "KeyEvent(key=Ctrl+C)" in str(event1)
        
        # Event with text
        event2 = replkit.KeyEvent(replkit.Key.NotDefined, b'a', "a")
        assert "KeyEvent(key=NotDefined, text='a')" in str(event2)


class TestKeyParser:
    """Test the KeyParser class functionality."""
    
    def test_parser_creation(self):
        """Test creating KeyParser instances."""
        parser = replkit.KeyParser()
        assert str(parser) == "KeyParser"
        assert repr(parser) == "KeyParser()"
        
    def test_simple_control_characters(self):
        """Test parsing simple control characters."""
        parser = replkit.KeyParser()
        
        # Test Ctrl+C
        events = parser.feed(b'\x03')
        assert len(events) == 1
        assert events[0].key == replkit.Key.ControlC
        assert events[0].raw_bytes == b'\x03'
        
        # Test Tab
        events = parser.feed(b'\x09')
        assert len(events) == 1
        assert events[0].key == replkit.Key.Tab
        
    def test_arrow_keys(self):
        """Test parsing arrow key sequences."""
        parser = replkit.KeyParser()
        
        # Test Up arrow (ESC[A)
        events = parser.feed(b'\x1b[A')
        assert len(events) == 1
        assert events[0].key == replkit.Key.Up
        assert events[0].raw_bytes == b'\x1b[A'
        
        # Test Down arrow
        events = parser.feed(b'\x1b[B')
        assert len(events) == 1
        assert events[0].key == replkit.Key.Down
        
    def test_partial_sequences(self):
        """Test handling of partial byte sequences."""
        parser = replkit.KeyParser()
        
        # Feed partial escape sequence
        events = parser.feed(b'\x1b')
        assert len(events) == 0  # No complete events yet
        
        events = parser.feed(b'[')
        assert len(events) == 0  # Still partial
        
        events = parser.feed(b'A')
        assert len(events) == 1  # Now complete
        assert events[0].key == replkit.Key.Up
        
    def test_mixed_input(self):
        """Test parsing mixed input with different key types."""
        parser = replkit.KeyParser()
        
        # Mix of control chars, escape sequences, and regular chars
        input_data = b'\x03\x1b[A\x61\x1b[B'
        events = parser.feed(input_data)
        
        assert len(events) == 4
        assert events[0].key == replkit.Key.ControlC
        assert events[1].key == replkit.Key.Up
        assert events[2].key == replkit.Key.NotDefined  # 'a'
        assert events[2].text == "a"
        assert events[3].key == replkit.Key.Down
        
    def test_function_keys(self):
        """Test parsing function keys."""
        parser = replkit.KeyParser()
        
        # Test F1 (ESC OP)
        events = parser.feed(b'\x1bOP')
        assert len(events) == 1
        assert events[0].key == replkit.Key.F1
        
    def test_bracketed_paste(self):
        """Test bracketed paste mode handling."""
        parser = replkit.KeyParser()
        
        # Complete bracketed paste sequence
        paste_data = b'\x1b[200~hello world\x1b[201~'
        events = parser.feed(paste_data)
        
        assert len(events) == 1
        assert events[0].key == replkit.Key.BracketedPaste
        assert events[0].text == "hello world"
        
    def test_flush_functionality(self):
        """Test flushing incomplete sequences."""
        parser = replkit.KeyParser()
        
        # Leave parser with partial sequence
        parser.feed(b'\x1b[')
        
        # Flush should handle the partial sequence
        events = parser.flush()
        assert len(events) > 0  # Should produce some events
        
    def test_reset_functionality(self):
        """Test parser reset functionality."""
        parser = replkit.KeyParser()
        
        # Put parser in non-normal state
        parser.feed(b'\x1b[')
        
        # Reset should clear everything
        parser.reset()
        
        # Parser should work normally after reset
        events = parser.feed(b'\x03')
        assert len(events) == 1
        assert events[0].key == replkit.Key.ControlC
        
    def test_empty_input(self):
        """Test handling of empty input."""
        parser = replkit.KeyParser()
        
        events = parser.feed(b'')
        assert len(events) == 0
        
    def test_error_handling(self):
        """Test that errors are properly converted to Python exceptions."""
        parser = replkit.KeyParser()
        
        # Normal operations should not raise exceptions
        try:
            events = parser.feed(b'\x03')
            parser.flush()
            parser.reset()
        except Exception as e:
            pytest.fail(f"Normal operations should not raise exceptions: {e}")
            
    def test_incremental_feeding(self):
        """Test feeding bytes incrementally."""
        parser = replkit.KeyParser()
        
        # Feed bytes one at a time for arrow key
        events = parser.feed(b'\x1b')
        assert len(events) == 0
        
        events = parser.feed(b'[')
        assert len(events) == 0
        
        events = parser.feed(b'A')
        assert len(events) == 1
        assert events[0].key == replkit.Key.Up
        
    def test_printable_characters(self):
        """Test handling of printable characters."""
        parser = replkit.KeyParser()
        
        # Test regular ASCII characters
        events = parser.feed(b'hello')
        assert len(events) == 5
        
        for i, char in enumerate(b'hello'):
            assert events[i].key == replkit.Key.NotDefined
            assert events[i].raw_bytes == bytes([char])
            assert events[i].text == chr(char)
            
    def test_modifier_combinations(self):
        """Test parsing modifier key combinations."""
        parser = replkit.KeyParser()
        
        # Test Shift+Up (ESC[1;2A)
        events = parser.feed(b'\x1b[1;2A')
        assert len(events) == 1
        assert events[0].key == replkit.Key.ShiftUp
        
        # Test Ctrl+Right (ESC[1;5C)
        events = parser.feed(b'\x1b[1;5C')
        assert len(events) == 1
        assert events[0].key == replkit.Key.ControlRight


if __name__ == "__main__":
    pytest.main([__file__])