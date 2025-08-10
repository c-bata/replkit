//! Basic REPL example demonstrating the complete ReplEngine functionality.
//!
//! This example shows how to create a simple REPL that echoes user input
//! and demonstrates the integration of all REPL components.

use replkit_core::repl::{ReplConfig, ReplEngine};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Basic REPL Example");
    println!("Type 'exit' to quit, or press Ctrl+D");
    println!("----------------------------------");

    // Create REPL configuration
    let config = ReplConfig {
        prompt: "basic> ".to_string(),
        executor: Box::new(|input| {
            if input.trim() == "exit" {
                println!("Goodbye!");
                std::process::exit(0);
            } else if input.trim().is_empty() {
                // Do nothing for empty input
            } else {
                println!("You entered: {}", input);
            }
            Ok(())
        }),
        exit_checker: Some(Box::new(|input, is_eof| {
            // Exit on Ctrl+D (EOF) or if user types "exit"
            is_eof || input.trim() == "exit"
        })),
        ..Default::default()
    };

    // Create REPL engine
    let mut engine = ReplEngine::new(config)?;

    // Note: In a real application, you would set console input/output here:
    // engine.set_console_input(Box::new(your_console_input));
    // engine.set_console_output(Box::new(your_console_output));

    println!("REPL engine created successfully!");
    println!("Note: This example doesn't run the actual REPL loop because");
    println!("it requires platform-specific console implementations.");
    println!("See the integration tests for complete workflow examples.");

    Ok(())
}
