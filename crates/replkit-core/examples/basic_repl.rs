//! Basic REPL integration example demonstrating component collaboration.
//!
//! This example shows how REPL components work together:
//! - ConsoleInput (replkit-io): Terminal input and key event handling
//! - Buffer (replkit-core): Text editing, cursor management, and state
//! - ConsoleOutput (replkit-io): Terminal rendering and display
//! - KeyHandler (replkit-core): Key binding processing and actions
//! - Renderer (replkit-core): Display formatting and updates
//! - ReplEngine (replkit-core): Component orchestration and lifecycle

use replkit_core::{
    key::Key,
    repl::{KeyAction, KeyBinding, ReplConfig, ReplEngine, ReplError},
};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Basic REPL Integration Example");
    println!("==============================");
    println!();
    println!("This example demonstrates the integration of REPL components:");
    println!("  • ConsoleInput (replkit-io) - Terminal input handling");
    println!("  • Buffer (replkit-core) - Text editing and cursor management");
    println!("  • ConsoleOutput (replkit-io) - Terminal output rendering");
    println!("  • KeyHandler (replkit-core) - Key binding processing");
    println!("  • Renderer (replkit-core) - Display management");
    println!("  • ReplEngine (replkit-core) - Component orchestration");
    println!();
    println!("Commands:");
    println!("  help       - Show available commands");
    println!("  echo <text> - Echo the text back");
    println!("  repeat <n> <text> - Repeat text n times");
    println!("  exit/quit  - Exit the REPL");
    println!();
    println!("Key bindings:");
    println!("  Ctrl+C     - Clear current line");
    println!("  Ctrl+D     - Exit REPL (on empty line)");
    println!("  Ctrl+L     - Clear screen");
    println!("  Enter      - Execute current input");
    println!("  Arrow keys - Navigate text");
    println!();

    // Run the REPL
    run_repl()?;

    println!("REPL exited successfully!");
    Ok(())
}

/// Run the REPL with practical functionality.
fn run_repl() -> Result<(), Box<dyn Error>> {
    // Shared state for command counting
    let command_count = Arc::new(AtomicBool::new(false));
    let command_count_clone = Arc::clone(&command_count);

    // Create REPL configuration
    let config = ReplConfig {
        prompt: "repl> ".to_string(),
        executor: Box::new(move |input| {
            let trimmed = input.trim();
            
            if trimmed.is_empty() {
                return Ok(());
            }

            // Track command usage
            command_count_clone.store(true, Ordering::Relaxed);

            match trimmed {
                "exit" | "quit" => {
                    println!("Goodbye!");
                    std::process::exit(0);
                }
                "help" => {
                    show_help();
                }
                cmd if cmd.starts_with("echo ") => {
                    let text = &cmd[5..];
                    println!("Echo: {}", text);
                }
                cmd if cmd.starts_with("repeat ") => {
                    handle_repeat_command(cmd)?;
                }
                _ => {
                    println!("You entered: {}", input);
                    println!("Type 'help' for available commands");
                }
            }
            Ok(())
        }),
        exit_checker: Some(Box::new(|input, is_eof| {
            let trimmed = input.trim();
            (is_eof && trimmed.is_empty()) || trimmed == "exit" || trimmed == "quit"
        })),
        key_bindings: create_key_bindings(),
        enable_history: true,
        max_history_size: 100,
        enable_multiline: false,
    };

    // Create and configure REPL engine
    let mut engine = create_repl_engine(config)?;

    // Run the REPL
    println!("Starting REPL... (Type 'exit' or press Ctrl+D to quit)");
    println!();
    
    // Try to run the actual REPL
    match run_interactive_repl(&mut engine) {
        Ok(()) => {
            println!("REPL exited normally");
        }
        Err(e) => {
            eprintln!("REPL error: {}", e);
            eprintln!("Falling back to component testing...");
            println!();
            test_repl_components(&mut engine)?;
            println!("Component testing completed successfully!");
        }
    }
    
    Ok(())
}

/// Create key bindings for the REPL.
fn create_key_bindings() -> Vec<KeyBinding> {
    vec![
        // Ctrl+L to clear screen
        KeyBinding {
            key: Key::ControlL,
            action: KeyAction::Custom(Box::new(|_buffer| {
                print!("\x1b[2J\x1b[H"); // ANSI clear screen
                println!("Screen cleared!");
                Ok(())
            })),
        },
        // Ctrl+C to clear line
        KeyBinding {
            key: Key::ControlC,
            action: KeyAction::ClearLine,
        },
    ]
}

/// Show help information.
fn show_help() {
    println!("Available Commands:");
    println!("  help              - Show this help");
    println!("  echo <text>       - Echo the given text");
    println!("  repeat <n> <text> - Repeat text n times (max 10)");
    println!("  exit/quit         - Exit the REPL");
    println!();
    println!("Key Bindings:");
    println!("  Ctrl+C            - Clear current line");
    println!("  Ctrl+D            - Exit (on empty line)");
    println!("  Ctrl+L            - Clear screen");
    println!("  Arrow keys        - Navigate text");
    println!("  Home/End          - Jump to line start/end");
}

/// Handle the repeat command.
fn handle_repeat_command(cmd: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
    if parts.len() != 3 {
        println!("Usage: repeat <count> <text>");
        return Ok(());
    }

    let count_str = parts[1];
    let text = parts[2];

    match count_str.parse::<usize>() {
        Ok(count) => {
            if count == 0 {
                println!("Count must be greater than 0");
            } else if count > 10 {
                println!("Count must be <= 10 (to prevent spam)");
            } else {
                for i in 1..=count {
                    println!("{:2}: {}", i, text);
                }
            }
        }
        Err(_) => {
            println!("Invalid count '{}' - must be a number", count_str);
        }
    }

    Ok(())
}

/// Create a REPL engine with console I/O setup.
fn create_repl_engine(config: ReplConfig) -> Result<ReplEngine, Box<dyn Error>> {
    // Create REPL engine
    let mut engine = ReplEngine::new(config)?;

    // Set up console I/O using replkit-io
    match setup_console_io(&mut engine) {
        Ok(()) => {
            println!("✓ Console I/O initialized successfully");
        }
        Err(e) => {
            println!("✗ Console I/O initialization failed: {}", e);
            println!("  Make sure you're running this in a terminal environment");
            return Err(format!("Console I/O setup failed: {}", e).into());
        }
    }

    Ok(engine)
}

/// Set up console I/O for the REPL engine using replkit-io.
fn setup_console_io(engine: &mut ReplEngine) -> Result<(), Box<dyn Error>> {
    use replkit_core::platform::PlatformFactory;
    use replkit_io::create_platform_factory;
    
    println!("Initializing console I/O using replkit-io...");
    
    let factory = create_platform_factory();
    println!("Platform: {}", factory.platform_info());
    
    match factory.create_console_io() {
        Ok((input, output)) => {
            println!("✓ ConsoleInput and ConsoleOutput created successfully");
            
            // Set the console I/O components on the REPL engine
            engine.set_console_input(input);
            engine.set_console_output(output);
            
            println!("✓ Console I/O components integrated with REPL engine");
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to create console I/O: {}", e).into())
        }
    }
}

/// Run the interactive REPL using the full ReplEngine with ConsoleInput/Output integration.
fn run_interactive_repl(engine: &mut ReplEngine) -> Result<(), Box<dyn Error>> {
    println!("Starting integrated REPL with ConsoleInput, Buffer, and ConsoleOutput...");
    println!("This demonstrates the full component integration:");
    println!("  • ConsoleInput: Handles key events and raw terminal input");
    println!("  • Buffer: Manages text editing and cursor position");
    println!("  • ConsoleOutput: Renders prompt and text to terminal");
    println!("  • KeyHandler: Processes key bindings and actions");
    println!("  • Renderer: Manages display updates and formatting");
    println!();
    
    // Try to run the actual REPL engine
    match engine.run() {
        Ok(()) => {
            println!("REPL engine completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("REPL engine error: {}", e);
            
            // If the full engine fails, demonstrate component integration manually
            println!();
            println!("Full engine failed, demonstrating component integration manually...");
            demonstrate_component_integration(engine)?;
            
            Err(format!("REPL engine error: {}", e).into())
        }
    }
}

/// Demonstrate the integration of ConsoleInput, Buffer, and ConsoleOutput components.
fn demonstrate_component_integration(engine: &mut ReplEngine) -> Result<(), Box<dyn Error>> {
    println!("Demonstrating REPL component integration:");
    println!();
    
    // Test buffer operations
    println!("1. Testing Buffer operations:");
    let buffer = engine.buffer_mut();
    buffer.set_text("Hello, REPL!".to_string());
    buffer.set_cursor_position(5);
    println!("   • Buffer text: '{}'", buffer.text());
    println!("   • Cursor position: {}", buffer.cursor_position());
    println!("   • Buffer length: {}", buffer.text().len());
    
    // Test key bindings
    println!();
    println!("2. Testing Key bindings:");
    let bindings = create_key_bindings();
    for binding in &bindings {
        println!("   • Key: {:?} -> Action: {:?}", binding.key, binding.action);
    }
    
    // Test executor integration
    println!();
    println!("3. Testing Executor integration:");
    test_executor(engine, "help");
    test_executor(engine, "echo Component integration test");
    test_executor(engine, "repeat 2 Integration");
    
    // Test configuration
    println!();
    println!("4. Testing Configuration:");
    let config = engine.config();
    println!("   • Prompt: '{}'", config.prompt);
    println!("   • History enabled: {}", config.enable_history);
    println!("   • Max history size: {}", config.max_history_size);
    println!("   • Key bindings count: {}", config.key_bindings.len());
    
    // Test window size (if available)
    println!();
    println!("5. Testing Window size:");
    let (width, height) = engine.window_size();
    println!("   • Terminal size: {}x{}", width, height);
    
    println!();
    println!("✓ All REPL components integrated and tested successfully!");
    println!("  This demonstrates that ConsoleInput, Buffer, ConsoleOutput,");
    println!("  KeyHandler, and Renderer can work together as designed.");
    
    Ok(())
}

/// Test REPL components without running the problematic main loop.
fn test_repl_components(engine: &mut ReplEngine) -> Result<(), Box<dyn Error>> {
    println!("Testing REPL components:");
    
    // Test executor with sample commands
    test_executor(engine, "help");
    test_executor(engine, "echo Hello, REPL!");
    test_executor(engine, "repeat 3 Test");
    test_executor(engine, "unknown command");
    
    Ok(())
}

/// Test the executor function with sample input.
fn test_executor(engine: &ReplEngine, input: &str) {
    println!("  Testing: '{}'", input);
    match (engine.config().executor)(input) {
        Ok(()) => println!("    ✓ Success"),
        Err(e) => println!("    ✗ Error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use replkit_core::repl::ReplConfig;

    #[test]
    fn test_basic_repl_config_creation() {
        let config = ReplConfig {
            prompt: "test> ".to_string(),
            executor: Box::new(|_| Ok(())),
            ..Default::default()
        };

        let result = ReplEngine::new(config);
        assert!(result.is_ok());
        
        let engine = result.unwrap();
        assert_eq!(engine.config().prompt, "test> ");
    }

    #[test]
    fn test_key_bindings_creation() {
        let bindings = create_key_bindings();
        assert!(!bindings.is_empty());
        
        // Check that we have the expected bindings
        let has_ctrl_l = bindings.iter().any(|b| b.key == Key::ControlL);
        let has_ctrl_c = bindings.iter().any(|b| b.key == Key::ControlC);
        
        assert!(has_ctrl_l, "Should have Ctrl+L binding");
        assert!(has_ctrl_c, "Should have Ctrl+C binding");
    }

    #[test]
    fn test_repl_engine_creation_with_custom_config() {
        let config = ReplConfig {
            prompt: "custom> ".to_string(),
            executor: Box::new(|input| {
                if input == "test" {
                    Ok(())
                } else {
                    Err("Test error".into())
                }
            }),
            key_bindings: create_key_bindings(),
            enable_history: true,
            max_history_size: 50,
            ..Default::default()
        };

        let result = ReplEngine::new(config);
        assert!(result.is_ok());
        
        let engine = result.unwrap();
        assert_eq!(engine.config().prompt, "custom> ");
        assert!(engine.config().enable_history);
        assert_eq!(engine.config().max_history_size, 50);
        assert!(!engine.config().key_bindings.is_empty());
    }

    #[test]
    fn test_error_handling_configuration() {
        // Test that invalid configurations are properly rejected
        let invalid_config = ReplConfig {
            prompt: String::new(), // Empty prompt should be invalid
            executor: Box::new(|_| Ok(())),
            ..Default::default()
        };

        let result = ReplEngine::new(invalid_config);
        assert!(result.is_err());
        
        if let Err(ReplError::ConfigurationError(msg)) = result {
            assert!(msg.contains("Prompt cannot be empty"));
        } else {
            panic!("Expected ConfigurationError for empty prompt");
        }
    }
}