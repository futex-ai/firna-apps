#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${FIRNA_REPOSITORY_TOKEN:-}" ]]; then
  printf 'FIRNA_REPOSITORY_TOKEN is required\n' >&2
  exit 1
fi
if [[ -z "${RUNNER_TEMP:-}" ]]; then
  printf 'RUNNER_TEMP is required\n' >&2
  exit 1
fi
if [[ -z "${GITHUB_ENV:-}" ]]; then
  printf 'GITHUB_ENV is required\n' >&2
  exit 1
fi

credential_directory="${RUNNER_TEMP}/firna-repository-credentials"
credential_file="${credential_directory}/git-credentials"

install -d -m 700 "$credential_directory"
install -m 600 /dev/null "$credential_file"
printf 'https://x-access-token:%s@github.com/futex-ai/firna.git\n' \
  "$FIRNA_REPOSITORY_TOKEN" \
  > "$credential_file"

git config --global --replace-all \
  credential.helper \
  "store --file=${credential_file}"
git config --global --replace-all credential.useHttpPath true
printf 'CARGO_NET_GIT_FETCH_WITH_CLI=true\n' >> "$GITHUB_ENV"
