#!/usr/bin/env python3
"""Audit package and optional tool icon assets under `apps/`."""

from __future__ import annotations

import base64
import json
import re
from pathlib import Path

import png_alpha


ICON_PNG = "assets/icon.png"
ICON_BASE64 = "assets/icon.png.base64"
TOOL_ASSET_DIRECTORY = "assets/tools"
EXPECTED_MEDIA_TYPE = "image/png"
MANIFEST_NAMES = ("manifest.yaml", "manifest.json")
MINIMUM_EDGE_PIXELS = 64
OPAQUE_ALPHA = 128
TILE_COVERAGE_MINIMUM = 0.85
CLEAR_SPACE_MINIMUM = 0.08
ICON_FIELD_RE = re.compile(r"^[ \t]+([a-z0-9_]+):[ \t]*(\S+)[ \t]*$", re.MULTILINE)


def audit_app_icons(root: Path) -> list[str]:
    """Audits every declared package and tool icon against the asset contract."""
    failures = []
    for manifest_path in sorted(root.glob("apps/*/manifest.*")):
        if manifest_path.name not in MANIFEST_NAMES:
            continue
        icon = declared_icon(manifest_path)
        if icon is not None:
            failures.extend(audit_app_icon(root, manifest_path, icon))
        failures.extend(audit_tool_icons(root, manifest_path))
    return failures


def audit_app_icon(root: Path, manifest_path: Path, icon: dict[str, str]) -> list[str]:
    """Audits one app's icon assets and the icon its manifest embeds."""
    app_root = manifest_path.parent
    label = manifest_path.relative_to(root)
    png_path = app_root / ICON_PNG
    base64_path = app_root / ICON_BASE64
    return audit_declared_icon(root, label, "icon", icon, png_path, base64_path)


def audit_tool_icons(root: Path, manifest_path: Path) -> list[str]:
    """Audits only explicitly declared tool icons; omission means package fallback."""
    failures = []
    seen_images: dict[str, str] = {}
    for tool_name, icon in declared_tool_icons(manifest_path):
        tool_asset = Path(TOOL_ASSET_DIRECTORY) / tool_name
        svg_path = manifest_path.parent / f"{tool_asset}.svg"
        png_path = manifest_path.parent / f"{tool_asset}.png"
        base64_path = manifest_path.parent / f"{tool_asset}.png.base64"
        label = manifest_path.relative_to(root)
        if not svg_path.is_file():
            failures.append(
                f"{svg_path.relative_to(root)} is required by the declared "
                f"tool `{tool_name}` icon"
            )
        failures.extend(
            audit_declared_icon(
                root,
                label,
                f"tool `{tool_name}` icon",
                icon,
                png_path,
                base64_path,
            )
        )
        encoded = icon.get("data_base64")
        if encoded is None:
            continue
        previous = seen_images.get(encoded)
        if previous is not None:
            failures.append(
                f"{label} tools `{previous}` and `{tool_name}` declare duplicate command artwork"
            )
        else:
            seen_images[encoded] = tool_name
    return failures


def audit_declared_icon(
    root: Path,
    manifest_label: Path,
    owner: str,
    icon: dict[str, str],
    png_path: Path,
    base64_path: Path,
) -> list[str]:
    """Audits one declared icon against its deterministic packaged assets."""
    failures = []
    relative_png = png_path.relative_to(root)
    relative_base64 = base64_path.relative_to(root)
    asset_png = relative_png.relative_to(manifest_label.parent)
    asset_base64 = relative_base64.relative_to(manifest_label.parent)
    if icon.get("media_type") != EXPECTED_MEDIA_TYPE:
        failures.append(
            f"{manifest_label} {owner} media_type `{icon.get('media_type')}` "
            f"must be `{EXPECTED_MEDIA_TYPE}`"
        )
    for path in (png_path, base64_path):
        if not path.is_file():
            failures.append(f"{path.relative_to(root)} is required by the declared {owner}")
    if not png_path.is_file():
        return failures
    png = png_path.read_bytes()
    encoded = base64.b64encode(png).decode("ascii")
    if base64_path.is_file() and base64_path.read_text(encoding="utf-8").strip() != encoded:
        failures.append(
            f"{relative_base64} does not encode {asset_png}; regenerate it from the PNG"
        )
    if icon.get("data_base64") != encoded:
        failures.append(
            f"{manifest_label} {owner} data_base64 does not embed {asset_png}; "
            f"re-embed the contents of {asset_base64}"
        )
    failures.extend(icon_shape_failures(relative_png, png))
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


def declared_tool_icons(path: Path) -> list[tuple[str, dict[str, str]]]:
    """Returns tool names and explicitly declared icon scalar fields."""
    contents = path.read_text(encoding="utf-8")
    if path.suffix == ".json":
        return json_tool_icons(contents)
    tools = yaml_block(contents, "tools")
    if tools is None:
        return []
    return yaml_tool_icons(tools)


def json_tool_icons(contents: str) -> list[tuple[str, dict[str, str]]]:
    """Extracts declared tool icons from a JSON manifest."""
    try:
        document = json.loads(contents)
    except json.JSONDecodeError:
        return []
    tools = document.get("tools") if isinstance(document, dict) else None
    if not isinstance(tools, list):
        return []
    icons = []
    for tool in tools:
        if not isinstance(tool, dict) or not isinstance(tool.get("name"), str):
            continue
        icon = tool.get("icon")
        if isinstance(icon, dict):
            icons.append(
                (tool["name"], {key: value for key, value in icon.items() if isinstance(value, str)})
            )
    return icons


def yaml_tool_icons(tools: str) -> list[tuple[str, dict[str, str]]]:
    """Extracts declared tool icons from the indentation-stable YAML tools list."""
    entries = re.split(r"(?m)^- name:[ \t]*", tools)
    icons = []
    for entry in entries[1:]:
        lines = entry.splitlines()
        if not lines:
            continue
        name = lines[0].strip()
        body = "\n".join(lines[1:])
        match = re.search(
            r"(?ms)^  icon:\s*\n(?P<block>(?:    [^\n]*(?:\n|$))*)",
            body,
        )
        if match is not None:
            icons.append((name, dict(ICON_FIELD_RE.findall(match.group("block")))))
    return icons


def yaml_block(contents: str, key: str) -> str | None:
    """Returns the indented lines nested under a top-level mapping key."""
    lines = contents.splitlines()
    for index, line in enumerate(lines):
        if line.strip() != f"{key}:" or line[:1].isspace():
            continue
        block = []
        for following in lines[index + 1 :]:
            if following.strip() and not following.startswith((" ", "\t", "-")):
                break
            block.append(following)
        return "\n".join(block)
    return None
