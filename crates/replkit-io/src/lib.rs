//! Cross-platform console input/output abstraction and backends.
//!
//! Provides platform implementations for console I/O:
//! - UnixVtConsoleInput (POSIX/VT)
//! - WindowsVtConsoleInput (VT in Windows Terminal/PowerShell) [stubbed here]
//! - WindowsLegacyConsoleInput (cmd.exe events) [stubbed here]

use std::io;

// Re-export core types and traits
pub use replkit_core::{
    KeyEvent, KeyParser,
    ConsoleInput, ConsoleOutput, ConsoleError, ConsoleResult, EventLoopError,
    RawModeGuard, ConsoleCapabilities, OutputCapabilities, BackendType,
    TextStyle, Color, ClearType, SanitizationPolicy, SafeTextFilter, AsAny
};

// Helper function to convert io::Error to ConsoleError
pub fn io_error_to_console_error(e: io::Error) -> ConsoleError {
    ConsoleError::IoError(e.to_string())
}

/// Create both console input and output for the current platform
pub fn create_console_io() -> ConsoleResult<(Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>)> {
    let input = create_console_input()?;
    let output = create_console_output()?;
    Ok((input, output))
}

/// Create console input for the current platform
pub fn create_console_input() -> ConsoleResult<Box<dyn ConsoleInput>> {
    #[cfg(unix)]
    {
        let input = unix::UnixConsoleInput::new().map_err(io_error_to_console_error)?;
        Ok(Box::new(input))
    }
    
    #[cfg(windows)]
    {
        // Try VT mode first, fall back to legacy
        match windows::WindowsVtConsoleInput::new() {
            Ok(vt_input) => Ok(Box::new(vt_input)),
            Err(_) => {
                let legacy_input = windows::WindowsLegacyConsoleInput::new().map_err(io_error_to_console_error)?;
                Ok(Box::new(legacy_input))
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        let input = wasm::WasmBridgeConsoleInput::new().map_err(io_error_to_console_error)?;
        Ok(Box::new(input))
    }
    
    #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
    {
        Err(ConsoleError::UnsupportedFeature {
            feature: "console input".to_string(),
            platform: std::env::consts::OS.to_string(),
        })
    }
}

/// Create console output for the current platform
pub fn create_console_output() -> ConsoleResult<Box<dyn ConsoleOutput>> {
    #[cfg(unix)]
    {
        let output = unix::UnixConsoleOutput::new()?;
        Ok(Box::new(output))
    }
    
    #[cfg(windows)]
    {
        // Try VT mode first, fall back to legacy
        match windows::WindowsVtConsoleOutput::new() {
            Ok(vt_output) => Ok(Box::new(vt_output)),
            Err(_) => {
                let legacy_output = windows::WindowsLegacyConsoleOutput::new().map_err(io_error_to_console_error)?;
                Ok(Box::new(legacy_output))
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        let output = wasm::WasmBridgeConsoleOutput::new().map_err(io_error_to_console_error)?;
        Ok(Box::new(output))
    }
    
    #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
    {
        Err(ConsoleError::UnsupportedFeature {
            feature: "console output".to_string(),
            platform: std::env::consts::OS.to_string(),
        })
    }
}

/// Create mock console I/O for testing
pub fn create_mock_console_io() -> (Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>) {
    (
        Box::new(mock::MockConsoleInput::new()),
        Box::new(mock::MockConsoleOutput::new()),
    )
}

/// Create a native platform factory using replkit-io implementations.
///
/// This function creates a `NativePlatformFactory` from replkit-core that
/// uses the actual console implementations from replkit-io. It handles
/// platform detection, VT mode detection on Windows, and proper fallback logic.
///
/// # Platform Support
///
/// - **Unix/Linux/macOS**: Uses UnixConsoleInput/Output
/// - **Windows**: Tries WindowsVtConsoleInput/Output first, falls back to WindowsLegacyConsoleInput/Output
/// - **WASM**: Uses WasmBridgeConsoleInput/Output
/// - **Other platforms**: Returns UnsupportedFeature error
///
/// # Examples
///
/// ```
/// use replkit_io::create_platform_factory;
/// use replkit_core::platform::PlatformFactory;
///
/// let factory = create_platform_factory();
/// let (input, output) = factory.create_console_io().unwrap();
/// ```
pub fn create_platform_factory() -> replkit_core::platform::NativePlatformFactory {
    use replkit_core::platform::NativePlatformFactory;
    use replkit_core::repl::ReplError;

    let input_factory = Box::new(|| -> Result<Box<dyn ConsoleInput>, ReplError> {
        create_console_input().map_err(|e| ReplError::ConsoleError(e))
    });

    let output_factory = Box::new(|| -> Result<Box<dyn ConsoleOutput>, ReplError> {
        create_console_output().map_err(|e| ReplError::ConsoleError(e))
    });

    let platform_info = get_platform_info();

    NativePlatformFactory::new(input_factory, output_factory, platform_info)
}

/// Get information about the current platform and its console capabilities.
///
/// This function returns a string describing the platform and the
/// console implementation that would be used.
fn get_platform_info() -> String {
    #[cfg(unix)]
    {
        format!("Unix platform using VT100-compatible console I/O (OS: {})", std::env::consts::OS)
    }

    #[cfg(windows)]
    {
        // We can't easily detect VT support without actually trying to create the console,
        // so we'll just indicate the fallback strategy
        "Windows platform with VT mode detection (fallback to Legacy mode if VT unavailable)".to_string()
    }

    #[cfg(target_arch = "wasm32")]
    {
        "WASM platform using bridge console I/O".to_string()
    }

    #[cfg(not(any(unix, windows, target_arch = "wasm32")))]
    {
        format!("Unsupported platform: {}", std::env::consts::OS)
    }
}

// Platform-specific modules
#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(target_arch = "wasm32")]
mod wasm;

// Mock implementation for testing
pub mod mock;

// Re-export platform implementations
#[cfg(unix)]
pub use unix::{UnixConsoleInput, UnixConsoleOutput};

#[cfg(windows)]
pub use windows::{WindowsLegacyConsoleInput, WindowsVtConsoleInput, WindowsLegacyConsoleOutput, WindowsVtConsoleOutput};

#[cfg(target_arch = "wasm32")]
pub use wasm::{WasmBridgeConsoleInput, WasmBridgeConsoleOutput};

#[cfg(test)]
mod platform_factory_tests {
    use super::*;
    use replkit_core::platform::PlatformFactory;

    #[test]
    fn test_create_platform_factory() {
        let factory = create_platform_factory();
        let info = factory.platform_info();
        assert!(!info.is_empty());
        
        // Platform info should contain some indication of the platform
        #[cfg(unix)]
        assert!(info.contains("Unix"));
        
        #[cfg(windows)]
        assert!(info.contains("Windows"));
        
        #[cfg(target_arch = "wasm32")]
        assert!(info.contains("WASM"));
    }

    #[test]
    fn test_platform_factory_trait_methods() {
        let factory = create_platform_factory();
        
        // Test that the trait methods work
        let info = factory.platform_info();
        assert!(!info.is_empty());
        
        // Note: We can't easily test actual console creation in unit tests
        // because it requires real terminal access. These would be better as
        // integration tests.
    }

    #[test]
    fn test_get_platform_info() {
        let info = get_platform_info();
        assert!(!info.is_empty());
        
        // Should contain platform-specific information
        #[cfg(unix)]
        assert!(info.contains("Unix") || info.contains("VT100"));
        
        #[cfg(windows)]
        assert!(info.contains("Windows"));
        
        #[cfg(target_arch = "wasm32")]
        assert!(info.contains("WASM"));
    }
}