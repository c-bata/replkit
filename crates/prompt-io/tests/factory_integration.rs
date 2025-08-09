//! Integration tests for the console factory functions
//!
//! These tests verify that the factory functions correctly select the appropriate
//! backend for each platform and handle fallbacks properly.

use prompt_io::{create_console_input, create_console_output, create_console_io, BackendType};

#[test]
fn test_create_console_input() {
    match create_console_input() {
        Ok(input) => {
            let caps = input.get_capabilities();
            
            #[cfg(unix)]
            {
                assert_eq!(caps.backend_type, BackendType::UnixVt);
                assert_eq!(caps.platform_name, "Unix VT");
            }
            
            #[cfg(windows)]
            {
                // Should be either VT or Legacy depending on system support
                assert!(caps.backend_type == BackendType::WindowsVt || 
                       caps.backend_type == BackendType::WindowsLegacy);
                assert!(caps.platform_name.contains("Windows"));
            }
            
            #[cfg(target_arch = "wasm32")]
            {
                assert_eq!(caps.backend_type, BackendType::WasmBridge);
                assert!(caps.platform_name.contains("WASM"));
            }
            
            println!("Created console input: {} ({:?})", caps.platform_name, caps.backend_type);
        }
        Err(e) => {
            panic!("Failed to create console input: {}", e);
        }
    }
}

#[test]
fn test_create_console_output() {
    match create_console_output() {
        Ok(output) => {
            let caps = output.get_capabilities();
            
            #[cfg(unix)]
            {
                assert_eq!(caps.backend_type, BackendType::UnixVt);
                assert_eq!(caps.platform_name, "Unix VT");
            }
            
            #[cfg(windows)]
            {
                // Should be either VT or Legacy depending on system support
                assert!(caps.backend_type == BackendType::WindowsVt || 
                       caps.backend_type == BackendType::WindowsLegacy);
                assert!(caps.platform_name.contains("Windows"));
            }
            
            #[cfg(target_arch = "wasm32")]
            {
                assert_eq!(caps.backend_type, BackendType::WasmBridge);
                assert!(caps.platform_name.contains("WASM"));
            }
            
            println!("Created console output: {} ({:?})", caps.platform_name, caps.backend_type);
        }
        Err(e) => {
            panic!("Failed to create console output: {}", e);
        }
    }
}

#[test]
fn test_create_console_io() {
    match create_console_io() {
        Ok((input, output)) => {
            let input_caps = input.get_capabilities();
            let output_caps = output.get_capabilities();
            
            // Input and output should use the same backend type
            assert_eq!(input_caps.backend_type, output_caps.backend_type);
            
            println!("Created console I/O pair: {} input, {} output", 
                    input_caps.platform_name, output_caps.platform_name);
        }
        Err(e) => {
            panic!("Failed to create console I/O: {}", e);
        }
    }
}

#[cfg(windows)]
#[test]
fn test_windows_vt_fallback_behavior() {
    // This test verifies that on Windows, we try VT first and fall back to legacy
    // We can't easily test the actual fallback without mocking, but we can verify
    // that the factory function returns a valid implementation
    
    match create_console_input() {
        Ok(input) => {
            let caps = input.get_capabilities();
            
            // Should be either VT or Legacy
            assert!(caps.backend_type == BackendType::WindowsVt || 
                   caps.backend_type == BackendType::WindowsLegacy);
            
            // Both should support basic functionality
            assert!(caps.supports_raw_mode);
            assert!(caps.supports_resize_events);
            assert!(caps.supports_unicode);
            
            println!("Windows console input backend: {} ({:?})", 
                    caps.platform_name, caps.backend_type);
        }
        Err(e) => {
            panic!("Failed to create Windows console input: {}", e);
        }
    }
}

#[test]
fn test_backend_type_debug() {
    // Test that BackendType implements Debug properly
    let types = [
        BackendType::UnixVt,
        BackendType::WindowsVt,
        BackendType::WindowsLegacy,
        BackendType::WasmBridge,
        BackendType::Mock,
    ];
    
    for backend_type in &types {
        let debug_str = format!("{:?}", backend_type);
        assert!(!debug_str.is_empty());
        println!("BackendType::{:?}", backend_type);
    }
}

#[test]
fn test_multiple_factory_calls() {
    // Test that multiple calls to factory functions work correctly
    let input1 = create_console_input().expect("First input creation failed");
    let input2 = create_console_input().expect("Second input creation failed");
    
    let caps1 = input1.get_capabilities();
    let caps2 = input2.get_capabilities();
    
    // Should create the same type of backend
    assert_eq!(caps1.backend_type, caps2.backend_type);
    assert_eq!(caps1.platform_name, caps2.platform_name);
    
    println!("Multiple factory calls successful: {}", caps1.platform_name);
}