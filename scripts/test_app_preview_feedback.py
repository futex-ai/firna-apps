"""Tests for correlated app-preview pull-request feedback."""

import unittest

from app_preview_contract import API_URL, PRODUCT_URL, correlation_id, parse_result
from app_preview_feedback import (
    CHECK_NAME,
    COMMENT_END,
    COMMENT_START,
    GitHubFeedbackApi,
    check_conclusion,
    publish,
    render_feedback,
)
from app_preview_metadata import pull_identity


HEAD_SHA = "1" * 40
PLATFORM_SHA = "2" * 40
RUN_URL = "https://github.com/futex-ai/firna/actions/runs/123"


def pull_request(
    *,
    sha: str = HEAD_SHA,
    state: str = "open",
    labelled: bool = True,
    base: str = "main",
) -> dict[str, object]:
    return {
        "number": 123,
        "head": {
            "sha": sha,
            "repo": {"full_name": "futex-ai/firna-apps"},
        },
        "base": {"ref": base},
        "state": state,
        "labels": [{"name": "preview"}] if labelled else [],
    }


def result(status: str = "ready"):
    payload: dict[str, object] = {
        "schema_version": 1,
        "status": status,
        "source_repository": "futex-ai/firna-apps",
        "pr_number": 123,
        "head_sha": HEAD_SHA,
        "correlation_id": correlation_id(123, HEAD_SHA),
        "platform_sha": PLATFORM_SHA,
        "product_url": None,
        "api_url": None,
        "run_url": RUN_URL,
        "failure_code": None,
        "owner_pr_number": None,
    }
    if status == "ready":
        payload.update(
            {
                "product_url": PRODUCT_URL,
                "api_url": API_URL,
                "owner_pr_number": 123,
            }
        )
    elif status == "busy":
        payload["owner_pr_number"] = 456
    elif status == "failed":
        payload["failure_code"] = "smoke_failed"
    return parse_result(payload)


class FakeFeedbackApi:
    def __init__(self, pull: dict[str, object]):
        self.pull = pull
        self.comments: list[tuple[int, str]] = []
        self.checks = []

    def pull_request(self, number: int) -> dict[str, object]:
        if number != 123:
            raise AssertionError(f"unexpected PR #{number}")
        return self.pull

    def upsert_comment(self, number: int, body: str) -> None:
        self.comments.append((number, body))

    def upsert_check(self, preview_result, title: str, summary: str) -> None:
        self.checks.append((preview_result, title, summary))


class FakeRestClient:
    def __init__(self):
        self.comment_rows: list[object] = []
        self.check_rows: list[object] = []
        self.posts: list[tuple[str, dict[str, object], set[int]]] = []
        self.patches: list[tuple[str, dict[str, object]]] = []

    def pages(self, path: str) -> list[object]:
        self.last_pages_path = path
        return self.comment_rows

    def get(self, path: str) -> object:
        self.last_get_path = path
        return {"check_runs": self.check_rows}

    def post(
        self, path: str, payload: dict[str, object], statuses: set[int]
    ) -> object:
        self.posts.append((path, payload, statuses))
        return {}

    def patch(self, path: str, payload: dict[str, object]) -> object:
        self.patches.append((path, payload))
        return {}


class PublishTests(unittest.TestCase):
    def test_ready_updates_one_check_and_marker_delimited_comment(self) -> None:
        api = FakeFeedbackApi(pull_request())

        outcome = publish(result(), api)

        self.assertIn("published ready", outcome)
        self.assertEqual(len(api.checks), 1)
        self.assertEqual(len(api.comments), 1)
        comment = api.comments[0][1]
        self.assertTrue(comment.startswith(COMMENT_START))
        self.assertTrue(comment.endswith(COMMENT_END))
        self.assertIn(HEAD_SHA, comment)
        self.assertIn(PLATFORM_SHA, comment)
        self.assertIn(PRODUCT_URL, comment)
        self.assertIn(API_URL, comment)

    def test_stale_sha_is_ignored_without_mutation(self) -> None:
        api = FakeFeedbackApi(pull_request(sha="3" * 40))

        outcome = publish(result(), api)

        self.assertIn("stale", outcome)
        self.assertEqual(api.checks, [])
        self.assertEqual(api.comments, [])

    def test_nonrelease_result_requires_open_labelled_main_pr(self) -> None:
        for pull in (pull_request(state="closed"), pull_request(labelled=False)):
            with self.subTest(identity=pull_identity(pull)):
                api = FakeFeedbackApi(pull)
                outcome = publish(result("failed"), api)
                self.assertIn("ineligible", outcome)
                self.assertEqual(api.checks, [])

    def test_matching_release_can_update_a_closed_or_unlabelled_pr(self) -> None:
        for pull in (
            pull_request(state="closed"),
            pull_request(labelled=False),
        ):
            with self.subTest(identity=pull_identity(pull)):
                api = FakeFeedbackApi(pull)

                outcome = publish(result("released"), api)

                self.assertIn("published released", outcome)
                self.assertEqual(len(api.checks), 1)
                self.assertEqual(len(api.comments), 1)

    def test_release_is_ignored_after_the_same_sha_is_relabelled(self) -> None:
        api = FakeFeedbackApi(pull_request())

        outcome = publish(result("released"), api)

        self.assertIn("stale release", outcome)
        self.assertEqual(api.checks, [])
        self.assertEqual(api.comments, [])

    def test_matching_release_can_update_after_an_unlabelled_pr_advances(self) -> None:
        api = FakeFeedbackApi(pull_request(sha="3" * 40, labelled=False))

        outcome = publish(result("released"), api)

        self.assertIn("published released", outcome)
        self.assertEqual(len(api.checks), 1)
        self.assertEqual(len(api.comments), 1)

    def test_matching_release_can_update_a_retargeted_pr(self) -> None:
        api = FakeFeedbackApi(pull_request(base="develop"))

        outcome = publish(result("released"), api)

        self.assertIn("published released", outcome)
        self.assertEqual(len(api.checks), 1)
        self.assertEqual(len(api.comments), 1)


class RenderingTests(unittest.TestCase):
    def test_busy_feedback_names_owner_without_environment_urls(self) -> None:
        title, summary, comment = render_feedback(result("busy"))

        self.assertIn("another pull request", title)
        self.assertIn("#456", comment)
        self.assertNotIn(PRODUCT_URL, comment)
        self.assertIn(PLATFORM_SHA, summary)

    def test_failure_feedback_uses_only_closed_failure_code(self) -> None:
        _, _, comment = render_feedback(result("failed"))

        self.assertIn("`smoke_failed`", comment)

    def test_check_conclusions_are_advisory_except_real_failure(self) -> None:
        self.assertEqual(check_conclusion(result().status), "success")
        self.assertEqual(check_conclusion(result("failed").status), "failure")
        self.assertEqual(check_conclusion(result("busy").status), "neutral")
        self.assertEqual(CHECK_NAME, "app preview")


class GitHubMutationTests(unittest.TestCase):
    def api(self) -> tuple[GitHubFeedbackApi, FakeRestClient]:
        api = GitHubFeedbackApi("https://api.github.com", "token")
        client = FakeRestClient()
        api.client = client
        return api, client

    def test_existing_marker_comment_is_updated_in_place(self) -> None:
        api, client = self.api()
        client.comment_rows = [
            {"id": 1, "body": "unrelated", "user": {"login": "someone"}},
            {
                "id": 42,
                "body": f"{COMMENT_START}\nold\n{COMMENT_END}",
                "user": {"login": "github-actions[bot]"},
            },
        ]

        api.upsert_comment(123, "new")

        self.assertEqual(client.posts, [])
        self.assertEqual(
            client.patches,
            [("/repos/futex-ai/firna-apps/issues/comments/42", {"body": "new"})],
        )

    def test_missing_marker_comment_is_created_once(self) -> None:
        api, client = self.api()

        api.upsert_comment(123, "new")

        self.assertEqual(client.patches, [])
        self.assertEqual(len(client.posts), 1)
        self.assertEqual(
            client.posts[0][0], "/repos/futex-ai/firna-apps/issues/123/comments"
        )

    def test_existing_named_check_is_updated_in_place(self) -> None:
        api, client = self.api()
        preview_result = result()
        client.check_rows = [
            {
                "id": 84,
                "external_id": preview_result.correlation_id,
                "app": {"slug": "github-actions"},
            }
        ]

        api.upsert_check(preview_result, "ready", "summary")

        self.assertEqual(client.posts, [])
        self.assertEqual(len(client.patches), 1)
        path, payload = client.patches[0]
        self.assertEqual(path, "/repos/futex-ai/firna-apps/check-runs/84")
        self.assertEqual(payload["name"], CHECK_NAME)
        self.assertEqual(payload["external_id"], preview_result.correlation_id)

    def test_same_named_check_from_another_app_is_not_updated(self) -> None:
        api, client = self.api()
        preview_result = result()
        client.check_rows = [
            {
                "id": 7,
                "external_id": preview_result.correlation_id,
                "app": {"slug": "another-app"},
            },
            {
                "id": 84,
                "external_id": preview_result.correlation_id,
                "app": {"slug": "github-actions"},
            },
        ]

        api.upsert_check(preview_result, "ready", "summary")

        self.assertEqual(client.posts, [])
        self.assertEqual(client.patches[0][0], "/repos/futex-ai/firna-apps/check-runs/84")


if __name__ == "__main__":
    unittest.main()
