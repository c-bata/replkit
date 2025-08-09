#!/usr/bin/env python3
"""
Python Key Input Parser Demo

This example demonstrates how to use the Python bindings for the replkit-core
key input parser. It sets up raw terminal mode and displays parsed key events
in real-time, showing how to handle terminal input in Python applications.

Features:
- Raw terminal mode setup using termios
- Real-time key event parsing and display
- Callback-based event handling
- Proper exception handling
- Graceful termination on Ctrl+C

Usage:
    uv run python-examples/vt100_debug.py

Press various keys to see how they are parsed. Press Ctrl+C to exit.
"""

import sys
import os
import termios
import tty
import select
import signal
from typing import List, Callable, Optional, Any

import replkit

# Try to access the classes to verify they're available
KeyParser = getattr(replkit, 'KeyParser', None)
KeyEvent = getattr(replkit, 'KeyEvent', None) 
Key = getattr(replkit, 'Key', None)


class TerminalManager:
    """Manages terminal state for raw input mode."""
    
    def __init__(self):
        self.original_settings = None
        self.stdin_fd = sys.stdin.fileno()
        
    def enter_raw_mode(self):
        """Enter raw terminal mode for character-by-character input."""
        try:
            # Save original terminal settings
            self.original_settings = termios.tcgetattr(self.stdin_fd)
            
            # Set raw mode
            tty.setraw(self.stdin_fd)
            
            # Make stdin non-blocking
            import fcntl
            flags = fcntl.fcntl(self.stdin_fd, fcntl.F_GETFL)
            fcntl.fcntl(self.stdin_fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)
            
            print("Entered raw terminal mode\r")
            sys.stdout.flush()
            return True
            
        except Exception as e:
            print(f"Failed to enter raw mode: {e}")
            return False
    
    def exit_raw_mode(self):
        """Restore original terminal settings."""
        if self.original_settings is not None:
            try:
                termios.tcsetattr(self.stdin_fd, termios.TCSADRAIN, self.original_settings)
                print("\r\nRestored terminal settings")
            except Exception as e:
                print(f"Error restoring terminal: {e}")


class KeyEventHandler:
    """Handles key events with callback-based processing."""
    
    def __init__(self):
        self.callbacks: List[Callable[[Any], None]] = []
        self.running = True
        
    def add_callback(self, callback: Callable[[Any], None]):
        """Add a callback function to handle key events."""
        self.callbacks.append(callback)
        
    def handle_event(self, event: Any):
        """Process a key event by calling all registered callbacks."""
        for callback in self.callbacks:
            try:
                callback(event)
            except Exception as e:
                print(f"Error in callback: {e}\r")
                sys.stdout.flush()
                
    def stop(self):
        """Signal that event handling should stop."""
        self.running = False


def format_raw_bytes(raw_bytes: bytes) -> str:
    """Format raw bytes for display, showing both hex and ASCII representation."""
    hex_repr = ' '.join(f'{b:02x}' for b in raw_bytes)
    ascii_repr = ''.join(chr(b) if 32 <= b <= 126 else f'\\x{b:02x}' for b in raw_bytes)
    return f"[{hex_repr}] \"{ascii_repr}\""


def create_key_display_callback() -> Callable[[Any], None]:
    """Create a callback that displays key events in a formatted way."""
    
    def display_callback(event: Any):
        """Display a key event with detailed information."""
        # Get raw bytes from the event
        raw_bytes = bytes(event.raw_bytes)
        
        # Format the display
        key_name = str(event.key)
        raw_display = format_raw_bytes(raw_bytes)
        
        # Build the display line
        parts = [f"Key: {key_name:<15}"]
        parts.append(f"Raw: {raw_display}")
        
        if event.has_text():
            parts.append(f"Text: '{event.text_or_empty()}'")
            
        print(" | ".join(parts) + "\r")
        sys.stdout.flush()
        
        # Handle special keys
        if hasattr(replkit, 'Key') and event.key == replkit.Key.ControlC:
            print("\r\nReceived Ctrl+C - exiting...\r")
            sys.stdout.flush()
            return False  # Signal to stop
            
        return True
    
    return display_callback


def create_statistics_callback() -> Callable[[Any], None]:
    """Create a callback that tracks parsing statistics."""
    
    stats = {
        'total_events': 0,
        'control_keys': 0,
        'navigation_keys': 0,
        'function_keys': 0,
        'printable_chars': 0,
        'special_sequences': 0
    }
    
    def stats_callback(event: Any):
        """Track statistics about parsed events."""
        stats['total_events'] += 1
        
        key_str = str(event.key)
        
        if key_str.startswith('Ctrl+'):
            stats['control_keys'] += 1
        elif hasattr(replkit, 'Key') and event.key in [replkit.Key.Up, replkit.Key.Down, replkit.Key.Left, replkit.Key.Right,
                          replkit.Key.Home, replkit.Key.End, replkit.Key.PageUp, replkit.Key.PageDown]:
            stats['navigation_keys'] += 1
        elif key_str.startswith('F') and key_str[1:].isdigit():
            stats['function_keys'] += 1
        elif event.has_text() and len(event.text_or_empty()) == 1:
            stats['printable_chars'] += 1
        elif hasattr(replkit, 'Key') and event.key in [replkit.Key.BracketedPaste, replkit.Key.Vt100MouseEvent, 
                          replkit.Key.CPRResponse]:
            stats['special_sequences'] += 1
            
        # Display stats every 10 events
        if stats['total_events'] % 10 == 0:
            print(f"\r\n--- Statistics (after {stats['total_events']} events) ---\r")
            for key, value in stats.items():
                if key != 'total_events':
                    print(f"{key.replace('_', ' ').title()}: {value}\r")
            print("---\r")
            sys.stdout.flush()
    
    return stats_callback


def setup_signal_handlers(handler: KeyEventHandler, terminal: TerminalManager):
    """Set up signal handlers for graceful shutdown."""
    
    def signal_handler(signum, frame):
        print(f"\r\nReceived signal {signum}\r")
        sys.stdout.flush()
        handler.stop()
        terminal.exit_raw_mode()
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)


def main():
    """Main demo function."""
    print("Python Key Input Parser Demo")
    print("=" * 40)
    print("This demo shows how to use the Python bindings for terminal key parsing.")
    print("Press various keys to see how they are parsed.")
    print("Press Ctrl+C to exit gracefully.")
    print("=" * 40)
    
    # Initialize components
    terminal = TerminalManager()
    handler = KeyEventHandler()
    parser = replkit.KeyParser()
    
    # Set up signal handlers
    setup_signal_handlers(handler, terminal)
    
    # Add callbacks
    display_callback = create_key_display_callback()
    stats_callback = create_statistics_callback()
    
    handler.add_callback(display_callback)
    handler.add_callback(stats_callback)
    
    # Enter raw mode
    if not terminal.enter_raw_mode():
        print("Failed to enter raw terminal mode")
        return 1
    
    try:
        print("\r\nStarting key event loop...\r")
        print("Try pressing: arrow keys, function keys, Ctrl combinations, etc.\r")
        print("\r")
        sys.stdout.flush()
        
        # Main event loop
        while handler.running:
            try:
                # Use select to wait for input with timeout
                ready, _, _ = select.select([sys.stdin], [], [], 0.1)
                
                if ready:
                    # Read available data
                    try:
                        data = os.read(sys.stdin.fileno(), 1024)
                        if not data:
                            break  # EOF
                            
                        # Parse the input
                        events = parser.feed(data)
                        
                        # Handle each event
                        for event in events:
                            handler.handle_event(event)
                            
                            # Check for exit condition
                            if hasattr(replkit, 'Key') and event.key == replkit.Key.ControlC:
                                handler.stop()
                                break
                                
                    except OSError as e:
                        if e.errno != 11:  # EAGAIN/EWOULDBLOCK
                            raise
                        # No data available, continue
                        
            except KeyboardInterrupt:
                print("\r\nKeyboard interrupt received\r")
                sys.stdout.flush()
                break
            except Exception as e:
                print(f"Error in main loop: {e}\r")
                sys.stdout.flush()
                break
                
        # Flush any remaining events
        try:
            remaining_events = parser.flush()
            if remaining_events:
                print(f"\r\nFlushed {len(remaining_events)} remaining events:\r")
                sys.stdout.flush()
                for event in remaining_events:
                    handler.handle_event(event)
        except Exception as e:
            print(f"Error during flush: {e}\r")
            sys.stdout.flush()
            
    except Exception as e:
        print(f"Unexpected error: {e}\r")
        sys.stdout.flush()
        return 1
        
    finally:
        # Always restore terminal
        terminal.exit_raw_mode()
        
    print("Demo completed successfully!")
    return 0


if __name__ == "__main__":
    try:
        exit_code = main()
        sys.exit(exit_code)
    except Exception as e:
        print(f"Fatal error: {e}")
        sys.exit(1)