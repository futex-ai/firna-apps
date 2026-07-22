//! Command-line argument definitions.

use clap::{Parser, Subcommand};

/// Repository automation arguments.
#[derive(Debug, Parser)]
#[command(name = "cargo xtask")]
pub(crate) struct Args {
    /// Automation task to run.
    #[command(subcommand)]
    pub(crate) task: Task,
}

/// Supported repository automation tasks.
#[derive(Debug, Subcommand)]
pub(crate) enum Task {
    /// Run every repository verification check.
    Check,
    /// Audit app layout, dependency pins, versions, docs, and file lengths.
    RepositoryAudit {
        /// Git base revision used by the app version audit.
        #[arg(long, default_value = "origin/main")]
        base: String,
    },
    /// Review the committed branch diff against origin/main with Codex.
    Review,
}
