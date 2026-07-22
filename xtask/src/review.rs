//! Post-push Codex review invocation.

use std::path::Path;

use crate::command::{CommandRunner, CommandSpec};
use crate::error::Result;

const REVIEW_PROMPT: &str = "Review this Firna-owned app repository change for correctness, security, missing tests, CI deployment risks, and accidental behavior changes. Return only actionable findings with severity and file/line context; say explicitly when there are no findings.";

/// Reviews committed changes against the target branch.
pub(crate) fn run_review(runner: &dyn CommandRunner, workspace_root: &Path) -> Result<()> {
    runner.run(
        workspace_root,
        &CommandSpec::new("codex", ["review", "--base", "origin/main", REVIEW_PROMPT]),
    )
}
