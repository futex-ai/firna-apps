#!/usr/bin/env python3
"""Dispatch trusted static app-preview requests from GitHub metadata events."""

from __future__ import annotations

import json
import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from app_preview_contract import (
    SOURCE_REPOSITORY,
    PreviewAction,
    PreviewRequest,
)
from app_preview_controller_github import GitHubRepositoryMetadata
from app_preview_metadata import (
    ALLOWED_PERMISSIONS,
    EventError,
    PREVIEW_LABEL,
    PullIdentity,
    TARGET_BRANCH,
    is_successful_ci_run,
    pull_identity,
    require_dict,
    require_positive_int,
    require_string,
)


class RepositoryMetadata(Protocol):
    """Read-only metadata and cross-repository dispatch boundary."""

    def pull_request(self, number: int) -> dict[str, object]:
        """Reload one pull request from GitHub."""

    def active_preview_label_actor(self, number: int) -> str | None:
        """Return the actor behind the currently active preview label."""

    def repository_permission(self, actor: str) -> str:
        """Return the actor's current repository permission."""

    def ci_succeeded(self, head_sha: str) -> bool:
        """Report whether canonical CI succeeded for the exact SHA."""

    def dispatch(self, request: PreviewRequest) -> None:
        """Dispatch one validated request to the platform repository."""


@dataclass(frozen=True)
class Decision:
    """One controller decision and its non-secret diagnostic reason."""

    request: PreviewRequest | None
    reason: str


def main() -> int:
    """Read the signed event, decide, and dispatch without running PR code."""

    try:
        event_name = required_environment("GITHUB_EVENT_NAME")
        event_path = Path(required_environment("GITHUB_EVENT_PATH"))
        event = require_dict(json.loads(event_path.read_text(encoding="utf-8")), "event")
        metadata = GitHubRepositoryMetadata(
            required_environment("GITHUB_API_URL"),
            required_environment("GITHUB_TOKEN"),
            required_environment("FIRNA_PLATFORM_PREVIEW_DISPATCH_TOKEN"),
        )
        decisions = decide(event_name, event, metadata)
        for decision in decisions:
            print(f"app-preview-controller: {decision.reason}")
            if decision.request is not None:
                metadata.dispatch(decision.request)
        return 0
    except (EventError, OSError, json.JSONDecodeError, RuntimeError, ValueError) as error:
        print(f"app-preview-controller: {error}", file=sys.stderr)
        return 1


def decide(
    event_name: str, event: dict[str, object], metadata: RepositoryMetadata
) -> list[Decision]:
    """Return fail-closed decisions for one supported GitHub event."""

    if event_name == "workflow_run":
        return decide_workflow_run(event, metadata)
    if event_name == "pull_request_target":
        return [decide_pull_request_target(event, metadata)]
    raise EventError(f"unsupported event `{event_name}`")


def decide_workflow_run(
    event: dict[str, object], metadata: RepositoryMetadata
) -> list[Decision]:
    """Deploy labelled PRs only after canonical CI succeeds for their SHA."""

    run = require_dict(event.get("workflow_run"), "workflow_run")
    head_sha = require_string(run, "head_sha", "workflow_run")
    if not is_successful_ci_run(run, head_sha):
        return [Decision(None, "ignored non-successful canonical CI run")]
    head_repository = require_dict(run.get("head_repository"), "head_repository")
    if head_repository.get("full_name") != SOURCE_REPOSITORY:
        return [Decision(None, "ignored CI run from a fork")]
    pulls = run.get("pull_requests")
    if not isinstance(pulls, list) or not pulls:
        return [Decision(None, "ignored CI run without an associated pull request")]
    numbers = sorted({require_positive_int(pull, "number", "pull request") for pull in pulls})
    return [
        decide_deploy(metadata.pull_request(number), head_sha, None, metadata)
        for number in numbers
    ]


def decide_pull_request_target(
    event: dict[str, object], metadata: RepositoryMetadata
) -> Decision:
    """Handle label ordering, reopening, and ownership-safe release hints."""

    action = require_string(event, "action", "event")
    event_pr = require_dict(event.get("pull_request"), "pull_request")
    identity = pull_identity(event_pr)
    label = event.get("label")
    label_name = label.get("name") if isinstance(label, dict) else None
    if action == "closed" or (action == "unlabeled" and label_name == PREVIEW_LABEL):
        if identity.head_repository != SOURCE_REPOSITORY:
            return Decision(None, "ignored release event for an ineligible pull request")
        return release_decision(identity)
    if action == "labeled" and label_name == PREVIEW_LABEL:
        sender = require_dict(event.get("sender"), "sender")
        actor = require_string(sender, "login", "sender")
    elif action == "reopened" and PREVIEW_LABEL in identity.labels:
        actor = None
    elif action == "edited":
        if not has_base_change(event):
            return Decision(None, "ignored pull request edit without a base change")
        actor = None
    else:
        return Decision(None, f"ignored pull_request_target action `{action}`")
    current = metadata.pull_request(identity.number)
    current_identity = pull_identity(current)
    if (
        current_identity.number != identity.number
        or current_identity.head_repository != identity.head_repository
        or current_identity.head_sha != identity.head_sha
        or current_identity.base_ref != identity.base_ref
        or current_identity.state != identity.state
    ):
        return Decision(None, "ignored stale pull request event")
    if action == "edited" and current_identity.base_ref != TARGET_BRANCH:
        if current_identity.head_repository != SOURCE_REPOSITORY:
            return Decision(None, "ignored retarget event from a fork")
        return release_decision(current_identity)
    if PREVIEW_LABEL not in current_identity.labels:
        return Decision(None, "pull request does not currently have the preview label")
    return decide_deploy(current, identity.head_sha, actor, metadata)


def release_decision(identity: PullIdentity) -> Decision:
    """Build an ownership-safe release hint for one canonical pull request."""

    request = PreviewRequest.create(
        PreviewAction.RELEASE, identity.number, identity.head_sha
    )
    return Decision(
        request,
        f"dispatching release for PR #{identity.number} at {identity.head_sha}",
    )


def has_base_change(event: dict[str, object]) -> bool:
    """Report whether an edited event changed the pull request base ref."""

    changes = require_dict(event.get("changes"), "pull request changes")
    if "base" not in changes:
        return False
    base = require_dict(changes.get("base"), "pull request base change")
    ref = require_dict(base.get("ref"), "pull request base ref change")
    require_string(ref, "from", "pull request base ref change")
    return True


def decide_deploy(
    pull: dict[str, object],
    requested_sha: str,
    label_actor: str | None,
    metadata: RepositoryMetadata,
) -> Decision:
    """Apply every deploy eligibility condition before creating a request."""

    identity = pull_identity(pull)
    reason = ineligible_reason(identity, requested_sha)
    if reason is not None:
        return Decision(None, reason)
    active_actor = metadata.active_preview_label_actor(identity.number)
    if active_actor is None:
        return Decision(None, "preview label has no active application actor")
    if label_actor is not None and active_actor != label_actor:
        return Decision(None, "ignored superseded label event")
    if metadata.repository_permission(active_actor) not in ALLOWED_PERMISSIONS:
        return Decision(
            None, f"preview label actor `{active_actor}` lacks write permission"
        )
    if not metadata.ci_succeeded(identity.head_sha):
        return Decision(None, "canonical CI is not green for the current head SHA")
    request = PreviewRequest.create(PreviewAction.DEPLOY, identity.number, identity.head_sha)
    return Decision(
        request,
        f"dispatching deploy for PR #{identity.number} at {identity.head_sha}",
    )


def ineligible_reason(identity: PullIdentity, requested_sha: str) -> str | None:
    if identity.head_repository != SOURCE_REPOSITORY:
        return "fork pull requests cannot claim the app preview"
    if identity.state != "open" or identity.base_ref != TARGET_BRANCH:
        return "pull request must be open and target main"
    if PREVIEW_LABEL not in identity.labels:
        return "pull request does not currently have the preview label"
    if identity.head_sha != requested_sha:
        return "requested SHA is no longer the current pull request head"
    return None


def required_environment(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise EventError(f"{name} must be set")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
