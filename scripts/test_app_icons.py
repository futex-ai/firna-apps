"""Tests for the packaged app icon contract audit."""

import base64
import tempfile
import unittest
import zlib
from pathlib import Path

import app_icons


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
    return (
        '{"id":"x","version":"1.0.0",'
        f'"icon":{{"media_type":"image/png","data_base64":"{encoded}"}}}}'
    )


def tile_png() -> bytes:
    return rgba_png(128, 128, rounded_tile(128, radius=28))


def rounded_tile(size: int, radius: int) -> list[int]:
    alpha = []
    for row in range(size):
        for column in range(size):
            inset_row = min(row, size - 1 - row)
            inset_column = min(column, size - 1 - column)
            if inset_row >= radius or inset_column >= radius:
                alpha.append(255)
                continue
            distance = (radius - inset_row) ** 2 + (radius - inset_column) ** 2
            alpha.append(255 if distance <= radius**2 else 0)
    return alpha


def bare_mark(size: int, margin: int, thickness: int = 6) -> list[int]:
    span = size - 2 * margin
    alpha = []
    for row in range(size):
        for column in range(size):
            inset_row = row - margin
            inset_column = column - margin
            inside = 0 <= inset_row < span and 0 <= inset_column < span
            on_stroke = (
                abs(inset_column - inset_row) <= thickness
                or abs(inset_column + inset_row - (span - 1)) <= thickness
            )
            alpha.append(255 if inside and on_stroke else 0)
    return alpha


def rgba_png(width: int, height: int, alpha: list[int]) -> bytes:
    raster = bytearray()
    for row in range(height):
        raster.append(0)
        for column in range(width):
            raster.extend((0, 0, 0, alpha[row * width + column]))
    header = width.to_bytes(4, "big") + height.to_bytes(4, "big") + bytes([8, 6, 0, 0, 0])
    return (
        png_alpha_signature()
        + png_chunk(b"IHDR", header)
        + png_chunk(b"IDAT", zlib.compress(bytes(raster)))
        + png_chunk(b"IEND", b"")
    )


def png_alpha_signature() -> bytes:
    return b"\x89PNG\r\n\x1a\n"


def png_chunk(kind: bytes, body: bytes) -> bytes:
    return (
        len(body).to_bytes(4, "big")
        + kind
        + body
        + zlib.crc32(kind + body).to_bytes(4, "big")
    )


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


def write_bytes(path: Path, contents: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)


if __name__ == "__main__":
    unittest.main()
