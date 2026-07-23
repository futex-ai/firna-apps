"""Tests for private Firna repository access configuration."""

import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("configure-firna-repository-access.sh")
REPOSITORY_ROOT = SCRIPT.parent.parent


class ConfigureFirnaRepositoryAccessTests(unittest.TestCase):
    def test_configures_scoped_ssh_access_without_logging_the_key(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary_root = Path(directory)
            runner_temp = temporary_root / "runner"
            fake_bin = temporary_root / "bin"
            github_env = temporary_root / "github-env"
            git_log = temporary_root / "git-log"
            fake_bin.mkdir()
            fake_git = fake_bin / "git"
            fake_git.write_text(
                "#!/usr/bin/env bash\n"
                'printf "%s\\0" "$@" >> "$FAKE_GIT_LOG"\n',
                encoding="utf-8",
            )
            fake_git.chmod(0o755)

            repository_token = "private-repository-token-fixture"
            environment = os.environ.copy()
            environment.update(
                {
                    "FAKE_GIT_LOG": str(git_log),
                    "FIRNA_REPOSITORY_TOKEN": repository_token,
                    "GITHUB_ENV": str(github_env),
                    "PATH": f"{fake_bin}:{environment['PATH']}",
                    "RUNNER_TEMP": str(runner_temp),
                }
            )
            result = subprocess.run(
                ["bash", str(SCRIPT)],
                cwd=REPOSITORY_ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn(repository_token, result.stdout)
            self.assertNotIn(repository_token, result.stderr)
            credential_file = (
                runner_temp
                / "firna-repository-credentials"
                / "git-credentials"
            )
            self.assertEqual(
                credential_file.read_text(encoding="utf-8"),
                (
                    "https://x-access-token:"
                    f"{repository_token}@github.com/futex-ai/firna.git\n"
                ),
            )
            self.assertEqual(
                stat.S_IMODE(credential_file.stat().st_mode),
                0o600,
            )
            self.assertEqual(
                github_env.read_text(encoding="utf-8"),
                "CARGO_NET_GIT_FETCH_WITH_CLI=true\n",
            )
            git_arguments = git_log.read_bytes().split(b"\0")
            self.assertIn(b"credential.helper", git_arguments)
            self.assertIn(b"credential.useHttpPath", git_arguments)

    def test_credentials_are_scoped_to_the_firna_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary_root = Path(directory)
            environment = os.environ.copy()
            repository_token = "private-repository-token-fixture"
            environment.update(
                {
                    "FIRNA_REPOSITORY_TOKEN": repository_token,
                    "GITHUB_ENV": str(temporary_root / "github-env"),
                    "GIT_CONFIG_GLOBAL": str(temporary_root / "gitconfig"),
                    "HOME": str(temporary_root),
                    "RUNNER_TEMP": str(temporary_root / "runner"),
                }
            )
            configured = subprocess.run(
                ["bash", str(SCRIPT)],
                cwd=REPOSITORY_ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(configured.returncode, 0, configured.stderr)

            firna_credential = self._fill_credential(
                environment,
                "futex-ai/firna.git",
            )
            self.assertEqual(firna_credential.returncode, 0)
            self.assertIn(f"password={repository_token}", firna_credential.stdout)

            other_credential = self._fill_credential(
                environment,
                "futex-ai/other.git",
            )
            self.assertNotIn(repository_token, other_credential.stdout)
            self.assertNotIn(repository_token, other_credential.stderr)

    def test_rejects_a_missing_deploy_key(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temporary_root = Path(directory)
            environment = os.environ.copy()
            environment.pop("FIRNA_REPOSITORY_TOKEN", None)
            environment["GITHUB_ENV"] = str(temporary_root / "github-env")
            environment["RUNNER_TEMP"] = str(temporary_root / "runner")

            result = subprocess.run(
                ["bash", str(SCRIPT)],
                cwd=REPOSITORY_ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("FIRNA_REPOSITORY_TOKEN is required", result.stderr)

    @staticmethod
    def _fill_credential(
        environment: dict[str, str],
        path: str,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", "-c", "credential.interactive=never", "credential", "fill"],
            env=environment,
            input=f"protocol=https\nhost=github.com\npath={path}\n\n",
            check=False,
            capture_output=True,
            text=True,
        )


if __name__ == "__main__":
    unittest.main()
