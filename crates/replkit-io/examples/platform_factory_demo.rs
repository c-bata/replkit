//! Platform Factory Demo with Real Implementations
//!
//! This example demonstrates how to use the replkit-io platform factory
//! to create real console implementations.

use replkit_core::platform::PlatformFactory;
use replkit_io::create_platform_factory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Platform Factory Demo with Real Implementations");
    println!("===============================================");

    // Create a platform factory with real implementations
    let factory = create_platform_factory();
    println!("Platform info: {}", factory.platform_info());

    println!();
    println!("Testing console creation...");

    // Test console input creation
    match factory.create_console_input() {
        Ok(_input) => {
            println!("✓ Console input created successfully");
        }
        Err(e) => {
            println!("✗ Console input creation failed: {}", e);
        }
    }

    // Test console output creation
    match factory.create_console_output() {
        Ok(_output) => {
            println!("✓ Console output created successfully");
        }
        Err(e) => {
            println!("✗ Console output creation failed: {}", e);
        }
    }

    // Test combined console I/O creation
    match factory.create_console_io() {
        Ok((_input, _output)) => {
            println!("✓ Console I/O created successfully");
            println!("  Both input and output implementations are ready for use");
        }
        Err(e) => {
            println!("✗ Console I/O creation failed: {}", e);
        }
    }

    println!();
    println!("Platform-specific information:");

    #[cfg(unix)]
    println!("  Running on Unix platform with VT100-compatible console");

    #[cfg(windows)]
    println!("  Running on Windows platform with VT mode detection and Legacy fallback");

    #[cfg(target_arch = "wasm32")]
    println!("  Running on WASM platform with bridge console I/O");

    println!();
    println!("Demo completed!");
    println!();
    println!("The factory successfully created platform-appropriate console implementations.");
    println!("These can now be used with the REPL engine or other console applications.");

    Ok(())
}
