"""Tests for the app manifest authoring contract audit."""

import importlib.util
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("repository_audit.py")
SPEC = importlib.util.spec_from_file_location("repository_audit", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
repository_audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(repository_audit)

REQUIRED_APP_FILES = (
    "README.md",
    "component/Cargo.toml",
    "component/Cargo.lock",
    "component/README.md",
    "tests/platform-runtime/Cargo.toml",
    "tests/platform-runtime/Cargo.lock",
    "tests/platform-runtime/README.md",
)


class ManifestAuthoringContractTests(unittest.TestCase):
    def test_top_level_events_are_rejected_even_when_empty(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(
                root,
                "exa",
                "id: exa\nversion: 1.0.0\ningress: []\nevents: []\n",
            )

            failures = repository_audit.audit_app_layout(root)

            self.assertEqual(
                failures,
                [
                    "apps/exa/manifest.yaml must not declare top-level events; "
                    "nest events under their owning ingress or omit the key"
                ],
            )

    def test_ingress_owned_events_are_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(
                root,
                "slack",
                "id: slack\n"
                "version: 1.0.0\n"
                "ingress:\n"
                "- id: slack_events\n"
                "  events: []\n",
            )

            self.assertEqual(repository_audit.audit_app_layout(root), [])

    def test_json_top_level_event_detection_is_structural(self) -> None:
        path = Path("manifest.json")

        self.assertTrue(
            repository_audit.manifest_declares_top_level_events(
                path, '{"id":"exa","events":[]}'
            )
        )
        self.assertFalse(
            repository_audit.manifest_declares_top_level_events(
                path, '{"id":"slack","ingress":[{"events":[]}]}'
            )
        )


def write_app(root: Path, app_id: str, manifest: str) -> None:
    app_root = root / "apps" / app_id
    write(app_root / "manifest.yaml", manifest)
    for relative in REQUIRED_APP_FILES:
        write(app_root / relative, "placeholder\n")


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
