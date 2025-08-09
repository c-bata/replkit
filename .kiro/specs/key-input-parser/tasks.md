# Implementation Plan

- [x] 1. Set up Rust core library structure and key definitions
  - Create `crates/prompt-core/src/key.rs` with comprehensive Key enum matching go-prompt
  - Define KeyEvent struct with key, raw_bytes, and text fields
  - Add basic Cargo.toml configuration for the prompt-core crate
  - _Requirements: 1.4, 1.5, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 2. Implement Trie-based sequence matcher
  - Create `crates/prompt-core/src/sequence_matcher.rs` with TrieNode structure
  - Implement MatchResult enum and LongestMatchResult struct
  - Build static sequence mapping table covering control chars, arrows, function keys
  - Implement match_sequence() and find_longest_match() methods with single-pass efficiency
  - Add comprehensive unit tests for sequence matching and prefix detection
  - Please refer to `references/snippets/prompt_toolkit_vt100_debug.py`.
  - _Requirements: 1.1, 1.3, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 3. Create state machine parser engine
  - Implement `crates/prompt-core/src/key_parser.rs` with ParserState enum
  - Create KeyParser struct with state, buffer, and sequence_matcher fields
  - Implement feed() method with proper state transitions and partial sequence handling
  - Add flush() method to handle incomplete sequences gracefully
  - Implement reset() method for parser state cleanup
  - Add comprehensive unit tests for state machine transitions and edge cases
  - _Requirements: 1.1, 1.2, 1.3, 1.6_

- [ ] 4. Handle special sequences (mouse events, CPR, bracketed paste)
  - Extend state machine to handle MouseEvent and BracketedPaste states
  - Add regex-based detection for variable-length sequences (CPR responses, mouse events)
  - Implement proper parsing for bracketed paste mode content
  - Add unit tests for special sequence handling
  - _Requirements: 1.6_

- [ ] 5. Create Rust example application with SIGIO-based input
  - Build `examples/rust_key_demo.rs` that demonstrates raw terminal input parsing
  - Set up raw terminal mode using termios and configure non-blocking stdin
  - Implement SIGIO signal handler to detect when stdin is ready for reading
  - Use non-blocking reads from file descriptor 0 with proper error handling
  - Display parsed key events with raw byte information
  - Handle Ctrl+C and other signals for graceful termination
  - Test with various key combinations and verify output
  - _Requirements: 4.1, 4.4, 4.5_

- [ ] 6. Set up Go binding infrastructure
  - Create `bindings/go/` directory structure with proper Go module setup
  - Write C header file `bindings/go/key_parser.h` for CGO interface
  - Implement Rust FFI functions in `crates/prompt-core/src/ffi.rs`
  - Set up proper build configuration and linking for shared library
  - _Requirements: 2.1, 2.4_

- [ ] 7. Implement Go binding API
  - Create `bindings/go/key_parser.go` with Go-idiomatic Key enum and KeyEvent struct
  - Implement KeyParser struct wrapping C interface with proper resource management
  - Add Feed(), Flush(), Reset(), and Close() methods with error handling
  - Implement proper memory management and cleanup with finalizers
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 8. Create Go example application
  - Build `examples/go_key_demo.go` demonstrating Go binding usage
  - Set up raw terminal mode using appropriate Go libraries
  - Display parsed key events in Go-native format
  - Handle errors according to Go conventions
  - Test integration with go-prompt-style applications
  - _Requirements: 4.2, 4.4, 4.5_

- [ ] 9. Set up Python binding infrastructure
  - Create `bindings/python/` directory with PyO3 project structure
  - Configure `bindings/python/Cargo.toml` for PyO3 and maturin build
  - Set up proper Python package configuration with pyproject.toml
  - _Requirements: 3.1, 3.4_

- [ ] 10. Implement Python binding API
  - Create `bindings/python/src/lib.rs` with PyO3-based KeyParser and KeyEvent classes
  - Implement feed(), flush(), and reset() methods with proper error handling
  - Add Python-native Key enum with proper string representations
  - Convert Rust panics to appropriate Python exceptions
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 11. Create Python example application
  - Build `examples/python_key_demo.py` demonstrating Python binding usage
  - Set up raw terminal mode using termios or similar library
  - Implement callback-based key event handling
  - Display parsed key events in Python-native format
  - Handle exceptions according to Python conventions
  - _Requirements: 4.3, 4.4, 4.5_

- [ ] 12. Add comprehensive cross-language testing
  - Create integration tests verifying identical parsing results across Rust, Go, and Python
  - Test memory management and resource cleanup in all bindings
  - Add performance benchmarks comparing parsing speed across languages
  - Test error handling and edge cases in all language bindings
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4_