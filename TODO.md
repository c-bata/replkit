# TODO: Implementation Roadmap for simple_prompt.rs

## Current Implementation Status

### ✅ Completed Foundation Components
- **KeyParser & KeyEvent**: Terminal input parsing infrastructure
- **Document**: Immutable text analysis and manipulation
- **Buffer**: Mutable text editing operations
- **Error Handling**: Basic error architecture (BufferError)
- **Unicode Support**: Comprehensive Unicode text processing
- **WASM Bindings**: Foundation for cross-language integration
- **Suggestion struct**: Completion suggestion data structure ✅
- **Completion System**: Trait-based completion framework ✅
- **Prompt Builder**: Core prompt interface with builder pattern ✅
- **Prelude module**: Convenient imports for users ✅
- **Terminal Renderer**: Comprehensive rendering system using ConsoleOutput implementations ✅

### ✅ Completed High-Level Components  
- **Unified Crate Structure**: Successfully created `replkit` crate with proper API organization ✅
- **Terminal Renderer**: Full-featured rendering system with 415 lines, 13 tests, completion menus ✅

### ❌ Remaining High-Level Components
- **Complete Prompt Loop**: Interactive input/output cycle with renderer integration ⚡ **TASK 4.4 - CRITICAL**
- **Final Integration**: Complete input() method implementation in Prompt struct ⚡ **TASK 4.4 - CRITICAL**

## Implementation Roadmap

### 🔥 Phase 1: Foundation Interfaces (High Priority) ✅ COMPLETED

**Status**: ✅ All 3 foundation tasks completed and tested successfully

#### ✅ Task 1.1: Implement Suggestion Structure - COMPLETED
**File**: `crates/replkit-core/src/suggestion.rs`
**Status**: ✅ COMPLETED
- ✅ Created Suggestion struct with text and description fields
- ✅ Implemented convenient constructors (new, text_only)
- ✅ Added From trait implementations for various input types
- ✅ Added comprehensive unit tests (8 tests passing)
- ✅ Updated lib.rs exports to include Suggestion
- ✅ Compilation and tests verified successful

#### ✅ Task 1.2: Create Prelude Module - COMPLETED
**File**: `crates/replkit-core/src/prelude.rs`
**Status**: ✅ COMPLETED
- ✅ Created prelude module with convenient re-exports
- ✅ Exported core types (Document, Buffer, Suggestion, Key, KeyEvent)
- ✅ Exported error handling types (BufferError, BufferResult)
- ✅ Exported commonly used Unicode utilities
- ✅ Exported console I/O types for future integration
- ✅ Added comprehensive unit tests (3 tests passing)
- ✅ Added documentation and usage examples
- ✅ Compilation and tests verified successful

#### Task 1.3: Define Completion Trait ✅ COMPLETED
**File**: `crates/replkit-core/src/completion.rs`

**Status**: ✅ Fully implemented and tested

**Implementation Summary**:
- ✅ `Completor` trait with `complete(&self, document: &Document) -> Vec<Suggestion>`
- ✅ Automatic trait implementation for function types `Fn(&Document) -> Vec<Suggestion>`
- ✅ `StaticCompleter` struct with factory methods
- ✅ Case-insensitive prefix matching
- ✅ Comprehensive test coverage (6 tests)
- ✅ Available through prelude imports

```rust
use crate::{Document, Suggestion};

/// Trait for providing completion suggestions based on document context
pub trait Completor {
    fn complete(&self, document: &Document) -> Vec<Suggestion>;
}

/// Implement Completor for function types to support closure-based completers
impl<F> Completor for F
where
    F: Fn(&Document) -> Vec<Suggestion>,
{
    fn complete(&self, document: &Document) -> Vec<Suggestion> {
        self(document)
    }
}

/// Static completion provider with prefix matching
pub struct StaticCompleter {
    suggestions: Vec<Suggestion>,
}
```

### 🚀 Phase 2: Prompt Builder System (Medium Priority) ✅ COMPLETED

**Status**: ✅ Core prompt structure and builder pattern implemented and tested

#### Task 2.1: Implement Core Prompt Structure ✅ COMPLETED
**File**: `crates/replkit-core/src/prompt.rs`

**Status**: ✅ Fully implemented and tested

**Implementation Summary**:
- ✅ `Prompt` struct with prefix, completer, and buffer management
- ✅ `PromptBuilder` with fluent API and method chaining
- ✅ `PromptError` hierarchy with proper error conversion
- ✅ Integration with completion system (StaticCompleter and function-based)
- ✅ Comprehensive test coverage (11 tests)
- ✅ Available through prelude imports
- ✅ Placeholder for input() method (to be implemented in Phase 4)

**Key Features Implemented**:
```rust
// Builder pattern with fluent API
let prompt = Prompt::builder()
    .with_prefix("myapp> ")
    .with_completer(StaticCompleter::from_strings(vec!["help", "quit"]))
    .build()
    .unwrap();

// Function-based completer support
let prompt = Prompt::builder()
    .with_completer(|doc: &Document| {
        vec![Suggestion::new("git status", "Show status")]
    })
    .build()
    .unwrap();
```

#### Task 2.2: Update lib.rs Exports ✅ COMPLETED
**Files**: `crates/replkit-core/src/lib.rs`, `crates/replkit-core/src/prelude.rs`

**Status**: ✅ All exports updated and tested

**Implementation Summary**:
- ✅ Added `prompt` module to lib.rs
- ✅ Exported `Prompt`, `PromptBuilder`, `PromptError`, `PromptResult`
- ✅ Updated prelude.rs with prompt types
- ✅ Added comprehensive prelude tests for prompt functionality

### 📺 Phase 3: Minimal Rendering System (Medium Priority) 🚧 DEFERRED

**Status**: 🚧 Deferred - Moving to unified crate structure first

#### Task 3.1: Basic Terminal Renderer ➡️ MOVED TO Task 4.2
This task has been moved to Task 4.2 after crate restructuring.

### 🎯 Phase 4: Crate Restructuring & Unified API (High Priority)

#### ✅ Task 4.1: Create Unified `replkit` Crate & Move High-Level Components
**Files**: 
- Create `crates/replkit/` 
- Move high-level components from `replkit-core`

**Objective**: Create a unified `replkit` crate that provides the complete API by combining `replkit-core` (low-level) and `replkit-io` (platform-specific I/O).

**Implementation Requirements**:
- [x] Create `crates/replkit/Cargo.toml` with dependencies on `replkit-core` and `replkit-io`
- [x] Create `crates/replkit/src/lib.rs` with unified API exports
- [x] Move high-level components from `replkit-core` to `replkit`:
  - [x] Move `prompt.rs` (Prompt, PromptBuilder, PromptError)
  - [x] Move `completion.rs` (Completor trait, StaticCompleter)
  - [x] Move `suggestion.rs` (Suggestion struct)
  - [x] Update `prelude.rs` to re-export from new locations
- [x] Update `replkit-core` to focus on low-level primitives:
  - [x] Keep: Document, Buffer, KeyParser, Unicode, Error handling
  - [x] Keep: Console trait definitions (but not implementations)
- [x] Update all imports and dependencies
- [x] Ensure all tests pass after migration

**✅ COMPLETED**: Successfully created unified crate architecture. Test results:
- `replkit-core`: 266 tests passing (201 unit + 65 doc tests)
- `replkit`: 56 tests passing (28 unit + 28 doc tests)
- Total codebase: 322 tests passing

**Expected Structure After Migration**:
```
crates/
├── replkit/              # NEW: Unified high-level API
│   ├── src/
│   │   ├── lib.rs        # Re-exports everything + convenience functions
│   │   ├── prompt.rs     # Moved from replkit-core
│   │   ├── completion.rs # Moved from replkit-core  
│   │   ├── suggestion.rs # Moved from replkit-core
│   │   └── prelude.rs    # Updated prelude
│   └── Cargo.toml        # Depends on replkit-core + replkit-io
├── replkit-core/         # Low-level primitives only
│   ├── src/
│   │   ├── document.rs   # Stays
│   │   ├── buffer.rs     # Stays  
│   │   ├── key*.rs       # Stays
│   │   ├── unicode.rs    # Stays
│   │   ├── error.rs      # Stays
│   │   └── console.rs    # Trait definitions only
│   └── Cargo.toml
├── replkit-io/           # Platform-specific I/O implementations
└── replkit-wasm/         # WASM bindings
```

#### ✅ Task 4.2: Implement Terminal Renderer (moved from Task 3.1) - COMPLETED
**File**: `crates/replkit/src/renderer.rs`

**Objective**: Implement terminal rendering using actual `ConsoleOutput` implementations from `replkit-io`.

**✅ COMPLETED**: Successfully implemented comprehensive terminal renderer

**Implementation Requirements**:
- [x] Create `renderer.rs` in the `replkit` crate
- [x] Use actual `ConsoleOutput` implementations (no more mocks)
- [x] Integrate with `go-prompt`-style rendering patterns
- [x] Support Unicode text width calculations
- [x] Handle completion display and cleanup
- [x] Provide styled output (colors, formatting)

**✅ Implementation Summary**:
- ✅ Created full-featured `TerminalRenderer` struct with 415 lines of code
- ✅ Implemented prompt rendering with proper cursor positioning
- ✅ Added completion menu display with scrolling and selection highlighting
- ✅ Implemented proper cleanup methods for completions and prompts
- ✅ Added terminal size tracking and adaptive rendering
- ✅ Comprehensive test coverage with 13 test methods
- ✅ All 45 tests passing in replkit crate
- ✅ Proper error handling with ConsoleError to io::Error conversion

**Implemented API**:
```rust
use replkit::console::ConsoleOutput;

pub struct TerminalRenderer {
    console: Box<dyn ConsoleOutput>,
    terminal_size: (u16, u16),
    cursor_position: (u16, u16),
    last_prompt_lines: u16,
    last_completion_lines: u16,
}

impl TerminalRenderer {
    pub fn new(console: Box<dyn ConsoleOutput>) -> io::Result<Self>;
    pub fn render_prompt(&mut self, prefix: &str, document: &Document) -> io::Result<()>;
    pub fn render_completions(&mut self, suggestions: &[Suggestion], selected: Option<usize>) -> io::Result<()>;
    pub fn clear_completions(&mut self) -> io::Result<()>;
    pub fn clear_prompt(&mut self) -> io::Result<()>;
    pub fn update_terminal_size(&mut self, width: u16, height: u16);
    // + comprehensive private helper methods
}
```

#### ✅ Task 4.3: Update Prompt with Renderer Integration - COMPLETED (Basic Architecture)  
**File**: `crates/replkit/src/prompt.rs` (update existing after move)

**Objective**: Integrate the new Renderer into the Prompt system.

**✅ COMPLETED**: Successfully integrated Renderer into Prompt architecture with 45 tests passing

**Implementation Requirements**:
- [x] Add renderer field to `Prompt` struct
- [x] Update `PromptBuilder` to accept renderer configuration
- [x] Implement basic architecture for input loop using renderer
- [x] Handle console I/O initialization and error handling
- [x] Integrate renderer creation into build process

**✅ Implementation Summary**:
- ✅ Added renderer, input, and key_parser fields to Prompt struct
- ✅ Implemented with_console_output(), with_console_input(), with_default_console() methods
- ✅ Updated build() method with automatic Console I/O initialization
- ✅ Added proper error handling with ConsoleError conversion
- ✅ All existing tests continue to pass (45 tests)
- ✅ Architecture ready for full interactive input loop implementation

**Implemented API Updates**:
```rust
impl PromptBuilder {
    pub fn with_console_output(mut self, output: Box<dyn ConsoleOutput>) -> Self;
    pub fn with_console_input(mut self, input: Box<dyn ConsoleInput>) -> Self;
    pub fn with_default_console(mut self) -> PromptResult<Self>;
    pub fn build(self) -> PromptResult<Prompt>; // Now creates full renderer integration
}

impl Prompt {
    pub fn input(&mut self) -> PromptResult<String>; // Placeholder - ready for full implementation
}
```

**📋 Ready for Next Phase**: The complete architecture is now in place. The input() method currently returns a placeholder indicating the infrastructure is ready for the full interactive input loop implementation.

#### ⚡ Task 4.4: Implement Full Interactive Input Loop - HIGH PRIORITY
**File**: `crates/replkit/src/prompt.rs`

**Objective**: Replace the placeholder input() method with a complete interactive input loop implementation.

**🎯 CRITICAL FOR DEMO**: This task is required to make `simple_prompt.rs` actually work.

**Implementation Requirements**:
- [ ] Replace placeholder input() method with full implementation
- [ ] Implement event-driven keyboard input handling
- [ ] Integrate real-time rendering with user input
- [ ] Handle basic key events (Enter, Backspace, arrow keys, Tab)
- [ ] Support completion menu navigation and selection
- [ ] Proper cleanup on exit (Ctrl+C, Enter)

**Expected Behavior**:
```rust
impl Prompt {
    pub fn input(&mut self) -> PromptResult<String> {
        // 1. Initialize: Clear buffer, render initial prompt
        // 2. Event loop: Listen for key events, update buffer, re-render
        // 3. Handle completions: Show/hide completion menu on Tab
        // 4. Return: Final input string on Enter, or error on Ctrl+C
    }
}
```

**Priority**: 🔥 **HIGHEST** - Required to run simple_prompt.rs example

### 🔧 Phase 5: Advanced Features (Low Priority)

#### Task 5.1: Enhanced Input Handling
- Implement advanced key bindings (Emacs/Vi modes)
- Add history support with up/down arrow navigation
- Implement proper raw mode terminal control
- Add cross-platform terminal size detection

#### Task 5.2: Advanced Rendering Features  
- Multi-line prompt support
- Syntax highlighting for input
- Advanced completion UI (scrollable, selectable)
- Progress indicators and status displays

### 🏗️ Phase 6: Integration & Testing (Low Priority)

#### Task 6.1: Integration Testing
- Verify `simple_prompt.rs` compiles and runs with new structure
- Test with Unicode input (Japanese, emoji, etc.)
- Validate error handling paths across crate boundaries
- Performance testing and optimization

#### Task 6.2: Documentation & Examples
- Add comprehensive API documentation for unified crate
- Create migration guide from replkit-core to replkit
- Update README with current capabilities and examples
- Add example projects demonstrating different use cases

## Priority Implementation Path

### Minimum Viable Implementation (1-2 weeks)
1. **Phase 1**: Foundation interfaces (Tasks 1.1-1.3) ✅ COMPLETED
2. **Phase 2**: Prompt builder (Tasks 2.1-2.2) ✅ COMPLETED
3. **Phase 4**: Crate restructuring & unified API (Tasks 4.1-4.4) 🚧 CURRENT
   - **Task 4.1**: Create unified `replkit` crate and move components ✅ COMPLETED
   - **Task 4.2**: Implement terminal renderer with real ConsoleOutput ✅ COMPLETED
   - **Task 4.3**: Integrate renderer into prompt system ✅ COMPLETED
   - **Task 4.4**: Implement full interactive input loop ⚡ **NEXT - CRITICAL FOR DEMO**

**Current Status**: Architecture complete (45 tests passing). Need Task 4.4 to make simple_prompt.rs work.

### Full Implementation (3-4 weeks)
Complete all phases including proper terminal control and cross-platform I/O integration.

## Dependencies and Integration Points

### Existing Strong Foundation
- **replkit-core**: Excellent foundation with KeyParser, Document, Buffer, and Unicode support
- **Error Handling**: Well-designed hierarchical error system
- **WASM Support**: Ready for multi-language bindings

### Integration Requirements
- Update `Cargo.toml` files to include new dependencies
- Ensure WASM compatibility for new components
- Maintain API consistency with go-prompt patterns
- Preserve existing test coverage while adding new tests

## Success Criteria

✅ **Clean crate separation**: `replkit-core` for low-level, `replkit` for high-level API  
✅ **simple_prompt.rs compiles without errors using unified `replkit` crate**
✅ **Basic prompt functionality works (input, display, completion)**  
✅ **Error handling works correctly across crate boundaries**
✅ **Unicode text input is properly supported**
✅ **Code follows existing project patterns and conventions**
✅ **Users can import complete API with `use replkit::prelude::*`**

The strong foundation of low-level components (KeyParser, Document, Buffer) in `replkit-core` combined with the high-level prompt interface in `replkit` will provide a clean, usable API similar to go-prompt.
