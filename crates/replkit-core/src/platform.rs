//! Platform factory for cross-platform console creation.
//!
//! This module provides a factory pattern for creating platform-appropriate
//! ConsoleInput and ConsoleOutput implementations. It handles platform detection,
//! VT mode detection on Windows, and proper fallback logic.

use crate::{
    console::{ConsoleError, ConsoleInput, ConsoleOutput},
    repl::ReplError,
};

/// Factory trait for creating platform-appropriate console implementations.
///
/// This trait provides a clean abstraction for creating ConsoleInput and
/// ConsoleOutput implementations that are appropriate for the current platform.
/// Implementations should handle platform detection, capability detection,
/// and fallback logic automatically.
pub trait PlatformFactory {
    /// Create a ConsoleInput implementation for the current platform.
    ///
    /// This method should:
    /// - Detect the current platform
    /// - Choose the most appropriate implementation
    /// - Handle fallback logic (e.g., VT -> Legacy on Windows)
    /// - Return appropriate errors for unsupported platforms
    fn create_console_input(&self) -> Result<Box<dyn ConsoleInput>, ReplError>;

    /// Create a ConsoleOutput implementation for the current platform.
    ///
    /// This method should:
    /// - Detect the current platform
    /// - Choose the most appropriate implementation
    /// - Handle fallback logic (e.g., VT -> Legacy on Windows)
    /// - Return appropriate errors for unsupported platforms
    fn create_console_output(&self) -> Result<Box<dyn ConsoleOutput>, ReplError>;

    /// Create both ConsoleInput and ConsoleOutput for the current platform.
    ///
    /// This is a convenience method that creates both input and output
    /// implementations. The default implementation calls the individual
    /// creation methods.
    fn create_console_io(
        &self,
    ) -> Result<(Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>), ReplError> {
        let input = self.create_console_input()?;
        let output = self.create_console_output()?;
        Ok((input, output))
    }

    /// Get information about the current platform and its capabilities.
    ///
    /// This method returns a string describing the platform and the
    /// console implementation that would be used.
    fn platform_info(&self) -> String;
}

/// Native platform factory for Rust applications.
///
/// This factory creates native console implementations by delegating to
/// factory functions provided by external crates (like replkit-io).
/// It handles platform detection and fallback logic automatically.
///
/// # Platform Support
///
/// - **Unix/Linux/macOS**: Uses Unix console implementations
/// - **Windows**: Tries VT mode first, falls back to Legacy mode
/// - **WASM**: Uses WASM bridge implementations
/// - **Other platforms**: Returns UnsupportedFeature error
///
/// # Windows VT Detection
///
/// On Windows, this factory first attempts to create VT-mode console implementations.
/// If VT mode is not supported (e.g., in older cmd.exe), it automatically falls
/// back to legacy console implementations.
///
/// # Note
///
/// This is a trait-based factory that doesn't directly depend on replkit-io
/// to avoid circular dependencies. The actual implementations are provided
/// by external factory functions that implement the platform-specific logic.
pub struct NativePlatformFactory {
    /// Factory function for creating console input
    input_factory: Box<dyn Fn() -> Result<Box<dyn ConsoleInput>, ReplError> + Send + Sync>,
    /// Factory function for creating console output
    output_factory: Box<dyn Fn() -> Result<Box<dyn ConsoleOutput>, ReplError> + Send + Sync>,
    /// Platform information string
    platform_info: String,
}

impl NativePlatformFactory {
    /// Create a new native platform factory with custom factory functions.
    ///
    /// This allows external crates (like replkit-io) to provide the actual
    /// implementation while keeping the factory pattern in replkit-core.
    pub fn new(
        input_factory: Box<dyn Fn() -> Result<Box<dyn ConsoleInput>, ReplError> + Send + Sync>,
        output_factory: Box<dyn Fn() -> Result<Box<dyn ConsoleOutput>, ReplError> + Send + Sync>,
        platform_info: String,
    ) -> Self {
        NativePlatformFactory {
            input_factory,
            output_factory,
            platform_info,
        }
    }

    /// Create a default native platform factory with stub implementations.
    ///
    /// This creates a factory that returns errors for all operations.
    /// It's primarily useful for testing or when no actual console
    /// implementations are available.
    pub fn default_stub() -> Self {
        NativePlatformFactory {
            input_factory: Box::new(|| {
                Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                    feature: "console input".to_string(),
                    platform: "stub".to_string(),
                }))
            }),
            output_factory: Box::new(|| {
                Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                    feature: "console output".to_string(),
                    platform: "stub".to_string(),
                }))
            }),
            platform_info: "Stub platform factory (no implementations available)".to_string(),
        }
    }
}

impl PlatformFactory for NativePlatformFactory {
    fn create_console_input(&self) -> Result<Box<dyn ConsoleInput>, ReplError> {
        (self.input_factory)()
    }

    fn create_console_output(&self) -> Result<Box<dyn ConsoleOutput>, ReplError> {
        (self.output_factory)()
    }

    fn platform_info(&self) -> String {
        self.platform_info.clone()
    }
}

/// Convenience function to create a stub native platform factory.
///
/// This creates a factory that returns errors for all operations.
/// It's primarily useful for testing or when no actual console
/// implementations are available.
///
/// # Examples
///
/// ```
/// use replkit_core::platform::{create_native_factory, PlatformFactory};
///
/// let factory = create_native_factory();
/// let info = factory.platform_info();
/// assert!(info.contains("Stub"));
/// ```
pub fn create_native_factory() -> NativePlatformFactory {
    NativePlatformFactory::default_stub()
}

/// Convenience function to create console I/O using the native platform factory.
///
/// This function creates a native platform factory and uses it to create
/// both ConsoleInput and ConsoleOutput implementations for the current platform.
///
/// Note: This stub implementation will return errors. External crates should
/// provide their own factory functions that create working implementations.
///
/// # Examples
///
/// ```should_panic
/// use replkit_core::platform::create_native_console_io;
///
/// // This will panic because the stub implementation returns errors
/// let (input, output) = create_native_console_io().unwrap();
/// ```
pub fn create_native_console_io(
) -> Result<(Box<dyn ConsoleInput>, Box<dyn ConsoleOutput>), ReplError> {
    let factory = create_native_factory();
    factory.create_console_io()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_factory_creation() {
        let factory = NativePlatformFactory::default_stub();
        let info = factory.platform_info();
        assert!(!info.is_empty());
        assert!(info.contains("Stub"));
    }

    #[test]
    fn test_create_native_factory_function() {
        let factory = create_native_factory();
        let info = factory.platform_info();
        assert!(!info.is_empty());
        assert!(info.contains("Stub"));
    }

    #[test]
    fn test_stub_factory_returns_errors() {
        let factory = NativePlatformFactory::default_stub();

        // Both input and output creation should return errors
        assert!(factory.create_console_input().is_err());
        assert!(factory.create_console_output().is_err());
        assert!(factory.create_console_io().is_err());
    }

    #[test]
    fn test_custom_factory_creation() {
        let input_factory = Box::new(|| {
            Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                feature: "test input".to_string(),
                platform: "test".to_string(),
            }))
        });

        let output_factory = Box::new(|| {
            Err(ReplError::ConsoleError(ConsoleError::UnsupportedFeature {
                feature: "test output".to_string(),
                platform: "test".to_string(),
            }))
        });

        let factory =
            NativePlatformFactory::new(input_factory, output_factory, "Test platform".to_string());

        assert_eq!(factory.platform_info(), "Test platform");
        assert!(factory.create_console_input().is_err());
        assert!(factory.create_console_output().is_err());
    }

    #[test]
    fn test_platform_factory_trait() {
        let factory = create_native_factory();

        // Test that the trait methods work
        let info = factory.platform_info();
        assert!(!info.is_empty());

        // Test that create_console_io calls the individual methods
        let result = factory.create_console_io();
        assert!(result.is_err()); // Should fail with stub implementation
    }

    // Note: Integration tests for actual console creation would be better placed
    // in a separate integration test file that can be run in different environments
    // with real terminal access.
}
