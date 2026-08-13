"""Tests for catalog-aware app deployment planning."""

from __future__ import annotations

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

    def test_only_newer_slack_is_selected_once(self) -> None:
        local_manifests = [
            self.local_manifest("dataforseo", "1.0.8"),
            self.local_manifest("slack", "1.1.26"),
            self.local_manifest("x", "1.2.4"),
        ]
        catalog_apps = [
            self.catalog_app("1.0.8", "dataforseo"),
            self.catalog_app("1.1.25", "slack"),
            self.catalog_app("1.2.4", "x"),
        ]

        result = self.run_plan(catalog_apps, local_manifests=local_manifests)

        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.splitlines(), ["slack"])
        self.assertEqual(result.stderr.count("decision=deploy_local_newer"), 1)
        self.assertEqual(result.stderr.count("decision=skip"), 2)

    def run_plan(
        self,
        catalog_apps: list[dict[str, str]],
        local_version: str = "1.1.3",
        local_manifests: list[dict[str, str]] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = root / "catalog.json"
            manifests = root / "manifests.jsonl"
            changed = root / "changed"
            catalog.write_text(json.dumps({"apps": catalog_apps}), encoding="utf-8")
            manifest_rows = local_manifests or [
                self.local_manifest("slack", local_version)
            ]
            manifests.write_text(
                "".join(json.dumps(manifest) + "\n" for manifest in manifest_rows),
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
                ],
                check=False,
                capture_output=True,
                text=True,
            )

    @staticmethod
    def catalog_app(version: str, app_id: str = "slack") -> dict[str, str]:
        return {"app_id": app_id, "current_version": version}

    @staticmethod
    def local_manifest(app_id: str, version: str) -> dict[str, str]:
        return {"directory": app_id, "id": app_id, "version": version}


if __name__ == "__main__":
    unittest.main()
