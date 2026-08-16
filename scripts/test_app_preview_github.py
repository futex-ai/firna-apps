"""Tests for the minimal app-preview GitHub REST client."""

import io
import json
import unittest
import urllib.error
from unittest.mock import patch

from app_preview_github import GitHubApiError, RestClient


class FakeResponse:
    def __init__(self, status: int, body: bytes):
        self.status = status
        self.body = body

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        return False

    def read(self) -> bytes:
        return self.body


class RestClientTests(unittest.TestCase):
    @patch("urllib.request.urlopen")
    def test_json_request_uses_fixed_headers_and_timeout(self, urlopen) -> None:
        urlopen.return_value = FakeResponse(201, b'{"id": 1}')
        client = RestClient("https://api.github.com/", "secret-token")

        result = client.post("/repos/o/r/check-runs", {"name": "preview"}, {201})

        self.assertEqual(result, {"id": 1})
        request = urlopen.call_args.args[0]
        self.assertEqual(request.full_url, "https://api.github.com/repos/o/r/check-runs")
        self.assertEqual(request.get_header("Authorization"), "Bearer secret-token")
        self.assertEqual(json.loads(request.data), {"name": "preview"})
        self.assertEqual(urlopen.call_args.kwargs["timeout"], 30)

    @patch("urllib.request.urlopen")
    def test_http_failure_never_includes_response_body_or_token(self, urlopen) -> None:
        urlopen.side_effect = urllib.error.HTTPError(
            "https://api.github.com/repos/o/r",
            403,
            "forbidden",
            {},
            io.BytesIO(b"secret response body"),
        )
        client = RestClient("https://api.github.com", "secret-token")

        with self.assertRaises(GitHubApiError) as raised:
            client.get("/repos/o/r")

        message = str(raised.exception)
        self.assertIn("HTTP 403", message)
        self.assertNotIn("secret response body", message)
        self.assertNotIn("secret-token", message)

    def test_relative_api_path_is_rejected_before_network_access(self) -> None:
        client = RestClient("https://api.github.com", "secret-token")

        with self.assertRaisesRegex(GitHubApiError, "must start with /"):
            client.get("repos/o/r")


if __name__ == "__main__":
    unittest.main()
