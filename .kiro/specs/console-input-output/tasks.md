# Implementation Plan

- [x] 1. Fix architectural issue: Move console traits to prompt-core
  - Create `crates/prompt-core/src/console.rs` with ConsoleInput and ConsoleOutput traits from design document
  - Add ConsoleError, ConsoleResult, and related error types as specified in design
  - Define ConsoleCapabilities, OutputCapabilities, BackendType, TextStyle, Color, ClearType, and RawModeGuard types
  - Update `crates/prompt-core/src/lib.rs` to export new console module
  - Update `crates/prompt-io/src/lib.rs` to import traits from prompt-core instead of defining them locally
  - Update all platform implementations (unix.rs, windows.rs) to use traits from prompt-core
  - Update examples/debug_key_input.rs to import ConsoleInput from prompt-core
  - Update Cargo.toml dependencies: prompt-io should depend on prompt-core
  - **REASON**: Design specifies traits in prompt-core for proper architectural separation
  - _Requirements: 1.1, 1.2, 1.3, 9.1, 9.2_

- [x] 2. Update prompt-io crate structure to match design
  - ~~Create `crates/prompt-io/` directory with Cargo.toml~~ ✓
  - ~~Set up platform-specific dependencies (libc for Unix, winapi for Windows, wasm-bindgen for WASM)~~ ✓
  - ~~Create basic module structure: lib.rs, unix.rs, windows/, wasm.rs, mock.rs~~ ✓ (partial)
  - Update trait signatures to match design document (enable_raw_mode should return RawModeGuard, not mutate self)
  - Add factory functions: create_console_io(), create_console_input(), create_console_output()
  - Add missing modules: mock.rs, wasm.rs
  - Update method signatures to match design (remove &mut self requirements)
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [x] 3. Implement mock console input for testing
  - Create `crates/prompt-io/src/mock.rs` with MockConsoleInput implementation
  - Add input queue management with VecDeque for simulating key sequences
  - Implement ConsoleInput trait methods with proper state tracking
  - Add test helper methods: queue_key_event(), queue_text_input(), process_queued_events()
  - Implement thread-safe callback storage and invocation
  - Write comprehensive unit tests for mock input functionality
  - _Requirements: 8.1, 8.2, 8.3, 12.1, 12.2_

- [x] 4. Implement mock console output for testing
  - Extend `crates/prompt-io/src/mock.rs` with MockConsoleOutput implementation
  - Add output capture with Vec<u8> buffer and styled output tracking
  - Implement ConsoleOutput trait methods with state simulation
  - Add test helper methods: get_output(), get_styled_output(), clear_output()
  - Track cursor position, styling, and terminal state changes
  - Write unit tests for output capture and state tracking
  - _Requirements: 8.1, 8.2, 8.3, 12.1, 12.2_

- [x] 5. Implement Unix console input
  - ~~Create `crates/prompt-io/src/unix.rs` with UnixConsoleInput implementation~~
  - ~~Set up termios-based raw mode configuration with proper error handling~~
  - ~~Implement non-blocking input reading using poll() system call~~
  - ~~Create self-pipe for clean event loop shutdown signaling~~
  - ~~Integrate with existing KeyParser for key event generation~~
  - ~~Add SIGWINCH handling for window resize detection~~
  - **COMPLETE**: UnixVtConsoleInput implemented with termios, poll(), and KeyParser integration
  - Write platform-specific tests for Unix input functionality (TODO)
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3_

- [x] 6. Implement Unix console output
  - Extend `crates/prompt-io/src/unix.rs` with UnixConsoleOutput implementation
  - Add ANSI escape sequence generation for cursor control and styling
  - Implement color support (16-color, 256-color, and true color)
  - Add text styling support (bold, italic, underline, etc.)
  - Implement screen clearing and cursor positioning
  - Add output buffering for efficient terminal updates
  - Write tests for ANSI sequence generation and output functionality
  - _Requirements: 1.1, 1.2, 1.3, 10.1, 10.2, 10.3, 10.4_

- [x] 7. Replace existing vt100_debug examples with ConsoleInput-based implementation
  - ~~Replace `examples/vt100_debug.rs` with ConsoleInput-based key event display~~
  - ~~Remove platform-specific raw mode setup and replace with ConsoleInput::enable_raw_mode()~~
  - ~~Replace manual select/poll loops with ConsoleInput event callbacks~~
  - ~~Add cross-platform support (Unix + Windows) using the same codebase~~
  - ~~Display key events in the same format as original for compatibility~~
  - ~~Add graceful shutdown with Ctrl+C handling and proper terminal restoration~~
  - **COMPLETE**: examples/debug_key_input.rs implemented with ConsoleInput abstraction
  - Test on available platforms to validate ConsoleInput implementation correctness (TODO)
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 5.1, 5.2, 5.3_

- [ ] 8. Validate cross-platform vt100_debug functionality
  - Test replaced vt100_debug example on Unix platforms
  - Verify key event detection matches original implementation behavior
  - Test graceful shutdown and terminal restoration
  - Document any behavioral differences from original implementation
  - Create baseline for Windows testing once Windows implementation is complete
  - _Requirements: 1.1, 1.2, 1.3, 12.1, 12.2_

- [ ] 9. Implement Windows VT console input
  - Create `crates/prompt-io/src/windows/vt.rs` with WindowsVtConsoleInput implementation
  - Enable VT input mode using SetConsoleMode with ENABLE_VIRTUAL_TERMINAL_INPUT
  - Implement non-blocking input reading using WaitForMultipleObjects
  - Integrate with KeyParser for VT sequence processing
  - Add console buffer size change detection for resize events
  - Handle VT mode setup failures with clear error reporting
  - Write Windows-specific tests for VT input functionality
  - _Requirements: 1.1, 1.2, 1.3, 6.1, 6.2, 6.3, 6.4_

- [ ] 10. Implement Windows VT console output
  - Extend `crates/prompt-io/src/windows/vt.rs` with WindowsVtConsoleOutput implementation
  - Enable VT output mode using ENABLE_VIRTUAL_TERMINAL_PROCESSING
  - Reuse Unix ANSI sequence generation for VT-compatible output
  - Add Windows-specific cursor position querying if available
  - Implement proper error handling for VT mode failures
  - Write tests for Windows VT output functionality
  - _Requirements: 1.1, 1.2, 1.3, 6.1, 6.2, 6.3, 6.4_

- [ ] 11. Test Windows vt100_debug functionality
  - Test vt100_debug example on Windows with VT implementation
  - Verify key event detection works correctly on Windows Terminal and PowerShell
  - Test fallback behavior on legacy Windows environments
  - Compare key event output with Unix version for consistency
  - Document Windows-specific behaviors and limitations
  - _Requirements: 6.1, 6.2, 6.3, 12.1, 12.2_

- [x] 12. Implement Windows Legacy console input
  - ~~Create `crates/prompt-io/src/windows.rs` with WindowsLegacyConsoleInput implementation~~
  - ~~Use ReadConsoleInputW to receive KEY_EVENT_RECORD and WINDOW_BUFFER_SIZE_EVENT~~
  - ~~Implement direct key mapping from virtual keys to Key enum values~~
  - ~~Handle modifier keys (Ctrl, Alt, Shift) and special key combinations~~
  - ~~Add proper Unicode character handling for text input~~
  - ~~Bypass KeyParser for structured Windows console events~~
  - **COMPLETE**: WindowsLegacyConsoleInput implemented with Win32 Console API
  - Write tests for Windows legacy key mapping and event handling (TODO)
  - _Requirements: 1.1, 1.2, 1.3, 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 13. Implement Windows Legacy console output
  - Extend `crates/prompt-io/src/windows/legacy.rs` with WindowsLegacyConsoleOutput implementation
  - Use Win32 Console API functions for cursor control and text output
  - Implement color support using SetConsoleTextAttribute
  - Add screen buffer manipulation for clearing and scrolling
  - Handle console buffer resizing and coordinate system differences
  - Provide fallback implementations for unsupported styling features
  - Write tests for Windows legacy output functionality
  - _Requirements: 1.1, 1.2, 1.3, 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 14. Implement WASM bridge console input
  - Create `crates/prompt-io/src/wasm.rs` with WasmBridgeConsoleInput implementation
  - Define serializable message protocol for host communication
  - Implement receive_message() for processing host-sent key events
  - Add callback invocation for key events and resize notifications
  - Create WASM-exported functions for host environment integration
  - Handle WASM-specific limitations and provide appropriate error messages
  - Write tests for WASM message serialization and callback handling
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 11.1, 11.2, 11.3_

- [ ] 15. Implement WASM bridge console output
  - Extend `crates/prompt-io/src/wasm.rs` with WasmBridgeConsoleOutput implementation
  - Add output message generation for host environment rendering
  - Implement state tracking for cursor position and styling
  - Create serializable output command protocol
  - Add WASM-exported functions for host communication
  - Handle output buffering and efficient host communication
  - Write tests for WASM output message generation and state tracking
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 11.1, 11.2, 11.3_

- [ ] 14. Add comprehensive error handling and validation
  - Implement detailed error messages for platform-specific failures
  - Add error recovery mechanisms for terminal setup failures
  - Create diagnostic information for unsupported features
  - Implement proper error propagation across all platform implementations
  - Add validation for terminal state consistency
  - Write tests for error conditions and recovery scenarios
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

- [ ] 15. Implement thread safety and concurrent access
  - Add proper synchronization for callback storage and invocation
  - Implement thread-safe event loop management
  - Add protection against concurrent raw mode operations
  - Ensure safe cleanup of background threads and resources
  - Add panic handling in user callbacks to prevent system corruption
  - Write tests for concurrent access patterns and thread safety
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

- [ ] 16. Add cross-platform performance optimizations and resource management
  - Implement efficient polling mechanisms using standard APIs (poll/select/WaitForMultipleObjects)
  - Add output buffering strategies to reduce system call overhead across all platforms
  - Optimize memory allocation patterns in hot paths (event processing, buffer management)
  - Add resource cleanup and leak prevention for threads, file descriptors, and handles
  - Implement proper background thread lifecycle management with clean shutdown
  - Write performance benchmarks and resource usage tests for all platforms
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ] 17. Create comprehensive integration tests with advanced testing strategies
  - Write cross-platform integration tests using mock implementations
  - Add Unix PTY-based tests using openpty() for realistic terminal simulation
  - Create ANSI sequence golden tests using insta for snapshot testing
  - Implement property-based tests for control sequence filtering using quickcheck
  - Add Windows pipe-based tests for console size fallback mechanisms
  - Write tests for complete input/output workflows and platform capability detection
  - Add stress tests for high-frequency input and output operations
  - Write tests for error recovery and graceful degradation
  - Add tests for proper resource cleanup and memory management
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

- [ ] 18. Extend WASM serialization for console I/O
  - Add WasmConsoleInputState and WasmConsoleOutputState to existing wasm.rs
  - Implement serialization methods for console state transfer
  - Create WASM-compatible event and command structures
  - Add efficient serialization for high-frequency operations
  - Test WASM compilation and serialization roundtrip
  - _Requirements: 7.1, 7.2, 7.3, 11.1, 11.2, 11.3_

- [ ] 19. Create Go bindings and replace Go vt100_debug example
  - Extend `bindings/go/` with console input/output Go interfaces
  - Implement WASM-based Go wrappers for ConsoleInput and ConsoleOutput
  - Add Go-idiomatic error handling and type conversions
  - Create channel-based event handling for Go concurrency patterns
  - Replace existing Go vt100_debug example with ConsoleInput-based implementation
  - Verify cross-platform compatibility (Unix + Windows) in Go
  - Implement proper resource management and cleanup in Go bindings
  - _Requirements: 11.1, 11.2, 11.4, 11.5, 11.6_

- [ ] 20. Create Python bindings and replace Python vt100_debug example
  - Extend `crates/prompt-pyo3/` with PyConsoleInput and PyConsoleOutput classes
  - Implement Python-native callback handling with proper GIL management
  - Add Python exception handling for console errors
  - Create Pythonic interfaces for styling and color management
  - Replace existing Python vt100_debug example with ConsoleInput-based implementation
  - Verify cross-platform compatibility (Unix + Windows) in Python
  - Implement proper cleanup and resource management in Python bindings
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6_

- [ ] 21. Add platform-specific optimizations and unique features
  - Implement Linux-specific optimizations (epoll, signalfd, eventfd)
  - Add macOS-specific optimizations (kqueue, dispatch queues)
  - Implement Windows-specific optimizations (IOCP, overlapped I/O)
  - Add platform-unique features (Linux: inotify for config changes, Windows: console modes)
  - Create runtime feature detection and capability reporting
  - Add platform-specific configuration options and tuning parameters
  - Implement graceful feature degradation with clear capability reporting
  - Document platform differences, limitations, and performance characteristics
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 22. Create comprehensive examples and documentation
  - Build `examples/cross_platform_demo.rs` showing all platform features
  - Create `examples/styling_demo.rs` demonstrating color and text styling
  - Add `examples/interactive_demo.rs` for real-time input/output interaction
  - Write comprehensive rustdoc documentation for all public APIs
  - Create platform-specific usage guides and troubleshooting information
  - Add performance tuning and best practices documentation
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

- [ ] 23. Finalize testing framework and validation
  - Create automated testing pipeline for all supported platforms
  - Add property-based tests for console state consistency
  - Implement visual testing framework for output validation
  - Add regression tests for platform-specific behavior
  - Create performance regression testing
  - Add memory leak detection and resource usage validation
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

- [ ] 24. Polish API and prepare for integration
  - Review and finalize public API design for consistency
  - Add missing documentation and usage examples
  - Implement any remaining error handling edge cases
  - Add final performance optimizations based on benchmarking
  - Create migration guide from other terminal libraries
  - Prepare integration points for higher-level components (line editor, rendering system)
  - _Requirements: 1.1, 1.2, 1.3, 9.1, 9.2, 9.3, 10.1, 10.2, 10.3_