"""Tests for deployment matrix generation."""

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


def load_module(name: str):
    path = Path(__file__).with_name(f"{name}.py")
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


load_module("deploy_config")
deploy_matrix = load_module("deploy_matrix")


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


class BuildMatrixTests(unittest.TestCase):
    def test_all_automatic_instances_are_included(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)

            matrix, failures = deploy_matrix.build_matrix(root, "all")

            self.assertEqual(failures, [])
            assert matrix is not None
            entries = {entry["instance"]: entry for entry in matrix["include"]}
            self.assertEqual(sorted(entries), ["br-main", "production"])
            self.assertNotIn("br-apps", entries)
            production = entries["production"]
            self.assertEqual(production["environment_class"], "production")
            self.assertEqual(production["api_url"], "https://api.firna.ai")
            self.assertEqual(production["secret_prefix"], "prod-app")
            self.assertEqual(production["admin_email"], "admin")
            self.assertEqual(
                production["bootstrap_password_secret"],
                "firna-prod-runtime-firna-bootstrap-password",
            )
            self.assertEqual(production["apps_project"], "firna-apps")
            self.assertEqual(production["platform_project"], "firna-498513")

    def test_single_instance_filter(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)

            matrix, failures = deploy_matrix.build_matrix(root, "br-main")

            self.assertEqual(failures, [])
            assert matrix is not None
            self.assertEqual(
                [entry["instance"] for entry in matrix["include"]], ["br-main"]
            )

    def test_unknown_instance_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)

            matrix, failures = deploy_matrix.build_matrix(root, "staging")

            self.assertIsNone(matrix)
            self.assertTrue(any("unknown instance `staging`" in item for item in failures))

    def test_dedicated_review_instance_cannot_be_selected_manually(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)

            matrix, failures = deploy_matrix.build_matrix(root, "br-apps")

            self.assertIsNone(matrix)
            self.assertTrue(any("unknown instance `br-apps`" in item for item in failures))

    def test_invalid_config_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", "[gcp]\n")

            matrix, failures = deploy_matrix.build_matrix(root, "all")

            self.assertIsNone(matrix)
            self.assertTrue(failures)


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
