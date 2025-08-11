//! Integration tests for the Executor API

use replkit::prelude::*;
use replkit_io::mock::MockConsoleInput;
use replkit_io::mock::MockConsoleOutput;

#[test]
fn test_executor_trait_implementation() {
    // Test that closures implement Executor
    let mut counter = 0;
    let mut executor = |input: &str| -> PromptResult<()> {
        counter += 1;
        if input == "error" {
            Err(PromptError::Interrupted)
        } else {
            Ok(())
        }
    };

    assert!(executor.execute("hello").is_ok());
    assert!(executor.execute("error").is_err());
    assert_eq!(counter, 2);
}

#[test]
fn test_exit_checker_trait_implementation() {
    // Test that closures implement ExitChecker
    let exit_checker = |input: &str, breakline: bool| -> bool {
        if breakline {
            input == "quit"
        } else {
            input == "exit"
        }
    };

    assert!(!exit_checker.should_exit("hello", false));
    assert!(!exit_checker.should_exit("hello", true));
    assert!(exit_checker.should_exit("exit", false));
    assert!(exit_checker.should_exit("quit", true));
    assert!(!exit_checker.should_exit("quit", false));
    assert!(!exit_checker.should_exit("exit", true));
}

#[test]
fn test_prompt_builder_with_executor_components() {
    // Test that we can build a prompt with executor-related components
    let prompt = Prompt::builder()
        .with_prefix("test> ")
        .with_exit_checker(|input: &str, _breakline: bool| input == "quit")
        .with_completer(StaticCompleter::from_strings(vec!["help", "quit"]))
        .with_console_output(Box::new(MockConsoleOutput::new()))
        .with_console_input(Box::new(MockConsoleInput::new()))
        .build();

    assert!(prompt.is_ok());
    let prompt = prompt.unwrap();
    assert_eq!(prompt.prefix(), "test> ");
    
    // Test that completions work
    let completions = prompt.get_completions();
    assert_eq!(completions.len(), 2);
}

#[test]
fn test_executor_api_types() {
    // Test that the types are properly exported and usable
    fn test_executor(input: &str) -> PromptResult<()> {
        if input == "fail" {
            Err(PromptError::Interrupted)
        } else {
            Ok(())
        }
    }

    fn test_exit_checker(input: &str, breakline: bool) -> bool {
        input == "exit" && breakline
    }

    // These should compile and work
    assert!(test_executor("hello").is_ok());
    assert!(test_executor("fail").is_err());
    assert!(!test_exit_checker("hello", true));
    assert!(test_exit_checker("exit", true));
    assert!(!test_exit_checker("exit", false));
}

#[test]
fn test_prompt_with_all_features() {
    // Test building a prompt with all the new features
    let result = Prompt::builder()
        .with_prefix("full> ")
        .with_completer(|_doc: &Document| -> Vec<Suggestion> {
            vec![
                Suggestion::new("test", "Test command"),
                Suggestion::new("quit", "Exit command"),
            ]
        })
        .with_exit_checker(|input: &str, breakline: bool| {
            (input == "quit" && breakline) || (input == "exit" && !breakline)
        })
        .with_console_output(Box::new(MockConsoleOutput::new()))
        .with_console_input(Box::new(MockConsoleInput::new()))
        .build();

    assert!(result.is_ok());
    let prompt = result.unwrap();
    
    // Test basic functionality
    assert_eq!(prompt.prefix(), "full> ");
    
    // Test completions
    let completions = prompt.get_completions();
    assert_eq!(completions.len(), 2);
    assert!(completions.iter().any(|s| s.text == "test"));
    assert!(completions.iter().any(|s| s.text == "quit"));
}