//! Complete verification plan for app packages and repository automation.

use std::fs;
use std::path::{Path, PathBuf};

use crate::command::{CommandRunner, CommandSpec};
use crate::error::{Error, Result};

struct StandaloneManifests {
    components: Vec<String>,
    runtime_tests: Vec<String>,
}

trait ManifestInventory {
    fn discover(&self, workspace_root: &Path) -> Result<StandaloneManifests>;
}

struct FilesystemManifestInventory;

impl ManifestInventory for FilesystemManifestInventory {
    fn discover(&self, workspace_root: &Path) -> Result<StandaloneManifests> {
        let apps_root = workspace_root.join("apps");
        let app_directories = read_app_directories(&apps_root)?;
        let mut components = Vec::new();
        let mut runtime_tests = Vec::new();
        for app_directory in app_directories {
            add_manifest(
                workspace_root,
                app_directory.join("component/Cargo.toml"),
                &mut components,
            );
            add_manifest(
                workspace_root,
                app_directory.join("tests/platform-runtime/Cargo.toml"),
                &mut runtime_tests,
            );
        }
        components.sort();
        runtime_tests.sort();
        Ok(StandaloneManifests {
            components,
            runtime_tests,
        })
    }
}

/// Runs the complete repository verification plan.
pub(crate) fn run_check(runner: &dyn CommandRunner, workspace_root: &Path) -> Result<()> {
    let inventory = FilesystemManifestInventory;
    for command in check_commands(workspace_root, &inventory)? {
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

fn check_commands(
    workspace_root: &Path,
    inventory: &dyn ManifestInventory,
) -> Result<Vec<CommandSpec>> {
    let manifests = inventory.discover(workspace_root)?;
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
                ".github/workflows/app-preview-request.yml",
                ".github/workflows/app-preview-result.yml",
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
    for manifest in &manifests.components {
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
    for manifest in &manifests.runtime_tests {
        commands.push(cargo_manifest_command("fmt", manifest, &["--", "--check"]));
        commands.push(cargo_manifest_command(
            "clippy",
            manifest,
            &["--all-targets", "--locked", "--", "-D", "warnings"],
        ));
    }
    for manifest in &manifests.components {
        commands.push(cargo_manifest_command("test", manifest, &["--locked"]));
    }
    for manifest in &manifests.runtime_tests {
        commands.push(cargo_manifest_command("test", manifest, &["--locked"]));
    }
    Ok(commands)
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

fn read_app_directories(apps_root: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(apps_root) {
        Ok(entries) => entries,
        Err(source) => {
            return Err(Error::ManifestInventory {
                path: apps_root.to_path_buf(),
                source,
            });
        }
    };
    let mut directories = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                return Err(Error::ManifestInventory {
                    path: apps_root.to_path_buf(),
                    source,
                });
            }
        };
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
        }
    }
    directories.sort();
    Ok(directories)
}

fn add_manifest(workspace_root: &Path, path: PathBuf, manifests: &mut Vec<String>) {
    if !path.is_file() {
        return;
    }
    let relative = path.strip_prefix(workspace_root).unwrap_or(path.as_path());
    manifests.push(relative.to_string_lossy().replace('\\', "/"));
}

#[cfg(test)]
#[path = "_tests_/check_tests.rs"]
mod check_tests;
