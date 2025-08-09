# Technology Stack

## Build System
- Standard build tools and package managers
- Automated dependency management
- Environment-specific configurations

## Core Technologies
- Modern language features and best practices
- Industry-standard frameworks and libraries
- Cross-platform compatibility considerations

## Development Tools
- Code formatting and linting
- Testing frameworks and coverage tools
- Documentation generation

## Common Commands

### `crates/replkit_pyo3`

`replkit_pyo3` is built using [maturin](https://github.com/PyO3/maturin) and [PyO3](https://github.com/PyO3/pyo3), with `uv` for dependency management.

**Setup Development Environment**

```bash
# Create virtual environment and install dependencies
uv sync --group dev

# Build the extension module for development
uv run maturin develop

# Run tests
uv run pytest tests

# Run type checking
uv run mypy replkit/

# Run linting
uv run ruff check replkit/ tests/ python-examples/
```

**Building Wheels**

```bash
# Build wheels for all supported Python versions
uv run maturin build --find-interpreter

# Build wheel for current Python version only
uv run maturin build
```


### `bindings/go`

Run the test suite:

```bash
go test -v
```
