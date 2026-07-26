"""Tests for repository-level package and compatibility audits."""

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("repository_audit.py")
SPEC = importlib.util.spec_from_file_location("repository_audit", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
repository_audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(repository_audit)


class RepositoryAuditTests(unittest.TestCase):
    def test_changed_app_requires_version_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn old() {}\n")
            repository.commit("seed")
            repository.write("apps/slack/component/src/lib.rs", "pub fn changed() {}\n")

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(len(failures), 1)
            self.assertIn("version `1.0.0` is not above `1.0.0`", failures[0])

    def test_changed_app_accepts_version_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn old() {}\n")
            repository.commit("seed")
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.1\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn changed() {}\n")

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(failures, [])

    def test_staged_app_change_requires_version_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn old() {}\n")
            repository.commit("seed")
            repository.write("apps/slack/component/src/lib.rs", "pub fn changed() {}\n")
            run_git(repository.root, "add", ".")

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(len(failures), 1)
            self.assertIn("version `1.0.0` is not above `1.0.0`", failures[0])

    def test_platform_dependencies_must_match_canonical_pin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            revision = "1" * 40
            repository = "https://github.com/futex-ai/firna.git"
            write(root / "platform.toml", f'repository = "{repository}"\nrevision = "{revision}"\n')
            manifest_root = root / "apps/slack/tests/platform-runtime"
            write(
                manifest_root / "Cargo.toml",
                "[dependencies]\n"
                f'fna-apps-interface = {{ git = "{repository}", rev = "{revision}" }}\n'
                f'fna-apps-wasm = {{ git = "{repository}", rev = "{revision}" }}\n',
            )
            write(
                manifest_root / "Cargo.lock",
                f'source = "git+{repository}?rev={revision}#{revision}"\n',
            )
            write(
                root / ".github/workflows/deploy-apps.yml",
                "env:\n"
                f"  FIRNA_PLATFORM_REPOSITORY: {repository}\n"
                f"  FIRNA_PLATFORM_REVISION: {revision}\n",
            )

            self.assertEqual(repository_audit.audit_platform_pins(root), [])

            write(
                manifest_root / "Cargo.toml",
                "[dependencies]\n"
                'fna-apps-interface = { path = "../../../../crates/fna-apps-interface" }\n'
                f'fna-apps-wasm = {{ git = "{repository}", rev = "{revision}" }}\n',
            )
            failures = repository_audit.audit_platform_pins(root)
            self.assertTrue(any("must match platform.toml" in item for item in failures))
            self.assertTrue(any("must not use a local path" in item for item in failures))

    def test_deploy_workflow_must_match_canonical_pin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = "https://github.com/futex-ai/firna.git"
            revision = "1" * 40
            write(
                root / "platform.toml",
                f'repository = "{repository}"\nrevision = "{revision}"\n',
            )
            write(
                root / ".github/workflows/deploy-apps.yml",
                "env:\n"
                "  FIRNA_PLATFORM_REPOSITORY: https://github.com/futex-ai/other.git\n"
                f'  FIRNA_PLATFORM_REVISION: {"2" * 40}\n',
            )

            failures = repository_audit.audit_platform_pins(root)

            self.assertEqual(
                failures,
                [
                    ".github/workflows/deploy-apps.yml FIRNA_PLATFORM_REPOSITORY "
                    "must match platform.toml",
                    ".github/workflows/deploy-apps.yml FIRNA_PLATFORM_REVISION "
                    "must match platform.toml",
                ],
            )

    def test_markdown_links_must_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "README.md", "[missing](docs/missing.md)\n")

            failures = repository_audit.audit_markdown_links(root)

            self.assertEqual(failures, ["README.md has missing link `docs/missing.md`"])

    def test_static_rust_includes_must_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "apps/slack/tests/platform-runtime/documentation_tests.rs"
            write(source, 'const DOC: &str = include_str!("../../../../missing.md");\n')

            failures = repository_audit.audit_static_rust_includes(root)

            self.assertEqual(
                failures,
                [
                    "apps/slack/tests/platform-runtime/documentation_tests.rs "
                    "includes missing file `../../../../missing.md`"
                ],
            )

    def test_rust_file_lengths_ignore_cargo_target_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "apps/slack/component/src/lib.rs", "pub fn app() {}\n")
            write(
                root / "apps/slack/component/target/generated.rs",
                "generated\n" * 301,
            )

            failures = repository_audit.audit_rust_file_lengths(root)

            self.assertEqual(failures, [])

    def test_workspace_metadata_uses_apps_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(
                root / "Cargo.toml",
                "[workspace.package]\n"
                'repository = "https://github.com/futex-ai/firna.git"\n',
            )

            failures = repository_audit.audit_workspace_metadata(root)

            self.assertEqual(
                failures,
                ["Cargo.toml workspace repository must be the firna-apps repository"],
            )


class TestRepository:
    def __init__(self) -> None:
        self._temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self._temporary_directory.name)
        run_git(self.root, "init", "--quiet")
        run_git(self.root, "config", "user.email", "ci@example.com")
        run_git(self.root, "config", "user.name", "CI")

    def __enter__(self):
        return self

    def __exit__(self, *_args) -> None:
        self._temporary_directory.cleanup()

    def write(self, relative: str, contents: str) -> None:
        write(self.root / relative, contents)

    def commit(self, message: str) -> None:
        run_git(self.root, "add", ".")
        run_git(self.root, "commit", "--quiet", "-m", message)


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


def run_git(root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)


if __name__ == "__main__":
    unittest.main()
