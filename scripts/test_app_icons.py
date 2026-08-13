"""Tests for the packaged app icon contract audit."""

from __future__ import annotations

import base64
import json
import tempfile
import unittest
from pathlib import Path

import app_icons
from icon_test_support import bare_mark, rgba_png, rounded_tile


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


class AppIconContractTests(unittest.TestCase):
    def test_repository_icons_satisfy_the_contract(self) -> None:
        self.assertEqual(app_icons.audit_app_icons(REPOSITORY_ROOT), [])

    def test_edge_to_edge_mark_without_a_background_is_rejected(self) -> None:
        png = rgba_png(128, 128, bare_mark(128, margin=2))

        failures = app_icons.icon_shape_failures(Path("apps/x/assets/icon.png"), png)

        self.assertEqual(len(failures), 1, failures)
        self.assertIn("clear space", failures[0])
        self.assertIn("apps/x/assets/icon.png", failures[0])

    def test_mark_on_its_own_background_is_accepted(self) -> None:
        png = rgba_png(128, 128, rounded_tile(128, radius=28))

        self.assertEqual(app_icons.icon_shape_failures(Path("icon.png"), png), [])

    def test_bare_mark_with_clear_space_is_accepted(self) -> None:
        png = rgba_png(128, 128, bare_mark(128, margin=16))

        self.assertEqual(app_icons.icon_shape_failures(Path("icon.png"), png), [])

    def test_fully_transparent_icon_is_rejected(self) -> None:
        png = rgba_png(128, 128, [0] * (128 * 128))

        failures = app_icons.icon_shape_failures(Path("icon.png"), png)

        self.assertEqual(len(failures), 1, failures)
        self.assertIn("clear space", failures[0])

    def test_small_and_non_square_icons_are_rejected(self) -> None:
        png = rgba_png(48, 32, [255] * (48 * 32))

        failures = app_icons.icon_shape_failures(Path("icon.png"), png)

        self.assertEqual(len(failures), 2, failures)
        self.assertIn("must be square", failures[0])
        self.assertIn("at least 64px", failures[1])

    def test_unreadable_icon_bytes_are_reported(self) -> None:
        failures = app_icons.icon_shape_failures(Path("icon.png"), b"not a png")

        self.assertEqual(len(failures), 1, failures)
        self.assertIn("not a readable PNG icon", failures[0])

    def test_stale_base64_sidecar_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(root, "x", tile_png())
            write(root / "apps/x/assets/icon.png.base64", "c3RhbGU=\n")

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("does not encode assets/icon.png", failures[0])

    def test_stale_embedded_manifest_icon_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(root, "x", tile_png(), embedded="c3RhbGU=")

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("data_base64 does not embed assets/icon.png", failures[0])

    def test_missing_icon_assets_are_reported(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(root, "x", tile_png())
            (root / "apps/x/assets/icon.png").unlink()
            (root / "apps/x/assets/icon.png.base64").unlink()

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(
                failures,
                [
                    "apps/x/assets/icon.png is required by the declared icon",
                    "apps/x/assets/icon.png.base64 is required by the declared icon",
                ],
            )

    def test_non_png_media_type_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(root, "x", tile_png(), media_type="image/svg+xml")

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("media_type `image/svg+xml`", failures[0])

    def test_manifest_without_an_icon_is_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "apps/x/manifest.yaml", "id: x\nversion: 1.0.0\n")

            self.assertEqual(app_icons.audit_app_icons(root), [])

    def test_json_manifest_icons_are_audited(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            png = tile_png()
            write(root / "apps/x/manifest.json", json_manifest("c3RhbGU="))
            write_bytes(root / "apps/x/assets/icon.png", png)
            write(
                root / "apps/x/assets/icon.png.base64",
                base64.b64encode(png).decode("ascii") + "\n",
            )

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("data_base64 does not embed assets/icon.png", failures[0])

    def test_tool_icon_matches_named_assets_and_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            png = tile_png()
            write_app(root, "slack", png)
            add_yaml_tool_icon(root, "slack", "slack_send_message", png)

            self.assertEqual(app_icons.audit_app_icons(root), [])

    def test_tool_without_icon_needs_no_assets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_app(root, "slack", tile_png())
            append_tool(root, "slack", "slack_list_channels")

            self.assertEqual(app_icons.audit_app_icons(root), [])

    def test_tool_icon_requires_tool_name_asset_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            png = tile_png()
            write_app(root, "slack", png)
            add_yaml_tool_icon(root, "slack", "slack_send_message", png, write_assets=False)

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 3, failures)
            self.assertIn("assets/tools/slack_send_message.svg is required", failures[0])
            self.assertIn("assets/tools/slack_send_message.png is required", failures[1])
            self.assertIn("assets/tools/slack_send_message.png.base64 is required", failures[2])

    def test_tool_icon_requires_editable_svg_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            png = tile_png()
            write_app(root, "slack", png)
            add_yaml_tool_icon(root, "slack", "slack_send_message", png)
            (root / "apps/slack/assets/tools/slack_send_message.svg").unlink()

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("slack_send_message.svg is required", failures[0])

    def test_duplicate_declared_tool_artwork_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            png = tile_png()
            write_app(root, "slack", png)
            add_yaml_tool_icon(root, "slack", "slack_send_message", png)
            add_yaml_tool_icon(root, "slack", "slack_search_messages", png)

            failures = app_icons.audit_app_icons(root)

            self.assertEqual(len(failures), 1, failures)
            self.assertIn("duplicate command artwork", failures[0])

def write_app(
    root: Path,
    app_id: str,
    png: bytes,
    embedded: str | None = None,
    media_type: str = "image/png",
) -> None:
    encoded = base64.b64encode(png).decode("ascii")
    write(
        root / "apps" / app_id / "manifest.yaml",
        yaml_manifest(app_id, embedded or encoded, media_type),
    )
    write_bytes(root / "apps" / app_id / "assets/icon.png", png)
    write(root / "apps" / app_id / "assets/icon.png.base64", encoded + "\n")


def append_tool(root: Path, app_id: str, tool_name: str, icon: bytes | None = None) -> None:
    manifest_path = root / "apps" / app_id / "manifest.yaml"
    contents = manifest_path.read_text(encoding="utf-8")
    declaration = f"- name: {tool_name}\n"
    if icon is not None:
        encoded = base64.b64encode(icon).decode("ascii")
        declaration += (
            "  icon:\n"
            "    media_type: image/png\n"
            f"    data_base64: {encoded}\n"
        )
    manifest_path.write_text(
        contents.replace("tools: []\n", f"tools:\n{declaration}"),
        encoding="utf-8",
    )


def add_yaml_tool_icon(
    root: Path,
    app_id: str,
    tool_name: str,
    png: bytes,
    *,
    write_assets: bool = True,
) -> None:
    manifest_path = root / "apps" / app_id / "manifest.yaml"
    if "tools: []" in manifest_path.read_text(encoding="utf-8"):
        append_tool(root, app_id, tool_name, png)
    else:
        contents = manifest_path.read_text(encoding="utf-8")
        encoded = base64.b64encode(png).decode("ascii")
        contents += (
            f"- name: {tool_name}\n"
            "  icon:\n"
            "    media_type: image/png\n"
            f"    data_base64: {encoded}\n"
        )
        manifest_path.write_text(contents, encoding="utf-8")
    if write_assets:
        write_icon_assets(root / "apps" / app_id / "assets/tools", tool_name, png)


def write_icon_assets(directory: Path, stem: str, png: bytes) -> None:
    write(directory / f"{stem}.svg", '<svg xmlns="http://www.w3.org/2000/svg"/>\n')
    write_bytes(directory / f"{stem}.png", png)
    write(directory / f"{stem}.png.base64", base64.b64encode(png).decode("ascii") + "\n")


def yaml_manifest(app_id: str, encoded: str, media_type: str) -> str:
    return (
        f"id: {app_id}\n"
        "version: 1.0.0\n"
        "icon:\n"
        f"  media_type: {media_type}\n"
        f"  data_base64: {encoded}\n"
        "  color_pair:\n"
        '    primary: "#000000"\n'
        '    secondary: "#FFFFFF"\n'
        "tools: []\n"
    )


def json_manifest(encoded: str) -> str:
    return json.dumps(
        {
            "id": "x",
            "version": "1.0.0",
            "icon": {"media_type": "image/png", "data_base64": encoded},
        }
    )


def tile_png() -> bytes:
    return rgba_png(128, 128, rounded_tile(128, radius=28))


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


def write_bytes(path: Path, contents: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)


if __name__ == "__main__":
    unittest.main()
