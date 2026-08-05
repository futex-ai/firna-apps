#!/usr/bin/env python3
"""Filter candidate app ids to those targeting an environment class.

Reads the newline-separated candidate app list produced by the deploy
workflow's selection step and keeps only apps whose deployment targeting
(per-app ``deploy.toml``, defaulting to every class) includes the requested
environment class. Unknown classes and invalid per-app configuration fail
closed.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import deploy_config


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--environment-class", required=True)
    parser.add_argument("--candidates", required=True, type=Path)
    args = parser.parse_args()
    root = (args.root or Path(__file__).resolve().parents[1]).resolve()
    selected, failures = select_apps(root, args.environment_class, args.candidates)
    if failures:
        for failure in failures:
            print(f"select-instance-apps: {failure}", file=sys.stderr)
        return 1
    for app_id in selected:
        print(app_id)
    return 0


def select_apps(
    root: Path, environment_class: str, candidates_path: Path
) -> tuple[list[str], list[str]]:
    """Return candidate app ids targeting ``environment_class``."""

    if environment_class not in deploy_config.KNOWN_CLASSES:
        return [], [f"unknown environment class `{environment_class}`"]
    try:
        lines = candidates_path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        return [], [f"cannot read {candidates_path}: {error}"]
    selected = []
    failures = []
    for app_id in [line.strip() for line in lines if line.strip()]:
        classes, class_failures = deploy_config.app_classes(root / "apps" / app_id)
        failures.extend(class_failures)
        if not class_failures and environment_class in classes:
            selected.append(app_id)
    return (selected, []) if not failures else ([], failures)


if __name__ == "__main__":
    raise SystemExit(main())
