//! Post-push Codex review invocation.

use std::path::Path;

use crate::command::{CommandRunner, CommandSpec};
use crate::error::Result;

/// Reviews committed changes against the target branch.
pub(crate) fn run_review(runner: &dyn CommandRunner, workspace_root: &Path) -> Result<()> {
    runner.run(workspace_root, &review_command())
}

fn review_command() -> CommandSpec {
    CommandSpec::new("codex", ["review", "--base", "origin/main"])
}

#[cfg(test)]
#[path = "_tests_/review_tests.rs"]
mod review_tests;
