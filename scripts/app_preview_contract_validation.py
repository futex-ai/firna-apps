#!/usr/bin/env python3
"""Primitive validators for the closed app-preview event contracts."""

from __future__ import annotations

import re
from enum import Enum
from typing import Any
from urllib.parse import urlparse


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
RUN_PATH_RE = re.compile(
    r"^/futex-ai/firna/actions/runs/[1-9][0-9]*(?:/attempts/[1-9][0-9]*)?$"
)


class ContractError(ValueError):
    """A request or result does not satisfy the closed preview contract."""


def require_object(document: object, fields: set[str]) -> dict[str, Any]:
    """Require an object containing exactly the named fields."""

    if not isinstance(document, dict) or any(
        not isinstance(key, str) for key in document
    ):
        raise ContractError("payload must be a JSON object with string keys")
    unknown = set(document) - fields
    missing = fields - set(document)
    if unknown:
        raise ContractError(f"unknown field(s): {', '.join(sorted(unknown))}")
    if missing:
        raise ContractError(f"missing field(s): {', '.join(sorted(missing))}")
    return document


def require_version(values: dict[str, Any]) -> int:
    """Require the only supported schema version."""

    version = require_positive_int(values, "schema_version")
    if version != 1:
        raise ContractError("schema_version must be 1")
    return version


def require_repository(values: dict[str, Any], expected: str) -> str:
    """Require the fixed source repository."""

    repository = require_string(values, "source_repository")
    if repository != expected:
        raise ContractError(f"source_repository must be {expected}")
    return repository


def require_string(values: dict[str, Any], field: str) -> str:
    """Require a non-empty string field."""

    value = values[field]
    if not isinstance(value, str) or not value:
        raise ContractError(f"{field} must be a non-empty string")
    return value


def require_positive_int(values: dict[str, Any], field: str) -> int:
    """Require a positive integer while rejecting JSON booleans."""

    value = values[field]
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ContractError(f"{field} must be a positive integer")
    return value


def require_sha(values: dict[str, Any], field: str) -> str:
    """Require an immutable lowercase commit SHA."""

    value = require_string(values, field)
    if SHA_RE.fullmatch(value) is None:
        raise ContractError(f"{field} must be a lowercase 40-character commit SHA")
    return value


def require_optional_sha(values: dict[str, Any], field: str) -> str | None:
    """Require either null or an immutable commit SHA."""

    if values[field] is None:
        return None
    return require_sha(values, field)


def require_optional_string(values: dict[str, Any], field: str) -> str | None:
    """Require either null or a non-empty string."""

    if values[field] is None:
        return None
    return require_string(values, field)


def require_optional_positive_int(
    values: dict[str, Any], field: str
) -> int | None:
    """Require either null or a positive integer."""

    if values[field] is None:
        return None
    return require_positive_int(values, field)


def require_enum(values: dict[str, Any], field: str, enum_type: type[Enum]):
    """Require a member of a closed string enum."""

    value = require_string(values, field)
    try:
        return enum_type(value)
    except ValueError as error:
        raise ContractError(f"{field} has unknown value `{value}`") from error


def require_optional_enum(
    values: dict[str, Any], field: str, enum_type: type[Enum]
):
    """Require either null or a member of a closed string enum."""

    if values[field] is None:
        return None
    return require_enum(values, field, enum_type)


def require_run_url(values: dict[str, Any]) -> str:
    """Require an HTTPS link to a platform-repository Actions run."""

    value = require_string(values, "run_url")
    parsed = urlparse(value)
    if parsed.scheme != "https" or parsed.netloc != "github.com":
        raise ContractError("run_url must identify a futex-ai/firna GitHub Actions run")
    if RUN_PATH_RE.fullmatch(parsed.path) is None or parsed.query or parsed.fragment:
        raise ContractError("run_url must identify a futex-ai/firna GitHub Actions run")
    return value
