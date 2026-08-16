"""Tests for ownership-safe app-preview release decisions."""

import unittest

from app_preview_contract import PreviewAction
from app_preview_controller import decide
from test_app_preview_controller import (
    HEAD_SHA,
    FakeMetadata,
    pull_request,
    retarget_event,
    target_event,
)


class ReleaseTests(unittest.TestCase):
    def test_preview_unlabel_and_close_dispatch_release(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        for event in (
            target_event("unlabeled", pull, label="preview"),
            target_event("closed", pull),
        ):
            with self.subTest(action=event["action"]):
                decision = decide("pull_request_target", event, metadata)[0]
                assert decision.request is not None
                self.assertEqual(decision.request.action, PreviewAction.RELEASE)

    def test_unrelated_label_does_not_release(self) -> None:
        pull = pull_request()
        decision = decide(
            "pull_request_target",
            target_event("unlabeled", pull, label="documentation"),
            FakeMetadata(pull),
        )[0]

        self.assertIsNone(decision.request)

    def test_fork_event_cannot_dispatch_release_for_a_canonical_pr_number(self) -> None:
        pull = pull_request(repository="someone/firna-apps")
        decision = decide(
            "pull_request_target", target_event("closed", pull), FakeMetadata(pull)
        )[0]

        self.assertIsNone(decision.request)

    def test_same_repository_release_still_dispatches_after_retargeting(self) -> None:
        pull = pull_request(base="develop")

        decision = decide(
            "pull_request_target",
            target_event("unlabeled", pull, label="preview"),
            FakeMetadata(pull),
        )[0]

        assert decision.request is not None
        self.assertEqual(decision.request.action, PreviewAction.RELEASE)

    def test_retarget_away_from_main_dispatches_release(self) -> None:
        pull = pull_request(base="develop")

        decision = decide(
            "pull_request_target", retarget_event(pull, "main"), FakeMetadata(pull)
        )[0]

        assert decision.request is not None
        self.assertEqual(decision.request.action, PreviewAction.RELEASE)

    def test_retarget_to_main_can_dispatch_a_green_labelled_candidate(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        metadata.green_shas.add(HEAD_SHA)

        decision = decide(
            "pull_request_target", retarget_event(pull, "develop"), metadata
        )[0]

        assert decision.request is not None
        self.assertEqual(decision.request.action, PreviewAction.DEPLOY)

    def test_title_edit_does_not_redeploy_a_labelled_pull_request(self) -> None:
        pull = pull_request()
        metadata = FakeMetadata(pull)
        metadata.green_shas.add(HEAD_SHA)
        event = target_event("edited", pull)
        event["changes"] = {"title": {"from": "old title"}}

        decision = decide("pull_request_target", event, metadata)[0]

        self.assertIsNone(decision.request)


if __name__ == "__main__":
    unittest.main()
