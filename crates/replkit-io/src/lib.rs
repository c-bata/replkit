//! Cross-platform console input/output abstraction and backends.
//!
//! Provides platform implementations for console I/O:
//! - UnixVtConsoleInput (POSIX/VT)
//! - WindowsVtConsoleInput (VT in Windows Terminal/PowerShell) [stubbed here]
//! - WindowsLegacyConsoleInput (cmd.exe events) [stubbed here]

use std::io;

// Debug logging utilities
pub mod debug;

// Re-export core types and traits
pub use replkit_core::{
    BackendType, ClearType, Color, ConsoleCapabilities, ConsoleError, ConsoleInput, ConsoleOutput,
    ConsoleResult, EventLoopError, KeyEvent, KeyParser, OutputCapabilities, RawModeGuard,
    SafeTextFilter, SanitizationPolicy, TextStyle,
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
                let legacy_input =
                    windows::WindowsLegacyConsoleInput::new().map_err(io_error_to_console_error)?;
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
                let legacy_output = windows::WindowsLegacyConsoleOutput::new()
                    .map_err(io_error_to_console_error)?;
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
pub use windows::{
    WindowsLegacyConsoleInput, WindowsLegacyConsoleOutput, WindowsVtConsoleInput,
    WindowsVtConsoleOutput,
};

#[cfg(target_arch = "wasm32")]
pub use wasm::{WasmBridgeConsoleInput, WasmBridgeConsoleOutput};
