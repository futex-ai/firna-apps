"""Tests for metadata-only app-preview request decisions."""

import unittest

from app_preview_contract import PreviewAction
from app_preview_controller import decide


HEAD_SHA = "1" * 40
NEW_SHA = "2" * 40


def pull_request(
    *,
    number: int = 123,
    sha: str = HEAD_SHA,
    repository: str = "futex-ai/firna-apps",
    state: str = "open",
    base: str = "main",
    labelled: bool = True,
) -> dict[str, object]:
    return {
        "number": number,
        "head": {"sha": sha, "repo": {"full_name": repository}},
        "base": {"ref": base},
        "state": state,
        "labels": [{"name": "preview"}] if labelled else [],
    }


def workflow_event(sha: str = HEAD_SHA, number: int = 123) -> dict[str, object]:
    return {
        "workflow_run": {
            "name": "CI",
            "event": "pull_request",
            "status": "completed",
            "conclusion": "success",
            "head_sha": sha,
            "head_repository": {"full_name": "futex-ai/firna-apps"},
            "pull_requests": [{"number": number}],
        }
    }


def target_event(
    action: str,
    pull: dict[str, object],
    *,
    label: str | None = None,
    sender: str = "maintainer",
) -> dict[str, object]:
    event: dict[str, object] = {
        "action": action,
        "pull_request": pull,
        "sender": {"login": sender},
    }
    if label is not None:
        event["label"] = {"name": label}
    return event


def retarget_event(pull: dict[str, object], from_base: str) -> dict[str, object]:
    event = target_event("edited", pull)
    event["changes"] = {
        "base": {
            "ref": {"from": from_base},
            "sha": {"from": "f" * 40},
        }
    }
    return event


class FakeMetadata:
    def __init__(self, pull: dict[str, object]):
        self.pull = pull
        self.label_actor: str | None = "maintainer"
        self.permission = "write"
        self.green_shas: set[str] = set()
        self.dispatched = []

    def pull_request(self, number: int) -> dict[str, object]:
        if number != self.pull["number"]:
            raise AssertionError(f"unexpected PR #{number}")
        return self.pull

    def active_preview_label_actor(self, number: int) -> str | None:
        if number != self.pull["number"]:
            raise AssertionError(f"unexpected PR #{number}")
        return self.label_actor

    def repository_permission(self, actor: str) -> str:
        if actor not in ("maintainer", "reader"):
            raise AssertionError(f"unexpected actor {actor}")
        return self.permission

    def ci_succeeded(self, head_sha: str) -> bool:
        return head_sha in self.green_shas

    def dispatch(self, request) -> None:
        self.dispatched.append(request)


class EventOrderTests(unittest.TestCase):
    def test_label_before_ci_dispatches_only_after_successful_workflow_run(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)

        label_decision = decide(
            "pull_request_target", target_event("labeled", pull, label="preview"), metadata
        )[0]
        metadata.green_shas.add(HEAD_SHA)
        ci_decision = decide("workflow_run", workflow_event(), metadata)[0]

        self.assertIsNone(label_decision.request)
        self.assertIn("not green", label_decision.reason)
        assert ci_decision.request is not None
        self.assertEqual(ci_decision.request.action, PreviewAction.DEPLOY)
        self.assertEqual(ci_decision.request.head_sha, HEAD_SHA)

    def test_ci_before_label_dispatches_when_label_is_applied(self) -> None:
        unlabelled = pull_request(labelled=False)
        metadata = FakeMetadata(unlabelled)
        metadata.green_shas.add(HEAD_SHA)

        ci_decision = decide("workflow_run", workflow_event(), metadata)[0]
        labelled = pull_request()
        metadata.pull = labelled
        label_decision = decide(
            "pull_request_target",
            target_event("labeled", labelled, label="preview"),
            metadata,
        )[0]

        self.assertIsNone(ci_decision.request)
        assert label_decision.request is not None
        self.assertEqual(label_decision.request.action, PreviewAction.DEPLOY)

    def test_same_owner_new_sha_can_dispatch_after_new_ci(self) -> None:
        metadata = FakeMetadata(pull_request(sha=NEW_SHA))
        metadata.green_shas.add(NEW_SHA)

        decision = decide("workflow_run", workflow_event(sha=NEW_SHA), metadata)[0]

        assert decision.request is not None
        self.assertEqual(decision.request.pr_number, 123)
        self.assertEqual(decision.request.head_sha, NEW_SHA)


class EligibilityTests(unittest.TestCase):
    def test_success_event_is_ignored_when_current_ci_state_is_not_green(self) -> None:
        metadata = FakeMetadata(pull_request())

        decision = decide("workflow_run", workflow_event(), metadata)[0]

        self.assertIsNone(decision.request)
        self.assertIn("not green", decision.reason)

    def test_stale_workflow_sha_is_ignored(self) -> None:
        metadata = FakeMetadata(pull_request(sha=NEW_SHA))

        decision = decide("workflow_run", workflow_event(sha=HEAD_SHA), metadata)[0]

        self.assertIsNone(decision.request)
        self.assertIn("no longer", decision.reason)

    def test_fork_closed_base_and_missing_label_are_rejected(self) -> None:
        cases = (
            pull_request(repository="someone/firna-apps"),
            pull_request(state="closed"),
            pull_request(base="develop"),
            pull_request(labelled=False),
        )
        for pull in cases:
            with self.subTest(pull=pull):
                metadata = FakeMetadata(pull)
                decision = decide("workflow_run", workflow_event(), metadata)[0]
                self.assertIsNone(decision.request)

    def test_label_actor_requires_write_permission(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        metadata.permission = "read"

        decision = decide("workflow_run", workflow_event(), metadata)[0]

        self.assertIsNone(decision.request)
        self.assertIn("lacks write permission", decision.reason)

    def test_label_event_uses_the_signed_event_actor(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        metadata.green_shas.add(HEAD_SHA)
        metadata.label_actor = "reader"
        metadata.permission = "read"

        decision = decide(
            "pull_request_target",
            target_event("labeled", pull, label="preview", sender="reader"),
            metadata,
        )[0]

        self.assertIsNone(decision.request)
        self.assertIn("reader", decision.reason)

    def test_relabel_race_cannot_reuse_an_earlier_authorized_actor(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        metadata.green_shas.add(HEAD_SHA)
        metadata.label_actor = "reader"

        decision = decide(
            "pull_request_target",
            target_event("labeled", pull, label="preview", sender="maintainer"),
            metadata,
        )[0]

        self.assertIsNone(decision.request)
        self.assertIn("superseded label event", decision.reason)

    def test_unrelated_label_change_does_not_make_preview_event_stale(self) -> None:
        event_pull = pull_request()
        current_pull = pull_request()
        current_pull["labels"] = [{"name": "preview"}, {"name": "documentation"}]
        metadata = FakeMetadata(current_pull)
        metadata.green_shas.add(HEAD_SHA)

        decision = decide(
            "pull_request_target",
            target_event("labeled", event_pull, label="preview"),
            metadata,
        )[0]

        self.assertIsNotNone(decision.request)

    def test_noncanonical_and_fork_workflow_runs_are_ignored(self) -> None:
        event = workflow_event()
        event["workflow_run"]["name"] = "Tests"
        self.assertIsNone(decide("workflow_run", event, FakeMetadata(pull_request()))[0].request)

        event = workflow_event()
        event["workflow_run"]["head_repository"] = {"full_name": "fork/apps"}
        self.assertIsNone(decide("workflow_run", event, FakeMetadata(pull_request()))[0].request)


if __name__ == "__main__":
    unittest.main()
