#!/usr/bin/env python3
"""
Test script to verify the Python package structure is correct.
This script tests the package without requiring installation.
"""

import sys
import os

# Add the prompt directory to Python path for testing
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'prompt'))

def test_package_structure():
    """Test that the package structure is correct."""
    print("Testing Python package structure...")
    
    # Test that we can import the package
    try:
        import __init__ as prompt_pkg
        print("✓ Package __init__.py can be imported")
    except ImportError as e:
        print(f"✗ Failed to import package: {e}")
        return False
    
    # Test that __version__ is accessible (will fail until _core is built)
    try:
        version = prompt_pkg.__version__
        print(f"✓ Version accessible: {version}")
    except Exception as e:
        print(f"⚠ Version not accessible (expected until _core is built): {e}")
    
    # Test that __all__ is defined
    if hasattr(prompt_pkg, '__all__'):
        print(f"✓ __all__ is defined: {prompt_pkg.__all__}")
    else:
        print("⚠ __all__ is not defined")
    
    print("Package structure test completed!")
    return True

if __name__ == "__main__":
    test_package_structure()