#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ci_workflow="$repo_root/.github/workflows/ci.yml"
deploy_workflow="$repo_root/.github/workflows/deploy-apps.yml"
preview_request_workflow="$repo_root/.github/workflows/app-preview-request.yml"
preview_result_workflow="$repo_root/.github/workflows/app-preview-result.yml"

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

require_line "$ci_workflow" '  pull_request:'
require_line "$ci_workflow" '  push:'
require_line "$ci_workflow" '        run: cargo xtask check'
require_line "$ci_workflow" '          fetch-depth: 0'
require_line "$ci_workflow" 'tool: wasm-tools@${{ env.WASM_TOOLS_VERSION }}'
require_line "$ci_workflow" 'FIRNA_REPOSITORY_TOKEN: ${{ secrets.FIRNA_REPOSITORY_TOKEN }}'
require_line "$ci_workflow" 'run: bash scripts/configure-firna-repository-access.sh'

require_line "$deploy_workflow" '  workflow_run:'
require_line "$deploy_workflow" '      - CI'
require_line "$deploy_workflow" '  schedule:'
require_line "$deploy_workflow" '  repository_dispatch:'
require_line "$deploy_workflow" '      - firna-platform-deployed'
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
require_line "$deploy_workflow" 'python3 scripts/deploy_matrix.py --instance "${MANUAL_INSTANCE:-all}"'
require_line "$deploy_workflow" 'python3 scripts/select_instance_apps.py \'
require_line "$deploy_workflow" 'matrix: ${{ fromJSON(needs.prepare.outputs.matrix) }}'
require_line "$deploy_workflow" 'fail-fast: false'
require_line "$deploy_workflow" 'environment: ${{ matrix.instance }}'
require_line "$deploy_workflow" 'workload_identity_provider: ${{ vars.APPS_GCP_WORKLOAD_IDENTITY_PROVIDER }}'
require_line "$deploy_workflow" 'service_account: ${{ vars.APPS_DEPLOY_SERVICE_ACCOUNT }}'
require_line "$deploy_workflow" 'firna config set-server "${{ matrix.api_url }}"'
require_line "$deploy_workflow" 'firna admin apps catalog --json'
require_line "$deploy_workflow" 'python3 scripts/plan-app-deploys.py \'
require_line "$deploy_workflow" 'firna admin apps submit "$app_dir" "${submit_args[@]}"'
require_line "$deploy_workflow" 'gcloud secrets versions access latest \'
require_line "$deploy_workflow" 'echo "::add-mask::$(escape_github_command "$value")"'
require_line "$deploy_workflow" 'submit_args+=(--secret-env "${secret_name}=${env_name}")'
require_line "$deploy_workflow" 'secret_id="${{ matrix.secret_prefix }}-${manifest_id}-${secret_name//_/-}"'

reject_line "$deploy_workflow" 'firna apps submit'
reject_line "$deploy_workflow" 'pull_request_target:'
reject_line "$deploy_workflow" '--secret-value'
reject_line "$deploy_workflow" 'cargo run -p fna-cli'
reject_line "$deploy_workflow" 'vars.GCP_SERVICE_ACCOUNT'
reject_line "$deploy_workflow" 'vars.GCP_WORKLOAD_IDENTITY_PROVIDER'
reject_line "$deploy_workflow" 'FIRNA_SECRET_MANAGER_PREFIX'
reject_line "$deploy_workflow" 'FIRNA_SERVER_URL:'
reject_line "$deploy_workflow" 'FIRNA_BOOTSTRAP_USERNAME'

require_line "$preview_request_workflow" '  workflow_run:'
require_line "$preview_request_workflow" '  pull_request_target:'
require_line "$preview_request_workflow" '      - labeled'
require_line "$preview_request_workflow" '      - unlabeled'
require_line "$preview_request_workflow" '      - closed'
require_line "$preview_request_workflow" '      - reopened'
require_line "$preview_request_workflow" '      - edited'
reject_line "$preview_request_workflow" '    branches:'
require_line "$preview_request_workflow" '  actions: read'
require_line "$preview_request_workflow" '  pull-requests: read'
require_line "$preview_request_workflow" '  cancel-in-progress: false'
require_line "$preview_request_workflow" '          ref: main'
require_line "$preview_request_workflow" '          persist-credentials: false'
require_line "$preview_request_workflow" 'FIRNA_PLATFORM_PREVIEW_DISPATCH_TOKEN: ${{ secrets.FIRNA_PLATFORM_PREVIEW_DISPATCH_TOKEN }}'
require_line "$preview_request_workflow" 'run: python3 scripts/app_preview_controller.py'
reject_line "$preview_request_workflow" 'github.event.pull_request.head.ref'
reject_line "$preview_request_workflow" 'github.head_ref'
reject_line "$preview_request_workflow" 'firna apps'
reject_line "$preview_request_workflow" 'cargo '

require_line "$preview_result_workflow" '  repository_dispatch:'
require_line "$preview_result_workflow" '      - firna-app-preview-result'
require_line "$preview_result_workflow" '  checks: write'
require_line "$preview_result_workflow" '  issues: write'
require_line "$preview_result_workflow" '  pull-requests: write'
require_line "$preview_result_workflow" '  cancel-in-progress: false'
require_line "$preview_result_workflow" '          ref: main'
require_line "$preview_result_workflow" '          persist-credentials: false'
require_line "$preview_result_workflow" 'run: python3 scripts/app_preview_feedback.py'
reject_line "$preview_result_workflow" 'FIRNA_PLATFORM_PREVIEW_DISPATCH_TOKEN'

printf 'deploy_workflow=ok\n'
