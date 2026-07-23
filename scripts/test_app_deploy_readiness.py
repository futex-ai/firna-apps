"""Tests for app deployment readiness validation."""

import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-app-deploy-readiness.sh")


class AppDeployReadinessTests(unittest.TestCase):
    def test_rejects_missing_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self._run(Path(directory))

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no manifest found", result.stderr)

    def test_rejects_registration_placeholder(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            app_root = Path(directory)
            self._write_manifest(
                app_root,
                "id: github\n"
                "client_id: replace-with-registered-github-app-client-id\n",
            )
            result = self._run(app_root)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("provider registration placeholder", result.stderr)

    def test_rejects_missing_github_private_key(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            app_root = Path(directory)
            self._write_manifest(
                app_root,
                "id: github\n"
                "client_id: registered-client-id\n"
                "secrets:\n"
                "- name: client_secret\n",
            )
            result = self._run(app_root)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must require the private_key", result.stderr)

    def test_accepts_complete_github_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            app_root = Path(directory)
            self._write_manifest(
                app_root,
                "id: github\n"
                "client_id: registered-client-id\n"
                "secrets:\n"
                "- name: client_secret\n"
                "- name: private_key\n",
            )
            result = self._run(app_root)

        self.assertEqual(result.returncode, 0, result.stderr)

    @staticmethod
    def _write_manifest(app_root: Path, contents: str) -> None:
        (app_root / "manifest.yaml").write_text(contents, encoding="utf-8")

    @staticmethod
    def _run(app_root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(SCRIPT), str(app_root)],
            check=False,
            capture_output=True,
            text=True,
        )


if __name__ == "__main__":
    unittest.main()
