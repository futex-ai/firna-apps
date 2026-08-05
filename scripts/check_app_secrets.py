#!/usr/bin/env python3
"""Merge gate: every declared app secret must have a provisioned value.

For each app and each environment class the app targets, this script ensures
the app-secrets project contains the secret container
``<class-prefix>-<app_id>-<secret-name-kebab>`` (creating empty containers
when missing) and verifies an enabled version exists. It reads secret
metadata only, never values, and exits non-zero listing the exact
``gcloud secrets versions add`` command for every missing value.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import deploy_config


SECRET_ENTRY_RE = re.compile(r"^- name:\s*([A-Za-z0-9_]+)\s*$")
TOP_LEVEL_KEY_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*:")


class CommandRunner:
    """Runs gcloud without a shell and reports success plus output."""

    def run(self, command: list[str]) -> tuple[bool, str]:
        result = subprocess.run(command, check=False, capture_output=True, text=True)
        output = result.stdout if result.returncode == 0 else result.stderr
        return result.returncode == 0, output.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    args = parser.parse_args()
    root = (args.root or Path(__file__).resolve().parents[1]).resolve()
    return run_gate(root, CommandRunner())


def run_gate(root: Path, runner: CommandRunner) -> int:
    config, failures = deploy_config.load_root(root)
    apps, app_failures = discover_apps(root)
    failures.extend(app_failures)
    if failures or config is None:
        for failure in failures:
            print(f"app-secrets: {failure}", file=sys.stderr)
        return 1
    project_id = config.gcp.project_id
    missing = []
    for app_id, classes, secret_names in apps:
        containers = sorted(
            {
                f"{deploy_config.CLASS_SECRET_PREFIXES[environment_class]}-"
                f"{app_id}-{secret_name.replace('_', '-')}"
                for environment_class in classes
                for secret_name in secret_names
            }
        )
        for container in containers:
            failure = ensure_container(runner, project_id, app_id, container)
            if failure is not None:
                print(f"app-secrets: {failure}", file=sys.stderr)
                return 1
            if not has_enabled_version(runner, project_id, container):
                missing.append(container)
    if missing:
        print("app-secrets: missing secret values", file=sys.stderr)
        for container in missing:
            print(
                "app-secrets: provision with: printf '%s' \"$VALUE\" | "
                f"gcloud secrets versions add {container} "
                f"--project={project_id} --data-file=-",
                file=sys.stderr,
            )
        return 1
    print(f"app-secrets: all declared secrets are provisioned in {project_id}")
    return 0


def discover_apps(
    root: Path,
) -> tuple[list[tuple[str, tuple[str, ...], list[str]]], list[str]]:
    """Collect (app id, targeted classes, declared secret names) per app."""

    apps = []
    failures = []
    for manifest in sorted(root.glob("apps/*/manifest.*")):
        app_root = manifest.parent
        classes, class_failures = deploy_config.app_classes(app_root)
        failures.extend(class_failures)
        names, name_failures = manifest_secret_names(manifest, root)
        failures.extend(name_failures)
        apps.append((app_root.name, classes, names))
    return apps, failures


def manifest_secret_names(path: Path, root: Path) -> tuple[list[str], list[str]]:
    """Read declared secret names from a manifest without a YAML dependency."""

    relative = path.relative_to(root)
    try:
        contents = path.read_text(encoding="utf-8")
    except OSError as error:
        return [], [f"cannot read {relative}: {error}"]
    if path.suffix == ".json":
        return json_secret_names(contents, relative)
    return yaml_secret_names(contents, relative)


def json_secret_names(contents: str, relative: Path) -> tuple[list[str], list[str]]:
    try:
        document = json.loads(contents)
    except json.JSONDecodeError as error:
        return [], [f"{relative} is not valid JSON: {error}"]
    secrets = document.get("secrets", []) if isinstance(document, dict) else None
    if not isinstance(secrets, list):
        return [], [f"{relative} secrets must be an array"]
    names = []
    for entry in secrets:
        name = entry.get("name") if isinstance(entry, dict) else None
        if not isinstance(name, str) or not name:
            return [], [f"{relative} has a secrets entry without a name"]
        names.append(name)
    return names, []


def yaml_secret_names(contents: str, relative: Path) -> tuple[list[str], list[str]]:
    """Parse the flat ``secrets:`` block used by app manifests."""

    lines = contents.splitlines()
    try:
        start = next(
            index for index, line in enumerate(lines) if line.startswith("secrets:")
        )
    except StopIteration:
        return [], []
    remainder = lines[start][len("secrets:") :].strip()
    if remainder:
        if remainder == "[]":
            return [], []
        return [], [f"{relative} secrets must be a block sequence or []"]
    names = []
    for line in lines[start + 1 :]:
        if TOP_LEVEL_KEY_RE.match(line):
            break
        entry = SECRET_ENTRY_RE.match(line)
        if entry is not None:
            names.append(entry.group(1))
    return names, []


def ensure_container(
    runner: CommandRunner, project_id: str, app_id: str, container: str
) -> str | None:
    """Create the container when missing; metadata access only."""

    exists, _ = runner.run(
        [
            "gcloud",
            "secrets",
            "describe",
            container,
            f"--project={project_id}",
            "--format=value(name)",
        ]
    )
    if exists:
        return None
    created, output = runner.run(
        [
            "gcloud",
            "secrets",
            "create",
            container,
            f"--project={project_id}",
            "--replication-policy=automatic",
            f"--labels=app={app_id}",
        ]
    )
    if not created:
        return f"cannot create secret container {container}: {output}"
    print(f"app-secrets: created empty container {container}")
    return None


def has_enabled_version(
    runner: CommandRunner, project_id: str, container: str
) -> bool:
    listed, output = runner.run(
        [
            "gcloud",
            "secrets",
            "versions",
            "list",
            container,
            f"--project={project_id}",
            "--filter=state=enabled",
            "--format=value(name)",
            "--limit=1",
        ]
    )
    return listed and bool(output)


if __name__ == "__main__":
    raise SystemExit(main())
