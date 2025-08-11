pub mod capture;
pub mod cli;
pub mod comparator;
pub mod config;
pub mod error;
pub mod executor;
pub mod pty;

pub use capture::{ScreenCapturer, Snapshot, ContentNormalizer, NormalizationOptions};
pub use cli::{Cli, Commands, RunConfig};
pub use comparator::{SnapshotComparator, ComparisonResult, ComparisonAction};
pub use config::{StepDefinition, CommandConfig, TtyConfig, Step, InputSpec, SnapshotConfig};
pub use error::{Result, SnapshotError, ComparisonError};
pub use executor::{StepExecutor, ExecutionResult};
pub use pty::{PtyManager, key_spec_to_bytes};