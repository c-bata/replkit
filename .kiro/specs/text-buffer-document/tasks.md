# Implementation Plan

- [x] 1. Set up crate structure and dependencies
  - Create `crates/prompt-core/src/buffer.rs` and `crates/prompt-core/src/document.rs` modules
  - Add Unicode dependencies (unicode-width) and serde to Cargo.toml
  - Create `crates/prompt-core/src/unicode.rs` utility module
  - Create `crates/prompt-core/src/error.rs` with BufferError enum and proper error handling
  - Update lib.rs to export new modules and maintain compatibility with existing key parser
  - _Requirements: 8.1, 9.1, 9.2, 9.3_

- [x] 2. Implement core Unicode utilities module
  - Create unicode.rs with rune_count and display_width functions using unicode-width
  - Implement rune_slice and byte_index_from_rune_index for safe string slicing
  - Add char_at_rune_index function with proper bounds checking
  - Write comprehensive unit tests for Unicode edge cases (CJK, emoji)
  - Validate WASM compatibility of unicode-width dependency
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 8.2_

- [x] 3. Implement Document structure with basic functionality
  - Create Document struct with text, cursor_position, and last_key fields
  - Implement new(), with_text(), and with_text_and_key() constructors
  - Add basic accessor methods: text(), cursor_position(), last_key_stroke()
  - Implement display_cursor_position() using Unicode width calculations
  - Add text_before_cursor() and text_after_cursor() with proper rune slicing
  - Write unit tests for basic Document functionality and Unicode handling
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

- [x] 4. Implement Document text analysis methods
  - Add get_char_relative_to_cursor() with bounds checking and error handling
  - Implement word finding methods: get_word_before_cursor(), get_word_after_cursor()
  - Add word boundary detection: find_start_of_previous_word(), find_end_of_current_word()
  - Implement separator-based word operations with custom separator support
  - Add whitespace-aware word operations (_with_space variants)
  - Write comprehensive tests for word operations with various Unicode text
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 5. Implement multi-line Document operations
  - Add current_line_before_cursor(), current_line_after_cursor(), current_line() methods
  - Implement lines() method to split text into line array
  - Add line_count() with proper handling of trailing newlines
  - Implement line_start_indexes() with caching for performance
  - Add cursor_position_row() and cursor_position_col() calculations
  - Implement position translation: translate_index_to_position() and translate_row_col_to_index()
  - Write tests for multi-line text operations and edge cases
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 6. Implement Document cursor movement calculations
  - Add get_cursor_left_position() and get_cursor_right_position() with line boundary respect
  - Implement get_cursor_up_position() and get_cursor_down_position() with preferred column support
  - Add on_last_line() and get_end_of_line_position() helper methods
  - Implement find_line_start_index() with efficient line index lookup
  - Add leading_whitespace_in_current_line() for indentation handling
  - Write tests for cursor movement calculations in multi-line scenarios
  - _Requirements: 3.4, 3.5, 3.6_

- [ ] 7. Implement Buffer structure with working lines management
  - Create Buffer struct with working_lines, working_index, cursor_position fields
  - Add cached_document, preferred_column, and last_key_stroke fields
  - Implement new() constructor with proper initialization
  - Add text() method to get current working line text
  - Implement basic cursor position and working index management
  - Write unit tests for Buffer initialization and basic state management
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

- [ ] 8. Implement Buffer text modification operations
  - Add insert_text() method with insert/overwrite modes and cursor movement options
  - Implement set_text() with proper cursor position validation
  - Add delete_before_cursor() and delete() methods with bounds checking
  - Implement proper Unicode character deletion (handle grapheme clusters)
  - Add cache invalidation when text is modified
  - Write comprehensive tests for text modification with Unicode content
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 6.1, 6.2_

- [ ] 9. Implement Buffer cursor movement operations
  - Add cursor_left(), cursor_right(), cursor_up(), cursor_down() methods
  - Implement preferred column tracking for vertical movement consistency
  - Add set_cursor_position() with bounds validation and cache invalidation
  - Implement ensure_cursor_bounds() helper for position validation
  - Write tests for cursor movement with multi-line text and preferred column behavior
  - _Requirements: 4.4, 4.5, 4.6, 5.4, 5.5_

- [ ] 10. Implement advanced Buffer editing operations
  - Add new_line() method with optional margin copying for indentation
  - Implement join_next_line() with configurable separator
  - Add swap_characters_before_cursor() with proper Unicode character handling
  - Implement set_last_key_stroke() for context-aware operations
  - Write tests for advanced editing operations and edge cases
  - _Requirements: 5.1, 5.2, 5.3, 5.6_

- [ ] 11. Implement Document caching system in Buffer
  - Add document() method with intelligent caching based on text, cursor, and key state
  - Implement update_cached_document() with cache validation logic
  - Add invalidate_cache() method called on all state changes
  - Implement display_cursor_position() delegation to cached document
  - Write performance tests to validate caching effectiveness
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 12. Add comprehensive error handling and validation
  - Implement BufferError enum with detailed error variants
  - Add proper error handling to all fallible operations
  - Implement bounds checking for cursor position and working index
  - Add validation for text operations and range checks
  - Write tests for error conditions and recovery scenarios
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [ ] 13. Create WASM serialization for Go integration
  - Extend existing wasm.rs with WasmBufferState and WasmDocumentState structs
  - Add to_wasm_state() and from_wasm_state() methods for Buffer and Document
  - Implement serde serialization for data marshaling with wazero runtime
  - Add WASM-compatible functions to existing wasm.rs module
  - Test WASM compilation and serialization roundtrip
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 14. Create Go binding wrapper
  - Build Go package in `bindings/go/text_buffer/` with WASM integration
  - Implement Go Buffer and Document structs wrapping WASM calls
  - Add proper Go error handling and type conversions
  - Create Go-idiomatic API that matches original go-prompt interface
  - Write Go example demonstrating text buffer usage
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 15. Create Python bindings with PyO3
  - Extend `crates/prompt-pyo3/` to include Buffer and Document classes
  - Implement Python-native error handling with proper exception types
  - Add Python methods that match the Rust API with Pythonic naming
  - Create Python example demonstrating text buffer operations
  - Test Python bindings with Unicode text and complex editing scenarios
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 16. Add property-based testing with quickcheck
  - Implement quickcheck tests for cursor position invariants
  - Add property tests for text consistency after editing operations
  - Create tests for Unicode handling correctness across all operations
  - Add buffer state consistency tests for complex operation sequences
  - Validate that all operations preserve UTF-8 validity
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 6.1, 6.2, 6.3_

- [ ] 17. Create comprehensive integration tests and examples
  - Write integration tests for realistic text editing workflows
  - Add tests for complex multi-line editing scenarios with Unicode
  - Create Rust example demonstrating full Buffer and Document API
  - Add performance benchmarks for large document operations
  - Test integration with existing key-input-parser functionality
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 18. Finalize documentation and API polish
  - Add comprehensive rustdoc documentation for all public APIs
  - Include usage examples in documentation with Unicode text
  - Review API for Rust naming conventions and ergonomics
  - Add module-level documentation explaining architecture
  - Create README with usage examples for all language bindings
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_