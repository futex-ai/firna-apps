"""Tests for the app secret provisioning merge gate."""

import contextlib
import importlib.util
import io
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
check_app_secrets = load_module("check_app_secrets")


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

[environments.br-main]
class = "preview"
api_url = "https://br-main.api.preview.firna.ai"
secret_prefix = "preview-app"
admin_email = "preview-admin"
bootstrap_password_secret = "firna-preview-test-runtime-firna-bootstrap-password"
"""

EXA_MANIFEST = """\
id: exa
version: 1.0.0
secrets:
- name: api_key
  required: true
install:
  policy: workspace_default
"""

X_MANIFEST = """\
id: x
version: 1.0.0
events:
- name: not_a_secret
secrets:
- name: client_secret
  required: true
install:
  policy: explicit
"""


class FakeRunner:
    """Maps expected commands to canned results and records every call."""

    def __init__(self, results: dict[tuple[str, ...], tuple[bool, str]]):
        self.results = results
        self.calls: list[tuple[str, ...]] = []

    def run(self, command: list[str]) -> tuple[bool, str]:
        key = tuple(command)
        self.calls.append(key)
        if key not in self.results:
            raise AssertionError(f"unexpected command: {command}")
        return self.results[key]


def describe(container: str) -> tuple[str, ...]:
    return (
        "gcloud",
        "secrets",
        "describe",
        container,
        "--project=firna-apps",
        "--format=value(name)",
    )


def create(container: str, app_id: str) -> tuple[str, ...]:
    return (
        "gcloud",
        "secrets",
        "create",
        container,
        "--project=firna-apps",
        "--replication-policy=automatic",
        f"--labels=app={app_id}",
    )


def versions(container: str) -> tuple[str, ...]:
    return (
        "gcloud",
        "secrets",
        "versions",
        "list",
        container,
        "--project=firna-apps",
        "--filter=state=enabled",
        "--format=value(name)",
        "--limit=1",
    )


class RunGateTests(unittest.TestCase):
    def test_provisioned_repository_passes_without_creates(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)
            write(root / "apps/exa/manifest.yaml", EXA_MANIFEST)
            write(root / "apps/x/manifest.yaml", X_MANIFEST)
            write(root / "apps/x/deploy.toml", 'classes = ["production"]\n')
            write(
                root / "apps/dataforseo/manifest.yaml",
                "id: dataforseo\nversion: 1.0.0\nsecrets: []\n",
            )
            runner = FakeRunner(
                {
                    describe("prod-app-exa-api-key"): (True, "ok"),
                    versions("prod-app-exa-api-key"): (True, "1"),
                    describe("preview-app-exa-api-key"): (True, "ok"),
                    versions("preview-app-exa-api-key"): (True, "1"),
                    describe("prod-app-x-client-secret"): (True, "ok"),
                    versions("prod-app-x-client-secret"): (True, "1"),
                }
            )

            with contextlib.redirect_stdout(io.StringIO()):
                code = check_app_secrets.run_gate(root, runner)

            self.assertEqual(code, 0)
            created = [call for call in runner.calls if call[1:3] == ("secrets", "create")]
            self.assertEqual(created, [])
            preview_x = [call for call in runner.calls if "preview-app-x-client-secret" in call]
            self.assertEqual(preview_x, [])

    def test_missing_value_creates_container_and_fails_with_remediation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)
            write(root / "apps/exa/manifest.yaml", EXA_MANIFEST)
            write(root / "apps/exa/deploy.toml", 'classes = ["production"]\n')
            runner = FakeRunner(
                {
                    describe("prod-app-exa-api-key"): (False, "not found"),
                    create("prod-app-exa-api-key", "exa"): (True, "created"),
                    versions("prod-app-exa-api-key"): (True, ""),
                }
            )
            stderr = io.StringIO()

            with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(
                io.StringIO()
            ):
                code = check_app_secrets.run_gate(root, runner)

            self.assertEqual(code, 1)
            self.assertIn(
                "gcloud secrets versions add prod-app-exa-api-key "
                "--project=firna-apps --data-file=-",
                stderr.getvalue(),
            )
            self.assertIn(create("prod-app-exa-api-key", "exa"), runner.calls)

    def test_secretless_app_makes_no_calls(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", VALID_ROOT)
            write(
                root / "apps/http/manifest.yaml",
                "id: http\nversion: 1.0.0\nsecrets: []\n",
            )
            runner = FakeRunner({})

            with contextlib.redirect_stdout(io.StringIO()):
                code = check_app_secrets.run_gate(root, runner)

            self.assertEqual(code, 0)
            self.assertEqual(runner.calls, [])

    def test_invalid_deploy_config_fails_before_any_calls(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "deploy.toml", "[gcp]\n")
            runner = FakeRunner({})
            stderr = io.StringIO()

            with contextlib.redirect_stderr(stderr):
                code = check_app_secrets.run_gate(root, runner)

            self.assertEqual(code, 1)
            self.assertEqual(runner.calls, [])

    def test_json_manifest_secrets_are_parsed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names, failures = check_app_secrets.manifest_secret_names(
                write(
                    root / "apps/slack/manifest.json",
                    '{"id": "slack", "secrets": [{"name": "client_secret"}]}',
                ),
                root,
            )

            self.assertEqual(failures, [])
            self.assertEqual(names, ["client_secret"])

    def test_yaml_parser_ignores_names_outside_secrets_block(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            names, failures = check_app_secrets.manifest_secret_names(
                write(root / "apps/x/manifest.yaml", X_MANIFEST), root
            )

            self.assertEqual(failures, [])
            self.assertEqual(names, ["client_secret"])


def write(path: Path, contents: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    return path


if __name__ == "__main__":
    unittest.main()
