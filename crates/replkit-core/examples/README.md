# REPL Examples

This directory contains a practical example demonstrating REPL integration functionality.

## Example

### `basic_repl.rs` - Basic REPL Implementation

A practical REPL example that demonstrates:

- **Echo functionality** and basic command processing
- **Custom key bindings** (Ctrl+C, Ctrl+L)
- **Cross-platform console I/O** using replkit-io
- **Command parsing** (help, echo, repeat)
- **Proper configuration** and component testing

**Usage:**
```bash
cargo run --example basic_repl
```

**Available Commands:**
- `help` - Show available commands
- `echo <text>` - Echo the given text
- `repeat <n> <text>` - Repeat text n times (max 10)
- `exit`/`quit` - Exit the REPL

**Key Bindings:**
- `Ctrl+C` - Clear current line
- `Ctrl+D` - Exit (on empty line)
- `Ctrl+L` - Clear screen
- Arrow keys - Navigate text
- Home/End - Jump to line start/end

**Note:** This example tests REPL components and configuration without running the full interactive loop to avoid current implementation constraints. All REPL functionality is demonstrated through component testing.

## Testing the Example

The example includes comprehensive unit tests:

```bash
# Test basic REPL example
cargo test --example basic_repl
```

## Cross-Platform Behavior

The example demonstrates cross-platform behavior:

- **Unix/Linux/macOS**: Uses Unix VT console implementations
- **Windows**: Attempts VT mode first, falls back to Legacy mode
- **Other platforms**: Shows appropriate error messages

## Error Handling Demonstration

The example shows various error handling scenarios:

1. **Console I/O errors**: Attempt to reinitialize console components
2. **Terminal state corruption**: Force terminal reset and restore raw mode
3. **Callback exceptions**: Catch and log errors, continue REPL operation
4. **Memory allocation failures**: Graceful degradation with reduced functionality
5. **Terminal disconnection**: Clean shutdown with resource cleanup

## Integration Patterns

This example demonstrates the recommended patterns for integrating the REPL engine:

1. **Configuration**: How to set up `ReplConfig` with custom executors and key bindings
2. **Console I/O Setup**: How to use platform factories to create console implementations
3. **Error Recovery**: How to handle and recover from various error conditions
4. **Graceful Shutdown**: How to properly clean up resources and restore terminal state
5. **Testing**: How to write unit tests for REPL configurations and components

## Requirements Validation

This example validates the following requirements from the specification:

- **4.1, 4.2**: Cross-platform Rust implementation with Unix and Windows support
- **4.5, 4.6**: Consistent behavior across terminal emulators and proper resource cleanup
- **9.1, 9.4**: Comprehensive testing and validation of REPL functionality

## macOS Usage

### Quick Start

```bash
cd crates/replkit-core
cargo run --example basic_repl
```

This will demonstrate:
- REPL configuration and component setup
- Console I/O initialization for macOS
- Command executor testing with sample inputs
- Platform detection and capability reporting
- All REPL components working together

### Troubleshooting on macOS

If you encounter issues:

1. **Make sure you're running in a proper terminal** (Terminal.app, iTerm2, etc.)
2. **Avoid running in IDEs or non-interactive environments**
3. **Check that your terminal supports ANSI escape sequences**

## Usage Instructions

### For Development

1. **Study the example** to understand REPL configuration patterns
2. **Run the example** to see component testing in action
3. **Examine the test cases** to understand validation approaches
4. **Use as a template** for your own REPL implementations

### For Integration

1. **Copy configuration patterns** from the example
2. **Adapt error handling** strategies to your needs
3. **Customize key bindings** and commands for your application
4. **Use platform factory patterns** for cross-platform support

### For Testing

1. **Run in different terminals** to test cross-platform behavior
2. **Test error conditions** by simulating various failure scenarios
3. **Validate key bindings** work correctly across platforms
4. **Check resource cleanup** by monitoring terminal state after exit

## Troubleshooting

### Common Issues

1. **"Console I/O setup failed"**: Expected when running without a real terminal (e.g., in CI)
2. **"Platform factory failed"**: Indicates missing or incompatible console implementations
3. **Key bindings not working**: May indicate terminal compatibility issues
4. **Memory or resource errors**: Check terminal state and available system resources

### Platform-Specific Notes

- **macOS/Linux**: Should work in most terminal emulators
- **Windows**: May require Windows Terminal or PowerShell for full VT support
- **SSH/Remote**: May have limited functionality depending on terminal forwarding
- **CI/Automated**: Example will show configuration but cannot run interactively