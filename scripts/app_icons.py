#!/usr/bin/env python3
"""Audit the packaged app icon contract shared by every app under `apps/`."""

from __future__ import annotations

import base64
import json
import re
from pathlib import Path

import png_alpha


ICON_PNG = "assets/icon.png"
ICON_BASE64 = "assets/icon.png.base64"
EXPECTED_MEDIA_TYPE = "image/png"
MANIFEST_NAMES = ("manifest.yaml", "manifest.json")
MINIMUM_EDGE_PIXELS = 64
OPAQUE_ALPHA = 128
TILE_COVERAGE_MINIMUM = 0.85
CLEAR_SPACE_MINIMUM = 0.08
ICON_FIELD_RE = re.compile(r"^[ \t]+([a-z0-9_]+):[ \t]*(\S+)[ \t]*$", re.MULTILINE)


def audit_app_icons(root: Path) -> list[str]:
    """Audits every declared app icon against the packaged icon contract."""
    failures = []
    for manifest_path in sorted(root.glob("apps/*/manifest.*")):
        if manifest_path.name not in MANIFEST_NAMES:
            continue
        icon = declared_icon(manifest_path)
        if icon is not None:
            failures.extend(audit_app_icon(root, manifest_path, icon))
    return failures


def audit_app_icon(root: Path, manifest_path: Path, icon: dict[str, str]) -> list[str]:
    """Audits one app's icon assets and the icon its manifest embeds."""
    app_root = manifest_path.parent
    label = manifest_path.relative_to(root)
    png_path = app_root / ICON_PNG
    base64_path = app_root / ICON_BASE64
    failures = []
    if icon.get("media_type") != EXPECTED_MEDIA_TYPE:
        failures.append(
            f"{label} icon media_type `{icon.get('media_type')}` must be `{EXPECTED_MEDIA_TYPE}`"
        )
    for path in (png_path, base64_path):
        if not path.is_file():
            failures.append(f"{path.relative_to(root)} is required by the declared icon")
    if not png_path.is_file():
        return failures
    png = png_path.read_bytes()
    encoded = base64.b64encode(png).decode("ascii")
    if base64_path.is_file() and base64_path.read_text(encoding="utf-8").strip() != encoded:
        failures.append(
            f"{base64_path.relative_to(root)} does not encode {ICON_PNG}; "
            "regenerate it from the PNG"
        )
    if icon.get("data_base64") != encoded:
        failures.append(
            f"{label} icon data_base64 does not embed {ICON_PNG}; "
            f"re-embed the contents of {ICON_BASE64}"
        )
    failures.extend(icon_shape_failures(png_path.relative_to(root), png))
    return failures


def icon_shape_failures(label: Path, png: bytes) -> list[str]:
    """Reports icons that are not square, are too small, or have no framing."""
    try:
        raster = png_alpha.read_alpha_raster(png)
    except png_alpha.PngFormatError as error:
        return [f"{label} is not a readable PNG icon: {error}"]
    failures = []
    if raster.width != raster.height:
        failures.append(f"{label} must be square; it is {raster.width}x{raster.height}")
    if min(raster.width, raster.height) < MINIMUM_EDGE_PIXELS:
        failures.append(
            f"{label} must be at least {MINIMUM_EDGE_PIXELS}px per side; "
            f"it is {raster.width}x{raster.height}"
        )
    coverage = opaque_coverage(raster)
    clear_space = clear_space_ratio(raster)
    if coverage < TILE_COVERAGE_MINIMUM and clear_space < CLEAR_SPACE_MINIMUM:
        failures.append(
            f"{label} paints {coverage:.0%} of its canvas and leaves {clear_space:.0%} clear "
            f"space around its mark; an icon must either paint its own background across at "
            f"least {TILE_COVERAGE_MINIMUM:.0%} of the canvas or keep at least "
            f"{CLEAR_SPACE_MINIMUM:.0%} clear space on every side"
        )
    return failures


def opaque_coverage(raster: png_alpha.AlphaRaster) -> float:
    """Returns the fraction of the canvas the icon paints."""
    opaque = sum(1 for value in raster.alpha if value >= OPAQUE_ALPHA)
    return opaque / (raster.width * raster.height)


def clear_space_ratio(raster: png_alpha.AlphaRaster) -> float:
    """Returns the smallest transparent margin around the painted mark."""
    left, top = raster.width, raster.height
    right = bottom = -1
    for row in range(raster.height):
        offset = row * raster.width
        for column in range(raster.width):
            if raster.alpha[offset + column] < OPAQUE_ALPHA:
                continue
            left = min(left, column)
            right = max(right, column)
            top = min(top, row)
            bottom = max(bottom, row)
    if right < 0:
        return 0.0
    return min(
        left / raster.width,
        top / raster.height,
        (raster.width - 1 - right) / raster.width,
        (raster.height - 1 - bottom) / raster.height,
    )


def declared_icon(path: Path) -> dict[str, str] | None:
    """Returns the manifest's scalar icon fields, or None when no icon is declared."""
    contents = path.read_text(encoding="utf-8")
    if path.suffix == ".json":
        try:
            document = json.loads(contents)
        except json.JSONDecodeError:
            return None
        icon = document.get("icon") if isinstance(document, dict) else None
        if not isinstance(icon, dict):
            return None
        return {key: value for key, value in icon.items() if isinstance(value, str)}
    block = yaml_block(contents, "icon")
    if block is None:
        return None
    return dict(ICON_FIELD_RE.findall(block))


def yaml_block(contents: str, key: str) -> str | None:
    """Returns the indented lines nested under a top-level mapping key."""
    lines = contents.splitlines()
    for index, line in enumerate(lines):
        if line.rstrip() != f"{key}:":
            continue
        block = []
        for following in lines[index + 1 :]:
            if following.strip() and not following.startswith((" ", "\t")):
                break
            block.append(following)
        return "\n".join(block)
    return None
