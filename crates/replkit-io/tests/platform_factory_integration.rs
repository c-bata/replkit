//! Integration tests for platform factory functionality.
//!
//! These tests verify that the platform factory can create working console
//! implementations on different platforms with proper fallback logic.

use replkit_core::platform::PlatformFactory;
use replkit_io::create_platform_factory;

#[test]
fn test_platform_factory_creates_console_implementations() {
    let factory = create_platform_factory();

    // Test platform info
    let info = factory.platform_info();
    assert!(!info.is_empty());
    println!("Platform info: {}", info);

    // Platform info should contain platform-specific information
    #[cfg(unix)]
    assert!(info.contains("Unix") || info.contains("VT100"));

    #[cfg(windows)]
    assert!(info.contains("Windows"));

    #[cfg(target_arch = "wasm32")]
    assert!(info.contains("WASM"));
}

#[test]
fn test_platform_factory_console_creation_errors_gracefully() {
    let factory = create_platform_factory();

    // On most platforms, console creation should work or fail gracefully
    // We can't guarantee success in all test environments, but we can
    // ensure that errors are handled properly

    match factory.create_console_input() {
        Ok(_input) => {
            println!("Console input created successfully");
        }
        Err(e) => {
            println!(
                "Console input creation failed (expected in some test environments): {}",
                e
            );
            // Error should be properly formatted
            assert!(!e.to_string().is_empty());
        }
    }

    match factory.create_console_output() {
        Ok(_output) => {
            println!("Console output created successfully");
        }
        Err(e) => {
            println!(
                "Console output creation failed (expected in some test environments): {}",
                e
            );
            // Error should be properly formatted
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
fn test_platform_factory_create_console_io() {
    let factory = create_platform_factory();

    // Test the convenience method that creates both input and output
    match factory.create_console_io() {
        Ok((_input, _output)) => {
            println!("Console I/O created successfully");
        }
        Err(e) => {
            println!(
                "Console I/O creation failed (expected in some test environments): {}",
                e
            );
            // Error should be properly formatted
            assert!(!e.to_string().is_empty());
        }
    }
}

#[cfg(unix)]
#[test]
fn test_unix_platform_detection() {
    let factory = create_platform_factory();
    let info = factory.platform_info();

    // On Unix platforms, should indicate VT100 compatibility
    assert!(info.contains("Unix") || info.contains("VT100"));
    assert!(info.contains(std::env::consts::OS));
}

#[cfg(windows)]
#[test]
fn test_windows_platform_detection() {
    let factory = create_platform_factory();
    let info = factory.platform_info();

    // On Windows platforms, should indicate VT mode detection
    assert!(info.contains("Windows"));
    assert!(info.contains("VT mode") || info.contains("fallback"));
}

#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasm_platform_detection() {
    let factory = create_platform_factory();
    let info = factory.platform_info();

    // On WASM platforms, should indicate bridge I/O
    assert!(info.contains("WASM"));
    assert!(info.contains("bridge"));
}

#[test]
fn test_platform_factory_trait_implementation() {
    let factory = create_platform_factory();

    // Verify that the factory properly implements the PlatformFactory trait
    let _: &dyn PlatformFactory = &factory;

    // Test that all trait methods are available
    let _info = factory.platform_info();
    let _input_result = factory.create_console_input();
    let _output_result = factory.create_console_output();
    let _io_result = factory.create_console_io();
}
