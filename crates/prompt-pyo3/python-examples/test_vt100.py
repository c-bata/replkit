#!/usr/bin/env python3
"""
Test script for the Python key demo to verify it works correctly
without requiring actual terminal interaction.
"""

import sys
from typing import Any

try:
    import prompt
    # Try to access the classes to verify they're available
    KeyParser = getattr(prompt, 'KeyParser', None)
    KeyEvent = getattr(prompt, 'KeyEvent', None) 
    Key = getattr(prompt, 'Key', None)
    
    if not all([KeyParser, KeyEvent, Key]):
        raise ImportError("Required classes not found in prompt module")
        
except ImportError as e:
    print(f"Error importing prompt module: {e}")
    sys.exit(1)

# Import the demo components
from python_example import (
    KeyEventHandler, 
    format_raw_bytes, 
    create_key_display_callback,
    create_statistics_callback
)


def test_key_event_handling():
    """Test the key event handling functionality."""
    print("Testing key event handling...")
    
    # Create handler and parser
    handler = KeyEventHandler()
    parser = KeyParser()
    
    # Add callbacks
    display_callback = create_key_display_callback()
    stats_callback = create_statistics_callback()
    
    handler.add_callback(display_callback)
    handler.add_callback(stats_callback)
    
    # Test various key sequences
    test_sequences = [
        (b'\x03', "Ctrl+C"),
        (b'\x1b[A', "Up arrow"),
        (b'\x1b[B', "Down arrow"),
        (b'\x09', "Tab"),
        (b'a', "Letter 'a'"),
        (b'\x1bOP', "F1 key"),
        (b'\x1b[1;2A', "Shift+Up"),
    ]
    
    print("\nTesting key sequences:")
    print("-" * 50)
    
    for sequence, description in test_sequences:
        print(f"\nTesting {description}:")
        try:
            events = parser.feed(sequence)
            for event in events:
                handler.handle_event(event)
                
                # Stop on Ctrl+C to test exit handling
                if hasattr(prompt, 'Key') and event.key == prompt.Key.ControlC:
                    print("Ctrl+C detected - would exit in real demo")
                    
        except Exception as e:
            print(f"Error processing {description}: {e}")
            return False
    
    print("\nAll key sequences processed successfully!")
    return True


def test_format_functions():
    """Test the formatting functions."""
    print("\nTesting format functions...")
    
    # Test raw bytes formatting
    test_bytes = [
        (b'\x03', "Control character"),
        (b'\x1b[A', "Escape sequence"),
        (b'hello', "Printable text"),
        (b'\x00\xff', "Binary data"),
    ]
    
    for raw_bytes, description in test_bytes:
        formatted = format_raw_bytes(raw_bytes)
        print(f"{description}: {formatted}")
    
    return True


def test_parser_functionality():
    """Test the parser with various inputs."""
    print("\nTesting parser functionality...")
    
    parser = KeyParser()
    
    # Test partial sequence handling
    print("Testing partial sequences...")
    events = parser.feed(b'\x1b')
    assert len(events) == 0, "Partial sequence should not produce events"
    
    events = parser.feed(b'[A')
    assert len(events) == 1, "Complete sequence should produce one event"
    assert events[0].key == prompt.Key.Up, "Should be Up arrow key"
    
    # Test mixed input
    print("Testing mixed input...")
    mixed_input = b'\x03hello\x1b[B'
    events = parser.feed(mixed_input)
    assert len(events) >= 3, "Should produce multiple events"
    
    # Test flush
    print("Testing flush...")
    parser.feed(b'\x1b[')  # Partial sequence
    flushed = parser.flush()
    print(f"Flushed {len(flushed)} events")
    
    # Test reset
    print("Testing reset...")
    parser.reset()
    events = parser.feed(b'\x03')
    assert len(events) == 1, "Parser should work after reset"
    
    print("Parser functionality tests passed!")
    return True


def test_module_constants():
    """Test that module constants are available."""
    print("\nTesting module constants...")
    
    # Test that key constants are available
    constants_to_test = [
        ('ESCAPE', 'Escape'),
        ('CTRL_C', 'ControlC'),
        ('ENTER', 'Enter'),
        ('TAB', 'Tab'),
        ('UP', 'Up'),
        ('DOWN', 'Down'),
        ('LEFT', 'Left'),
        ('RIGHT', 'Right'),
    ]
    
    for const_name, key_name in constants_to_test:
        if hasattr(prompt, const_name):
            const_value = getattr(prompt, const_name)
            key_value = getattr(prompt.Key, key_name)
            assert const_value == key_value, f"{const_name} should equal Key.{key_name}"
            print(f"✓ {const_name} = {const_value}")
        else:
            print(f"⚠ {const_name} not found in module")
    
    return True


def main():
    """Run all tests."""
    print("Python Key Demo Test Suite")
    print("=" * 40)
    
    tests = [
        test_format_functions,
        test_parser_functionality,
        test_module_constants,
        test_key_event_handling,
    ]
    
    for test in tests:
        try:
            if not test():
                print(f"Test {test.__name__} failed!")
                return 1
        except Exception as e:
            print(f"Test {test.__name__} raised exception: {e}")
            return 1
    
    print("\n" + "=" * 40)
    print("All tests passed! The Python demo is working correctly.")
    print("You can now run 'uv run python_example.py' to try it interactively.")
    return 0


if __name__ == "__main__":
    sys.exit(main())