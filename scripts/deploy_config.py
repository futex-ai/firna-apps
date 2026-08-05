#!/usr/bin/env python3
"""Parse and validate the repository deployment configuration.

The root ``deploy.toml`` declares the app-secrets Google Cloud project and
the long-lived environment instances this repository deploys. An optional
``apps/<app_id>/deploy.toml`` restricts an app to a subset of environment
classes. The schema and the fixed instance topology are defined by
``docs/protocol/app-deployment.md``; anything outside that contract is a
validation failure so deployment stays fail-closed.
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path


ROOT_FILE = "deploy.toml"
KNOWN_CLASSES = ("production", "preview")
CLASS_SECRET_PREFIXES = {"production": "prod-app", "preview": "preview-app"}
INSTANCE_CLASSES = {"production": "production", "br-main": "preview"}
INSTANCE_URLS = {
    "production": "https://api.firna.ai",
    "br-main": "https://br-main.api.preview.firna.ai",
}
INSTANCE_KEYS = frozenset(
    ("class", "api_url", "secret_prefix", "admin_email", "bootstrap_password_secret")
)
PROJECT_ID_RE = re.compile(r"^[a-z][a-z0-9-]{4,28}[a-z0-9]$")
ADMIN_EMAIL_RE = re.compile(r"^[a-z0-9][a-z0-9._@+-]*$")
SECRET_ID_RE = re.compile(r"^[a-z][a-z0-9-]*$")


@dataclass(frozen=True)
class GcpConfig:
    """Google Cloud projects referenced by deployment automation."""

    project_id: str
    platform_project_id: str


@dataclass(frozen=True)
class EnvironmentInstance:
    """One long-lived environment instance deployed by this repository."""

    name: str
    environment_class: str
    api_url: str
    secret_prefix: str
    admin_email: str
    bootstrap_password_secret: str


@dataclass(frozen=True)
class DeployConfig:
    """Validated contents of the root ``deploy.toml``."""

    gcp: GcpConfig
    instances: tuple[EnvironmentInstance, ...]


def load_root(root: Path) -> tuple[DeployConfig | None, list[str]]:
    """Load and validate the root ``deploy.toml`` under ``root``."""

    path = root / ROOT_FILE
    document, failures = parse_toml(path, root)
    if document is None:
        return None, failures
    failures.extend(
        unknown_key_failures(ROOT_FILE, document, frozenset(("gcp", "environments")))
    )
    gcp, gcp_failures = load_gcp_table(document.get("gcp"))
    failures.extend(gcp_failures)
    instances, instance_failures = load_instance_tables(document.get("environments"))
    failures.extend(instance_failures)
    if failures or gcp is None:
        return None, failures
    return DeployConfig(gcp=gcp, instances=tuple(instances)), []


def load_gcp_table(table: object) -> tuple[GcpConfig | None, list[str]]:
    """Validate the ``[gcp]`` table."""

    if not isinstance(table, dict):
        return None, [f"{ROOT_FILE} must declare a [gcp] table"]
    failures = unknown_key_failures(
        f"{ROOT_FILE} [gcp]", table, frozenset(("project_id", "platform_project_id"))
    )
    values = {}
    for key in ("project_id", "platform_project_id"):
        value = table.get(key)
        if not isinstance(value, str) or PROJECT_ID_RE.fullmatch(value) is None:
            failures.append(
                f"{ROOT_FILE} [gcp] {key} must be a valid Google Cloud project id"
            )
            continue
        values[key] = value
    if failures:
        return None, failures
    return GcpConfig(
        project_id=values["project_id"],
        platform_project_id=values["platform_project_id"],
    ), []


def load_instance_tables(
    table: object,
) -> tuple[list[EnvironmentInstance], list[str]]:
    """Validate the ``[environments.<instance>]`` tables."""

    if not isinstance(table, dict):
        return [], [f"{ROOT_FILE} must declare an [environments] table"]
    failures = unknown_key_failures(
        f"{ROOT_FILE} [environments]", table, frozenset(INSTANCE_CLASSES)
    )
    for name in sorted(INSTANCE_CLASSES):
        if name not in table:
            failures.append(f"{ROOT_FILE} must declare [environments.{name}]")
    instances = []
    for name in sorted(name for name in table if name in INSTANCE_CLASSES):
        instance, instance_failures = load_instance_table(name, table[name])
        failures.extend(instance_failures)
        if instance is not None:
            instances.append(instance)
    return instances, failures


def load_instance_table(
    name: str, table: object
) -> tuple[EnvironmentInstance | None, list[str]]:
    """Validate one ``[environments.<instance>]`` table."""

    context = f"{ROOT_FILE} [environments.{name}]"
    if not isinstance(table, dict):
        return None, [f"{context} must be a table"]
    failures = unknown_key_failures(context, table, INSTANCE_KEYS)
    values = {}
    for key in sorted(INSTANCE_KEYS):
        value = table.get(key)
        if not isinstance(value, str) or not value:
            failures.append(f"{context} {key} must be a non-empty string")
            continue
        values[key] = value
    if failures:
        return None, failures
    expected_class = INSTANCE_CLASSES[name]
    checks = (
        ("class", values["class"] == expected_class, f"must be `{expected_class}`"),
        (
            "api_url",
            values["api_url"] == INSTANCE_URLS[name],
            f"must be `{INSTANCE_URLS[name]}`",
        ),
        (
            "secret_prefix",
            values["secret_prefix"] == CLASS_SECRET_PREFIXES[expected_class],
            f"must be `{CLASS_SECRET_PREFIXES[expected_class]}`",
        ),
        (
            "admin_email",
            ADMIN_EMAIL_RE.fullmatch(values["admin_email"]) is not None,
            "must be a plain admin login",
        ),
        (
            "bootstrap_password_secret",
            SECRET_ID_RE.fullmatch(values["bootstrap_password_secret"]) is not None,
            "must be a Secret Manager secret id",
        ),
    )
    failures.extend(
        f"{context} {key} {message}" for key, valid, message in checks if not valid
    )
    if failures:
        return None, failures
    return EnvironmentInstance(
        name=name,
        environment_class=values["class"],
        api_url=values["api_url"],
        secret_prefix=values["secret_prefix"],
        admin_email=values["admin_email"],
        bootstrap_password_secret=values["bootstrap_password_secret"],
    ), []


def app_classes(app_root: Path) -> tuple[tuple[str, ...], list[str]]:
    """Return the environment classes targeted by the app at ``app_root``.

    A missing per-app ``deploy.toml`` targets every known class.
    """

    path = app_root / ROOT_FILE
    if not path.is_file():
        return KNOWN_CLASSES, []
    context = f"apps/{app_root.name}/{ROOT_FILE}"
    document, failures = parse_toml(path, app_root.parent.parent)
    if document is None:
        return (), failures
    failures.extend(unknown_key_failures(context, document, frozenset(("classes",))))
    classes = document.get("classes")
    if not isinstance(classes, list) or not classes:
        failures.append(f"{context} classes must be a non-empty array")
        return (), failures
    for entry in classes:
        if not isinstance(entry, str) or entry not in KNOWN_CLASSES:
            failures.append(f"{context} classes entry `{entry}` is not a known class")
    if len(set(classes)) != len(classes):
        failures.append(f"{context} classes must not repeat entries")
    if failures:
        return (), failures
    return tuple(name for name in KNOWN_CLASSES if name in classes), []


def validate_repository(root: Path) -> list[str]:
    """Validate the root and every per-app deployment configuration."""

    _, failures = load_root(root)
    for app_root in sorted(path.parent for path in root.glob("apps/*/manifest.*")):
        _, app_failures = app_classes(app_root)
        failures.extend(app_failures)
    return failures


def parse_toml(path: Path, root: Path) -> tuple[dict[str, object] | None, list[str]]:
    """Parse ``path`` as TOML, reporting failures relative to ``root``."""

    relative = path.relative_to(root) if path.is_relative_to(root) else path
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle), []
    except OSError as error:
        return None, [f"cannot read {relative}: {error}"]
    except tomllib.TOMLDecodeError as error:
        return None, [f"{relative} is not valid TOML: {error}"]


def unknown_key_failures(
    context: str, table: dict[str, object], known: frozenset[str]
) -> list[str]:
    """Reject keys outside the documented schema."""

    return [
        f"{context} has unknown key `{key}`" for key in sorted(set(table) - known)
    ]
