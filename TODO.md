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
- **Prelude module**: Convenient imports for users ✅

### ❌ Missing High-Level Components
- **Prompt struct**: Core prompt interface with builder pattern
- **Completion System**: Trait-based completion framework
- **Rendering System**: Terminal output and display management
- **Prompt Loop**: Interactive input/output cycle

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

### 📺 Phase 3: Minimal Rendering System (Medium Priority)

#### Task 3.1: Basic Terminal Renderer
**File**: `crates/replkit-core/src/renderer.rs`
```rust
use crate::{Document, Suggestion};
use std::io::{self, Write, stdout};

pub struct Renderer {
    // Basic rendering state
}

impl Renderer {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn render_prompt(&mut self, prefix: &str, document: &Document) -> io::Result<()> {
        // Clear current line and render prompt with current text
        print!("\r\x1b[K{}{}", prefix, document.text());
        
        // Position cursor correctly
        let cursor_pos = prefix.len() + document.cursor_position();
        print!("\r\x1b[{}C", cursor_pos + 1);
        
        stdout().flush()
    }
    
    pub fn render_completions(&mut self, suggestions: &[Suggestion]) -> io::Result<()> {
        if suggestions.is_empty() {
            return Ok(());
        }
        
        println!(); // New line for completions
        for suggestion in suggestions.iter().take(10) { // Limit to 10 suggestions
            println!("  {} - {}", suggestion.text, suggestion.description);
        }
        
        Ok(())
    }
    
    pub fn clear_completions(&mut self, count: usize) -> io::Result<()> {
        for _ in 0..count {
            print!("\x1b[A\x1b[K"); // Move up and clear line
        }
        stdout().flush()
    }
}
```

### 🎯 Phase 4: Basic Prompt Loop (Medium Priority)

#### Task 4.1: Implement Input Loop
**File**: `crates/replkit-core/src/prompt.rs` (update existing)
```rust
use crate::{KeyParser, Key, renderer::Renderer};
use std::io::{self, Read, stdin};

impl Prompt {
    pub fn input(&mut self) -> Result<String, PromptError> {
        let mut parser = KeyParser::new();
        let mut renderer = Renderer::new();
        let stdin = stdin();
        let mut buffer = [0u8; 1024];
        
        // Set up raw mode (simplified version for now)
        // TODO: Implement proper terminal mode handling
        
        loop {
            // Render current state
            let document = self.buffer.document();
            renderer.render_prompt(&self.prefix, &document)?;
            
            // Check for completions if we have a completer
            if let Some(ref completer) = self.completer {
                let suggestions = completer.complete(&document);
                if !suggestions.is_empty() {
                    renderer.render_completions(&suggestions)?;
                }
            }
            
            // Read input
            match stdin.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let events = parser.feed(&buffer[..n]);
                    
                    for event in events {
                        match event.key {
                            Key::Enter => {
                                println!(); // New line after input
                                return Ok(self.buffer.text().to_string());
                            }
                            Key::ControlC => {
                                return Err(PromptError::Interrupted);
                            }
                            // Handle other keys
                            _ => {
                                if let Some(text) = event.text {
                                    self.buffer.insert_text(&text, false, true);
                                }
                            }
                        }
                    }
                }
                Ok(_) => break, // EOF
                Err(e) => return Err(PromptError::IoError(e.to_string())),
            }
        }
        
        Ok(String::new())
    }
}
```

### 🔧 Phase 5: Console I/O Integration (Low Priority)

#### Task 5.1: Implement replkit-io Crate
**File**: `crates/replkit-io/src/lib.rs`
```rust
//! Console input/output abstraction for cross-platform terminal handling

mod console;
mod terminal;

pub use console::{ConsoleInput, ConsoleOutput, ConsoleError};
pub use terminal::Terminal;

// Re-export from core for convenience
pub use replkit_core::prelude::*;
```

#### Task 5.2: Terminal Control Integration
- Implement proper raw mode terminal control
- Add cross-platform terminal size detection
- Integrate with existing console I/O specifications

### 🏗️ Phase 6: Integration & Testing (Low Priority)

#### Task 6.1: Integration Testing
- Verify `simple_prompt.rs` compiles and runs
- Test with Unicode input (Japanese, emoji, etc.)
- Validate error handling paths
- Test completion functionality

#### Task 6.2: Documentation & Examples
- Add comprehensive API documentation
- Create additional usage examples
- Update README with current capabilities

## Priority Implementation Path

### Minimum Viable Implementation (1-2 weeks)
1. **Phase 1**: Foundation interfaces (Tasks 1.1-1.3) ✅ COMPLETED
2. **Phase 2**: Prompt builder (Tasks 2.1-2.2) ✅ COMPLETED
3. **Phase 3**: Basic rendering (Task 3.1) 🚧 NEXT
4. **Phase 4**: Simple input loop (Task 4.1) 

This path will enable `simple_prompt.rs` to compile and provide basic functionality using `std::io::stdin()` for input.

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

✅ **simple_prompt.rs compiles without errors**
✅ **Basic prompt functionality works (input, display, completion)**  
✅ **Error handling works correctly**
✅ **Unicode text input is properly supported**
✅ **Code follows existing project patterns and conventions**

The strong foundation of low-level components (KeyParser, Document, Buffer) means we primarily need to build the high-level user interface layer to achieve a working prompt system.
