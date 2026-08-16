#!/usr/bin/env python3
"""Minimal GitHub REST client used by trusted app-preview workflows."""

from __future__ import annotations

import json
import urllib.error
import urllib.request


class GitHubApiError(RuntimeError):
    """A GitHub API request did not return its required status."""


class RestClient:
    """JSON-only GitHub client which never logs credentials or response bodies."""

    def __init__(self, api_url: str, token: str):
        self.api_url = api_url.rstrip("/")
        self.token = token

    def get(self, path: str) -> object:
        """Fetch and decode one JSON response."""

        return self.request("GET", path, None, {200})

    def post(self, path: str, payload: dict[str, object], statuses: set[int]) -> object | None:
        """Post JSON and optionally decode the response."""

        return self.request("POST", path, payload, statuses)

    def patch(self, path: str, payload: dict[str, object]) -> object:
        """Patch JSON and decode the response."""

        result = self.request("PATCH", path, payload, {200})
        if result is None:
            raise GitHubApiError(f"GitHub API PATCH {path} returned an empty response")
        return result

    def pages(self, path: str) -> list[object]:
        """Fetch every page from a list endpoint with a 100-row page size."""

        separator = "&" if "?" in path else "?"
        rows: list[object] = []
        page = 1
        while True:
            document = self.get(f"{path}{separator}per_page=100&page={page}")
            if not isinstance(document, list):
                raise GitHubApiError(f"GitHub API GET {path} did not return a list")
            rows.extend(document)
            if len(document) < 100:
                return rows
            page += 1

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, object] | None,
        statuses: set[int],
    ) -> object | None:
        """Issue one request with the repository's fixed API version."""

        if not path.startswith("/"):
            raise GitHubApiError("GitHub API paths must start with /")
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            f"{self.api_url}{path}",
            data=body,
            method=method,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
                "User-Agent": "firna-apps-preview-controller",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                response_body = response.read()
                status = response.status
        except urllib.error.HTTPError as error:
            raise GitHubApiError(
                f"GitHub API {method} {path} returned HTTP {error.code}"
            ) from error
        except urllib.error.URLError as error:
            raise GitHubApiError(f"GitHub API {method} {path} was unavailable") from error
        if status not in statuses:
            raise GitHubApiError(f"GitHub API {method} {path} returned HTTP {status}")
        if not response_body:
            return None
        try:
            return json.loads(response_body)
        except json.JSONDecodeError as error:
            raise GitHubApiError(
                f"GitHub API {method} {path} returned invalid JSON"
            ) from error
