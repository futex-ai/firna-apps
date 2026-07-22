//! Developer automation entrypoint for the Firna app repository.
#![warn(missing_docs, unreachable_pub)]

mod args;
mod check;
mod command;
mod error;
mod review;

use std::path::Path;

use clap::Parser;

use crate::args::{Args, Task};
use crate::check::run_check;
use crate::command::SystemCommandRunner;
use crate::error::Result;
use crate::review::run_review;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or(crate::error::Error::WorkspaceRootUnavailable)?;
    let runner = SystemCommandRunner;

    match args.task {
        Task::Check => run_check(&runner, workspace_root),
        Task::RepositoryAudit { base } => {
            check::run_repository_audit(&runner, workspace_root, &base)
        }
        Task::Review => run_review(&runner, workspace_root),
    }
}
