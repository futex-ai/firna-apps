#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ci_workflow="$repo_root/.github/workflows/ci.yml"
deploy_workflow="$repo_root/.github/workflows/deploy-apps.yml"

require_line() {
  local file="$1"
  local pattern="$2"
  if ! grep -Fq -- "$pattern" "$file"; then
    printf 'workflow missing in %s: %s\n' "$file" "$pattern" >&2
    exit 1
  fi
}

reject_line() {
  local file="$1"
  local pattern="$2"
  if grep -Fq -- "$pattern" "$file"; then
    printf 'unexpected workflow content in %s: %s\n' "$file" "$pattern" >&2
    exit 1
  fi
}

line_number() {
  local file="$1"
  local pattern="$2"
  local match
  while IFS= read -r match; do
    printf '%s' "${match%%:*}"
    return
  done < <(grep -nF -- "$pattern" "$file")
  return 1
}

require_line "$ci_workflow" '  pull_request:'
require_line "$ci_workflow" '  push:'
require_line "$ci_workflow" '        run: cargo xtask check'
require_line "$ci_workflow" '          fetch-depth: 0'
require_line "$ci_workflow" 'tool: wasm-tools@${{ env.WASM_TOOLS_VERSION }}'
require_line "$ci_workflow" 'FIRNA_REPOSITORY_TOKEN: ${{ secrets.FIRNA_REPOSITORY_TOKEN }}'
require_line "$ci_workflow" 'run: bash scripts/configure-firna-repository-access.sh'

require_line "$deploy_workflow" '  workflow_run:'
require_line "$deploy_workflow" '      - CI'
require_line "$deploy_workflow" "github.event.workflow_run.conclusion == 'success'"
require_line "$deploy_workflow" "github.event.workflow_run.head_branch == 'main'"
require_line "$deploy_workflow" 'github.event.workflow_run.head_repository.full_name == github.repository'
require_line "$deploy_workflow" "github.ref == 'refs/heads/main'"
require_line "$deploy_workflow" '  id-token: write'
require_line "$deploy_workflow" '          fetch-depth: 0'
require_line "$deploy_workflow" 'FIRNA_REPOSITORY_TOKEN: ${{ secrets.FIRNA_REPOSITORY_TOKEN }}'
require_line "$deploy_workflow" 'run: bash scripts/configure-firna-repository-access.sh'
require_line "$deploy_workflow" '--git "${FIRNA_PLATFORM_REPOSITORY}"'
require_line "$deploy_workflow" '--rev "${FIRNA_PLATFORM_REVISION}"'
require_line "$deploy_workflow" 'firna admin apps catalog --json'
require_line "$deploy_workflow" 'python3 scripts/plan-app-deploys.py \'
require_line "$deploy_workflow" '      - name: Validate planned app deployment readiness'
require_line "$deploy_workflow" 'bash scripts/check-app-deploy-readiness.sh "$app_dir"'
require_line "$deploy_workflow" 'firna admin apps submit "$app_dir" "${submit_args[@]}"'
require_line "$deploy_workflow" 'gcloud secrets versions access latest \'
require_line "$deploy_workflow" 'echo "::add-mask::$(escape_github_command "$value")"'
require_line "$deploy_workflow" 'submit_args+=(--secret-env "${secret_name}=${env_name}")'
require_line "$deploy_workflow" 'environment: production'
require_line "$repo_root/apps/github/manifest.yaml" '- name: client_secret'
require_line "$repo_root/apps/github/manifest.yaml" '- name: private_key'

reject_line "$deploy_workflow" 'firna apps submit'
reject_line "$deploy_workflow" 'pull_request_target:'
reject_line "$deploy_workflow" '--secret-value'
reject_line "$deploy_workflow" 'cargo run -p fna-cli'
reject_line "$repo_root/apps/github/manifest.yaml" '- name: client_id'

readiness_step_line="$(line_number "$deploy_workflow" '      - name: Validate planned app deployment readiness')"
readiness_command_line="$(line_number "$deploy_workflow" 'bash scripts/check-app-deploy-readiness.sh "$app_dir"')"
readiness_command_count="$(grep -cF -- 'bash scripts/check-app-deploy-readiness.sh "$app_dir"' "$deploy_workflow")"
submit_step_line="$(line_number "$deploy_workflow" '      - name: Submit through trusted admin auto-approval')"
if [ "$readiness_command_count" -ne 1 ] \
  || [ "$readiness_step_line" -ge "$readiness_command_line" ] \
  || [ "$readiness_command_line" -ge "$submit_step_line" ]; then
  printf 'deployment readiness must run before app submission\n' >&2
  exit 1
fi

printf 'deploy_workflow=ok\n'
