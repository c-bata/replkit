use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("PTY management error: {0}")]
    PtyError(#[from] PtyError),
    
    #[error("Process execution error: {0}")]
    ExecutionError(#[from] ExecutionError),
    
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("Snapshot comparison failed: {0}")]
    CompareError(#[from] CompareError),
    
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_yaml::Error),
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("Failed to create PTY: {0}")]
    CreateFailed(String),
    
    #[error("Failed to spawn command: {0}")]
    SpawnFailed(String),
    
    #[error("PTY I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Invalid key specification: {0}")]
    InvalidKey(String),
    
    #[error("Process timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Process exited unexpectedly with code {0}")]
    UnexpectedExit(i32),
    
    #[error("Wait condition failed: {0}")]
    WaitConditionFailed(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid window size format: {0}")]
    InvalidWindowSize(String),
    
    #[error("Invalid environment variable format: {0}")]
    InvalidEnvironmentVariable(String),
    
    #[error("Invalid duration format: {0}")]
    InvalidDuration(String),
    
    #[error("Step definition file not found: {0}")]
    StepFileNotFound(PathBuf),
    
    #[error("Invalid step definition: {0}")]
    InvalidStepDefinition(String),
}

#[derive(Debug, Error)]
pub enum CompareError {
    #[error("Golden snapshot file not found: {0}")]
    GoldenFileNotFound(PathBuf),
    
    #[error("Failed to read snapshot file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Snapshot content mismatch for: {0}")]
    ContentMismatch(String),
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Terminal capture failed: {0}")]
    TerminalError(String),
    
    #[error("Content normalization failed: {0}")]
    NormalizationError(String),
}

pub type Result<T, E = SnapshotError> = std::result::Result<T, E>;