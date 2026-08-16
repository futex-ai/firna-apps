"""Tests for the closed app-preview request and result contracts."""

import unittest

from app_preview_contract import (
    API_URL,
    PRODUCT_URL,
    ContractError,
    FailureCode,
    PreviewAction,
    PreviewStatus,
    correlation_id,
    parse_request,
    parse_result,
)


HEAD_SHA = "1" * 40
PLATFORM_SHA = "2" * 40
RUN_URL = "https://github.com/futex-ai/firna/actions/runs/123"


def request_payload() -> dict[str, object]:
    return {
        "schema_version": 1,
        "action": "deploy",
        "source_repository": "futex-ai/firna-apps",
        "pr_number": 123,
        "head_sha": HEAD_SHA,
        "correlation_id": correlation_id(123, HEAD_SHA),
    }


def result_payload(status: str = "ready") -> dict[str, object]:
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
        payload["product_url"] = PRODUCT_URL
        payload["api_url"] = API_URL
        payload["owner_pr_number"] = 123
    elif status == "busy":
        payload["owner_pr_number"] = 456
    elif status == "failed":
        payload["failure_code"] = "smoke_failed"
    return payload


class RequestContractTests(unittest.TestCase):
    def test_valid_request_parses_and_round_trips(self) -> None:
        request = parse_request(request_payload())

        self.assertEqual(request.action, PreviewAction.DEPLOY)
        self.assertEqual(request.payload(), request_payload())

    def test_release_is_accepted(self) -> None:
        payload = request_payload()
        payload["action"] = "release"

        self.assertEqual(parse_request(payload).action, PreviewAction.RELEASE)

    def test_unknown_field_is_rejected(self) -> None:
        payload = request_payload()
        payload["branch"] = "feature"

        with self.assertRaisesRegex(ContractError, "unknown field"):
            parse_request(payload)

    def test_unknown_version_action_repository_and_sha_are_rejected(self) -> None:
        invalid = (
            ("schema_version", 2, "schema_version"),
            ("action", "replace", "unknown value"),
            ("source_repository", "fork/firna-apps", "source_repository"),
            ("head_sha", "main", "40-character commit SHA"),
            ("pr_number", True, "positive integer"),
        )
        for field, value, message in invalid:
            with self.subTest(field=field):
                payload = request_payload()
                payload[field] = value
                with self.assertRaisesRegex(ContractError, message):
                    parse_request(payload)

    def test_correlation_must_match_exact_identity(self) -> None:
        payload = request_payload()
        payload["correlation_id"] = correlation_id(124, HEAD_SHA)

        with self.assertRaisesRegex(ContractError, "correlation_id does not match"):
            parse_request(payload)


class ResultContractTests(unittest.TestCase):
    def test_every_valid_terminal_status_parses(self) -> None:
        for status in PreviewStatus:
            with self.subTest(status=status.value):
                result = parse_result(result_payload(status.value))
                self.assertEqual(result.status, status)

    def test_unknown_status_failure_code_and_field_are_rejected(self) -> None:
        payload = result_payload()
        payload["status"] = "pending"
        with self.assertRaisesRegex(ContractError, "unknown value"):
            parse_result(payload)

        payload = result_payload("failed")
        payload["failure_code"] = "broken"
        with self.assertRaisesRegex(ContractError, "unknown value"):
            parse_result(payload)

        payload = result_payload()
        payload["extra"] = None
        with self.assertRaisesRegex(ContractError, "unknown field"):
            parse_result(payload)

    def test_ready_requires_fixed_urls_platform_sha_and_matching_owner(self) -> None:
        invalid = (
            ("product_url", "https://example.com"),
            ("api_url", None),
            ("platform_sha", None),
            ("owner_pr_number", 456),
        )
        for field, value in invalid:
            with self.subTest(field=field):
                payload = result_payload()
                payload[field] = value
                with self.assertRaises(ContractError):
                    parse_result(payload)

    def test_non_ready_rejects_environment_urls(self) -> None:
        payload = result_payload("released")
        payload["product_url"] = PRODUCT_URL

        with self.assertRaisesRegex(ContractError, "only allowed for ready"):
            parse_result(payload)

    def test_busy_requires_a_different_owner(self) -> None:
        for owner in (None, 123):
            with self.subTest(owner=owner):
                payload = result_payload("busy")
                payload["owner_pr_number"] = owner
                with self.assertRaisesRegex(ContractError, "different owner"):
                    parse_result(payload)

    def test_failure_code_is_required_only_for_failed(self) -> None:
        payload = result_payload("failed")
        payload["failure_code"] = None
        with self.assertRaisesRegex(ContractError, "failed requires"):
            parse_result(payload)

        payload = result_payload("released")
        payload["failure_code"] = FailureCode.STALE_REQUEST.value
        with self.assertRaisesRegex(ContractError, "only allowed for failed"):
            parse_result(payload)

    def test_run_url_is_restricted_to_the_platform_actions_run(self) -> None:
        payload = result_payload()
        payload["run_url"] = "https://example.com/futex-ai/firna/actions/runs/123"

        with self.assertRaisesRegex(ContractError, "GitHub Actions run"):
            parse_result(payload)


if __name__ == "__main__":
    unittest.main()
