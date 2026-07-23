//! Verification command-plan tests.

use super::{COMPONENT_MANIFESTS, RUNTIME_TEST_MANIFESTS, check_commands};

#[test]
fn check_plan_covers_every_standalone_package() {
    let commands = check_commands();

    for manifest in COMPONENT_MANIFESTS.iter().chain(RUNTIME_TEST_MANIFESTS) {
        for cargo_command in ["fmt", "clippy"] {
            assert!(commands.iter().any(|command| {
                command.program() == "cargo"
                    && command.args().first().map(String::as_str) == Some(cargo_command)
                    && command.args().iter().any(|argument| argument == manifest)
            }));
        }
    }
    for manifest in COMPONENT_MANIFESTS {
        assert!(commands.iter().any(|command| {
            command.program() == "cargo"
                && command.args().first().map(String::as_str) == Some("build")
                && command.args().iter().any(|argument| argument == manifest)
        }));
    }
    for manifest in RUNTIME_TEST_MANIFESTS {
        assert!(commands.iter().any(|command| {
            command.program() == "cargo"
                && command.args().first().map(String::as_str) == Some("test")
                && command.args().iter().any(|argument| argument == manifest)
        }));
    }
}

#[test]
fn check_plan_runs_repository_and_workflow_audits_before_rust() {
    let commands = check_commands();

    assert_eq!(commands[0].program(), "python3");
    assert!(commands[0].args().iter().any(|arg| arg == "test_*.py"));
    assert_eq!(commands[1].args(), ["scripts/test-deploy-workflow.sh"]);
    assert_eq!(
        commands[2].args().first().map(String::as_str),
        Some("scripts/repository_audit.py")
    );
    assert_eq!(commands[3].program(), "actionlint");
}

#[test]
fn dependency_resolving_cargo_commands_use_lockfiles() {
    let commands = check_commands();

    for command in commands.iter().filter(|command| {
        command.program() == "cargo" && command.args().first().map(String::as_str) != Some("fmt")
    }) {
        assert!(
            command.args().iter().any(|argument| argument == "--locked"),
            "Cargo command is not locked: {}",
            command.args().join(" ")
        );
    }
}
