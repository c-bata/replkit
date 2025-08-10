//! Platform Factory Demo
//!
//! This example demonstrates how to use the PlatformFactory trait to create
//! console implementations in a platform-agnostic way.

use replkit_core::console::ConsoleError;
use replkit_core::platform::{NativePlatformFactory, PlatformFactory};
use replkit_core::repl::ReplError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Platform Factory Demo");
    println!("====================");

    // Create a stub factory (this is what's available in replkit-core by default)
    let stub_factory = replkit_core::platform::create_native_factory();
    println!(
        "Stub factory platform info: {}",
        stub_factory.platform_info()
    );

    // Try to create console I/O with the stub factory (this will fail)
    match stub_factory.create_console_io() {
        Ok((_input, _output)) => {
            println!("✓ Console I/O created successfully with stub factory");
        }
        Err(e) => {
            println!(
                "✗ Console I/O creation failed with stub factory (expected): {}",
                e
            );
        }
    }

    println!();

    // Demonstrate how to create a custom factory that returns errors
    println!("Creating custom factory with error implementations...");

    let custom_factory = create_error_factory();
    println!(
        "Custom factory platform info: {}",
        custom_factory.platform_info()
    );

    match custom_factory.create_console_io() {
        Ok((_input, _output)) => {
            println!("✓ Console I/O created successfully with custom factory");
        }
        Err(e) => {
            println!(
                "✗ Console I/O creation failed with custom factory (expected): {}",
                e
            );
        }
    }

    println!();

    // Demonstrate the trait interface
    println!("Testing PlatformFactory trait interface...");
    test_platform_factory_interface(&stub_factory);
    test_platform_factory_interface(&custom_factory);

    println!();
    println!("Demo completed!");
    println!();
    println!("Note: To use real console implementations, use the replkit-io crate:");
    println!("  use replkit_io::create_platform_factory;");
    println!("  let factory = create_platform_factory();");
    println!("  let (input, output) = factory.create_console_io()?;");

    Ok(())
}

/// Create a custom factory that returns specific errors for demonstration.
fn create_error_factory() -> NativePlatformFactory {
    let input_factory = Box::new(
        || -> Result<Box<dyn replkit_core::console::ConsoleInput>, ReplError> {
            Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                feature: "custom input".to_string(),
                platform: "demo".to_string(),
            }))
        },
    );

    let output_factory = Box::new(
        || -> Result<Box<dyn replkit_core::console::ConsoleOutput>, ReplError> {
            Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                feature: "custom output".to_string(),
                platform: "demo".to_string(),
            }))
        },
    );

    NativePlatformFactory::new(
        input_factory,
        output_factory,
        "Custom error factory for demonstration".to_string(),
    )
}

/// Test the PlatformFactory trait interface with any implementation.
fn test_platform_factory_interface(factory: &dyn PlatformFactory) {
    println!("  Platform info: {}", factory.platform_info());

    // Test individual creation methods
    match factory.create_console_input() {
        Ok(_) => println!("  ✓ Console input creation succeeded"),
        Err(e) => println!("  ✗ Console input creation failed: {}", e),
    }

    match factory.create_console_output() {
        Ok(_) => println!("  ✓ Console output creation succeeded"),
        Err(e) => println!("  ✗ Console output creation failed: {}", e),
    }

    // Test combined creation method
    match factory.create_console_io() {
        Ok(_) => println!("  ✓ Console I/O creation succeeded"),
        Err(e) => println!("  ✗ Console I/O creation failed: {}", e),
    }
}
