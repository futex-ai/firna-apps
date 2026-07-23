//! Complete verification plan for app packages and repository automation.

use std::path::Path;

use crate::command::{CommandRunner, CommandSpec};
use crate::error::Result;

const COMPONENT_MANIFESTS: &[&str] = &[
    "apps/dataforseo/component/Cargo.toml",
    "apps/exa/component/Cargo.toml",
    "apps/http/component/Cargo.toml",
    "apps/slack/component/Cargo.toml",
];
const RUNTIME_TEST_MANIFESTS: &[&str] = &[
    "apps/dataforseo/tests/platform-runtime/Cargo.toml",
    "apps/exa/tests/platform-runtime/Cargo.toml",
    "apps/http/tests/platform-runtime/Cargo.toml",
    "apps/slack/tests/platform-runtime/Cargo.toml",
];

/// Runs the complete repository verification plan.
pub(crate) fn run_check(runner: &dyn CommandRunner, workspace_root: &Path) -> Result<()> {
    for command in check_commands() {
        runner.run(workspace_root, &command)?;
    }
    Ok(())
}

/// Runs only the repository structural audit.
pub(crate) fn run_repository_audit(
    runner: &dyn CommandRunner,
    workspace_root: &Path,
    base: &str,
) -> Result<()> {
    runner.run(
        workspace_root,
        &CommandSpec::new("python3", ["scripts/repository_audit.py", "--base", base])
            .with_environment("PYTHONDONTWRITEBYTECODE", "1"),
    )
}

fn check_commands() -> Vec<CommandSpec> {
    let mut commands = vec![
        CommandSpec::new(
            "python3",
            [
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_*.py",
            ],
        )
        .with_environment("PYTHONDONTWRITEBYTECODE", "1"),
        CommandSpec::new("bash", ["scripts/test-deploy-workflow.sh"]),
        CommandSpec::new(
            "python3",
            ["scripts/repository_audit.py", "--base", "origin/main"],
        )
        .with_environment("PYTHONDONTWRITEBYTECODE", "1"),
        CommandSpec::new(
            "actionlint",
            [
                "-shellcheck",
                "",
                ".github/workflows/ci.yml",
                ".github/workflows/deploy-apps.yml",
            ],
        ),
        CommandSpec::new("cargo", ["fmt", "--all", "--", "--check"]),
        CommandSpec::new(
            "cargo",
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        ),
        CommandSpec::new("cargo", ["test", "--workspace", "--locked"]),
    ];
    for manifest in COMPONENT_MANIFESTS {
        commands.push(cargo_manifest_command("fmt", manifest, &["--", "--check"]));
        commands.push(cargo_manifest_command(
            "clippy",
            manifest,
            &[
                "--locked",
                "--lib",
                "--target",
                "wasm32-unknown-unknown",
                "--",
                "-D",
                "warnings",
            ],
        ));
        commands.push(cargo_manifest_command(
            "build",
            manifest,
            &["--target", "wasm32-unknown-unknown", "--locked"],
        ));
    }
    for manifest in RUNTIME_TEST_MANIFESTS {
        commands.push(cargo_manifest_command("fmt", manifest, &["--", "--check"]));
        commands.push(cargo_manifest_command(
            "clippy",
            manifest,
            &["--all-targets", "--locked", "--", "-D", "warnings"],
        ));
    }
    for manifest in [
        "apps/dataforseo/component/Cargo.toml",
        "apps/http/component/Cargo.toml",
    ] {
        commands.push(cargo_manifest_command("test", manifest, &["--locked"]));
    }
    for manifest in RUNTIME_TEST_MANIFESTS {
        commands.push(cargo_manifest_command("test", manifest, &["--locked"]));
    }
    commands
}

fn cargo_manifest_command(command: &str, manifest: &str, suffix: &[&str]) -> CommandSpec {
    let args = [command, "--manifest-path", manifest]
        .into_iter()
        .chain(suffix.iter().copied());
    CommandSpec::new("cargo", args)
        .with_environment("CARGO_INCREMENTAL", "0")
        .with_environment("CARGO_PROFILE_DEV_DEBUG", "0")
        .with_environment("CARGO_PROFILE_TEST_DEBUG", "0")
}

#[cfg(test)]
#[path = "_tests_/check_tests.rs"]
mod check_tests;
