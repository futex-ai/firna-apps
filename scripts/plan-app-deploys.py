#!/usr/bin/env python3
"""Plan Firna-owned app deployments from local manifests and remote catalog."""

import argparse
import json
import re
import sys
from pathlib import Path


SEMVER_RE = re.compile(
    r"^(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)
APP_DIRECTORY_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
REGISTRATION_PLACEHOLDERS = Path(__file__).with_name(
    "provider-registration-placeholders.txt"
)


class VersionParseError(ValueError):
    """Raised when a catalog version is not valid semantic version syntax."""


class PackageManifestError(ValueError):
    """Raised when a planned app does not resolve to one package manifest."""


def main() -> int:
    args = parse_args()
    catalog = load_catalog(args.catalog)
    changed_apps = load_changed_apps(args.changed_apps)
    registration_placeholders = load_registration_placeholders()
    apps_root = Path(args.apps_root)
    deploy_dirs = []
    failures = []
    with open(args.local_manifests, encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            manifest = json.loads(line)
            app_dir = manifest["directory"]
            app_id = manifest["id"]
            local_version = manifest["version"]
            remote_version = catalog.get(app_id)
            changed = "yes" if app_dir in changed_apps else "no"
            try:
                manifest_source = load_package_manifest(apps_root, app_dir)
            except PackageManifestError as error:
                decision = "fail_invalid_package"
                failures.append(f"app {app_id}: {error}")
            else:
                if contains_registration_placeholder(
                    manifest_source, registration_placeholders
                ):
                    decision = "skip_registration_pending"
                elif remote_version is None:
                    decision = "deploy_missing"
                    deploy_dirs.append(app_dir)
                else:
                    try:
                        version_order = compare_semver(
                            local_version, remote_version
                        )
                    except VersionParseError as error:
                        decision = "fail_invalid_version"
                        failures.append(f"app {app_id}: {error}")
                    else:
                        if version_order > 0:
                            decision = "deploy_local_newer"
                            deploy_dirs.append(app_dir)
                        elif version_order < 0:
                            decision = "fail_remote_newer"
                            failures.append(
                                f"app {app_id}: remote version "
                                f"`{remote_version}` is newer than local "
                                f"version `{local_version}`"
                            )
                        else:
                            decision = "skip"
            print(
                "app "
                f"{app_id} local={local_version} "
                f"remote={remote_version or 'missing'} "
                f"changed={changed} decision={decision}",
                file=sys.stderr,
            )
    if failures:
        for failure in failures:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    for app_dir in deploy_dirs:
        print(app_dir)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", required=True)
    parser.add_argument("--local-manifests", required=True)
    parser.add_argument("--changed-apps", required=True)
    parser.add_argument("--apps-root", required=True)
    return parser.parse_args()


def load_catalog(path: str) -> dict[str, str]:
    with open(path, encoding="utf-8") as handle:
        payload = json.load(handle)
    return {
        app["app_id"]: app["current_version"]
        for app in payload.get("apps", [])
        if app.get("app_id") and app.get("current_version")
    }


def load_changed_apps(path: str) -> set[str]:
    with open(path, encoding="utf-8") as handle:
        return {line.strip() for line in handle if line.strip()}


def load_registration_placeholders() -> frozenset[str]:
    with REGISTRATION_PLACEHOLDERS.open(encoding="utf-8") as handle:
        placeholders = frozenset(line.strip() for line in handle if line.strip())
    if not placeholders:
        raise ValueError("provider registration placeholder list is empty")
    return placeholders


def load_package_manifest(apps_root: Path, app_dir: str) -> str:
    if APP_DIRECTORY_RE.fullmatch(app_dir) is None:
        raise PackageManifestError(f"invalid app directory `{app_dir}`")
    package_root = apps_root / app_dir
    candidates = [
        path
        for path in [
            package_root / "manifest.yaml",
            package_root / "manifest.json",
        ]
        if path.is_file()
    ]
    if len(candidates) != 1:
        raise PackageManifestError(
            f"`{app_dir}` must contain exactly one package manifest"
        )
    try:
        return candidates[0].read_text(encoding="utf-8")
    except OSError as error:
        raise PackageManifestError(
            f"could not read `{candidates[0]}`"
        ) from error


def contains_registration_placeholder(
    manifest_source: str, placeholders: frozenset[str]
) -> bool:
    return any(placeholder in manifest_source for placeholder in placeholders)


def compare_semver(left: str, right: str) -> int:
    left_core, left_prerelease = parse_semver(left)
    right_core, right_prerelease = parse_semver(right)
    if left_core != right_core:
        return 1 if left_core > right_core else -1
    return compare_prerelease(left_prerelease, right_prerelease)


def parse_semver(value: str) -> tuple[tuple[int, int, int], tuple[str, ...]]:
    match = SEMVER_RE.fullmatch(value)
    if match is None:
        raise VersionParseError(f"version `{value}` is not valid semver")
    prerelease_text = match.group(4)
    prerelease = () if prerelease_text is None else tuple(prerelease_text.split("."))
    for identifier in prerelease:
        if identifier.isdigit() and len(identifier) > 1 and identifier.startswith("0"):
            raise VersionParseError(f"version `{value}` is not valid semver")
    return (
        (int(match.group(1)), int(match.group(2)), int(match.group(3))),
        prerelease,
    )


def compare_prerelease(left: tuple[str, ...], right: tuple[str, ...]) -> int:
    if left == right:
        return 0
    if not left:
        return 1
    if not right:
        return -1
    for left_identifier, right_identifier in zip(left, right):
        result = compare_identifier(left_identifier, right_identifier)
        if result != 0:
            return result
    return compare_values(len(left), len(right))


def compare_identifier(left: str, right: str) -> int:
    left_numeric = left.isdigit()
    right_numeric = right.isdigit()
    if left_numeric and right_numeric:
        return compare_values(int(left), int(right))
    if left_numeric:
        return -1
    if right_numeric:
        return 1
    return compare_values(left, right)


def compare_values(left, right) -> int:
    if left == right:
        return 0
    return 1 if left > right else -1


if __name__ == "__main__":
    raise SystemExit(main())
