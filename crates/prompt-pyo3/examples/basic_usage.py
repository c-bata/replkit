#!/usr/bin/env python3
"""
Basic usage example for the prompt key parser Python bindings.

This example demonstrates how to import and use the key parser
once the full implementation is complete.
"""

def main():
    """Demonstrate basic usage of the key parser."""
    try:
        import prompt_key_parser
        print(f"Prompt Key Parser version: {prompt_key_parser.__version__}")
        print("Python bindings successfully imported!")
        
        # TODO: Add actual parser usage once implementation is complete
        # parser = prompt_key_parser.KeyParser()
        # events = parser.feed(b'\x1b[A')  # Up arrow
        # print(f"Parsed key: {events[0].key}")
        
    except ImportError as e:
        print(f"Failed to import prompt_key_parser: {e}")
        print("Make sure to build the package with 'maturin develop' first")


if __name__ == "__main__":
    main()