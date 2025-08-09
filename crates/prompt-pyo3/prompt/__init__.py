"""
Prompt Key Parser - Fast terminal key input parser

This package provides a fast, cross-platform key input parser for terminal applications.
It can handle complex key sequences including escape sequences, function keys,
mouse events, and bracketed paste mode.

The core parsing engine is implemented in Rust for performance and safety,
with Python bindings provided through PyO3.
"""

try:
    from ._core import __version__  # type: ignore[import-untyped]
except ImportError:
    # Fallback version when the extension module is not built
    __version__ = "0.1.0"

__all__ = ["__version__"]
