"""Tests for catalog-aware app deployment planning."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("plan-app-deploys.py")


class PlanAppDeploysTests(unittest.TestCase):
    def test_missing_app_is_deployed(self) -> None:
        result = self.run_plan([])

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "slack")
        self.assertIn("decision=deploy_missing", result.stderr)

    def test_pending_provider_registration_is_skipped(self) -> None:
        result = self.run_plan(
            [],
            manifest_contents=(
                "id: slack\n"
                "client_id: replace-with-registered-github-app-client-id\n"
            ),
        )

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("decision=skip_registration_pending", result.stderr)

    def test_matching_version_is_skipped(self) -> None:
        result = self.run_plan([self.catalog_app("1.1.3")])

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("decision=skip", result.stderr)

    def test_newer_local_version_is_deployed(self) -> None:
        result = self.run_plan([self.catalog_app("1.1.2")])

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "slack")
        self.assertIn("decision=deploy_local_newer", result.stderr)

    def test_newer_remote_version_fails_closed(self) -> None:
        result = self.run_plan([self.catalog_app("1.1.4")])

        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("decision=fail_remote_newer", result.stderr)

    def test_invalid_version_fails_closed(self) -> None:
        result = self.run_plan([self.catalog_app("not-semver")])

        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("decision=fail_invalid_version", result.stderr)

    def test_prerelease_precedence_matches_semver(self) -> None:
        result = self.run_plan(
            [self.catalog_app("1.1.3-beta.1")], local_version="1.1.3"
        )

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "slack")

    def run_plan(
        self,
        catalog_apps: list[dict[str, str]],
        local_version: str = "1.1.3",
        manifest_contents: str = "id: slack\n",
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = root / "catalog.json"
            manifests = root / "manifests.jsonl"
            changed = root / "changed"
            app_root = root / "apps" / "slack"
            app_root.mkdir(parents=True)
            (app_root / "manifest.yaml").write_text(
                manifest_contents,
                encoding="utf-8",
            )
            catalog.write_text(json.dumps({"apps": catalog_apps}), encoding="utf-8")
            manifests.write_text(
                json.dumps(
                    {
                        "directory": "slack",
                        "id": "slack",
                        "version": local_version,
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            changed.write_text("slack\n", encoding="utf-8")
            return subprocess.run(
                [
                    "python3",
                    str(SCRIPT),
                    "--catalog",
                    str(catalog),
                    "--local-manifests",
                    str(manifests),
                    "--changed-apps",
                    str(changed),
                    "--apps-root",
                    str(root / "apps"),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

    @staticmethod
    def catalog_app(version: str) -> dict[str, str]:
        return {"app_id": "slack", "current_version": version}


if __name__ == "__main__":
    unittest.main()
