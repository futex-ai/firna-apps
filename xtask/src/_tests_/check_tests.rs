//! Verification command-plan tests.

use std::fs;
use std::path::Path;

use super::{COMPONENT_MANIFESTS, RUNTIME_TEST_MANIFESTS, check_commands};

#[test]
fn check_plan_lists_every_manifest_on_disk() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live below the workspace root");
    let apps_root = workspace_root.join("apps");
    let mut expected_components = Vec::new();
    let mut expected_runtime_tests = Vec::new();

    for entry in fs::read_dir(apps_root).expect("read apps directory") {
        let app_root = entry.expect("read app entry").path();
        if !app_root.is_dir() {
            continue;
        }
        let app_id = app_root
            .file_name()
            .expect("app directory must have a name")
            .to_string_lossy();
        expected_components.push(format!("apps/{app_id}/component/Cargo.toml"));
        expected_runtime_tests.push(format!("apps/{app_id}/tests/platform-runtime/Cargo.toml"));
    }
    expected_components.sort();
    expected_runtime_tests.sort();

    let mut actual_components = COMPONENT_MANIFESTS.to_vec();
    let mut actual_runtime_tests = RUNTIME_TEST_MANIFESTS.to_vec();
    actual_components.sort();
    actual_runtime_tests.sort();

    assert_eq!(actual_components, expected_components);
    assert_eq!(actual_runtime_tests, expected_runtime_tests);
}

#[test]
fn check_plan_covers_every_standalone_package() {
    let commands = check_commands();

    for manifest in COMPONENT_MANIFESTS.iter().chain(RUNTIME_TEST_MANIFESTS) {
        for cargo_command in ["fmt", "clippy"] {
            assert!(
                commands.iter().any(|command| {
                    command.program() == "cargo"
                        && command.args().first().map(String::as_str) == Some(cargo_command)
                        && command.args().iter().any(|argument| argument == manifest)
                }),
                "missing cargo {cargo_command} for {manifest}"
            );
        }
    }
    for manifest in COMPONENT_MANIFESTS {
        for cargo_command in ["build", "test"] {
            assert!(
                commands.iter().any(|command| {
                    command.program() == "cargo"
                        && command.args().first().map(String::as_str) == Some(cargo_command)
                        && command.args().iter().any(|argument| argument == manifest)
                }),
                "missing cargo {cargo_command} for {manifest}"
            );
        }
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
