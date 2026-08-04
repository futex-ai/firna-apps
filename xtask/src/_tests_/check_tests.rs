//! Verification command-plan tests.

use std::path::{Path, PathBuf};

use super::{FilesystemManifestInventory, ManifestInventory, StandaloneManifests, check_commands};
use crate::error::Result;

struct TestManifestInventory;

impl ManifestInventory for TestManifestInventory {
    fn discover(&self, _workspace_root: &Path) -> Result<StandaloneManifests> {
        Ok(StandaloneManifests {
            components: vec![String::from("apps/future/component/Cargo.toml")],
            runtime_tests: vec![String::from(
                "apps/future/tests/platform-runtime/Cargo.toml",
            )],
        })
    }
}

#[test]
fn check_plan_covers_every_discovered_standalone_package() {
    let commands =
        check_commands(Path::new("."), &TestManifestInventory).expect("command plan should build");

    for manifest in [
        "apps/future/component/Cargo.toml",
        "apps/future/tests/platform-runtime/Cargo.toml",
    ] {
        for cargo_command in ["fmt", "clippy"] {
            assert_has_cargo_command(&commands, cargo_command, manifest);
        }
    }
    for cargo_command in ["build", "test"] {
        assert_has_cargo_command(&commands, cargo_command, "apps/future/component/Cargo.toml");
    }
    assert_has_cargo_command(
        &commands,
        "test",
        "apps/future/tests/platform-runtime/Cargo.toml",
    );
}

#[test]
fn filesystem_inventory_discovers_current_component_manifests() {
    let manifests = FilesystemManifestInventory
        .discover(&workspace_root())
        .expect("manifest inventory should succeed");

    assert!(
        manifests
            .components
            .contains(&String::from("apps/x/component/Cargo.toml"))
    );
    for manifest in manifests.components.iter().chain(&manifests.runtime_tests) {
        assert!(workspace_root().join(manifest).is_file());
    }
}

#[test]
fn check_plan_runs_repository_and_workflow_audits_before_rust() {
    let commands = commands();

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
    let commands = commands();

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

fn commands() -> Vec<crate::command::CommandSpec> {
    check_commands(&workspace_root(), &FilesystemManifestInventory)
        .expect("command plan should build")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should have a workspace parent")
        .to_path_buf()
}

fn assert_has_cargo_command(
    commands: &[crate::command::CommandSpec],
    cargo_command: &str,
    manifest: &str,
) {
    assert!(
        commands.iter().any(|command| {
            command.program() == "cargo"
                && command.args().first().map(String::as_str) == Some(cargo_command)
                && command.args().iter().any(|argument| argument == manifest)
        }),
        "missing cargo {cargo_command} for {manifest}"
    );
}
