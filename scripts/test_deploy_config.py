"""Tests for repository deployment configuration validation."""

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("deploy_config.py")
SPEC = importlib.util.spec_from_file_location("deploy_config", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
deploy_config = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = deploy_config
SPEC.loader.exec_module(deploy_config)


VALID_ROOT = """\
[gcp]
project_id = "firna-apps"
platform_project_id = "firna-498513"

[environments.production]
class = "production"
api_url = "https://api.firna.ai"
secret_prefix = "prod-app"
admin_email = "admin"
bootstrap_password_secret = "firna-prod-runtime-firna-bootstrap-password"
automatic = true

[environments.br-main]
class = "preview"
api_url = "https://br-main.api.preview.firna.ai"
secret_prefix = "preview-app"
admin_email = "preview-admin"
bootstrap_password_secret = "firna-preview-test-runtime-firna-bootstrap-password"
automatic = true

[environments.br-apps]
class = "review"
api_url = "https://br-apps.api.preview.firna.ai"
secret_prefix = "review-app"
admin_email = "app-preview-admin"
bootstrap_password_secret = "firna-app-review-runtime-firna-bootstrap-password"
automatic = false
"""


class LoadRootTests(unittest.TestCase):
    def test_valid_root_loads(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)

            config, failures = deploy_config.load_root(root)

            self.assertEqual(failures, [])
            assert config is not None
            self.assertEqual(config.gcp.project_id, "firna-apps")
            self.assertEqual(config.gcp.platform_project_id, "firna-498513")
            self.assertEqual(
                [instance.name for instance in config.instances],
                ["br-apps", "br-main", "production"],
            )
            review = config.instances[0]
            self.assertEqual(review.environment_class, "review")
            self.assertEqual(review.secret_prefix, "review-app")
            self.assertFalse(review.automatic)
            production = config.instances[2]
            self.assertEqual(production.environment_class, "production")
            self.assertEqual(production.api_url, "https://api.firna.ai")
            self.assertEqual(production.secret_prefix, "prod-app")
            self.assertEqual(production.admin_email, "admin")
            self.assertEqual(
                production.bootstrap_password_secret,
                "firna-prod-runtime-firna-bootstrap-password",
            )
            self.assertTrue(production.automatic)

    def test_missing_root_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config, failures = deploy_config.load_root(Path(directory))

            self.assertIsNone(config)
            self.assertTrue(any("cannot read deploy.toml" in item for item in failures))

    def test_invalid_toml_fails(self) -> None:
        self.assert_root_failure("[gcp\n", "not valid TOML")

    def test_unknown_top_level_key_fails(self) -> None:
        self.assert_root_failure(VALID_ROOT + "\nextra = 1\n", "unknown key `extra`")

    def test_missing_gcp_table_fails(self) -> None:
        contents = VALID_ROOT.replace("[gcp]", "[gcp2]")
        self.assert_root_failure(contents, "must declare a [gcp] table")

    def test_invalid_project_id_fails(self) -> None:
        contents = VALID_ROOT.replace('project_id = "firna-apps"', 'project_id = "X"')
        self.assert_root_failure(contents, "project_id must be a valid")

    def test_missing_instance_fails(self) -> None:
        contents = VALID_ROOT.replace("[environments.br-main]", "[environments.br-2]")
        self.assert_root_failure(contents, "must declare [environments.br-main]")
        self.assert_root_failure(contents, "unknown key `br-2`")

    def test_missing_review_instance_fails(self) -> None:
        contents = VALID_ROOT.replace("[environments.br-apps]", "[environments.br-2]")
        self.assert_root_failure(contents, "must declare [environments.br-apps]")

    def test_wrong_class_fails(self) -> None:
        contents = VALID_ROOT.replace('class = "preview"', 'class = "production"')
        self.assert_root_failure(contents, "[environments.br-main] class must be `preview`")

    def test_wrong_url_fails(self) -> None:
        contents = VALID_ROOT.replace(
            'api_url = "https://api.firna.ai"',
            'api_url = "https://api.example.com"',
        )
        self.assert_root_failure(
            contents, "api_url must be `https://api.firna.ai`"
        )

    def test_wrong_secret_prefix_fails(self) -> None:
        contents = VALID_ROOT.replace(
            'secret_prefix = "preview-app"', 'secret_prefix = "prod-app"'
        )
        self.assert_root_failure(
            contents, "[environments.br-main] secret_prefix must be `preview-app`"
        )

    def test_review_instance_must_be_nonautomatic(self) -> None:
        contents = VALID_ROOT.replace("automatic = false", "automatic = true")
        self.assert_root_failure(
            contents, "[environments.br-apps] automatic must be `false`"
        )

    def test_automatic_defaults_to_true_for_ordinary_instances(self) -> None:
        contents = VALID_ROOT.replace("automatic = true\n", "", 1)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", contents)

            config, failures = deploy_config.load_root(root)

            self.assertEqual(failures, [])
            assert config is not None
            production = next(
                instance for instance in config.instances if instance.name == "production"
            )
            self.assertTrue(production.automatic)

    def test_automatic_must_be_boolean(self) -> None:
        contents = VALID_ROOT.replace("automatic = false", 'automatic = "false"')
        self.assert_root_failure(contents, "automatic must be a boolean")

    def test_empty_admin_email_fails(self) -> None:
        contents = VALID_ROOT.replace('admin_email = "admin"', 'admin_email = ""')
        self.assert_root_failure(
            contents, "[environments.production] admin_email must be a non-empty string"
        )

    def test_invalid_bootstrap_secret_fails(self) -> None:
        contents = VALID_ROOT.replace(
            'bootstrap_password_secret = "firna-prod-runtime-firna-bootstrap-password"',
            'bootstrap_password_secret = "Firna Secret"',
        )
        self.assert_root_failure(
            contents, "bootstrap_password_secret must be a Secret Manager secret id"
        )

    def test_unknown_instance_key_fails(self) -> None:
        contents = VALID_ROOT + '\n[environments.production.extra]\nvalue = "x"\n'
        self.assert_root_failure(
            contents, "[environments.production] has unknown key `extra`"
        )

    def assert_root_failure(self, contents: str, expected: str) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", contents)

            config, failures = deploy_config.load_root(root)

            self.assertIsNone(config)
            self.assertTrue(
                any(expected in item for item in failures),
                f"expected `{expected}` in {failures}",
            )


class AppClassesTests(unittest.TestCase):
    def test_missing_file_targets_all_classes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            app_root = Path(directory) / "apps" / "exa"
            app_root.mkdir(parents=True)

            classes, failures = deploy_config.app_classes(app_root)

            self.assertEqual(failures, [])
            self.assertEqual(
                classes, ("production", "preview", "review", "ephemeral")
            )

    def test_review_class_is_accepted(self) -> None:
        classes, failures = self.load_app('classes = ["review"]\n')

        self.assertEqual(failures, [])
        self.assertEqual(classes, ("review",))

    def test_production_only_app(self) -> None:
        classes, failures = self.load_app('classes = ["production"]\n')

        self.assertEqual(failures, [])
        self.assertEqual(classes, ("production",))

    def test_unknown_class_fails(self) -> None:
        _, failures = self.load_app('classes = ["production", "staging"]\n')

        self.assertTrue(any("`staging` is not a known class" in item for item in failures))

    def test_nested_list_entry_fails_without_crashing(self) -> None:
        _, failures = self.load_app('classes = [["production"]]\n')

        self.assertTrue(
            any("is not a known class" in item for item in failures),
            f"expected type failure in {failures}",
        )

    def test_empty_classes_fails(self) -> None:
        _, failures = self.load_app("classes = []\n")

        self.assertTrue(any("non-empty array" in item for item in failures))

    def test_duplicate_classes_fail(self) -> None:
        _, failures = self.load_app('classes = ["preview", "preview"]\n')

        self.assertTrue(any("must not repeat entries" in item for item in failures))

    def test_unknown_key_fails(self) -> None:
        _, failures = self.load_app('classes = ["preview"]\nenvironment = "x"\n')

        self.assertTrue(any("unknown key `environment`" in item for item in failures))

    def load_app(self, contents: str) -> tuple[tuple[str, ...], list[str]]:
        with tempfile.TemporaryDirectory() as directory:
            app_root = Path(directory) / "apps" / "x"
            write(app_root / "deploy.toml", contents)
            return deploy_config.app_classes(app_root)


class ValidateRepositoryTests(unittest.TestCase):
    def test_valid_repository_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)
            write(root / "apps/exa/manifest.yaml", "id: exa\nversion: 1.0.0\n")
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["production"]\n')

            self.assertEqual(deploy_config.validate_repository(root), [])

    def test_app_failures_are_aggregated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["staging"]\n')

            failures = deploy_config.validate_repository(root)

            self.assertTrue(
                any("apps/x/deploy.toml classes entry `staging`" in item for item in failures)
            )


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
