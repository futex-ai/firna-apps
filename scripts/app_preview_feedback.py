#!/usr/bin/env python3
"""Publish one correlated PR comment and check for app-preview results."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Protocol
from urllib.parse import quote

from app_preview_contract import (
    SOURCE_REPOSITORY,
    PreviewResult,
    PreviewStatus,
    parse_result,
)
from app_preview_github import RestClient
from app_preview_metadata import PREVIEW_LABEL, TARGET_BRANCH, pull_identity


CHECK_NAME = "app preview"
CHECK_APP_SLUG = "github-actions"
COMMENT_START = "<!-- firna-app-preview:start -->"
COMMENT_END = "<!-- firna-app-preview:end -->"
BOT_LOGIN = "github-actions[bot]"


class FeedbackApi(Protocol):
    """GitHub mutation boundary for result feedback."""

    def pull_request(self, number: int) -> dict[str, object]:
        """Reload one pull request from GitHub."""

    def upsert_comment(self, number: int, body: str) -> None:
        """Create or replace the marker-delimited bot comment."""

    def upsert_check(self, result: PreviewResult, title: str, summary: str) -> None:
        """Create or update the named check for the result SHA."""


class GitHubFeedbackApi:
    """GitHub-backed feedback implementation."""

    def __init__(self, api_url: str, token: str):
        self.client = RestClient(api_url, token)

    def pull_request(self, number: int) -> dict[str, object]:
        document = self.client.get(f"/repos/{SOURCE_REPOSITORY}/pulls/{number}")
        if not isinstance(document, dict):
            raise ValueError("pull request response must be an object")
        return document

    def upsert_comment(self, number: int, body: str) -> None:
        comments = self.client.pages(f"/repos/{SOURCE_REPOSITORY}/issues/{number}/comments")
        existing_id = None
        for document in comments:
            if not isinstance(document, dict):
                raise ValueError("issue comment response must contain objects")
            author = document.get("user")
            author_login = author.get("login") if isinstance(author, dict) else None
            existing_body = document.get("body")
            if (
                author_login == BOT_LOGIN
                and isinstance(existing_body, str)
                and COMMENT_START in existing_body
                and COMMENT_END in existing_body
            ):
                existing_id = document.get("id")
                break
        payload = {"body": body}
        if isinstance(existing_id, int) and not isinstance(existing_id, bool):
            self.client.patch(
                f"/repos/{SOURCE_REPOSITORY}/issues/comments/{existing_id}", payload
            )
        else:
            self.client.post(
                f"/repos/{SOURCE_REPOSITORY}/issues/{number}/comments", payload, {201}
            )

    def upsert_check(self, result: PreviewResult, title: str, summary: str) -> None:
        escaped_name = quote(CHECK_NAME, safe="")
        document = self.client.get(
            f"/repos/{SOURCE_REPOSITORY}/commits/{result.head_sha}/check-runs"
            f"?check_name={escaped_name}&filter=latest&per_page=100"
        )
        if not isinstance(document, dict) or not isinstance(
            document.get("check_runs"), list
        ):
            raise ValueError("check-runs response is missing check_runs")
        payload: dict[str, object] = {
            "name": CHECK_NAME,
            "status": "completed",
            "conclusion": check_conclusion(result.status),
            "details_url": result.run_url,
            "external_id": result.correlation_id,
            "output": {"title": title, "summary": summary},
        }
        existing_id = matching_check_id(document["check_runs"], result.correlation_id)
        if isinstance(existing_id, int) and not isinstance(existing_id, bool):
            self.client.patch(
                f"/repos/{SOURCE_REPOSITORY}/check-runs/{existing_id}", payload
            )
        else:
            payload["head_sha"] = result.head_sha
            self.client.post(f"/repos/{SOURCE_REPOSITORY}/check-runs", payload, {201})


def matching_check_id(check_runs: list[object], correlation: str) -> int | None:
    """Find the check created by this workflow for the exact candidate."""

    for document in check_runs:
        if not isinstance(document, dict):
            continue
        app = document.get("app")
        if (
            isinstance(app, dict)
            and app.get("slug") == CHECK_APP_SLUG
            and document.get("external_id") == correlation
        ):
            check_id = document.get("id")
            if isinstance(check_id, int) and not isinstance(check_id, bool):
                return check_id
    return None


def main() -> int:
    """Validate one dispatch result and publish it only when still current."""

    try:
        event_path = Path(required_environment("GITHUB_EVENT_PATH"))
        event = json.loads(event_path.read_text(encoding="utf-8"))
        if not isinstance(event, dict):
            raise ValueError("repository dispatch event must be an object")
        result = parse_result(event.get("client_payload"))
        api = GitHubFeedbackApi(
            required_environment("GITHUB_API_URL"), required_environment("GITHUB_TOKEN")
        )
        outcome = publish(result, api)
        print(f"app-preview-feedback: {outcome}")
        return 0
    except (OSError, json.JSONDecodeError, RuntimeError, ValueError) as error:
        print(f"app-preview-feedback: {error}", file=sys.stderr)
        return 1


def publish(result: PreviewResult, api: FeedbackApi) -> str:
    """Ignore stale results; otherwise update the single comment and check."""

    identity = pull_identity(api.pull_request(result.pr_number))
    if identity.number != result.pr_number or identity.head_repository != SOURCE_REPOSITORY:
        return "ignored result whose pull request identity no longer matches"
    if result.status is PreviewStatus.RELEASED:
        if (
            identity.state == "open"
            and identity.base_ref == TARGET_BRANCH
            and PREVIEW_LABEL in identity.labels
        ):
            return "ignored stale release for a currently eligible pull request"
    else:
        if identity.head_sha != result.head_sha:
            return "ignored result for a stale head SHA"
        if (
            identity.state != "open"
            or identity.base_ref != TARGET_BRANCH
            or PREVIEW_LABEL not in identity.labels
        ):
            return "ignored result for an ineligible pull request"
    title, summary, comment = render_feedback(result)
    api.upsert_check(result, title, summary)
    api.upsert_comment(result.pr_number, comment)
    return f"published {result.status.value} for PR #{result.pr_number}"


def render_feedback(result: PreviewResult) -> tuple[str, str, str]:
    """Render developer-facing feedback from validated, closed values only."""

    title = status_title(result.status)
    lines = [
        COMMENT_START,
        "### App preview",
        "",
        title,
        "",
        f"- App revision: `{result.head_sha}`",
        f"- Platform revision: `{result.platform_sha or 'unavailable'}`",
        f"- Deployment run: [view run]({result.run_url})",
    ]
    if result.status is PreviewStatus.READY:
        lines.extend(
            [
                f"- Product: [open preview]({result.product_url})",
                f"- API: [open API]({result.api_url})",
            ]
        )
    elif result.status is PreviewStatus.BUSY:
        lines.append(f"- Current owner: #{result.owner_pr_number}")
    elif result.status is PreviewStatus.FAILED:
        failure = result.failure_code.value if result.failure_code is not None else "unknown"
        lines.append(f"- Failure code: `{failure}`")
    lines.extend(["", COMMENT_END])
    comment = "\n".join(lines)
    summary = "\n".join(line for line in lines[4:-2] if line)
    return title, summary, comment


def status_title(status: PreviewStatus) -> str:
    """Return concise product-neutral GitHub feedback for one status."""

    return {
        PreviewStatus.READY: "The app preview is ready.",
        PreviewStatus.FAILED: "The app preview failed.",
        PreviewStatus.BUSY: "The app preview slot is in use by another pull request.",
        PreviewStatus.RELEASED: "The app preview slot was released.",
        PreviewStatus.SUPERSEDED: "The app preview request was superseded.",
    }[status]


def check_conclusion(status: PreviewStatus) -> str:
    """Map terminal preview status to an advisory GitHub check conclusion."""

    if status is PreviewStatus.READY:
        return "success"
    if status is PreviewStatus.FAILED:
        return "failure"
    return "neutral"


def required_environment(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise ValueError(f"{name} must be set")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
