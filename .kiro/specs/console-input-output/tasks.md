# Implementation Plan

## Current Status

The console I/O system has been redesigned to use **synchronous, non-blocking methods** instead of async event loops. Core traits are implemented in `replkit-core`, platform implementations in `replkit-io`, and basic functionality is working across Unix and Windows.

## Remaining Tasks

### Core Platform Implementation

- [x] **1. Implement Unix synchronous input methods**
  - Update `crates/replkit-io/src/unix.rs` to implement new synchronous API
  - Implement `try_read_key()` using `poll()` with 0 timeout
  - Implement `read_key_timeout()` using `select()` for timeout support
  - Remove old event loop and callback-based code
  - Fix any compilation issues with the new trait methods
  - Add proper error handling for system call failures
  - _Requirements: Non-blocking I/O, WASM compatibility, simplified architecture_

- [x] **2. Implement Windows synchronous input methods**
  - Update `crates/replkit-io/src/windows.rs` to implement new synchronous API
  - Implement `try_read_key()` using `PeekConsoleInput` + `ReadConsoleInput`
  - Implement `read_key_timeout()` using `WaitForSingleObject` for timeout support
  - Remove old event loop and callback-based code
  - Handle both VT and Legacy console modes in a single implementation
  - Add proper error handling for Win32 API failures
  - _Requirements: Cross-platform consistency, timeout support_

- [x] **3. Implement WASM synchronous input methods**
  - Create or update `crates/replkit-io/src/wasm.rs` with new synchronous API
  - Implement `try_read_key()` using internal event queue
  - Implement `read_key_timeout()` to return `UnsupportedFeature` error
  - Add methods for host to queue key events: `queue_key_event()`
  - Add methods for host to update window size: `set_window_size()`
  - Remove any async/threading code that doesn't work in WASM
  - _Requirements: WASM compatibility, host integration_

- [x] **4. Fix mock implementation for new API**
  - Update `crates/replkit-io/src/mock.rs` to match new synchronous trait
  - Remove old event loop, callback methods, and thread-based code
  - Simplify to basic queue-based `try_read_key()` and `read_key_timeout()` 
  - Ensure mock works with new testing patterns
  - Add helper methods for testing: `queue_text_input()`, `clear_queue()`
  - _Requirements: Testing support, API consistency_

### Platform Integration and Testing

- [x] **5. Fix compilation errors across all platforms**
  - Ensure all crates compile successfully on Unix/Linux
  - Ensure all crates compile successfully on Windows 
  - Fix any trait method signature mismatches
  - Remove dead code related to old async design
  - Update dependencies if needed for new implementation
  - _Requirements: Platform compatibility, build system_

- [-] **6. Add comprehensive synchronous API tests**
  - Write unit tests for `try_read_key()` behavior (blocking/non-blocking)
  - Write unit tests for `read_key_timeout()` with various timeout values
  - Test error conditions and edge cases for new methods
  - Add integration tests using mock implementation
  - Test window size queries and error handling
  - Verify raw mode guard still works correctly
  - _Requirements: Quality assurance, regression prevention_

### Language Bindings and External Integration

- [ ] **7. Create WASM output bridge for Go bindings**
  - Create `crates/replkit-wasm/src/lib.rs` with console output functions
  - Export `wasm_output_command()` function for JSON-based command protocol
  - Implement JSON serialization for all `ConsoleOutput` operations
  - Support commands: WriteText, SetStyle, MoveCursorTo, Clear, Flush, etc.
  - Create Go wrapper in `bindings/go/wasm_output.go` to call WASM functions
  - Add proper error handling and status codes
  - _Requirements: Go-WASM integration, cross-platform rendering_

- [ ] **8. Update Python bindings for synchronous API**
  - Update `crates/replkit-pyo3/src/console.rs` for new trait methods
  - Remove callback-based methods and event loop management
  - Expose `try_read_key()` and `read_key_timeout()` to Python
  - Add proper Python exception handling for new error types
  - Update Python examples to use synchronous methods
  - Test Python bindings on available platforms
  - _Requirements: Python ecosystem compatibility, synchronous API_

### Advanced Features and Optimizations

- [ ] **9. Add mouse event support**
  - Extend `KeyEvent` to support mouse events or create `InputEvent` enum
  - Implement mouse event detection in platform implementations
  - Add mouse event parsing to key parsers where applicable
  - Support mouse clicks, movement, and scroll events
  - Add mouse event examples and documentation
  - _Requirements: Rich terminal applications, modern terminal support_

- [ ] **10. Add bracketed paste support**
  - Implement bracketed paste mode detection in terminal initialization
  - Add paste event detection and content extraction
  - Distinguish between typed and pasted content in key events
  - Handle large paste content efficiently
  - Add security considerations for paste content validation
  - _Requirements: User experience, security, large content handling_

- [ ] **11. Implement advanced terminal feature detection**
  - Add runtime detection of terminal capabilities
  - Detect true color, mouse support, bracketed paste availability
  - Implement graceful fallback for unsupported features
  - Add capability reporting to applications
  - Cache capability detection results for performance
  - _Requirements: Adaptive behavior, performance, compatibility_

## Notes

- **Completed tasks removed**: All tasks marked as complete in the previous version have been removed since the work is done
- **Async design abandoned**: Tasks related to event loops, callbacks, and async patterns have been removed or updated
- **Focus on simplicity**: New tasks emphasize the simpler synchronous design
- **WASM compatibility**: All tasks consider WASM constraints and limitations
- **Go-first bindings**: Go bindings get priority due to the project's Go focus
