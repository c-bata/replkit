//! Example demonstrating the Executor API similar to go-prompt's Run functionality
//!
//! This example shows how to use the `run` method with an executor function
//! to create an interactive command-line application.
//!
//! Run with: cargo run --example executor_example

use replkit::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Interactive Command Prompt");
    println!("Commands: help, echo <text>, quit");
    println!("Press Ctrl+C to interrupt, or type 'quit' to exit");
    println!();

    // Create a prompt with completion and exit checker
    let mut prompt = Prompt::builder()
        .with_prefix("cmd> ")
        .with_completer(StaticCompleter::from_strings(vec![
            "help", "echo", "quit", "status", "version",
        ]))
        .with_exit_checker(|input: &str, breakline: bool| {
            // Exit immediately when user types "quit" (before pressing Enter)
            if !breakline && input == "quit" {
                return true;
            }
            // Also exit after executing "quit" command
            if breakline && input == "quit" {
                return true;
            }
            false
        })
        .build()?;

    // Run the prompt with an executor
    let result = prompt.run(|input: &str| -> PromptResult<()> {
        let input = input.trim();

        if input.is_empty() {
            return Ok(());
        }

        match input {
            "help" => {
                println!("Available commands:");
                println!("  help     - Show this help message");
                println!("  echo <text> - Echo the given text");
                println!("  status   - Show application status");
                println!("  version  - Show version information");
                println!("  quit     - Exit the application");
            }
            "status" => {
                println!("Application is running normally");
            }
            "version" => {
                println!("Replkit Example v1.0.0");
            }
            "quit" => {
                println!("Goodbye!");
                // The exit checker will handle the actual exit
            }
            _ => {
                if let Some(text) = input.strip_prefix("echo ") {
                    // Remove "echo " prefix
                    println!("Echo: {}", text);
                } else {
                    println!(
                        "Unknown command: {}. Type 'help' for available commands.",
                        input
                    );
                }
            }
        }

        Ok(())
    });

    match result {
        Ok(()) => println!("\nExited normally"),
        Err(PromptError::Interrupted) => println!("\nInterrupted by user"),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
