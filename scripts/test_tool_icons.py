"""Repository-level regressions for optional command artwork."""

import unittest
from pathlib import Path

import app_icons


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


class ToolIconRepositoryTests(unittest.TestCase):
    def test_slack_declares_distinct_overrides_and_package_defaults(self) -> None:
        icons = dict(
            app_icons.declared_tool_icons(REPOSITORY_ROOT / "apps/slack/manifest.yaml")
        )

        self.assertEqual(set(icons), {"slack_search_messages", "slack_send_message"})
        self.assertNotEqual(
            icons["slack_send_message"]["data_base64"],
            icons["slack_search_messages"]["data_base64"],
        )


if __name__ == "__main__":
    unittest.main()
