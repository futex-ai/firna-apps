#!/usr/bin/env python3
"""Strict projections of GitHub metadata used by app-preview automation."""

from __future__ import annotations

from dataclasses import dataclass


PREVIEW_LABEL = "preview"
TARGET_BRANCH = "main"
CI_WORKFLOW = "CI"
ALLOWED_PERMISSIONS = frozenset(("write", "maintain", "admin"))


class EventError(ValueError):
    """The signed GitHub event is missing required metadata."""


@dataclass(frozen=True)
class PullIdentity:
    """Security-relevant pull request metadata reloaded from GitHub."""

    number: int
    head_repository: str
    head_sha: str
    base_ref: str
    state: str
    labels: frozenset[str]


def pull_identity(document: dict[str, object]) -> PullIdentity:
    """Project a GitHub pull request onto its trusted identity fields."""

    head = require_dict(document.get("head"), "pull request head")
    head_repository = require_dict(head.get("repo"), "pull request head repository")
    base = require_dict(document.get("base"), "pull request base")
    labels = document.get("labels")
    if not isinstance(labels, list):
        raise EventError("pull request labels must be an array")
    label_names = frozenset(
        require_string(require_dict(label, "pull request label"), "name", "label")
        for label in labels
    )
    return PullIdentity(
        number=require_positive_int(document, "number", "pull request"),
        head_repository=require_string(head_repository, "full_name", "head repository"),
        head_sha=require_string(head, "sha", "pull request head"),
        base_ref=require_string(base, "ref", "pull request base"),
        state=require_string(document, "state", "pull request"),
        labels=label_names,
    )


def require_dict(document: object, context: str) -> dict[str, object]:
    """Require an object from a signed event or GitHub response."""

    if not isinstance(document, dict):
        raise EventError(f"{context} must be an object")
    return document


def require_string(document: dict[str, object], field: str, context: str) -> str:
    """Require a non-empty string from GitHub metadata."""

    value = document.get(field)
    if not isinstance(value, str) or not value:
        raise EventError(f"{context} {field} must be a non-empty string")
    return value


def require_positive_int(document: object, field: str, context: str) -> int:
    """Require a positive integer while rejecting JSON booleans."""

    values = require_dict(document, context)
    value = values.get(field)
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise EventError(f"{context} {field} must be a positive integer")
    return value


def is_successful_ci_run(document: object, head_sha: str) -> bool:
    """Recognize canonical pull-request CI success for one exact SHA."""

    return is_canonical_ci_run(document, head_sha) and (
        document.get("conclusion") == "success"
    )


def is_canonical_ci_run(document: object, head_sha: str) -> bool:
    """Recognize a completed canonical pull-request CI run for one SHA."""

    return is_matching_ci_run(document, head_sha) and (
        document.get("status") == "completed"
    )


def is_matching_ci_run(document: object, head_sha: str) -> bool:
    """Recognize canonical pull-request CI metadata for one exact SHA."""

    return isinstance(document, dict) and (
        document.get("name") == CI_WORKFLOW
        and document.get("event") == "pull_request"
        and document.get("head_sha") == head_sha
    )
