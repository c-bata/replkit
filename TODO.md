# TODO: Implementation Roadmap for simple_prompt.rs

## Current Implementation Status

### ✅ Completed Foundation Components
- **KeyParser & KeyEvent**: Terminal input parsing infrastructure
- **Document**: Immutable text analysis and manipulation
- **Buffer**: Mutable text editing operations
- **Error Handling**: Hierarchical error architecture (BufferError → ReplkitError → PromptError)
- **Unicode Support**: Comprehensive Unicode text processing
- **WASM Bindings**: Foundation for cross-language integration

### ❌ Missing High-Level Components
- **Prompt struct**: Core prompt interface with builder pattern
- **Suggestion struct**: Completion suggestion data structure
- **replkit::prelude::*** module: Convenient imports for users
- **Completion System**: Trait-based completion framework
- **Rendering System**: Terminal output and display management
- **Prompt Loop**: Interactive input/output cycle

## Implementation Roadmap

### 🔥 Phase 1: Foundation Interfaces (High Priority)

#### ✅ Task 1.1: Implement Suggestion Structure - COMPLETED
**File**: `crates/replkit-core/src/suggestion.rs`
**Status**: ✅ COMPLETED
- ✅ Created Suggestion struct with text and description fields
- ✅ Implemented convenient constructors (new, text_only)
- ✅ Added From trait implementations for various input types
- ✅ Added comprehensive unit tests (8 tests passing)
- ✅ Updated lib.rs exports to include Suggestion
- ✅ Compilation and tests verified successful

#### Task 1.2: Create Prelude Module
**File**: `crates/replkit-core/src/suggestion.rs`
```rust
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub description: String,
}

impl Suggestion {
    pub fn new(text: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            description: description.into(),
        }
    }
}
```

#### Task 1.2: Create Prelude Module
**File**: `crates/replkit-core/src/prelude.rs`
```rust
//! Convenient re-exports for common usage patterns

pub use crate::{
    suggestion::Suggestion,
    document::Document,
    buffer::Buffer,
    key::{Key, KeyEvent},
    error::{ReplkitError, PromptError},
};

// Re-export completion traits when implemented
pub use crate::completion::Completor;

// Re-export Prompt types when implemented
pub use crate::prompt::{Prompt, PromptBuilder};
```

#### Task 1.3: Define Completion Trait
**File**: `crates/replkit-core/src/completion.rs`
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
```

### 🚀 Phase 2: Prompt Builder System (Medium Priority)

#### Task 2.1: Implement Core Prompt Structure
**File**: `crates/replkit-core/src/prompt.rs`
```rust
use crate::{Buffer, Document, Suggestion, completion::Completor, error::PromptError};
use std::io::{self, Write};

pub struct Prompt {
    prefix: String,
    completer: Option<Box<dyn Completor>>,
    buffer: Buffer,
}

pub struct PromptBuilder {
    prefix: String,
    completer: Option<Box<dyn Completor>>,
}

impl Prompt {
    pub fn builder() -> PromptBuilder {
        PromptBuilder::new()
    }
    
    pub fn input(&mut self) -> Result<String, PromptError> {
        // Implementation will be added in Phase 4
        todo!("Prompt input loop implementation")
    }
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            prefix: "> ".to_string(),
            completer: None,
        }
    }
    
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }
    
    pub fn with_completer<F>(mut self, completer: F) -> Self 
    where 
        F: Fn(&Document) -> Vec<Suggestion> + 'static 
    {
        self.completer = Some(Box::new(completer));
        self
    }
    
    pub fn build(self) -> Result<Prompt, PromptError> {
        Ok(Prompt {
            prefix: self.prefix,
            completer: self.completer,
            buffer: Buffer::new(),
        })
    }
}
```

#### Task 2.2: Update lib.rs Exports
**File**: `crates/replkit-core/src/lib.rs`
Add new modules to the existing exports:
```rust
// Add these new modules
pub mod completion;
pub mod prompt;
pub mod suggestion;
pub mod prelude;

// Add to existing re-exports
pub use completion::Completor;
pub use prompt::{Prompt, PromptBuilder};
pub use suggestion::Suggestion;
```

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
1. **Phase 1**: Foundation interfaces (Tasks 1.1-1.3)
2. **Phase 2**: Prompt builder (Tasks 2.1-2.2)  
3. **Phase 3**: Basic rendering (Task 3.1)
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
