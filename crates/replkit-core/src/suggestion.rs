//! Completion suggestion data structures
//!
//! This module provides the core data structures for representing completion suggestions
//! in the replkit prompt system. Suggestions consist of the text to be completed and
//! an optional description for user guidance.

/// A completion suggestion with text and description
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The text that will be inserted when this suggestion is selected
    pub text: String,
    /// A human-readable description of what this suggestion represents
    pub description: String,
}

impl Suggestion {
    /// Create a new suggestion with the given text and description
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::suggestion::Suggestion;
    ///
    /// let suggestion = Suggestion::new("users", "Store the username and age");
    /// assert_eq!(suggestion.text, "users");
    /// assert_eq!(suggestion.description, "Store the username and age");
    /// ```
    pub fn new(text: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            description: description.into(),
        }
    }

    /// Create a suggestion with just text and no description
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::suggestion::Suggestion;
    ///
    /// let suggestion = Suggestion::text_only("users");
    /// assert_eq!(suggestion.text, "users");
    /// assert_eq!(suggestion.description, "");
    /// ```
    pub fn text_only(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            description: String::new(),
        }
    }
}

impl From<&str> for Suggestion {
    /// Create a suggestion from a string with no description
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::suggestion::Suggestion;
    ///
    /// let suggestion: Suggestion = "users".into();
    /// assert_eq!(suggestion.text, "users");
    /// assert_eq!(suggestion.description, "");
    /// ```
    fn from(text: &str) -> Self {
        Self::text_only(text)
    }
}

impl From<String> for Suggestion {
    /// Create a suggestion from a String with no description
    fn from(text: String) -> Self {
        Self::text_only(text)
    }
}

impl From<(String, String)> for Suggestion {
    /// Create a suggestion from a tuple of (text, description)
    ///
    /// # Examples
    ///
    /// ```
    /// use replkit_core::suggestion::Suggestion;
    ///
    /// let suggestion: Suggestion = ("users".to_string(), "Store user data".to_string()).into();
    /// assert_eq!(suggestion.text, "users");
    /// assert_eq!(suggestion.description, "Store user data");
    /// ```
    fn from((text, description): (String, String)) -> Self {
        Self::new(text, description)
    }
}

impl From<(&str, &str)> for Suggestion {
    /// Create a suggestion from a tuple of (&str, &str)
    fn from((text, description): (&str, &str)) -> Self {
        Self::new(text, description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_new() {
        let suggestion = Suggestion::new("test", "A test suggestion");
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "A test suggestion");
    }

    #[test]
    fn test_suggestion_text_only() {
        let suggestion = Suggestion::text_only("test");
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "");
    }

    #[test]
    fn test_from_str() {
        let suggestion: Suggestion = "test".into();
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "");
    }

    #[test]
    fn test_from_string() {
        let suggestion: Suggestion = "test".to_string().into();
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "");
    }

    #[test]
    fn test_from_tuple_string() {
        let suggestion: Suggestion = ("test".to_string(), "description".to_string()).into();
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "description");
    }

    #[test]
    fn test_from_tuple_str() {
        let suggestion: Suggestion = ("test", "description").into();
        assert_eq!(suggestion.text, "test");
        assert_eq!(suggestion.description, "description");
    }

    #[test]
    fn test_suggestion_clone() {
        let original = Suggestion::new("test", "description");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_suggestion_debug() {
        let suggestion = Suggestion::new("test", "description");
        let debug_str = format!("{:?}", suggestion);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("description"));
    }
}
