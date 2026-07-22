//! Trait-backed subprocess execution for repository automation.

use std::fmt::{self, Display};
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// One subprocess invocation in a repository task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommandSpec {
    program: String,
    args: Vec<String>,
    environment: Vec<(String, String)>,
}

impl CommandSpec {
    /// Creates a subprocess specification.
    pub(crate) fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            environment: Vec::new(),
        }
    }

    /// Adds one environment variable to the subprocess.
    pub(crate) fn with_environment(mut self, key: &str, value: &str) -> Self {
        self.environment.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Returns the subprocess program.
    #[cfg(test)]
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// Returns the subprocess arguments.
    #[cfg(test)]
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }
}

impl Display for CommandSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.environment {
            write!(formatter, "{key}={value} ")?;
        }
        write!(formatter, "{}", self.program)?;
        for arg in &self.args {
            write!(formatter, " {arg}")?;
        }
        Ok(())
    }
}

/// Executes subprocess specifications.
pub(crate) trait CommandRunner {
    /// Runs one command from the repository root.
    fn run(&self, workspace_root: &Path, command: &CommandSpec) -> Result<()>;
}

/// Operating-system subprocess runner.
pub(crate) struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, workspace_root: &Path, command: &CommandSpec) -> Result<()> {
        println!("running {command}");
        let mut process = Command::new(&command.program);
        process
            .args(&command.args)
            .envs(command.environment.iter().cloned())
            .current_dir(workspace_root);
        if command.program == "cargo" && std::env::var_os("CARGO_TARGET_DIR").is_none() {
            process.env("CARGO_TARGET_DIR", workspace_root.join("target"));
        }
        let status = match process.status() {
            Ok(status) => status,
            Err(source) => {
                return Err(Error::CommandStart {
                    command: command.clone(),
                    source,
                });
            }
        };
        if status.success() {
            return Ok(());
        }
        Err(Error::CommandFailed {
            command: command.clone(),
            status,
        })
    }
}
