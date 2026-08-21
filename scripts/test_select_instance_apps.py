"""Tests for per-instance candidate app selection."""

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
select_instance_apps = load_module("select_instance_apps")


class SelectAppsTests(unittest.TestCase):
    def test_github_targets_both_fixed_previews_but_not_ephemeral(self) -> None:
        root = Path(__file__).resolve().parents[1]

        with tempfile.TemporaryDirectory() as directory:
            candidate_list = write(Path(directory) / "candidates", "github\n")
            production, production_failures = select_instance_apps.select_apps(
                root, "production", candidate_list
            )
            preview, preview_failures = select_instance_apps.select_apps(
                root, "preview", candidate_list
            )
            preview_static, preview_static_failures = (
                select_instance_apps.select_apps(
                    root, "preview-static", candidate_list
                )
            )
            ephemeral, ephemeral_failures = select_instance_apps.select_apps(
                root, "ephemeral", candidate_list
            )

        self.assertEqual(production_failures, [])
        self.assertEqual(preview_failures, [])
        self.assertEqual(preview_static_failures, [])
        self.assertEqual(ephemeral_failures, [])
        self.assertEqual(production, ["github"])
        self.assertEqual(preview, ["github"])
        self.assertEqual(preview_static, ["github"])
        self.assertEqual(ephemeral, [])

    def test_production_only_app_is_excluded_from_preview(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "apps/exa/manifest.yaml", "id: exa\nversion: 1.0.0\n")
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["production"]\n')
            candidates = write(root / "candidates", "exa\nx\n")

            preview, failures = select_instance_apps.select_apps(
                root, "preview", candidates
            )
            production, production_failures = select_instance_apps.select_apps(
                root, "production", candidates
            )

            self.assertEqual(failures, [])
            self.assertEqual(production_failures, [])
            self.assertEqual(preview, ["exa"])
            self.assertEqual(production, ["exa", "x"])

    def test_ephemeral_excludes_stable_preview_only_app(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "apps/exa/manifest.yaml", "id: exa\nversion: 1.0.0\n")
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["production", "preview"]\n')
            candidates = write(root / "candidates", "exa\nx\n")

            ephemeral, failures = select_instance_apps.select_apps(
                root, "ephemeral", candidates
            )
            preview, preview_failures = select_instance_apps.select_apps(
                root, "preview", candidates
            )

            self.assertEqual(failures, [])
            self.assertEqual(preview_failures, [])
            self.assertEqual(ephemeral, ["exa"])
            self.assertEqual(preview, ["exa", "x"])

    def test_unknown_class_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            candidates = write(root / "candidates", "exa\n")

            selected, failures = select_instance_apps.select_apps(
                root, "staging", candidates
            )

            self.assertEqual(selected, [])
            self.assertTrue(any("unknown environment class" in item for item in failures))

    def test_invalid_per_app_config_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")
            write(root / "apps/x/deploy.toml", 'classes = ["staging"]\n')
            candidates = write(root / "candidates", "x\n")

            selected, failures = select_instance_apps.select_apps(
                root, "production", candidates
            )

            self.assertEqual(selected, [])
            self.assertTrue(failures)

    def test_missing_candidates_file_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            selected, failures = select_instance_apps.select_apps(
                root, "production", root / "absent"
            )

            self.assertEqual(selected, [])
            self.assertTrue(any("cannot read" in item for item in failures))


def write(path: Path, contents: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    return path


if __name__ == "__main__":
    unittest.main()
