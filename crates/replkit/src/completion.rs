//! Completion system for providing suggestions based on document context
//!
//! This module defines the core traits and types for implementing completion
//! functionality in the replkit prompt system. It supports both trait-based
//! and function-based completion providers.

use crate::{Document, Suggestion};

/// Trait for providing completion suggestions based on document context
///
/// This trait allows for flexible completion implementations. You can implement
/// this trait directly on your own types, or use function types which automatically
/// implement this trait.
///
/// # Examples
///
/// ## Using a function
///
/// ```
/// use replkit::{Document, Suggestion, completion::Completor};
///
/// fn my_completer(document: &Document) -> Vec<Suggestion> {
///     vec![
///         Suggestion::new("hello", "A greeting"),
///         Suggestion::new("help", "Show help information"),
///     ]
/// }
///
/// // Functions automatically implement Completor
/// let suggestions = my_completer.complete(&Document::new());
/// ```
///
/// ## Implementing the trait directly
///
/// ```
/// use replkit::{Document, Suggestion, completion::Completor};
///
/// struct MyCompleter {
///     commands: Vec<(String, String)>,
/// }
///
/// impl Completor for MyCompleter {
///     fn complete(&self, document: &Document) -> Vec<Suggestion> {
///         let prefix = document.get_word_before_cursor();
///         self.commands
///             .iter()
///             .filter(|(cmd, _)| cmd.starts_with(&prefix))
///             .map(|(cmd, desc)| Suggestion::new(cmd, desc))
///             .collect()
///     }
/// }
/// ```
pub trait Completor {
    /// Generate completion suggestions for the given document context
    ///
    /// # Arguments
    ///
    /// * `document` - The current document state including cursor position and text
    ///
    /// # Returns
    ///
    /// A vector of suggestions. An empty vector means no completions are available.
    /// The suggestions should be ordered by relevance, with the most relevant first.
    fn complete(&self, document: &Document) -> Vec<Suggestion>;
}

/// Implement Completor for function types to support closure-based completers
///
/// This implementation allows any function with the signature
/// `Fn(&Document) -> Vec<Suggestion>` to be used as a completer without
/// needing to implement the trait explicitly.
///
/// # Examples
///
/// ```
/// use replkit::{Document, Suggestion, completion::Completor};
///
/// let completer = |document: &Document| -> Vec<Suggestion> {
///     vec![Suggestion::new("example", "An example completion")]
/// };
///
/// let suggestions = completer.complete(&Document::new());
/// assert_eq!(suggestions.len(), 1);
/// assert_eq!(suggestions[0].text, "example");
/// ```
impl<F> Completor for F
where
    F: Fn(&Document) -> Vec<Suggestion>,
{
    fn complete(&self, document: &Document) -> Vec<Suggestion> {
        self(document)
    }
}

/// A simple completion provider that matches against a static list of suggestions
///
/// This is useful for simple cases where you have a fixed set of completions
/// and want to filter them based on the current input.
///
/// # Examples
///
/// ```
/// use replkit::{Document, Suggestion, completion::{Completor, StaticCompleter}};
///
/// let completer = StaticCompleter::new(vec![
///     Suggestion::new("users", "Manage users"),
///     Suggestion::new("upload", "Upload files"),
///     Suggestion::new("update", "Update records"),
/// ]);
///
/// let mut doc = Document::with_text("up".to_string(), 2);
/// let suggestions = completer.complete(&doc);
/// assert_eq!(suggestions.len(), 2); // "upload" and "update"
/// ```
#[derive(Debug, Clone)]
pub struct StaticCompleter {
    suggestions: Vec<Suggestion>,
}

impl StaticCompleter {
    /// Create a new static completer with the given suggestions
    pub fn new(suggestions: Vec<Suggestion>) -> Self {
        Self { suggestions }
    }

    /// Create a static completer from string pairs
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit::completion::StaticCompleter;
    ///
    /// let completer = StaticCompleter::from_pairs(vec![
    ///     ("help", "Show help"),
    ///     ("hello", "Say hello"),
    /// ]);
    /// ```
    pub fn from_pairs<S1, S2>(pairs: Vec<(S1, S2)>) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        let suggestions = pairs
            .into_iter()
            .map(|(text, desc)| Suggestion::new(text, desc))
            .collect();
        Self::new(suggestions)
    }

    /// Create a static completer from just text strings (no descriptions)
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit::completion::StaticCompleter;
    ///
    /// let completer = StaticCompleter::from_strings(vec![
    ///     "help", "hello", "history"
    /// ]);
    /// ```
    pub fn from_strings<S: Into<String>>(strings: Vec<S>) -> Self {
        let suggestions = strings
            .into_iter()
            .map(|s| Suggestion::text_only(s))
            .collect();
        Self::new(suggestions)
    }
}

impl Completor for StaticCompleter {
    fn complete(&self, document: &Document) -> Vec<Suggestion> {
        let prefix = document.get_word_before_cursor().to_lowercase();

        if prefix.is_empty() {
            return self.suggestions.clone();
        }

        self.suggestions
            .iter()
            .filter(|suggestion| suggestion.text.to_lowercase().starts_with(&prefix))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_completor() {
        let completer = |document: &Document| -> Vec<Suggestion> {
            let word = document.get_word_before_cursor();
            if word.starts_with("te") {
                vec![
                    Suggestion::new("test", "Run tests"),
                    Suggestion::new("template", "Create template"),
                ]
            } else {
                vec![]
            }
        };

        let doc = Document::with_text("te".to_string(), 2);
        let suggestions = completer.complete(&doc);
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].text, "test");
        assert_eq!(suggestions[1].text, "template");

        let doc_empty = Document::new();
        let suggestions_empty = completer.complete(&doc_empty);
        assert_eq!(suggestions_empty.len(), 0);
    }

    #[test]
    fn test_static_completer_basic() {
        let suggestions = vec![
            Suggestion::new("users", "Manage users"),
            Suggestion::new("upload", "Upload files"),
            Suggestion::new("update", "Update records"),
            Suggestion::new("help", "Show help"),
        ];
        let completer = StaticCompleter::new(suggestions);

        // Test prefix matching
        let doc = Document::with_text("up".to_string(), 2);
        let results = completer.complete(&doc);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|s| s.text == "upload"));
        assert!(results.iter().any(|s| s.text == "update"));

        // Test no matches
        let doc_no_match = Document::with_text("xyz".to_string(), 3);
        let results_no_match = completer.complete(&doc_no_match);
        assert_eq!(results_no_match.len(), 0);

        // Test empty prefix (should return all)
        let doc_empty = Document::new();
        let results_empty = completer.complete(&doc_empty);
        assert_eq!(results_empty.len(), 4);
    }

    #[test]
    fn test_static_completer_from_pairs() {
        let completer = StaticCompleter::from_pairs(vec![
            ("help", "Show help"),
            ("hello", "Say hello"),
            ("history", "Show history"),
        ]);

        let doc = Document::with_text("he".to_string(), 2);
        let results = completer.complete(&doc);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|s| s.text == "help"));
        assert!(results.iter().any(|s| s.text == "hello"));
    }

    #[test]
    fn test_static_completer_from_strings() {
        let completer = StaticCompleter::from_strings(vec!["cat", "cd", "cp", "chmod"]);

        let doc = Document::with_text("c".to_string(), 1);
        let results = completer.complete(&doc);
        assert_eq!(results.len(), 4);

        let doc_ch = Document::with_text("ch".to_string(), 2);
        let results_ch = completer.complete(&doc_ch);
        assert_eq!(results_ch.len(), 1);
        assert_eq!(results_ch[0].text, "chmod");
    }

    #[test]
    fn test_case_insensitive_matching() {
        let completer = StaticCompleter::from_strings(vec!["Hello", "HELP", "HeLLo"]);

        let doc = Document::with_text("hel".to_string(), 3);
        let results = completer.complete(&doc);
        assert_eq!(results.len(), 3); // All should match despite case differences
    }

    #[test]
    fn test_trait_object_usage() {
        let completer: Box<dyn Completor> =
            Box::new(StaticCompleter::from_strings(vec!["test1", "test2"]));

        let doc = Document::with_text("test".to_string(), 4);
        let results = completer.complete(&doc);
        assert_eq!(results.len(), 2);
    }
}
