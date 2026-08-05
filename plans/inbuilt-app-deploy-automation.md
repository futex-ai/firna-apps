# Inbuilt App Deploy Automation

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-05
- Spec: [`docs/protocol/app-deployment.md`](../docs/protocol/app-deployment.md)

## Outcome

Adding or changing a first-party app requires exactly one `firna-apps` pull
request. A dedicated app-secrets Google Cloud project, owned by this
repository's Terraform, replaces the platform-owned secret containers; a
merge gate blocks any PR whose declared secrets lack values; and this
repository deploys `production` and `br-main` directly on merge, on a daily
schedule, and on a platform poke. The platform repository stops enumerating
app ids and app secrets entirely; its only remaining app duty is seeding
ephemeral `pr-N` previews from a `firna-apps@main` checkout, driven by this
repository's `deploy.toml`.

Human actions that automation cannot perform are called out as operator
steps: creating the Google Cloud project, linking billing, and pasting
provider secret values. Everything else is code in this plan or the
coordinated platform change.

## Current Constraints

- Prod app submission already auto-discovers `apps/*/manifest.*`
  (`deploy-apps.yml` selection step) and diffs against the live catalog with
  `scripts/plan-app-deploys.py`; it stalls only on missing secret
  containers/values.
- Preview seeding is platform-owned and fail-closed on the hardcoded
  `FIRNA_PREVIEW_APP_IDS: dataforseo,exa,http,slack` in firna's
  `preview-deploy.yml` and `deploy-api.yml`, enforced by
  `scripts/deploy_preview_apps.py` + `scripts/preview_app_deploy_support.py`.
- Platform Terraform (`infra/gcp/prod/main.tf`) provisions app containers
  from the static `app_provider_keys` list (exa + slack only); the prod X
  container `firna-prod-app-x-client-secret` was created out-of-band on
  2026-08-04 (see `plans/x-api-access.md`). The External Secrets Operator
  grant includes an unused `-app-` prefix.
- Existing secret values to copy (never move; values stay valid in both
  places until platform cleanup):
  - `firna-prod-app-exa-api-key`, `firna-prod-app-slack-client-secret`,
    `firna-prod-app-slack-signing-secret`, `firna-prod-app-x-client-secret`;
  - `firna-preview-test-runtime-exa-api-key`,
    `firna-preview-test-runtime-slack-client-secret`,
    `firna-preview-test-runtime-slack-signing-secret`.
  - `dataforseo` and `http` declare no deployment secrets; `x` has no
    preview values because its X developer app registers only the
    production callback (`apps/x/deploy.toml` will declare
    `classes = ["production"]`).
- IAM conditions can prefix-scope reads of existing secrets but cannot
  prefix-scope `secretmanager.secrets.create`; container creation is safe to
  grant only inside a project that contains nothing but app secrets. Secret
  Manager conditions must match the project-number resource-name form.
- `production` and `br-main` servers both track `firna@main` while this
  repository pins its CLI via `platform.toml`; pinned-CLI-vs-main-server
  compatibility is one shared invariant, fixed by bumping `platform.toml`.
  `pr-N` previews run unmerged platform code, so their seeding must keep
  using the PR-built CLI inside the platform repository.
- Admin logins: production uses `vars.FIRNA_BOOTSTRAP_USERNAME` with
  `firna-prod-runtime-firna-bootstrap-password`; previews use
  `preview-admin` with
  `firna-preview-test-runtime-firna-bootstrap-password`. Both password
  secrets stay in platform project `firna-498513`.
- `cargo xtask check` runs `scripts/test_*.py`, `test-deploy-workflow.sh`,
  `repository_audit.py`, and actionlint over both workflows, so workflow and
  script changes need their sibling tests updated in the same milestone.

## Milestone 1: Deployment Protocol and Plan

Write the target contract and this plan. Documentation-only: validate the
Markdown and review the diff; `cargo xtask check` is not required.

- [x] Write `docs/protocol/app-deployment.md` covering ownership boundary,
      environments model, `deploy.toml` schemas, app-secrets project,
      merge gate, deployment workflow, platform integration, and migration.
- [x] Write this plan and register it in `plans/README.md`.
- [x] Link the protocol doc from the repository `README.md`.
- [x] Validate changed Markdown, review the diff against `origin/main`,
      commit with Conventional Commits, and push.

## Milestone 2: App-Secrets Project and Deploy Config

Stand up the dedicated project and the repository deployment configuration.
Nothing consumes them yet, so live deployment behavior is unchanged and the
repository gate stays green.

- [x] Operator: create the app-secrets Google Cloud project, link billing,
      and create Terraform state bucket
      `gs://<project-id>-terraform-state`; record the chosen project id.
      Project `firna-apps` (number 712421637485) created 2026-08-05. The
      billing account's 5-project link quota was freed by deleting the
      empty projects `terminal-498309` and `bowser-503012` (no enabled
      compute/SQL/run APIs, no buckets; user-approved 2026-08-05), then
      billing was linked and `gs://firna-apps-terraform-state` created.
- [x] Add root `deploy.toml` exactly as specified in the protocol doc, with
      the recorded project id.
- [x] Add `apps/x/deploy.toml` with `classes = ["production"]`.
- [x] Add `infra/gcp/apps/` Terraform (`versions.tf`, `variables.tf`,
      `main.tf`, `outputs.tf`, `README.md`) defining: WIF pool `github` with
      provider `github-firna-apps` restricted to `futex-ai/firna-apps`;
      service account `apps-ci` (any-ref binding) with
      `roles/secretmanager.viewer` plus custom role `appSecretCreator`
      (`secretmanager.secrets.create` only); service account `apps-deploy`
      (`refs/heads/main` binding) with `roles/secretmanager.secretAccessor`;
      and the cross-project `preview-app-` conditional accessor grant for
      `github-firna-preview@firna-498513` using both project-id and
      project-number name forms.
- [x] Operator: `terraform init && terraform validate && terraform fmt
      -check && terraform apply` in `infra/gcp/apps/`. Applied 2026-08-05
      (16 resources); the three `APPS_*` GitHub repository variables were
      set from the outputs.
- [x] Add `scripts/deploy_config.py`: parse and validate root and per-app
      `deploy.toml` (known instances, exact URLs, prefix/class pairing,
      non-empty known `classes`), expose app-to-class targeting; add
      `scripts/test_deploy_config.py` unit tests for valid, missing,
      malformed, unknown-class, and unknown-key cases.
- [x] Extend `scripts/repository_audit.py` to require a valid root
      `deploy.toml` and validate any per-app `deploy.toml` via
      `deploy_config`; extend `scripts/test_repository_audit.py`.
- [x] Exempt changes touching only `apps/<app_id>/deploy.toml` from the
      audit's manifest version-bump requirement, with tests, and record the
      exemption in the protocol doc (discovered during implementation:
      targeting is repository metadata, not package content).
- [x] Operator: copy the seven existing secret values listed in Current
      Constraints into the new containers
      (`gcloud secrets versions access latest --project=firna-498513
      --secret=<old> | gcloud secrets versions add <new>
      --project=<apps-project> --data-file=-`), creating containers with
      `gcloud secrets create <new> --project=<apps-project>` first; verify
      with `gcloud secrets versions list`. Copied 2026-08-05; all seven
      containers hold enabled version 1 with `app` labels.
- [x] Run the full gate: `cargo xtask check`.
- [x] Update `README.md` and `infra/gcp/apps/README.md` for the new
      directory; keep the Deployment section describing live behavior.
- [x] `git add -A`, Conventional Commit, push.
- [x] Run `cargo xtask review`; report findings with recommendations, do not
      auto-fix. Ran 2026-08-05: two P2 findings (Terraform IAM resources
      missing `depends_on` API enablement; `deploy_config` duplicate check
      can raise `TypeError` on non-string class entries) reported to the
      user for a fix decision.

## Milestone 2.1: Review Fixes

Fix the two P2 findings from the Milestone 2 `cargo xtask review` run. The
next milestone's review covers the same cumulative branch diff, so this
milestone ends at commit and push.

- [x] Add `depends_on = [google_project_service.required]` to the
      `apps_ci`/`apps_deploy` service accounts and the `appSecretCreator`
      custom role in `infra/gcp/apps/main.tf` so a fresh-project bootstrap
      cannot race API enablement; run `terraform fmt -check`, `validate`,
      and a no-op `apply`.
- [x] In `scripts/deploy_config.py` `app_classes`, return the collected
      failures before the duplicate-entry check when any class entry fails
      type validation, so malformed-but-valid TOML such as
      `classes = [["production"]]` produces validation failures instead of
      a `TypeError` crash; add a nested-list regression test.
- [x] `cargo xtask check`; `git add -A`, Conventional Commit, push.

## Milestone 3: Merge Gate

Block merging any app change whose declared secrets lack values. Deployment
paths are untouched.

- [x] Add `scripts/check_app_secrets.py`: read `apps/*/manifest.yaml`
      `secrets[].name` and per-app classes via `deploy_config`; for each
      targeted class and secret, ensure container
      `<prefix>-<app_id>-<kebab>` exists (create with label `app=<app_id>`
      when missing) and has an enabled latest version (metadata only, no
      value reads); on failure list every missing value with its exact
      `gcloud secrets versions add` remediation command and exit non-zero.
- [x] Add `scripts/test_check_app_secrets.py` with a mocked command runner:
      all-present, missing-container-created, missing-value-fails,
      production-only app skips preview, secretless app passes.
- [x] Add `ci.yml` job `app-secrets` (pull requests from this repository and
      `main` pushes only; `id-token: write`): authenticate with
      `google-github-actions/auth` against repository variables
      `APPS_GCP_WORKLOAD_IDENTITY_PROVIDER` and `APPS_CI_SERVICE_ACCOUNT`,
      then run `python3 scripts/check_app_secrets.py`.
- [x] Operator: set those two repository variables from Terraform outputs
      (set 2026-08-05 during Milestone 2).
- [ ] Verify actionlint still passes via `cargo xtask check`; open a
      scratch PR touching `apps/exa` to watch the job pass, and confirm the
      failure message by temporarily pointing the script at a bogus secret
      name locally (not committed).
- [ ] Update `README.md` (Development section: the gate and how to provision
      a new secret) and `docs/protocol/app-deployment.md` status notes if
      behavior details shifted.
- [ ] `cargo xtask check`; `git add -A`, Conventional Commit, push.
- [ ] Run `cargo xtask review`; report findings with recommendations, do not
      auto-fix.

## Milestone 4: Platform Enabling Grants

Coordinated change in `futex-ai/firna` (separate workspace and PR following
that repository's conventions). Smallest possible enabling step so Milestone
5 can cut over; no platform behavior changes.

- [ ] Platform Terraform: grant `apps-deploy@<apps-project>` per-secret
      `roles/secretmanager.secretAccessor` on exactly
      `firna-prod-runtime-firna-bootstrap-password` and
      `firna-preview-test-runtime-firna-bootstrap-password`.
- [ ] Update platform `docs/deployment/apps.md` to record the new identity
      and its scope.
- [ ] Land through the platform repository's own checks and review.

## Milestone 5: Multi-Environment Deployment Cutover

`deploy-apps.yml` deploys `production` and `br-main` from `deploy.toml`,
reading app secrets from the app-secrets project. The platform's own
br-main seeding keeps running until Milestone 6; both paths are
version-compare idempotent, so duplicate submission is harmless.

- [x] Restructure `deploy-apps.yml`: a `prepare` job selects candidate apps
      and emits the instance matrix via new `scripts/deploy_matrix.py`; a
      `deploy` matrix job per instance authenticates as `apps-deploy`,
      reads that instance's bootstrap password from `firna-498513`, logs in
      with the instance `admin_email`, filters candidate apps to the
      instance class via new `scripts/select_instance_apps.py`, plans with
      `scripts/plan-app-deploys.py`, and submits with secrets from
      `<secret_prefix>-<app_id>-<secret-kebab>` in the apps project.
- [x] Enforce fail-closed rules in script code, not workflow YAML: the
      matrix and class filter derive only from the validated `deploy.toml`
      (exact URL and prefix/class pinning in `deploy_config`), and empty
      secret values still fail the submit step.
- [x] Use GitHub environment `production` for the production instance and
      new unprotected environment `br-main` (created 2026-08-05); instances
      run under one serialized `firna-apps-deploy` concurrency group with
      `fail-fast: false` so instances deploy independently.
- [x] Add triggers: `schedule` (`0 6 * * *`) and `repository_dispatch`
      types `[firna-platform-deployed]`; extend `workflow_dispatch` with an
      optional `instance` input.
- [x] `scripts/plan-app-deploys.py` stays catalog-diff-only; added
      `scripts/test_deploy_matrix.py` and
      `scripts/test_select_instance_apps.py`, and updated
      `scripts/test-deploy-workflow.sh` assertions (requires matrix, APPS_*
      variables, new triggers; rejects every legacy single-instance
      variable).
- [x] Operator: set `APPS_DEPLOY_SERVICE_ACCOUNT` repository variable (done
      2026-08-05 in Milestone 2); confirmed `vars.FIRNA_BOOTSTRAP_USERNAME`
      is `admin`, matching `deploy.toml`, and retired the variable from the
      workflow.
- [ ] Smoke: verify both instances deploy from the new workflow (idempotent
      no-op or missing-version submit), verify both admin catalogs and the
      public `/apps/catalog` on each instance, and verify `x` is absent
      from br-main.
- [x] Retire now-unused repository variables/env
      (`GCP_SERVICE_ACCOUNT`, `GCP_WORKLOAD_IDENTITY_PROVIDER`,
      `FIRNA_SECRET_MANAGER_PREFIX`, `GCP_PROJECT_ID`,
      `FIRNA_SERVER_URL` single-instance env) from `deploy-apps.yml`; the
      GitHub repository variables themselves are deleted in Milestone 7
      after the platform retires the legacy identity.
- [x] Update `README.md` Deployment section to the new behavior.
- [ ] `cargo xtask check`; `git add -A`, Conventional Commit, push.
- [ ] Run `cargo xtask review`; report findings with recommendations, do not
      auto-fix.

## Milestone 6: Platform Cleanup and pr-N Config-Driven Seeding

Coordinated change in `futex-ai/firna` (separate workspace and PR following
that repository's conventions). Removes every platform-side app enumeration.
Deletions below are pre-approved by this plan.

- [ ] `preview-deploy.yml`: drop `FIRNA_PREVIEW_APP_IDS`; pass the apps
      GCP project id; `deploy_preview_apps.py` +
      `preview_app_deploy_support.py` derive the app list from the
      `firna-apps` checkout (`deploy.toml` `preview` class) and read
      `preview-app-<app_id>-<secret-kebab>` from the apps project; keep the
      fail-closed `pr-N`/`br-main` URL and prefix checks; update
      `scripts/test_deploy_preview_apps.py`,
      `scripts/test_preview_deployment.py`, and
      `scripts/test-preview-deploy-workflow.sh`.
- [ ] `deploy-api.yml`: delete the stable-main app seeding steps (br-main
      is deployed by `firna-apps` since Milestone 5); add an optional
      post-deploy `repository_dispatch` poke `firna-platform-deployed` to
      `futex-ai/firna-apps` using a fine-grained token secret
      `FIRNA_APPS_DISPATCH_TOKEN`; drop `FIRNA_PREVIEW_APP_IDS`.
- [ ] Platform Terraform: remove `app_provider_keys` and the app-provider
      entries (`EXA_API_KEY`, `SLACK_CLIENT_SECRET`, `SLACK_SIGNING_SECRET`)
      from `preview_test_runtime_secret_keys`; remove the `-app-` prefix
      from the External Secrets Operator and GitHub-actions IAM condition
      expressions; retire the platform-owned deploy identity and WIF
      provider previously dedicated to `firna-apps`; delete the legacy
      containers `firna-prod-app-*` and the three app-provider
      `firna-preview-test-runtime-*` secrets after confirming the new
      containers hold enabled values.
- [ ] Rewrite platform `docs/deployment/apps.md` against
      `docs/protocol/app-deployment.md` in this repository.
- [ ] Smoke: run a labelled `pr-N` preview and confirm `dataforseo`, `exa`,
      `http`, and `slack` seed (and `x` does not) with no allowlist
      variables anywhere; run a platform deploy and confirm the poke
      triggers `deploy-apps.yml` here.
- [ ] Land through the platform repository's own checks and review.

## Milestone 7: Completion

- [ ] Flip `docs/protocol/app-deployment.md` status to implemented; sweep it
      and `README.md` for any drift against landed behavior.
- [ ] Confirm no reference to `FIRNA_PREVIEW_APP_IDS`, `firna-prod-app-`, or
      platform-owned app identities remains in either repository except
      historical plans.
- [ ] Run the full gate: `cargo xtask check`.
- [ ] Move this plan to Completed in `plans/README.md`.
- [ ] `git add -A`, Conventional Commit, push.
- [ ] Run `cargo xtask review`; report findings with recommendations, do not
      auto-fix.

## Completion Criteria

- A new secretless app merged to `main` here appears in production and
  br-main with zero operator or platform actions, and in the next `pr-N`
  preview with zero platform edits.
- A new app with secrets is blocked at PR time until an operator pastes
  values using commands printed by CI, and then deploys everywhere
  automatically.
- The platform repository contains no app ids, app secret names, or app
  allowlists; its Terraform provisions no app-provider containers.
- Pull-request CI holds no credential that can read secret values.
- `production` and `br-main` recover from environment resets within one day
  via the scheduled run, or immediately via the platform poke or a manual
  dispatch.
