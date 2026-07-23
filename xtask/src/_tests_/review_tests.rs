//! Review command regression tests.

use super::review_command;

#[test]
fn review_uses_the_supported_base_only_invocation() {
    let command = review_command();

    assert_eq!(command.program(), "codex");
    assert_eq!(command.args(), ["review", "--base", "origin/main"]);
}
