#!/usr/bin/env python3
"""Emit the GitHub Actions deployment matrix from the root ``deploy.toml``.

Prints a JSON object with one ``include`` entry per automatic environment
instance this repository deploys, carrying everything the deploy job needs:
API URL, secret prefix, admin login, bootstrap-password secret, and the two
Google Cloud project ids. Dedicated instances such as ``br-apps`` remain in
the validated configuration but can never enter this matrix.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import deploy_config


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument(
        "--instance",
        default="all",
        help="Restrict the matrix to one instance name, or `all`.",
    )
    args = parser.parse_args()
    root = (args.root or Path(__file__).resolve().parents[1]).resolve()
    matrix, failures = build_matrix(root, args.instance)
    if failures:
        for failure in failures:
            print(f"deploy-matrix: {failure}", file=sys.stderr)
        return 1
    print(json.dumps(matrix, sort_keys=True))
    return 0


def build_matrix(
    root: Path, instance_filter: str
) -> tuple[dict[str, list[dict[str, str]]] | None, list[str]]:
    """Build the matrix, restricted to ``instance_filter`` unless ``all``."""

    config, failures = deploy_config.load_root(root)
    if failures or config is None:
        return None, failures
    automatic_instances = [
        instance for instance in config.instances if instance.automatic
    ]
    names = [instance.name for instance in automatic_instances]
    if instance_filter != "all" and instance_filter not in names:
        return None, [f"unknown instance `{instance_filter}`; known: {names}"]
    include = [
        {
            "instance": instance.name,
            "environment_class": instance.environment_class,
            "api_url": instance.api_url,
            "secret_prefix": instance.secret_prefix,
            "admin_email": instance.admin_email,
            "bootstrap_password_secret": instance.bootstrap_password_secret,
            "apps_project": config.gcp.project_id,
            "platform_project": config.gcp.platform_project_id,
        }
        for instance in automatic_instances
        if instance_filter in ("all", instance.name)
    ]
    return {"include": include}, []


if __name__ == "__main__":
    raise SystemExit(main())
