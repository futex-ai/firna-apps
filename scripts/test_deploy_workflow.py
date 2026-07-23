"""Tests for the deployment workflow assertion script."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("test-deploy-workflow.sh")
REPOSITORY_ROOT = SCRIPT.parent.parent


class DeployWorkflowScriptTests(unittest.TestCase):
    def test_runs_without_ripgrep(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            command_directory = Path(directory)
            for command in ("bash", "dirname", "grep"):
                executable = shutil.which(command)
                self.assertIsNotNone(executable)
                (command_directory / command).symlink_to(executable)

            environment = os.environ.copy()
            environment["PATH"] = str(command_directory)
            result = subprocess.run(
                [str(command_directory / "bash"), str(SCRIPT)],
                cwd=REPOSITORY_ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
