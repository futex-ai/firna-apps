"""Tests for GitHub-backed app-preview controller metadata."""

import unittest

from app_preview_controller_github import GitHubRepositoryMetadata


HEAD_SHA = "1" * 40


class FakeIssueEventsClient:
    def __init__(self, events: list[object]):
        self.events = events

    def pages(self, path: str) -> list[object]:
        self.path = path
        return self.events


class FakeWorkflowRunsClient:
    def __init__(self, runs: list[object]):
        self.runs = runs

    def get(self, path: str) -> object:
        self.path = path
        return {"workflow_runs": self.runs}


def label_event(
    event_id: int, event: str, actor: str | None = None
) -> dict[str, object]:
    document: dict[str, object] = {
        "id": event_id,
        "event": event,
        "label": {"name": "preview"},
    }
    if actor is not None:
        document["actor"] = {"login": actor}
    return document


def ci_run(
    run_id: int, conclusion: str | None, status: str = "completed"
) -> dict[str, object]:
    return {
        "id": run_id,
        "run_number": run_id,
        "run_attempt": 1,
        "name": "CI",
        "event": "pull_request",
        "status": status,
        "conclusion": conclusion,
        "head_sha": HEAD_SHA,
    }


class ActiveLabelActorTests(unittest.TestCase):
    def metadata(self, events: list[object]) -> GitHubRepositoryMetadata:
        metadata = GitHubRepositoryMetadata("https://api.github.com", "repo", "platform")
        metadata.repository = FakeIssueEventsClient(events)
        return metadata

    def test_event_order_from_the_api_does_not_change_the_active_actor(self) -> None:
        events = [
            label_event(3, "labeled", "current-writer"),
            label_event(2, "unlabeled"),
            label_event(1, "labeled", "old-writer"),
        ]

        actor = self.metadata(events).active_preview_label_actor(123)

        self.assertEqual(actor, "current-writer")

    def test_deleted_historical_actor_does_not_block_a_current_actor(self) -> None:
        events = [
            label_event(1, "labeled"),
            label_event(2, "unlabeled"),
            label_event(3, "labeled", "current-writer"),
        ]

        actor = self.metadata(events).active_preview_label_actor(123)

        self.assertEqual(actor, "current-writer")


class CurrentCiTests(unittest.TestCase):
    def test_newest_canonical_run_controls_green_state_in_any_api_order(self) -> None:
        for runs in (
            [ci_run(1, "success"), ci_run(2, "failure")],
            [ci_run(2, "failure"), ci_run(1, "success")],
        ):
            with self.subTest(runs=runs):
                metadata = GitHubRepositoryMetadata(
                    "https://api.github.com", "repo", "platform"
                )
                metadata.repository = FakeWorkflowRunsClient(runs)

                self.assertFalse(metadata.ci_succeeded(HEAD_SHA))

    def test_newer_in_progress_run_keeps_the_candidate_non_green(self) -> None:
        metadata = GitHubRepositoryMetadata(
            "https://api.github.com", "repo", "platform"
        )
        metadata.repository = FakeWorkflowRunsClient(
            [
                ci_run(1, "success"),
                ci_run(2, None, status="in_progress"),
            ]
        )

        self.assertFalse(metadata.ci_succeeded(HEAD_SHA))

if __name__ == "__main__":
    unittest.main()
