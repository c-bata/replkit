//! Convenient re-exports for common use cases
//!
//! The prelude module provides all the commonly used types and traits
//! for building interactive prompts with Replkit.
//!
//! # Usage
//!
//! ```rust
//! use replkit::prelude::*;
//!
//! // Now you have access to all commonly used types
//! let prompt = Prompt::builder()
//!     .with_prefix("$ ")
//!     .build()
//!     .unwrap();
//! ```

// Re-export commonly used types from low-level crates
pub use replkit_core::{
    // Console abstractions
    console::{
        BackendType, ClearType, Color, ConsoleCapabilities, ConsoleError, ConsoleInput,
        ConsoleOutput, ConsoleResult, EventLoopError, OutputCapabilities, RawModeGuard,
        SafeTextFilter, SanitizationPolicy, TextStyle,
    },
    // Unicode utilities
    unicode::{display_width, rune_count, rune_slice},
    Buffer,
    // Error handling
    BufferError,
    BufferResult,
    // Text handling
    Document,
    // Key input
    Key,
    KeyEvent,
    KeyParser,
};

// Re-export high-level components from this crate
pub use crate::{
    Completor,
    // Prompt system
    Prompt,
    PromptBuilder,
    PromptError,
    PromptResult,
    // Rendering system
    Renderer,
    StaticCompleter,
    // Completion system
    Suggestion,
};

// Re-export I/O implementations for direct access
pub use replkit_io::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // Test that high-level types are available through prelude
        let _doc = Document::new();
        let _buffer = Buffer::new();
        let _suggestion = Suggestion::new("test", "description");

        // Test unicode utilities
        let text = "hello";
        let _count = rune_count(text);
        let _width = display_width(text);
        let _slice = rune_slice(text, 0, 2);

        // Test key types
        let _key = Key::Enter;
        let _parser = KeyParser::new();

        // Test error types
        let _error = BufferError::invalid_cursor_position(10, 5);
        let _result: BufferResult<String> = Ok("test".to_string());

        // Test console types
        let _style = TextStyle::default();
        let _color = Color::Red;
        let _clear = ClearType::All;
    }

    #[test]
    fn test_suggestion_from_prelude() {
        // Test that Suggestion can be created using types from prelude
        let suggestion = Suggestion::new("users", "Store user data");
        assert_eq!(suggestion.text, "users");
        assert_eq!(suggestion.description, "Store user data");
    }

    #[test]
    fn test_document_from_prelude() {
        // Test that Document can be used through prelude
        let doc = Document::with_text("hello world".to_string(), 5);
        assert_eq!(doc.text(), "hello world");
        assert_eq!(doc.cursor_position(), 5);
    }

    #[test]
    fn test_completion_from_prelude() {
        // Test that Completor trait and StaticCompleter are available through prelude
        let completer = StaticCompleter::from_strings(vec!["hello", "help", "history"]);
        let doc = Document::with_text("he".to_string(), 2);
        let suggestions = completer.complete(&doc);
        assert_eq!(suggestions.len(), 2);

        // Test function-based completer
        let func_completer = |document: &Document| -> Vec<Suggestion> {
            if document.text().starts_with("test") {
                vec![Suggestion::new("testing", "Run tests")]
            } else {
                vec![]
            }
        };

        let test_doc = Document::with_text("test".to_string(), 4);
        let test_suggestions = func_completer.complete(&test_doc);
        assert_eq!(test_suggestions.len(), 1);
        assert_eq!(test_suggestions[0].text, "testing");
    }

    #[test]
    fn test_prompt_from_prelude() {
        // Test that Prompt and PromptBuilder are available through prelude
        let prompt = Prompt::builder().with_prefix("test> ").build().unwrap();

        assert_eq!(prompt.prefix(), "test> ");

        // Test with completer
        let completer = StaticCompleter::from_strings(vec!["hello", "help"]);
        let mut prompt_with_completer = Prompt::builder()
            .with_prefix("$ ")
            .with_completer(completer)
            .build()
            .unwrap();

        prompt_with_completer.insert_text("he").unwrap();
        let suggestions = prompt_with_completer.get_completions();
        assert_eq!(suggestions.len(), 2);

        // Test error types are available
        let _error: PromptResult<String> = Err(PromptError::Interrupted);
    }

    #[test]
    fn test_renderer_from_prelude() {
        use replkit_io::mock::MockConsoleOutput;

        // Test that Renderer is available through prelude
        let console = Box::new(MockConsoleOutput::new());
        let renderer = Renderer::new(console);

        assert_eq!(renderer.terminal_size(), (80, 24));
    }

    #[test]
    fn test_convenience_functions_from_prelude() {
        // Test that convenience functions are available through crate::convenience
        // Note: These would normally be called interactively, so we just test they exist
        use crate::convenience::{prompt_with_completer, prompt_with_completions, simple_prompt};

        let _simple = simple_prompt("test> ");
        let _with_completions = prompt_with_completions("$ ", vec!["help", "quit"]);

        // Test function-based completer
        let completer = |_doc: &Document| vec![Suggestion::new("test", "Test command")];
        let _with_completer = prompt_with_completer("$ ", completer);
    }
}
