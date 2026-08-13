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


VALID_DEPLOY_ROOT = """\
[gcp]
project_id = "firna-apps"
platform_project_id = "firna-498513"

[environments.production]
class = "production"
api_url = "https://api.firna.ai"
secret_prefix = "prod-app"
admin_email = "admin"
bootstrap_password_secret = "firna-prod-runtime-firna-bootstrap-password"

[environments.br-main]
class = "preview"
api_url = "https://br-main.api.preview.firna.ai"
secret_prefix = "preview-app"
admin_email = "preview-admin"
bootstrap_password_secret = "firna-preview-test-runtime-firna-bootstrap-password"
"""


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

    def test_deploy_config_only_change_skips_version_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn app() {}\n")
            repository.commit("seed")
            repository.write("apps/slack/deploy.toml", 'classes = ["production"]\n')

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(failures, [])

    def test_platform_runtime_pin_only_change_skips_version_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write(
                "apps/slack/tests/platform-runtime/Cargo.toml",
                'fna-apps-interface = { rev = "old" }\n',
            )
            repository.commit("seed")
            repository.write(
                "apps/slack/tests/platform-runtime/Cargo.toml",
                'fna-apps-interface = { rev = "new" }\n',
            )
            repository.write(
                "apps/slack/tests/platform-runtime/Cargo.lock",
                'source = "git+platform#new"\n',
            )
            repository.write(
                "apps/slack/tests/platform-runtime/asset_tests.rs",
                "assert_eq!(icon.image.data_base64, expected);\n",
            )

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(failures, [])

    def test_deploy_config_change_beside_code_change_requires_bump(self) -> None:
        with TestRepository() as repository:
            repository.write("apps/slack/manifest.yaml", "id: slack\nversion: 1.0.0\n")
            repository.write("apps/slack/component/src/lib.rs", "pub fn old() {}\n")
            repository.commit("seed")
            repository.write("apps/slack/deploy.toml", 'classes = ["production"]\n')
            repository.write("apps/slack/component/src/lib.rs", "pub fn changed() {}\n")

            failures = repository_audit.audit_changed_versions(repository.root, "HEAD")

            self.assertEqual(len(failures), 1)
            self.assertIn("version `1.0.0` is not above `1.0.0`", failures[0])

    def test_audit_deploy_config_reports_invalid_configuration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_DEPLOY_ROOT)
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["staging"]\n')

            failures = repository_audit.audit_deploy_config(root)

            self.assertTrue(
                any("apps/x/deploy.toml classes entry `staging`" in item for item in failures)
            )

    def test_audit_deploy_config_accepts_valid_configuration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_DEPLOY_ROOT)
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["production"]\n')

            self.assertEqual(repository_audit.audit_deploy_config(root), [])

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

    def test_missing_runtime_lockfile_is_reported_without_crashing(self) -> None:
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
                f"  FIRNA_PLATFORM_REPOSITORY: {repository}\n"
                f"  FIRNA_PLATFORM_REVISION: {revision}\n",
            )
            write(
                root / "apps/x/tests/platform-runtime/Cargo.toml",
                "[dependencies]\n"
                f'fna-apps-interface = {{ git = "{repository}", rev = "{revision}" }}\n'
                f'fna-apps-wasm = {{ git = "{repository}", rev = "{revision}" }}\n',
            )

            failures = repository_audit.audit_platform_pins(root)

            self.assertEqual(
                failures,
                ["apps/x/tests/platform-runtime/Cargo.lock is required"],
            )

    def test_partial_platform_revision_update_rejects_every_stale_surface(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = "https://github.com/futex-ai/firna.git"
            old_revision = "1" * 40
            new_revision = "2" * 40
            write(
                root / "platform.toml",
                f'repository = "{repository}"\nrevision = "{new_revision}"\n',
            )
            write(
                root / ".github/workflows/deploy-apps.yml",
                "env:\n"
                f"  FIRNA_PLATFORM_REPOSITORY: {repository}\n"
                f"  FIRNA_PLATFORM_REVISION: {old_revision}\n",
            )
            for app_id in ("slack", "x"):
                manifest_root = root / f"apps/{app_id}/tests/platform-runtime"
                extra_dependencies = ""
                if app_id == "x":
                    extra_dependencies = (
                        f'fna-apps = {{ git = "{repository}", rev = "{old_revision}" }}\n'
                        f'fna-apps-store-interface = {{ git = "{repository}", '
                        f'rev = "{old_revision}" }}\n'
                    )
                write(
                    manifest_root / "Cargo.toml",
                    "[dependencies]\n"
                    f"{extra_dependencies}"
                    f'fna-apps-interface = {{ git = "{repository}", rev = "{old_revision}" }}\n'
                    f'fna-apps-wasm = {{ git = "{repository}", rev = "{old_revision}" }}\n',
                )
                write(
                    manifest_root / "Cargo.lock",
                    f'source = "git+{repository}?rev={old_revision}#{old_revision}"\n',
                )

            failures = repository_audit.audit_platform_pins(root)

            self.assertEqual(len(failures), 9)
            self.assertTrue(any("FIRNA_PLATFORM_REVISION" in item for item in failures))
            self.assertEqual(
                sum("apps/slack/tests/platform-runtime" in failure for failure in failures),
                3,
            )
            self.assertEqual(
                sum("apps/x/tests/platform-runtime" in failure for failure in failures),
                5,
            )
            self.assertTrue(any("fna-apps must match" in item for item in failures))
            self.assertTrue(
                any("fna-apps-store-interface must match" in item for item in failures)
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
