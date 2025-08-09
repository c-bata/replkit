# Prompt Key Parser - Python Bindings

Fast terminal key input parser with support for complex sequences, implemented in Rust with Python bindings.

## Features

- **High Performance**: Core parsing engine implemented in Rust
- **Comprehensive Key Support**: Handles control characters, function keys, arrow keys, and modifier combinations
- **Complex Sequences**: Supports mouse events, bracketed paste mode, and cursor position reports
- **State Machine**: Properly handles partial byte sequences and maintains parsing state
- **Cross-Platform**: Works on Linux, macOS, and other Unix-like systems

## Installation

```bash
pip install prompt
```

Or with `uv`:

```bash
uv add prompt
```

## Quick Start

```python
from prompt import KeyParser

# Create a parser instance
parser = KeyParser()

# Feed raw terminal input bytes
events = parser.feed(b'\x1b[A')  # Up arrow key
print(events[0].key)  # Key.Up

# Handle partial sequences
parser.feed(b'\x1b')     # Partial escape sequence
events = parser.feed(b'[A')  # Complete the sequence
print(events[0].key)  # Key.Up
```

## Requirements

- Python 3.8 or higher
- Unix-like operating system (Linux, macOS, BSD)

## Development

This package is built using [maturin](https://github.com/PyO3/maturin) and [PyO3](https://github.com/PyO3/pyo3), with `uv` for dependency management.

### Setup Development Environment

```bash
# Install uv if you haven't already
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment and install dependencies
uv sync --group dev

# Build the extension module for development
uv run maturin develop

# Run tests
uv run pytest

# Run type checking
uv run mypy prompt/

# Run linting
uv run ruff check prompt/ python_tests/
```

### Building Wheels

```bash
# Build wheels for all supported Python versions
uv run maturin build --find-interpreter

# Build wheel for current Python version only
uv run maturin build
```

## License

MIT License - see LICENSE file for details.