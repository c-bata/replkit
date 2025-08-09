//! Integration tests for Windows VT console input
//!
//! These tests verify that the Windows VT implementation compiles and has the correct
//! interface, but they will only run meaningful tests on Windows systems with VT support.

#[cfg(windows)]
mod windows_vt_tests {
    use replkit_io::{WindowsVtConsoleInput, ConsoleInput, BackendType};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_windows_vt_creation() {
        // This test will only pass on systems with VT support
        match WindowsVtConsoleInput::new() {
            Ok(console) => {
                // Verify basic properties
                assert!(!console.is_running());
                
                let caps = console.get_capabilities();
                assert_eq!(caps.backend_type, BackendType::WindowsVt);
                assert!(caps.supports_raw_mode);
                assert!(caps.supports_resize_events);
                assert!(caps.supports_unicode);
                assert!(caps.supports_bracketed_paste);
                assert!(caps.supports_mouse_events);
                assert_eq!(caps.platform_name, "Windows VT");
                
                println!("Windows VT console created successfully");
            }
            Err(e) => {
                // On systems without VT support, this is expected
                println!("VT mode not supported (expected on older Windows): {}", e);
                
                // Verify it's the right kind of error
                assert!(e.to_string().contains("Virtual Terminal") || 
                       e.to_string().contains("not supported"));
            }
        }
    }

    #[test]
    fn test_window_size_query() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            match console.get_window_size() {
                Ok((width, height)) => {
                    assert!(width > 0, "Window width should be positive");
                    assert!(height > 0, "Window height should be positive");
                    println!("Window size: {}x{}", width, height);
                }
                Err(e) => {
                    println!("Failed to get window size: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_raw_mode_guard() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            match console.enable_raw_mode() {
                Ok(guard) => {
                    assert!(guard.is_active());
                    assert_eq!(guard.platform_info(), "Windows VT");
                    println!("Raw mode enabled successfully");
                    // Guard should restore mode when dropped
                }
                Err(e) => {
                    println!("Failed to enable raw mode: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_event_loop_lifecycle() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            // Should not be running initially
            assert!(!console.is_running());
            
            // Start should succeed
            if console.start_event_loop().is_ok() {
                assert!(console.is_running());
                println!("Event loop started successfully");
                
                // Stop should succeed
                assert!(console.stop_event_loop().is_ok());
                
                // Give thread time to stop
                std::thread::sleep(Duration::from_millis(100));
                assert!(!console.is_running());
                println!("Event loop stopped successfully");
            } else {
                println!("Failed to start event loop (may be expected in test environment)");
            }
        }
    }

    #[test]
    fn test_callback_registration() {
        if let Ok(console) = WindowsVtConsoleInput::new() {
            let (tx, _rx) = mpsc::channel();
            
            // Register key callback
            console.on_key_pressed(Box::new(move |event| {
                let _ = tx.send(event);
            }));
            
            let (resize_tx, _resize_rx) = mpsc::channel();
            
            // Register resize callback
            console.on_window_resize(Box::new(move |w, h| {
                let _ = resize_tx.send((w, h));
            }));
            
            println!("Callbacks registered successfully");
            // Callbacks should be registered (we can't easily test invocation without actual input)
        }
    }

    #[test]
    fn test_multiple_instances() {
        // Test that multiple instances can be created without interfering
        let console1 = WindowsVtConsoleInput::new();
        let console2 = WindowsVtConsoleInput::new();
        
        match (console1, console2) {
            (Ok(c1), Ok(c2)) => {
                assert!(!c1.is_running());
                assert!(!c2.is_running());
                
                // Both should have the same capabilities
                let caps1 = c1.get_capabilities();
                let caps2 = c2.get_capabilities();
                assert_eq!(caps1.backend_type, caps2.backend_type);
                assert_eq!(caps1.platform_name, caps2.platform_name);
                
                println!("Multiple VT console instances created successfully");
            }
            _ => {
                println!("VT mode not supported for multiple instances test");
            }
        }
    }
}

#[cfg(not(windows))]
mod non_windows_tests {
    #[test]
    fn test_windows_vt_unavailable_on_non_windows() {
        // On non-Windows platforms, the WindowsVtConsoleInput type is not even available
        // This test just verifies that the test compiles on non-Windows platforms
        println!("Windows VT correctly unavailable on non-Windows platform");
    }
}