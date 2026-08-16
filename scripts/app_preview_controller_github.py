#!/usr/bin/env python3
"""GitHub-backed metadata adapter for the trusted app-preview controller."""

from __future__ import annotations

from urllib.parse import quote

from app_preview_contract import (
    PLATFORM_REPOSITORY,
    SOURCE_REPOSITORY,
    PreviewRequest,
)
from app_preview_github import RestClient
from app_preview_metadata import (
    EventError,
    PREVIEW_LABEL,
    is_matching_ci_run,
    is_successful_ci_run,
    require_dict,
    require_positive_int,
    require_string,
)


class GitHubRepositoryMetadata:
    """GitHub-backed implementation used only from trusted workflow code."""

    def __init__(
        self,
        api_url: str,
        repository_token: str,
        platform_dispatch_token: str,
    ):
        self.repository = RestClient(api_url, repository_token)
        self.platform = RestClient(api_url, platform_dispatch_token)

    def pull_request(self, number: int) -> dict[str, object]:
        """Reload one pull request from GitHub."""

        document = self.repository.get(f"/repos/{SOURCE_REPOSITORY}/pulls/{number}")
        return require_dict(document, "pull request")

    def active_preview_label_actor(self, number: int) -> str | None:
        """Return the actor behind the currently active preview label."""

        events = self.repository.pages(
            f"/repos/{SOURCE_REPOSITORY}/issues/{number}/events"
        )
        return preview_label_actor(events)

    def repository_permission(self, actor: str) -> str:
        """Return an actor's current permission in the source repository."""

        escaped_actor = quote(actor, safe="")
        document = self.repository.get(
            f"/repos/{SOURCE_REPOSITORY}/collaborators/{escaped_actor}/permission"
        )
        return require_string(
            require_dict(document, "permission"), "permission", "permission"
        )

    def ci_succeeded(self, head_sha: str) -> bool:
        """Report whether canonical pull-request CI passed for one SHA."""

        escaped_sha = quote(head_sha, safe="")
        document = self.repository.get(
            f"/repos/{SOURCE_REPOSITORY}/actions/workflows/ci.yml/runs"
            f"?event=pull_request&head_sha={escaped_sha}&per_page=100"
        )
        runs = require_dict(document, "workflow runs").get("workflow_runs")
        if not isinstance(runs, list):
            raise EventError("workflow runs response is missing workflow_runs")
        return latest_ci_succeeded(runs, head_sha)

    def dispatch(self, request: PreviewRequest) -> None:
        """Send one validated request to the platform repository."""

        self.platform.post(
            f"/repos/{PLATFORM_REPOSITORY}/dispatches",
            {
                "event_type": "firna-app-preview-request",
                "client_payload": request.payload(),
            },
            {204},
        )


def preview_label_actor(events: list[object]) -> str | None:
    """Resolve active label ownership independent of API response ordering."""

    relevant = []
    for document in events:
        event = require_dict(document, "issue event")
        label = event.get("label")
        event_name = event.get("event")
        if (
            not isinstance(label, dict)
            or label.get("name") != PREVIEW_LABEL
            or event_name not in ("labeled", "unlabeled")
        ):
            continue
        event_id = require_positive_int(event, "id", "issue event")
        relevant.append((event_id, event_name, event.get("actor")))
    actor = None
    for _, event_name, actor_document in sorted(relevant, key=lambda item: item[0]):
        if event_name == "unlabeled" or not isinstance(actor_document, dict):
            actor = None
            continue
        login = actor_document.get("login")
        actor = login if isinstance(login, str) and login else None
    return actor


def latest_ci_succeeded(runs: list[object], head_sha: str) -> bool:
    """Use only the latest canonical run and attempt for one candidate."""

    candidates = []
    for document in runs:
        if not is_matching_ci_run(document, head_sha):
            continue
        run = require_dict(document, "workflow run")
        run_number = require_positive_int(run, "run_number", "workflow run")
        run_attempt = require_positive_int(run, "run_attempt", "workflow run")
        candidates.append((run_number, run_attempt, run))
    if not candidates:
        return False
    latest = max(candidates, key=lambda item: (item[0], item[1]))[2]
    return is_successful_ci_run(latest, head_sha)
