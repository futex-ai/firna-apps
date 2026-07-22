//! Typed errors returned by repository automation.

use std::io;
use std::process::ExitStatus;

use thiserror::Error;

use crate::command::CommandSpec;

/// Result returned by xtask operations.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Failure returned by repository automation.
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// The workspace root cannot be derived from the xtask manifest directory.
    #[error("[xtask/error] workspace root is unavailable")]
    WorkspaceRootUnavailable,
    /// A subprocess could not be started.
    #[error("[xtask/error] failed to start `{command}`: {source}")]
    CommandStart {
        /// Command that failed to start.
        command: CommandSpec,
        /// Underlying operating-system failure.
        source: io::Error,
    },
    /// A subprocess returned a non-success status.
    #[error("[xtask/error] `{command}` failed with {status}")]
    CommandFailed {
        /// Command that failed.
        command: CommandSpec,
        /// Process exit status.
        status: ExitStatus,
    },
}
